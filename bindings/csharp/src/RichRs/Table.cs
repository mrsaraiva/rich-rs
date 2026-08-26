namespace RichRs;

/// <summary>A table builder with plain or reusable-renderable cells.</summary>
public sealed unsafe class Table : IDisposable
{
    private readonly NativeOwner<TableHandle> owner;

    /// <summary>Creates an empty table.</summary>
    public unsafe Table()
    {
        RichRuntime.EnsureAbi();
        owner = new(NativeFactory.Create(new TableHandle(), NativeMethods.TableNew));
    }

    /// <summary>Sets a plain title.</summary>
    public void SetTitle(string title) => WithString(title, NativeMethods.TableSetTitle);
    /// <summary>Sets a plain caption.</summary>
    public void SetCaption(string caption) => WithString(caption, NativeMethods.TableSetCaption);

    /// <summary>Sets a markup title parsed by the given console.</summary>
    public void SetTitleMarkup(RichConsole console, string markup) => SetMarkup(console, markup, NativeMethods.TableSetTitleMarkup);
    /// <summary>Sets a markup caption parsed by the given console.</summary>
    public void SetCaptionMarkup(RichConsole console, string markup) => SetMarkup(console, markup, NativeMethods.TableSetCaptionMarkup);

    /// <summary>Sets the table border style.</summary>
    public BoxStyle Box { set => owner.Use(pointer => NativeResult.Check(NativeMethods.TableSetBox(pointer, (uint)value))); }
    /// <summary>Shows or hides the header.</summary>
    public bool ShowHeader { set => owner.Use(pointer => NativeResult.Check(NativeMethods.TableSetShowHeader(pointer, NativeFactory.Bool(value)))); }
    /// <summary>Shows or hides row separators.</summary>
    public bool ShowLines { set => owner.Use(pointer => NativeResult.Check(NativeMethods.TableSetShowLines(pointer, NativeFactory.Bool(value)))); }
    /// <summary>Shows or hides outer edges.</summary>
    public bool ShowEdge { set => owner.Use(pointer => NativeResult.Check(NativeMethods.TableSetShowEdge(pointer, NativeFactory.Bool(value)))); }
    /// <summary>Enables or disables table expansion.</summary>
    public bool Expand { set => owner.Use(pointer => NativeResult.Check(NativeMethods.TableSetExpand(pointer, NativeFactory.Bool(value)))); }

    /// <summary>Sets left and right cell padding.</summary>
    public void SetPadding(uint left, uint right) => owner.Use(pointer =>
        NativeResult.Check(NativeMethods.TableSetPadding(pointer, left, right)));

    /// <summary>Sets the table style.</summary>
    public void SetStyle(string style) => WithString(style, NativeMethods.TableSetStyle);

    /// <summary>Adds a plain column header.</summary>
    public void AddColumn(string header) => WithString(header, NativeMethods.TableAddColumn);

    /// <summary>Adds a borrowed renderable column header.</summary>
    public void AddColumn(Renderable header)
    {
        ArgumentNullException.ThrowIfNull(header);
        owner.Use(table => header.Borrow(renderable =>
            NativeResult.Check(NativeMethods.TableAddColumnRenderable(table, renderable))));
    }

    /// <summary>Adds an atomically validated row of strings.</summary>
    public unsafe void AddRow(IReadOnlyList<string> cells)
    {
        ArgumentNullException.ThrowIfNull(cells);
        owner.Use(table => NativeResult.Check(Utf8.WithArray(cells,
            (values, count) => NativeMethods.TableAddRowStrings(table, values, count))));
    }

    /// <summary>Adds an atomically borrowed row of reusable renderables.</summary>
    public unsafe void AddRow(IReadOnlyList<Renderable> cells)
    {
        Renderable[] snapshot = CollectionSnapshot.Copy(cells);
        NativeSafeHandle[] handles = new NativeSafeHandle[snapshot.Length];
        for (int i = 0; i < snapshot.Length; i++)
        {
            ArgumentNullException.ThrowIfNull(snapshot[i]);
            handles[i] = snapshot[i].Handle;
        }
        using NativeHandleGroup group = new(handles);
        owner.Use(table =>
        {
            fixed (nint* values = group.Pointers)
            {
                NativeResult.Check(NativeMethods.TableAddRowRenderables(
                    table, snapshot.Length == 0 ? null : values, checked((uint)snapshot.Length)));
            }
        });
    }

    /// <summary>Consumes this table and returns a reusable renderable.</summary>
    public unsafe Renderable Finish() => Renderable.Finish(owner, NativeMethods.TableFinish);

    /// <summary>Releases an unfinished table.</summary>
    public void Dispose() => owner.Dispose();

    private delegate uint StringSetter(nint table, NativeRichStr value);
    private delegate uint MarkupSetter(nint table, nint console, NativeRichStr value);

    private void WithString(string value, StringSetter setter) => owner.Use(pointer =>
        NativeResult.Check(Utf8.With(value, native => setter(pointer, native))));

    private void SetMarkup(RichConsole console, string markup, MarkupSetter setter)
    {
        ArgumentNullException.ThrowIfNull(console);
        owner.Use(table => console.Borrow(consolePointer =>
            NativeResult.Check(Utf8.With(markup, native => setter(table, consolePointer, native)))));
    }
}
