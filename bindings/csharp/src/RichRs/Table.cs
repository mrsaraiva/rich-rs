namespace RichRs;

/// <summary>A table builder with plain or reusable-renderable cells.</summary>
public sealed unsafe class Table : IDisposable
{
    private readonly NativeOwner<TableHandle> owner;

    /// <summary>Creates an empty table.</summary>
    public Table()
        : this(NativeMethods.TableNew)
    {
    }

    private Table(NewHandleCall create)
    {
        CallbackGuard.ThrowIfActive();
        TableHandle handle = new();
        owner = new(handle);
        try
        {
            RichRuntime.EnsureAbi();
            NativeFactory.Initialize(handle, create);
        }
        catch
        {
            owner.Dispose();
            throw;
        }
    }

    /// <summary>Creates a borderless grid with native grid defaults.</summary>
    public static Table CreateGrid() => new(NativeMethods.TableNewGrid);

    /// <summary>Sets a plain title.</summary>
    public void SetTitle(string title) => WithString(title, NativeMethods.TableSetTitle);
    /// <summary>Sets a plain caption.</summary>
    public void SetCaption(string caption) => WithString(caption, NativeMethods.TableSetCaption);

    /// <summary>Sets a markup title parsed by the given console.</summary>
    public void SetTitleMarkup(RichConsole console, string markup) => SetMarkup(console, markup, NativeMethods.TableSetTitleMarkup);
    /// <summary>Sets a markup caption parsed by the given console.</summary>
    public void SetCaptionMarkup(RichConsole console, string markup) => SetMarkup(console, markup, NativeMethods.TableSetCaptionMarkup);

    /// <summary>Sets the table border style.</summary>
    public Box Box { set => owner.Use(pointer => NativeResult.Check(NativeMethods.TableSetBox(pointer, (uint)value))); }
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

    /// <summary>Controls padding at the outer table edge.</summary>
    public void SetPadEdge(bool pad) => owner.Use(pointer => NativeResult.Check(
        NativeMethods.TableSetPadEdge(pointer, NativeFactory.Bool(pad))));

    /// <summary>Atomically replaces the alternating row styles.</summary>
    public void SetRowStyles(IReadOnlyList<string> styles) => owner.Use(table =>
        NativeResult.Check(Utf8.WithArray(styles,
            (values, count) => NativeMethods.TableSetRowStyles(table, values, count))));

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

    /// <summary>Adds an atomically configured column with an optional borrowed header.</summary>
    public void AddColumnOptions(ColumnOptions options, Renderable? header = null)
    {
        byte[] styleBytes = Utf8.Encode(options.Style);
        Utf8.Map(styleBytes, nativeStyle =>
        {
            NativeRichTableColumnOptions native = new()
            {
                Flags = ColumnFlags(options),
                Justify = (uint)options.Justification.GetValueOrDefault(),
                Vertical = (uint)options.Vertical.GetValueOrDefault(),
                Width = options.Width.GetValueOrDefault(),
                MinWidth = options.MinimumWidth.GetValueOrDefault(),
                Ratio = options.Ratio.GetValueOrDefault(),
                Style = nativeStyle,
            };
            owner.Use(table =>
            {
                if (header is null)
                {
                    NativeResult.Check(AddColumnOptionsCore(table, 0, native));
                }
                else
                {
                    header.Borrow(headerPointer => NativeResult.Check(
                        AddColumnOptionsCore(table, headerPointer, native)));
                }
            });
            return 0;
        });
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

    private static uint ColumnFlags(ColumnOptions options)
    {
        return (options.Justification.HasValue ? NativeColumnFlags.HasJustify : 0)
            | (options.Vertical.HasValue ? NativeColumnFlags.HasVertical : 0)
            | (options.Width.HasValue ? NativeColumnFlags.HasWidth : 0)
            | (options.MinimumWidth.HasValue ? NativeColumnFlags.HasMinimumWidth : 0)
            | (options.Ratio.HasValue ? NativeColumnFlags.HasRatio : 0)
            | (options.NoWrap ? NativeColumnFlags.NoWrap : 0);
    }

    private static unsafe uint AddColumnOptionsCore(
        nint table, nint header, NativeRichTableColumnOptions options) =>
        NativeMethods.TableAddColumnOptions(table, header, &options);
}
