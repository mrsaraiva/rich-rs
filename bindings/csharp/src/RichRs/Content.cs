namespace RichRs;

/// <summary>A syntax highlighting builder.</summary>
public sealed unsafe class Syntax : IDisposable
{
    private readonly NativeOwner<SyntaxHandle> owner;

    /// <summary>Creates syntax highlighting for source text and a lexer name.</summary>
    public unsafe Syntax(string code, string lexer)
    {
        byte[] encodedCode = Utf8.Encode(code);
        byte[] encodedLexer = Utf8.Encode(lexer);
        RichRuntime.EnsureAbi();
        owner = new(Utf8.Map(encodedCode, nativeCode => Utf8.Map(encodedLexer, nativeLexer => NativeFactory.Create(
            new SyntaxHandle(), output => NativeMethods.SyntaxNew(nativeCode, nativeLexer, output)))));
    }

    private Syntax(SyntaxHandle handle) => owner = new(handle);

    /// <summary>Reads source code from a native filesystem path.</summary>
    public static unsafe Syntax FromPath(string path)
    {
        byte[] encoded = Utf8.Encode(path);
        RichRuntime.EnsureAbi();
        SyntaxHandle handle = Utf8.Map(encoded, native => NativeFactory.Create(
            new SyntaxHandle(), output => NativeMethods.SyntaxFromPath(native, output)));
        return new(handle);
    }

    /// <summary>Sets the syntax theme name.</summary>
    public void SetTheme(string theme) => owner.Use(pointer =>
        NativeResult.Check(Utf8.With(theme, native => NativeMethods.SyntaxSetTheme(pointer, native))));
    /// <summary>Enables or disables line numbers.</summary>
    public bool LineNumbers { set => owner.Use(pointer => NativeResult.Check(NativeMethods.SyntaxSetLineNumbers(pointer, NativeFactory.Bool(value)))); }
    /// <summary>Enables or disables word wrapping.</summary>
    public bool WordWrap { set => owner.Use(pointer => NativeResult.Check(NativeMethods.SyntaxSetWordWrap(pointer, NativeFactory.Bool(value)))); }
    /// <summary>Enables or disables native indentation guides.</summary>
    public void SetIndentGuides(bool enabled) => owner.Use(pointer => NativeResult.Check(
        NativeMethods.SyntaxSetIndentGuides(pointer, NativeFactory.Bool(enabled))));
    /// <summary>Sets optional inclusive line range endpoints.</summary>
    public void SetLineRange(uint? start, uint? end) => owner.Use(pointer => NativeResult.Check(
        NativeMethods.SyntaxSetLineRange(pointer, NativeFactory.Bool(start.HasValue), start.GetValueOrDefault(),
            NativeFactory.Bool(end.HasValue), end.GetValueOrDefault())));
    /// <summary>Consumes this syntax builder.</summary>
    public unsafe Renderable Finish() => Renderable.Finish(owner, NativeMethods.SyntaxFinish);
    /// <summary>Releases an unfinished syntax builder.</summary>
    public void Dispose() => owner.Dispose();
}

/// <summary>A native Debug/repr highlighter builder.</summary>
public sealed unsafe class Pretty : IDisposable
{
    private readonly NativeOwner<PrettyHandle> owner;

    /// <summary>Creates a Pretty builder from a copied, length-delimited representation.</summary>
    public Pretty(string debugRepresentation)
    {
        byte[] encoded = Utf8.Encode(debugRepresentation);
        RichRuntime.EnsureAbi();
        owner = new(Utf8.Map(encoded, native => NativeFactory.Create(
            new PrettyHandle(), output => NativeMethods.PrettyNew(native, output))));
    }

    /// <summary>Enables or disables native indentation guides.</summary>
    public void SetIndentGuides(bool enabled) => owner.Use(pointer => NativeResult.Check(
        NativeMethods.PrettySetIndentGuides(pointer, NativeFactory.Bool(enabled))));

    /// <summary>Consumes this builder and returns a reusable renderable.</summary>
    public Renderable Finish() => Renderable.Finish(owner, NativeMethods.PrettyFinish);

    /// <summary>Releases an unfinished builder.</summary>
    public void Dispose() => owner.Dispose();
}

/// <summary>A Markdown rendering builder.</summary>
public sealed unsafe class Markdown : IDisposable
{
    private readonly NativeOwner<MarkdownHandle> owner;

    /// <summary>Creates Markdown content.</summary>
    public unsafe Markdown(string source)
    {
        byte[] encoded = Utf8.Encode(source);
        RichRuntime.EnsureAbi();
        owner = new(Utf8.Map(encoded, native => NativeFactory.Create(
            new MarkdownHandle(), output => NativeMethods.MarkdownNew(native, output))));
    }

    /// <summary>Sets the fenced-code syntax theme.</summary>
    public void SetCodeTheme(string theme) => owner.Use(pointer =>
        NativeResult.Check(Utf8.With(theme, native => NativeMethods.MarkdownSetCodeTheme(pointer, native))));
    /// <summary>Enables or disables terminal hyperlinks.</summary>
    public bool Hyperlinks { set => owner.Use(pointer => NativeResult.Check(NativeMethods.MarkdownSetHyperlinks(pointer, NativeFactory.Bool(value)))); }
    /// <summary>Sets paragraph justification.</summary>
    public Justify Justification { set => owner.Use(pointer => NativeResult.Check(NativeMethods.MarkdownSetJustify(pointer, (uint)value))); }
    /// <summary>Consumes this Markdown builder.</summary>
    public unsafe Renderable Finish() => Renderable.Finish(owner, NativeMethods.MarkdownFinish);
    /// <summary>Releases an unfinished Markdown builder.</summary>
    public void Dispose() => owner.Dispose();
}

/// <summary>A validated JSON rendering builder.</summary>
public sealed unsafe class Json : IDisposable
{
    private readonly NativeOwner<JsonHandle> owner;

    /// <summary>Creates validated JSON with default rendering options.</summary>
    public Json(string data)
        : this(data, new JsonOptions())
    {
    }

    /// <summary>Creates validated JSON with explicit rendering options.</summary>
    public Json(string data, JsonOptions options)
    {
        byte[] encoded = Utf8.Encode(data);
        RichRuntime.EnsureAbi();
        owner = new(Utf8.Map(encoded, native => NativeFactory.Create(
            new JsonHandle(), output => NativeMethods.JsonNew(native, options.Indent,
                NativeFactory.Bool(options.Highlight), NativeFactory.Bool(options.SortKeys), output))));
    }

    /// <summary>Consumes this JSON builder.</summary>
    public unsafe Renderable Finish() => Renderable.Finish(owner, NativeMethods.JsonFinish);
    /// <summary>Releases an unfinished JSON builder.</summary>
    public void Dispose() => owner.Dispose();
}
