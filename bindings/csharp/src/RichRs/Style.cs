namespace RichRs;

/// <summary>An owned validated native style.</summary>
public sealed class Style : IDisposable
{
    private readonly NativeOwner<StyleHandle> owner;

    private Style(StyleHandle handle) => owner = new(handle);

    /// <summary>Parses and validates a style expression.</summary>
    public static unsafe Style Parse(string style)
    {
        byte[] encoded = Utf8.Encode(style);
        RichRuntime.EnsureAbi();
        StyleHandle handle = Utf8.Map(encoded, native => NativeFactory.Create(
            new StyleHandle(), output => NativeMethods.StyleParse(native, output)));
        return new(handle);
    }

    /// <summary>Releases the native style.</summary>
    public void Dispose() => owner.Dispose();
}
