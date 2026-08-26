namespace RichRs;

/// <summary>A panel builder around a borrowed reusable renderable.</summary>
public sealed class Panel : IDisposable
{
    private readonly NativeOwner<PanelHandle> owner;

    /// <summary>Creates a panel and retains the native renderable Arc.</summary>
    public unsafe Panel(Renderable content)
    {
        ArgumentNullException.ThrowIfNull(content);
        RichRuntime.EnsureAbi();
        owner = new(content.Borrow(renderable => NativeFactory.Create(
            new PanelHandle(), output => NativeMethods.PanelNew(renderable, output))));
    }

    /// <summary>Sets the panel title.</summary>
    public void SetTitle(string title) => WithString(title, NativeMethods.PanelSetTitle);
    /// <summary>Sets the panel subtitle.</summary>
    public void SetSubtitle(string subtitle) => WithString(subtitle, NativeMethods.PanelSetSubtitle);
    /// <summary>Sets the box style.</summary>
    public Box Box { set => owner.Use(pointer => NativeResult.Check(NativeMethods.PanelSetBox(pointer, (uint)value))); }
    /// <summary>Enables or disables expansion.</summary>
    public bool Expand { set => owner.Use(pointer => NativeResult.Check(NativeMethods.PanelSetExpand(pointer, NativeFactory.Bool(value)))); }
    /// <summary>Sets the panel width.</summary>
    public uint Width { set => owner.Use(pointer => NativeResult.Check(NativeMethods.PanelSetWidth(pointer, value))); }
    /// <summary>Sets panel padding.</summary>
    public void SetPadding(uint top, uint right, uint bottom, uint left) => owner.Use(pointer =>
        NativeResult.Check(NativeMethods.PanelSetPadding(pointer, top, right, bottom, left)));
    /// <summary>Sets the panel content style.</summary>
    public void SetStyle(string style) => WithString(style, NativeMethods.PanelSetStyle);
    /// <summary>Sets the panel border style.</summary>
    public void SetBorderStyle(string style) => WithString(style, NativeMethods.PanelSetBorderStyle);
    /// <summary>Consumes this panel and returns a renderable.</summary>
    public unsafe Renderable Finish() => Renderable.Finish(owner, NativeMethods.PanelFinish);
    /// <summary>Releases an unfinished panel.</summary>
    public void Dispose() => owner.Dispose();

    private delegate uint StringSetter(nint panel, NativeRichStr value);
    private void WithString(string value, StringSetter setter) => owner.Use(pointer =>
        NativeResult.Check(Utf8.With(value, native => setter(pointer, native))));
}
