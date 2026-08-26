namespace RichRs;

/// <summary>Native runtime metadata and ABI validation.</summary>
public static unsafe class RichRuntime
{
    /// <summary>The compiled native ABI version.</summary>
    public const uint AbiVersion = 2;
    private static readonly object Sync = new();
    private static volatile bool validated;

    /// <summary>Validates the loaded native library ABI.</summary>
    public static void EnsureAbi()
    {
        CallbackGuard.ThrowIfActive();
        if (validated)
        {
            return;
        }
        lock (Sync)
        {
            if (validated)
            {
                return;
            }
            uint actual = NativeMethods.FfiAbiVersion();
            if (actual != AbiVersion)
            {
                throw new RichException(RichErrorCode.AbiMismatch,
                    $"rich-rs ABI mismatch: managed package expects {AbiVersion}, native library reports {actual}.");
            }
            validated = true;
        }
    }

    /// <summary>The native package version, copied from native-owned UTF-8.</summary>
    public static string Version
    {
        get
        {
            EnsureAbi();
            return NativeBuffer.Call(NativeMethods.FfiVersion);
        }
    }
}

/// <summary>An owned reusable native renderable.</summary>
public sealed class Renderable : IDisposable
{
    // Focused managed-allocation failure injection for tests; null in production.
    internal static Action? AfterNativeCreateForTest { get; set; }
    private readonly RenderableHandle handle;
    private readonly NativeOwner<RenderableHandle> owner;

    private Renderable()
    {
        handle = new();
        owner = new(handle);
    }

    internal static unsafe Renderable Finish<THandle>(NativeOwner<THandle> builder, FinishCall finish)
        where THandle : NativeSafeHandle
    {
        Renderable result = new();
        try
        {
            builder.Finish(finish, result.handle);
            return result;
        }
        catch
        {
            result.Dispose();
            throw;
        }
    }

    internal static Renderable Create(NewHandleCall create)
    {
        Renderable result = new();
        try
        {
            NativeFactory.Initialize(result.handle, create);
            AfterNativeCreateForTest?.Invoke();
            return result;
        }
        catch
        {
            result.Dispose();
            throw;
        }
    }

    internal RenderableHandle Handle => owner.DangerousHandle;
    internal T Borrow<T>(Func<nint, T> call) => owner.Use(call);
    internal void Borrow(Action<nint> call) => owner.Use(call);

    /// <summary>Releases this managed reference to the native renderable.</summary>
    public void Dispose() => owner.Dispose();
}

/// <summary>A capture console used to render markup and renderables.</summary>
public sealed unsafe class RichConsole : IDisposable
{
    private readonly NativeOwner<ConsoleHandle> owner;

    /// <summary>Creates a console.</summary>
    public unsafe RichConsole()
    {
        RichRuntime.EnsureAbi();
        owner = new(NativeFactory.Create(new ConsoleHandle(), NativeMethods.ConsoleNew));
    }

    internal T Borrow<T>(Func<nint, T> call) => owner.Use(call);
    internal void Borrow(Action<nint> call) => owner.Use(call);

    /// <summary>Gets or sets forced terminal behavior.</summary>
    public bool ForceTerminal
    {
        set => owner.Use(pointer => NativeResult.Check(NativeMethods.ConsoleSetForceTerminal(pointer, NativeFactory.Bool(value))));
    }

    /// <summary>Gets or sets the native color system.</summary>
    public ColorSystem ColorSystem
    {
        set => owner.Use(pointer => NativeResult.Check(NativeMethods.ConsoleSetColorSystem(pointer, (uint)value)));
    }

    /// <summary>Enables or disables markup parsing.</summary>
    public bool MarkupEnabled
    {
        set => owner.Use(pointer => NativeResult.Check(NativeMethods.ConsoleSetMarkupEnabled(pointer, NativeFactory.Bool(value))));
    }

    /// <summary>Enables or disables emoji replacement.</summary>
    public bool EmojiEnabled
    {
        set => owner.Use(pointer => NativeResult.Check(NativeMethods.ConsoleSetEmojiEnabled(pointer, NativeFactory.Bool(value))));
    }

    /// <summary>Gets the configured console width.</summary>
    public uint Width => owner.Use(pointer => NativeScalar.Uint(output => NativeMethods.ConsoleWidth(pointer, output)));

    /// <summary>Sets console dimensions.</summary>
    public void SetSize(uint width, uint height) =>
        owner.Use(pointer => NativeResult.Check(NativeMethods.ConsoleSetSize(pointer, width, height)));

    /// <summary>Renders length-delimited markup without appending a newline.</summary>
    public string RenderMarkup(string markup) => owner.Use(pointer =>
        Utf8.Map(markup, native => NativeBuffer.Call(output => NativeMethods.ConsoleRenderMarkup(pointer, native, output))));

    /// <summary>Renders a borrowed reusable renderable.</summary>
    public string Render(Renderable renderable)
    {
        ArgumentNullException.ThrowIfNull(renderable);
        CallbackGuard.ThrowIfActive();
        CallbackGuard.BeginOuterRender();
        try
        {
            return owner.Use(console => renderable.Borrow(content =>
                NativeBuffer.Call(output => NativeMethods.ConsoleRender(console, content, output))));
        }
        catch
        {
            CallbackGuard.ThrowCapturedException();
            throw;
        }
        finally
        {
            CallbackGuard.EndOuterRender();
        }
    }

    /// <summary>Releases the native console.</summary>
    public void Dispose() => owner.Dispose();
}
