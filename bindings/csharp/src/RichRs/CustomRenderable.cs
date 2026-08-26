using System.Runtime.CompilerServices;
using System.Runtime.ExceptionServices;
using System.Runtime.InteropServices;

namespace RichRs;

/// <summary>Supplies synchronous native segments for a reusable renderable.</summary>
public interface ICustomRenderer
{
    /// <summary>Stages every segment for one render pass.</summary>
    void Render(RenderOptions options, SegmentSink sink);

    /// <summary>Reports the renderable's width bounds.</summary>
    Measurement Measure(RenderOptions options) => new(1, options.MaxWidth);
}

/// <summary>
/// A thread-bound segment sink whose lifetime is limited to one custom-render callback.
/// Retaining this object is permitted only so that later use can fail locally and safely.
/// </summary>
public sealed class SegmentSink
{
    private readonly SegmentSinkState state;

    internal SegmentSink(SegmentSinkState state) => this.state = state;

    /// <summary>Stages copied UTF-8 text with an optional native style.</summary>
    public void Push(string text, string style = "")
    {
        state.Validate();
        byte[] encodedText = Utf8.Encode(text);
        byte[] encodedStyle = Utf8.Encode(style);
        state.Validate();
        uint result = Utf8.Map(encodedText, nativeText => Utf8.Map(encodedStyle, nativeStyle =>
            NativeMethods.SegmentSinkPush(state.Pointer, nativeText, nativeStyle)));
        NativeResult.Check(result);
    }

    /// <summary>Stages a native line break.</summary>
    public void Line()
    {
        state.Validate();
        NativeResult.Check(NativeMethods.SegmentSinkLine(state.Pointer));
    }
}

internal sealed class SegmentSinkState
{
    private readonly int threadId = Environment.CurrentManagedThreadId;
    private nint pointer;

    internal SegmentSinkState(nint pointer) => this.pointer = pointer;

    internal nint Pointer => pointer;

    internal void Validate()
    {
        if (pointer == 0 || threadId != Environment.CurrentManagedThreadId || !CallbackGuard.IsCurrent(this))
        {
            throw new InvalidOperationException("The segment sink is no longer active on this callback thread.");
        }
    }

    internal void Tombstone() => pointer = 0;
}

/// <summary>Creates callback-backed reusable renderables.</summary>
public static unsafe class CustomRenderable
{
    // Focused allocation-failure injection for tests; null in production.
    internal static Action? BeforeNativeCreateForTest { get; set; }

    /// <summary>
    /// Creates a renderable and transfers a managed renderer context to its last native Arc.
    /// </summary>
    public static Renderable Create(ICustomRenderer renderer)
    {
        ArgumentNullException.ThrowIfNull(renderer);
        CallbackGuard.ThrowIfActive();
        RichRuntime.EnsureAbi();

        GCHandle context = default;
        bool transferred = false;
        try
        {
            context = GCHandle.Alloc(new CustomRendererContext(renderer));
            BeforeNativeCreateForTest?.Invoke();
            Renderable result = Renderable.Create(output =>
            {
                uint nativeResult = CreateNative(GCHandle.ToIntPtr(context), output);
                transferred = nativeResult == NativeResult.Ok;
                return nativeResult;
            });
            return result;
        }
        catch
        {
            if (!transferred && context.IsAllocated)
            {
                context.Free();
            }
            throw;
        }
    }

