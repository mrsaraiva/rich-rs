using System.Runtime.InteropServices;
using System.Text;

namespace RichRs;

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct NativeRichStr
{
    internal byte* Data;
    internal ulong Length;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct NativeRichBuffer
{
    internal byte* Data;
    internal ulong Length;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct NativeRichRenderOptions
{
    internal uint MinWidth;
    internal uint MaxWidth;
    internal uint MaxHeight;
    internal uint ColorSystem;
    internal byte IsTerminal;
    internal fixed byte Reserved[7];
}

[StructLayout(LayoutKind.Sequential)]
internal struct NativeRichMeasurement
{
    internal uint Minimum;
    internal uint Maximum;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct NativeRichCustomRenderableVTable
{
    internal delegate* unmanaged[Cdecl]<nint, NativeRichRenderOptions*, nint, uint> Render;
    internal delegate* unmanaged[Cdecl]<nint, NativeRichRenderOptions*, NativeRichMeasurement*, uint> Measure;
    internal delegate* unmanaged[Cdecl]<nint, void> Drop;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct NativeRichTableColumnOptions
{
    internal uint Flags;
    internal uint Justify;
    internal uint Vertical;
    internal uint Width;
    internal uint MinWidth;
    internal uint Ratio;
    internal NativeRichStr Style;
}

internal static class NativeColumnFlags
{
    internal const uint HasJustify = 1U << 0;
    internal const uint HasVertical = 1U << 1;
    internal const uint HasWidth = 1U << 2;
    internal const uint HasMinimumWidth = 1U << 3;
    internal const uint HasRatio = 1U << 4;
    internal const uint NoWrap = 1U << 5;
}

internal unsafe delegate uint Utf8Call(NativeRichStr value);
internal unsafe delegate T Utf8Func<T>(NativeRichStr value);
internal unsafe delegate uint RichStrArrayCall(NativeRichStr* values, uint count);
internal unsafe delegate uint FinishCall(nint* input, nint* output);
internal unsafe delegate uint BufferCall(NativeRichBuffer* output);
internal unsafe delegate uint NewHandleCall(nint* output);
internal unsafe delegate uint UlongOutputCall(ulong* output);
internal unsafe delegate uint UintOutputCall(uint* output);

internal static class NativeFactory
{
    internal static THandle Create<THandle>(THandle handle, NewHandleCall create)
        where THandle : NativeSafeHandle
    {
        Initialize(handle, create);
        return handle;
    }

    internal static unsafe void Initialize(NativeSafeHandle handle, NewHandleCall create)
    {
        CallbackGuard.ThrowIfActive();
        nint output = 0;
        try
        {
            uint result = create(&output);
            if (output != 0)
            {
                handle.Initialize(output);
            }
            NativeResult.Check(result);
            if (output == 0)
            {
                throw new RichException(RichErrorCode.AbiMismatch, "Native constructor returned a null handle.");
            }
        }
        catch
        {
            if (output != 0 && handle.IsInvalid)
            {
                handle.Initialize(output);
            }
            handle.Dispose();
            throw;
        }
    }

    internal static byte Bool(bool value) => value ? (byte)1 : (byte)0;
}

internal static class Utf8
{
    internal static readonly Encoding Strict = Encoding.GetEncoding(
        Encoding.UTF8.CodePage,
        EncoderFallback.ExceptionFallback,
        DecoderFallback.ExceptionFallback);

    internal static byte[] Encode(string value)
    {
        ArgumentNullException.ThrowIfNull(value);
        return Strict.GetBytes(value);
    }

    internal static unsafe uint With(string value, Utf8Call call)
    {
        ArgumentNullException.ThrowIfNull(value);
        byte[] bytes = Strict.GetBytes(value);
        fixed (byte* data = bytes)
        {
            NativeRichStr native = new()
            {
                Data = bytes.Length == 0 ? null : data,
                Length = checked((ulong)bytes.LongLength),
            };
            return call(native);
        }
    }

    internal static unsafe T Map<T>(string value, Utf8Func<T> call)
    {
        byte[] bytes = Encode(value);
        return Map(bytes, call);
    }

    internal static unsafe T Map<T>(byte[] bytes, Utf8Func<T> call)
    {
        ArgumentNullException.ThrowIfNull(bytes);
        fixed (byte* data = bytes)
        {
            NativeRichStr native = new()
            {
                Data = bytes.Length == 0 ? null : data,
                Length = checked((ulong)bytes.LongLength),
            };
            return call(native);
        }
    }

    internal static unsafe uint WithArray(IReadOnlyList<string> values, RichStrArrayCall call)
    {
        string[] snapshot = CollectionSnapshot.Copy(values);
        byte[][] bytes = new byte[snapshot.Length][];
        GCHandle[] pins = new GCHandle[snapshot.Length];
        NativeRichStr[] native = new NativeRichStr[snapshot.Length];
        try
        {
            for (int i = 0; i < snapshot.Length; i++)
            {
                ArgumentNullException.ThrowIfNull(snapshot[i]);
                bytes[i] = Strict.GetBytes(snapshot[i]);
                if (bytes[i].Length != 0)
                {
                    pins[i] = GCHandle.Alloc(bytes[i], GCHandleType.Pinned);
                    native[i].Data = (byte*)pins[i].AddrOfPinnedObject();
                }
                native[i].Length = checked((ulong)bytes[i].LongLength);
            }

            fixed (NativeRichStr* pointer = native)
            {
                return call(snapshot.Length == 0 ? null : pointer, checked((uint)snapshot.Length));
            }
        }
        finally
        {
            foreach (GCHandle pin in pins)
            {
                if (pin.IsAllocated)
                {
                    pin.Free();
                }
            }
        }
    }
}

internal static class CollectionSnapshot
{
    internal static T[] Copy<T>(IEnumerable<T> values)
    {
        ArgumentNullException.ThrowIfNull(values);
        List<T> snapshot = [];
        foreach (T value in values)
        {
            snapshot.Add(value);
        }
        return snapshot.ToArray();
    }
}

internal static class NativeScalar
{
    internal static unsafe ulong Ulong(UlongOutputCall call)
    {
        ulong output = 0;
        NativeResult.Check(call(&output));
        return output;
    }

    internal static unsafe uint Uint(UintOutputCall call)
    {
        uint output = 0;
        NativeResult.Check(call(&output));
        return output;
    }
}

internal static class NativeBuffer
{
    // Focused failure injection for tests; null in production.
    internal static Action? BeforeDecodeForTest { get; set; }
    internal static Action<nint, ulong>? AfterFreeForTest { get; set; }
    internal static Func<uint, uint>? ResultOverrideForTest { get; set; }

    internal static unsafe string Call(BufferCall call)
    {
        CallbackGuard.ThrowIfActive();
        NativeRichBuffer buffer = default;
        try
        {
            uint result = call(&buffer);
            result = ResultOverrideForTest?.Invoke(result) ?? result;
            NativeResult.Check(result);
            if ((buffer.Data == null) != (buffer.Length == 0))
            {
                throw new RichException(RichErrorCode.AbiMismatch, "Native RichBuffer pointer/length invariant was violated.");
            }
            if (buffer.Length > int.MaxValue)
            {
                throw new RichException(RichErrorCode.AbiMismatch, "Native RichBuffer is too large for a managed string.");
            }

            BeforeDecodeForTest?.Invoke();
            return buffer.Length == 0
                ? string.Empty
                : Utf8.Strict.GetString(new ReadOnlySpan<byte>(buffer.Data, checked((int)buffer.Length)));
        }
        finally
        {
            NativeMethods.BufferFree(&buffer);
            AfterFreeForTest?.Invoke((nint)buffer.Data, buffer.Length);
        }
    }
}

internal sealed class NativeHandleGroup : IDisposable
{
    private readonly NativeSafeHandle[] handles;
    private readonly bool[] added;

    internal NativeHandleGroup(NativeSafeHandle[] source)
    {
        handles = (NativeSafeHandle[])source.Clone();
        added = new bool[handles.Length];
        Pointers = new nint[handles.Length];
        try
        {
            for (int i = 0; i < handles.Length; i++)
            {
                bool success = false;
                handles[i].DangerousAddRef(ref success);
                added[i] = success;
                if (!success || handles[i].IsInvalid || handles[i].IsClosed)
                {
                    throw new ObjectDisposedException("Renderable");
                }
                Pointers[i] = handles[i].DangerousGetHandle();
            }
        }
        catch
        {
            Dispose();
            throw;
        }
    }

    internal nint[] Pointers { get; }

    public void Dispose()
    {
        for (int i = added.Length - 1; i >= 0; i--)
        {
            if (added[i])
            {
                handles[i].DangerousRelease();
                added[i] = false;
            }
        }
    }
}

internal sealed class NativeOwner<THandle> : IDisposable where THandle : NativeSafeHandle
{
    private readonly object sync = new();
    private THandle handle;

    internal NativeOwner(THandle handle)
    {
        this.handle = handle;
    }

    internal T Use<T>(Func<nint, T> call)
    {
        CallbackGuard.ThrowIfActive();
        lock (sync)
        {
            ThrowIfDisposed();
            bool added = false;
            try
            {
                handle.DangerousAddRef(ref added);
                if (!added || handle.IsInvalid)
                {
                    throw new ObjectDisposedException(typeof(THandle).Name);
                }
                return call(handle.DangerousGetHandle());
            }
            finally
            {
                if (added)
                {
                    handle.DangerousRelease();
                }
            }
        }
    }

    internal void Use(Action<nint> call) => Use(pointer =>
    {
        call(pointer);
        return 0;
    });

    internal THandle DangerousHandle
    {
        get
        {
            CallbackGuard.ThrowIfActive();
            lock (sync)
            {
                ThrowIfDisposed();
                return handle;
            }
        }
    }

    internal unsafe void Finish(FinishCall finish, NativeSafeHandle outputHandle)
    {
        CallbackGuard.ThrowIfActive();
        lock (sync)
        {
            ThrowIfDisposed();
            bool added = false;
            try
            {
                handle.DangerousAddRef(ref added);
                nint original = handle.DangerousGetHandle();
                nint input = original;
                nint output = 0;
                uint result = finish(&input, &output);
                if (result != NativeResult.Ok)
                {
                    ReconcileReturnedPointers(original, input, output, outputHandle);
                    NativeResult.Check(result);
                }

                if (input != 0 || output == 0)
                {
                    ReconcileReturnedPointers(original, input, output, outputHandle);
                    throw new RichException(RichErrorCode.AbiMismatch, "Native finish violated its pointer transfer contract.");
                }

                handle.MarkConsumed();
                outputHandle.Initialize(output);
            }
            finally
            {
                if (added)
                {
                    handle.DangerousRelease();
                }
            }
        }
    }

    private void ReconcileReturnedPointers(nint original, nint input, nint output, NativeSafeHandle outputHandle)
    {
        if (input == 0)
        {
            handle.MarkConsumed();
        }
        else if (input != original)
        {
            handle.CleanupReplacement(input);
        }

        CleanupOutputUnlessAliased(original, input, output, outputHandle);
    }

    private static void CleanupOutputUnlessAliased(nint original, nint input, nint output, NativeSafeHandle outputHandle)
    {
        if (output == 0 || (output == original && input != 0) || output == input)
        {
            return;
        }
        outputHandle.CleanupReplacement(output);
    }

    internal void ThrowIfDisposed()
    {
        if (handle.IsClosed || handle.IsInvalid)
        {
            throw new ObjectDisposedException(typeof(THandle).Name);
        }
    }

    public void Dispose()
    {
        CallbackGuard.ThrowIfActive();
        lock (sync)
        {
            handle.Dispose();
        }
    }
}

/// <summary>Native color capability.</summary>
public enum ColorSystem : uint
{
    /// <summary>No color.</summary>
    None = 0,
    /// <summary>Standard 16-color output.</summary>
    Standard = 1,
    /// <summary>256-color output.</summary>
    EightBit = 2,
    /// <summary>24-bit color output.</summary>
    TrueColor = 3,
    /// <summary>Windows-compatible color output.</summary>
    Windows = 4,
}
/// <summary>Horizontal alignment.</summary>
public enum HorizontalAlignment : uint
{
    /// <summary>Left alignment.</summary>
    Left = 0,
    /// <summary>Center alignment.</summary>
    Center = 1,
    /// <summary>Right alignment.</summary>
    Right = 2,
}
/// <summary>Vertical alignment.</summary>
public enum VerticalAlignment : uint
{
    /// <summary>Top alignment.</summary>
    Top = 0,
    /// <summary>Middle alignment.</summary>
    Middle = 1,
    /// <summary>Bottom alignment.</summary>
    Bottom = 2,
}
/// <summary>Text justification.</summary>
public enum Justify : uint
{
    /// <summary>Native default justification.</summary>
    Default = 0,
    /// <summary>Left justification.</summary>
    Left = 1,
    /// <summary>Center justification.</summary>
    Center = 2,
    /// <summary>Right justification.</summary>
    Right = 3,
    /// <summary>Full justification.</summary>
    Full = 4,
}
/// <summary>Table or panel border style.</summary>
public enum Box : uint
{
    /// <summary>Rounded border.</summary>
    Rounded = 0,
    /// <summary>Heavy border.</summary>
    Heavy = 1,
    /// <summary>Double border.</summary>
    Double = 2,
    /// <summary>ASCII border.</summary>
    Ascii = 3,
    /// <summary>Minimal border.</summary>
    Minimal = 4,
    /// <summary>Square border.</summary>
    Square = 5,
    /// <summary>Simple border.</summary>
    Simple = 6,
    /// <summary>Heavy heading border.</summary>
    HeavyHead = 7,
    /// <summary>Heavy outer edge.</summary>
    HeavyEdge = 8,
    /// <summary>Double outer edge.</summary>
    DoubleEdge = 9,
    /// <summary>Horizontal lines only.</summary>
    Horizontals = 10,
    /// <summary>Minimal border with heavy heading.</summary>
    MinimalHeavyHead = 11,
    /// <summary>Simple heavy border.</summary>
    SimpleHeavy = 12,
    /// <summary>Markdown-compatible border.</summary>
    Markdown = 13,
    /// <summary>No table border or divider lines. Panels reject this value.</summary>
    None = 14,
}

/// <summary>Atomic options for one table column.</summary>
public readonly record struct ColumnOptions(
    Justify? Justification = null,
    VerticalAlignment? Vertical = null,
    uint? Width = null,
    uint? MinimumWidth = null,
    uint? Ratio = null,
    string Style = "",
    bool NoWrap = false)
{
    /// <summary>Creates native default column options.</summary>
    public ColumnOptions()
        : this(null, null, null, null, null, "", false)
    {
    }
}

/// <summary>Immutable rendering constraints supplied to a custom renderer.</summary>
public readonly record struct RenderOptions(
    uint MinWidth,
    uint MaxWidth,
    uint MaxHeight,
    ColorSystem ColorSystem,
    bool IsTerminal);

/// <summary>Immutable minimum and maximum width reported by a custom renderer.</summary>
public readonly record struct Measurement(uint Minimum, uint Maximum);

/// <summary>Options for JSON rendering.</summary>
public readonly record struct JsonOptions(uint Indent = 2, bool Highlight = true, bool SortKeys = false)
{
    /// <summary>Creates the native defaults: indent 2, highlighting on, and original key order.</summary>
    public JsonOptions()
        : this(2, true, false)
    {
    }
}

/// <summary>A stable tree node identifier.</summary>
public readonly record struct NodeId(ulong Value)
{
    /// <summary>The tree root.</summary>
    public static NodeId Root => new(0);
    /// <summary>Implicit conversion to the ABI scalar.</summary>
    public static implicit operator ulong(NodeId value) => value.Value;
}
