namespace RichRs;

/// <summary>A mutable text builder consumed by <see cref="Finish"/>.</summary>
public sealed class Text : IDisposable
{
    private readonly NativeOwner<TextHandle> owner;

    /// <summary>Creates plain text, preserving embedded NUL.</summary>
    public unsafe Text(string text)
    {
        byte[] encoded = Utf8.Encode(text);
        RichRuntime.EnsureAbi();
        owner = new(Utf8.Map(encoded, native => NativeFactory.Create(
            new TextHandle(), output => NativeMethods.TextNew(native, output))));
    }

    private Text(TextHandle handle) => owner = new(handle);

    /// <summary>Creates text by parsing markup with a console.</summary>
    public static unsafe Text FromMarkup(RichConsole console, string markup)
    {
        ArgumentNullException.ThrowIfNull(console);
        byte[] encoded = Utf8.Encode(markup);
        RichRuntime.EnsureAbi();
        TextHandle handle = console.Borrow(consolePointer => Utf8.Map(encoded, native => NativeFactory.Create(
            new TextHandle(), output => NativeMethods.TextNewMarkup(consolePointer, native, output))));
        return new(handle);
    }

    /// <summary>Sets a native style expression.</summary>
    public void SetStyle(string style) => owner.Use(pointer =>
        NativeResult.Check(Utf8.With(style, native => NativeMethods.TextSetStyle(pointer, native))));

    /// <summary>Consumes this builder and returns a reusable renderable.</summary>
    public unsafe Renderable Finish() => Renderable.Finish(owner, NativeMethods.TextFinish);

    /// <summary>Releases an unfinished text builder.</summary>
    public void Dispose() => owner.Dispose();
}