    private static uint CreateNative(nint context, nint* output)
    {
        NativeRichCustomRenderableVTable vtable = new()
        {
            Render = &RenderCallback,
            Measure = &MeasureCallback,
            Drop = &DropCallback,
        };
        return NativeMethods.CustomRenderableNew(context, &vtable, output);
    }

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    private static uint RenderCallback(
        nint context, NativeRichRenderOptions* nativeOptions, nint nativeSink)
    {
        SegmentSinkState? sinkState = null;
        bool entered = false;
        try
        {
            if (context == 0 || nativeOptions == null || nativeSink == 0)
            {
                return (uint)RichErrorCode.NullArgument;
            }

            sinkState = new(nativeSink);
            CallbackGuard.Enter(sinkState);
            entered = true;
            CustomRendererContext renderer = GetContext(context);
            renderer.Renderer.Render(CopyOptions(nativeOptions), new SegmentSink(sinkState));
            return NativeResult.Ok;
        }
        catch (Exception error)
        {
            CallbackGuard.Capture(error);
            return ResultFor(error);
        }
        finally
        {
            sinkState?.Tombstone();
            if (entered)
            {
                CallbackGuard.Exit();
            }
        }
    }

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    private static uint MeasureCallback(
        nint context, NativeRichRenderOptions* nativeOptions, NativeRichMeasurement* output)
    {
        bool entered = false;
        try
        {
            if (output != null)
            {
                *output = default;
            }
            if (context == 0 || nativeOptions == null || output == null)
            {
                return (uint)RichErrorCode.NullArgument;
            }

            CallbackGuard.Enter(null);
            entered = true;
            Measurement measurement = GetContext(context).Renderer.Measure(CopyOptions(nativeOptions));
            output->Minimum = measurement.Minimum;
            output->Maximum = measurement.Maximum;
            return NativeResult.Ok;
        }
        catch (Exception error)
        {
            CallbackGuard.Capture(error);
            return ResultFor(error);
        }
        finally
        {
            if (entered)
            {
                CallbackGuard.Exit();
            }
        }
    }

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    private static void DropCallback(nint context)
    {
        bool entered = false;
        try
        {
            if (context == 0)
            {
                return;
            }
            CallbackGuard.Enter(null);
            entered = true;
            GCHandle handle = GCHandle.FromIntPtr(context);
            if (handle.IsAllocated)
            {
                handle.Free();
            }
        }
        catch
        {
            // Reverse P/Invoke frames must never observe a managed exception.
        }
        finally
        {
            if (entered)
            {
                CallbackGuard.Exit();
            }
        }
    }

    private static CustomRendererContext GetContext(nint context)
    {
        object? target = GCHandle.FromIntPtr(context).Target;
        return target as CustomRendererContext
            ?? throw new InvalidOperationException("The custom renderer context is unavailable.");
    }

    private static RenderOptions CopyOptions(NativeRichRenderOptions* options) => new(
        options->MinWidth,
        options->MaxWidth,
        options->MaxHeight,
        (ColorSystem)options->ColorSystem,
        options->IsTerminal != 0);

    private static uint ResultFor(Exception error)
    {
        if (error is RichException rich && rich.Code >= RichErrorCode.NullArgument && rich.Code <= RichErrorCode.NativePanic)
        {
            return (uint)rich.Code;
        }
        return (uint)RichErrorCode.Render;
    }
}

internal sealed class CustomRendererContext
{
    internal CustomRendererContext(ICustomRenderer renderer) => Renderer = renderer;
    internal ICustomRenderer Renderer { get; }
}

internal static class CallbackGuard
{
    [ThreadStatic]
    private static bool active;
    [ThreadStatic]
    private static SegmentSinkState? currentSink;
    [ThreadStatic]
    private static ExceptionDispatchInfo? capturedException;
    [ThreadStatic]
    private static bool outerRender;

    internal static void ThrowIfActive()
    {
        if (active)
        {
            throw new InvalidOperationException(
                "rich-rs native calls other than the active SegmentSink are prohibited during callbacks.");
        }
    }

    internal static void Enter(SegmentSinkState? sink)
    {
        if (active)
        {
            throw new InvalidOperationException("Nested managed callback entry is prohibited.");
        }
        active = true;
        currentSink = sink;
    }

    internal static bool IsCurrent(SegmentSinkState sink) => active && ReferenceEquals(currentSink, sink);

    internal static void Exit()
    {
        currentSink = null;
        active = false;
    }

    internal static void BeginOuterRender()
    {
        if (outerRender)
        {
            throw new InvalidOperationException("Nested managed render entry is prohibited.");
        }
        outerRender = true;
        capturedException = null;
    }

    internal static void EndOuterRender()
    {
        capturedException = null;
        outerRender = false;
    }

    internal static void Capture(Exception error)
    {
        capturedException ??= ExceptionDispatchInfo.Capture(error);
    }

    internal static void ThrowCapturedException()
    {
        capturedException?.Throw();
    }
}
