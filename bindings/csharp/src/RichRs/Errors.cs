namespace RichRs;

/// <summary>Stable error categories returned by the rich-rs native ABI.</summary>
public enum RichErrorCode : uint
{
    /// <summary>A required native argument was null.</summary>
    NullArgument = 1,
    /// <summary>Input bytes were not valid UTF-8.</summary>
    InvalidUtf8 = 2,
    /// <summary>An argument was outside the native domain.</summary>
    InvalidArgument = 3,
    /// <summary>A builder, task, or node was not in a valid state.</summary>
    InvalidState = 4,
    /// <summary>A native I/O operation failed.</summary>
    Io = 5,
    /// <summary>Rendering failed.</summary>
    Render = 6,
    /// <summary>A Rust panic was contained at the ABI boundary.</summary>
    NativePanic = 7,
    /// <summary>The native ABI returned a value that this package cannot understand.</summary>
    AbiMismatch = uint.MaxValue,
}
/// <summary>An error reported by the rich-rs native library.</summary>
public sealed class RichException : Exception
{
    /// <summary>Creates a rich-rs exception.</summary>
    public RichException(RichErrorCode code, string message)
        : base(message)
    {
        Code = code;
    }

    /// <summary>The stable native error category.</summary>
    public RichErrorCode Code { get; }
}

internal static class NativeResult
{
    internal const uint Ok = 0;

    internal static void Check(uint result)
    {
        if (result == Ok)
        {
            return;
        }

        RichErrorCode code = result switch
        {
            1 => RichErrorCode.NullArgument,
            2 => RichErrorCode.InvalidUtf8,
            3 => RichErrorCode.InvalidArgument,
            4 => RichErrorCode.InvalidState,
            5 => RichErrorCode.Io,
            6 => RichErrorCode.Render,
            7 => RichErrorCode.NativePanic,
            _ => RichErrorCode.AbiMismatch,
        };
        throw new RichException(code, code == RichErrorCode.AbiMismatch
            ? $"The native rich-rs library returned unknown result code {result}."
            : $"The native rich-rs call failed with {code} (code {result}).");
    }
}
