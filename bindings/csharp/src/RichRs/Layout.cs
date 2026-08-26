namespace RichRs;

/// <summary>A horizontal rule builder.</summary>
public sealed class Rule : IDisposable
{
    private readonly NativeOwner<RuleHandle> owner;

    /// <summary>Creates an empty rule.</summary>
    public unsafe Rule()
    {
        RichRuntime.EnsureAbi();
        owner = new(NativeFactory.Create(new RuleHandle(), NativeMethods.RuleNew));
    }

    /// <summary>Sets the title.</summary>
    public void SetTitle(string title) => WithString(title, NativeMethods.RuleSetTitle);
    /// <summary>Sets the repeated characters.</summary>
    public void SetCharacters(string characters) => WithString(characters, NativeMethods.RuleSetCharacters);
    /// <summary>Sets the rule style.</summary>
    public void SetStyle(string style) => WithString(style, NativeMethods.RuleSetStyle);
    /// <summary>Sets title alignment.</summary>
    public HorizontalAlignment Alignment { set => owner.Use(pointer => NativeResult.Check(NativeMethods.RuleSetAlign(pointer, (uint)value))); }
    /// <summary>Consumes this rule.</summary>
    public unsafe Renderable Finish() => Renderable.Finish(owner, NativeMethods.RuleFinish);
    /// <summary>Releases an unfinished rule.</summary>
    public void Dispose() => owner.Dispose();

    private delegate uint StringSetter(nint rule, NativeRichStr value);
    private void WithString(string value, StringSetter setter) => owner.Use(pointer =>
        NativeResult.Check(Utf8.With(value, native => setter(pointer, native))));
}

/// <summary>A columns builder retaining borrowed renderables.</summary>
public sealed class Columns : IDisposable
{
    private readonly NativeOwner<ColumnsHandle> owner;

    /// <summary>Creates empty columns.</summary>
    public unsafe Columns()
    {
        RichRuntime.EnsureAbi();
        owner = new(NativeFactory.Create(new ColumnsHandle(), NativeMethods.ColumnsNew));
    }

    /// <summary>Adds a borrowed reusable renderable.</summary>
    public void Add(Renderable renderable)
    {
        ArgumentNullException.ThrowIfNull(renderable);
        owner.Use(columns => renderable.Borrow(value => NativeResult.Check(NativeMethods.ColumnsAdd(columns, value))));
    }

    /// <summary>Adds plain text.</summary>
    public void Add(string text) => owner.Use(columns =>
        NativeResult.Check(Utf8.With(text, value => NativeMethods.ColumnsAddString(columns, value))));
    /// <summary>Makes all columns equal width.</summary>
    public bool Equal { set => owner.Use(pointer => NativeResult.Check(NativeMethods.ColumnsSetEqual(pointer, NativeFactory.Bool(value)))); }
    /// <summary>Enables or disables expansion.</summary>
    public bool Expand { set => owner.Use(pointer => NativeResult.Check(NativeMethods.ColumnsSetExpand(pointer, NativeFactory.Bool(value)))); }
    /// <summary>Sets vertical and horizontal padding.</summary>
    public void SetPadding(uint vertical, uint horizontal) => owner.Use(pointer =>
        NativeResult.Check(NativeMethods.ColumnsSetPadding(pointer, vertical, horizontal)));
    /// <summary>Consumes these columns.</summary>
    public unsafe Renderable Finish() => Renderable.Finish(owner, NativeMethods.ColumnsFinish);
    /// <summary>Releases unfinished columns.</summary>
    public void Dispose() => owner.Dispose();
}

/// <summary>An alignment builder around retained content.</summary>
public sealed class Align : IDisposable
{
    private readonly NativeOwner<AlignHandle> owner;

    /// <summary>Creates aligned content.</summary>
    public unsafe Align(Renderable content, HorizontalAlignment horizontal)
    {
        ArgumentNullException.ThrowIfNull(content);
        RichRuntime.EnsureAbi();
        owner = new(content.Borrow(renderable => NativeFactory.Create(
            new AlignHandle(), output => NativeMethods.AlignNew(renderable, (uint)horizontal, output))));
    }

    private Align(AlignHandle handle) => owner = new(handle);

    /// <summary>Creates centered content using the native convenience function.</summary>
    public static unsafe Align Center(Renderable content)
    {
        ArgumentNullException.ThrowIfNull(content);
        RichRuntime.EnsureAbi();
        AlignHandle handle = content.Borrow(renderable => NativeFactory.Create(
            new AlignHandle(), output => NativeMethods.AlignCenter(renderable, output)));
        return new(handle);
    }

    /// <summary>Sets vertical alignment.</summary>
    public VerticalAlignment Vertical { set => owner.Use(pointer => NativeResult.Check(NativeMethods.AlignSetVertical(pointer, (uint)value))); }
    /// <summary>Sets alignment width.</summary>
    public uint Width { set => owner.Use(pointer => NativeResult.Check(NativeMethods.AlignSetWidth(pointer, value))); }
    /// <summary>Consumes this alignment builder.</summary>
    public unsafe Renderable Finish() => Renderable.Finish(owner, NativeMethods.AlignFinish);
    /// <summary>Releases an unfinished alignment builder.</summary>
    public void Dispose() => owner.Dispose();
}

/// <summary>A padding builder around retained content.</summary>
public sealed class Padding : IDisposable
{
    private readonly NativeOwner<PaddingHandle> owner;

    /// <summary>Creates padded content.</summary>
    public unsafe Padding(Renderable content, uint top, uint right, uint bottom, uint left)
    {
        ArgumentNullException.ThrowIfNull(content);
        RichRuntime.EnsureAbi();
        owner = new(content.Borrow(renderable => NativeFactory.Create(
            new PaddingHandle(), output => NativeMethods.PaddingNew(renderable, top, right, bottom, left, output))));
    }

    /// <summary>Sets the fill style.</summary>
    public void SetStyle(string style) => owner.Use(pointer =>
        NativeResult.Check(Utf8.With(style, value => NativeMethods.PaddingSetStyle(pointer, value))));
    /// <summary>Enables or disables expansion.</summary>
    public bool Expand { set => owner.Use(pointer => NativeResult.Check(NativeMethods.PaddingSetExpand(pointer, NativeFactory.Bool(value)))); }
    /// <summary>Consumes this padding builder.</summary>
    public unsafe Renderable Finish() => Renderable.Finish(owner, NativeMethods.PaddingFinish);
    /// <summary>Releases an unfinished padding builder.</summary>
    public void Dispose() => owner.Dispose();
}
