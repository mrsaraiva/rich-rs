using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace RichRs;

internal static unsafe partial class NativeMethods
{
    private const string Library = NativeLibraryResolver.LogicalName;

    [LibraryImport(Library, EntryPoint = "rich_ffi_abi_version")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint FfiAbiVersion();
    [LibraryImport(Library, EntryPoint = "rich_ffi_version")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint FfiVersion(NativeRichBuffer* output);
    [LibraryImport(Library, EntryPoint = "rich_buffer_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void BufferFree(NativeRichBuffer* buffer);

    [LibraryImport(Library, EntryPoint = "rich_console_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ConsoleNew(nint* output);
    [LibraryImport(Library, EntryPoint = "rich_console_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void ConsoleFree(nint value);
    [LibraryImport(Library, EntryPoint = "rich_console_set_force_terminal")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ConsoleSetForceTerminal(nint value, byte force);
    [LibraryImport(Library, EntryPoint = "rich_console_set_size")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ConsoleSetSize(nint value, uint width, uint height);
    [LibraryImport(Library, EntryPoint = "rich_console_set_color_system")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ConsoleSetColorSystem(nint value, uint system);
    [LibraryImport(Library, EntryPoint = "rich_console_set_markup_enabled")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ConsoleSetMarkupEnabled(nint value, byte enabled);
    [LibraryImport(Library, EntryPoint = "rich_console_set_emoji_enabled")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ConsoleSetEmojiEnabled(nint value, byte enabled);
    [LibraryImport(Library, EntryPoint = "rich_console_width")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ConsoleWidth(nint value, uint* output);
    [LibraryImport(Library, EntryPoint = "rich_console_render_markup")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ConsoleRenderMarkup(nint value, NativeRichStr markup, NativeRichBuffer* output);
    [LibraryImport(Library, EntryPoint = "rich_console_render")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ConsoleRender(nint value, nint renderable, NativeRichBuffer* output);

    [LibraryImport(Library, EntryPoint = "rich_renderable_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void RenderableFree(nint value);

    [LibraryImport(Library, EntryPoint = "rich_text_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TextNew(NativeRichStr text, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_text_new_markup")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TextNewMarkup(nint console, NativeRichStr markup, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_text_set_style")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TextSetStyle(nint text, NativeRichStr style);
    [LibraryImport(Library, EntryPoint = "rich_text_finish")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TextFinish(nint* text, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_text_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void TextFree(nint value);

    [LibraryImport(Library, EntryPoint = "rich_style_parse")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint StyleParse(NativeRichStr style, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_style_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void StyleFree(nint value);

    [LibraryImport(Library, EntryPoint = "rich_syntax_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint SyntaxNew(NativeRichStr code, NativeRichStr lexer, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_syntax_from_path")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint SyntaxFromPath(NativeRichStr path, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_syntax_set_theme")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint SyntaxSetTheme(nint value, NativeRichStr theme);
    [LibraryImport(Library, EntryPoint = "rich_syntax_set_line_numbers")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint SyntaxSetLineNumbers(nint value, byte enabled);
    [LibraryImport(Library, EntryPoint = "rich_syntax_set_word_wrap")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint SyntaxSetWordWrap(nint value, byte enabled);
    [LibraryImport(Library, EntryPoint = "rich_syntax_set_line_range")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint SyntaxSetLineRange(nint value, byte hasStart, uint start, byte hasEnd, uint end);
    [LibraryImport(Library, EntryPoint = "rich_syntax_finish")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint SyntaxFinish(nint* value, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_syntax_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void SyntaxFree(nint value);

    [LibraryImport(Library, EntryPoint = "rich_markdown_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint MarkdownNew(NativeRichStr source, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_markdown_set_code_theme")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint MarkdownSetCodeTheme(nint value, NativeRichStr theme);
    [LibraryImport(Library, EntryPoint = "rich_markdown_set_hyperlinks")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint MarkdownSetHyperlinks(nint value, byte enabled);
    [LibraryImport(Library, EntryPoint = "rich_markdown_set_justify")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint MarkdownSetJustify(nint value, uint justify);
    [LibraryImport(Library, EntryPoint = "rich_markdown_finish")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint MarkdownFinish(nint* value, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_markdown_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void MarkdownFree(nint value);

    [LibraryImport(Library, EntryPoint = "rich_json_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint JsonNew(NativeRichStr data, uint indent, byte highlight, byte sortKeys, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_json_finish")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint JsonFinish(nint* value, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_json_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void JsonFree(nint value);

    [LibraryImport(Library, EntryPoint = "rich_rule_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint RuleNew(nint* output);
    [LibraryImport(Library, EntryPoint = "rich_rule_set_title")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint RuleSetTitle(nint value, NativeRichStr title);
    [LibraryImport(Library, EntryPoint = "rich_rule_set_characters")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint RuleSetCharacters(nint value, NativeRichStr characters);
    [LibraryImport(Library, EntryPoint = "rich_rule_set_style")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint RuleSetStyle(nint value, NativeRichStr style);
    [LibraryImport(Library, EntryPoint = "rich_rule_set_align")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint RuleSetAlign(nint value, uint align);
    [LibraryImport(Library, EntryPoint = "rich_rule_finish")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint RuleFinish(nint* value, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_rule_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void RuleFree(nint value);

    [LibraryImport(Library, EntryPoint = "rich_columns_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ColumnsNew(nint* output);
    [LibraryImport(Library, EntryPoint = "rich_columns_add")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ColumnsAdd(nint value, nint renderable);
    [LibraryImport(Library, EntryPoint = "rich_columns_add_str")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ColumnsAddString(nint value, NativeRichStr text);
    [LibraryImport(Library, EntryPoint = "rich_columns_set_equal")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ColumnsSetEqual(nint value, byte equal);
    [LibraryImport(Library, EntryPoint = "rich_columns_set_expand")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ColumnsSetExpand(nint value, byte expand);
    [LibraryImport(Library, EntryPoint = "rich_columns_set_padding")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ColumnsSetPadding(nint value, uint vertical, uint horizontal);
    [LibraryImport(Library, EntryPoint = "rich_columns_finish")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ColumnsFinish(nint* value, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_columns_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void ColumnsFree(nint value);

    [LibraryImport(Library, EntryPoint = "rich_align_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint AlignNew(nint content, uint horizontal, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_align_center")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint AlignCenter(nint content, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_align_set_vertical")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint AlignSetVertical(nint value, uint vertical);
    [LibraryImport(Library, EntryPoint = "rich_align_set_width")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint AlignSetWidth(nint value, uint width);
    [LibraryImport(Library, EntryPoint = "rich_align_finish")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint AlignFinish(nint* value, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_align_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void AlignFree(nint value);

    [LibraryImport(Library, EntryPoint = "rich_padding_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint PaddingNew(nint content, uint top, uint right, uint bottom, uint left, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_padding_set_style")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint PaddingSetStyle(nint value, NativeRichStr style);
    [LibraryImport(Library, EntryPoint = "rich_padding_set_expand")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint PaddingSetExpand(nint value, byte expand);
    [LibraryImport(Library, EntryPoint = "rich_padding_finish")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint PaddingFinish(nint* value, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_padding_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void PaddingFree(nint value);

    [LibraryImport(Library, EntryPoint = "rich_progress_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ProgressNew(nint* output);
    [LibraryImport(Library, EntryPoint = "rich_progress_add_task")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ProgressAddTask(nint value, NativeRichStr description, double total, ulong* output);
    [LibraryImport(Library, EntryPoint = "rich_progress_update")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ProgressUpdate(nint value, ulong taskId, double completed);
    [LibraryImport(Library, EntryPoint = "rich_progress_render_frame")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint ProgressRenderFrame(nint value, nint console, NativeRichBuffer* output);
    [LibraryImport(Library, EntryPoint = "rich_progress_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void ProgressFree(nint value);

    [LibraryImport(Library, EntryPoint = "rich_status_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint StatusNew(NativeRichStr message, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_status_render_frame")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint StatusRenderFrame(nint value, nint console, NativeRichBuffer* output);
    [LibraryImport(Library, EntryPoint = "rich_status_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void StatusFree(nint value);

    [LibraryImport(Library, EntryPoint = "rich_spinner_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint SpinnerNew(NativeRichStr name, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_spinner_render_frame")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint SpinnerRenderFrame(nint value, nint console, double timeSeconds, NativeRichBuffer* output);
    [LibraryImport(Library, EntryPoint = "rich_spinner_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void SpinnerFree(nint value);

    [LibraryImport(Library, EntryPoint = "rich_panel_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint PanelNew(nint content, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_panel_set_title")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint PanelSetTitle(nint value, NativeRichStr title);
    [LibraryImport(Library, EntryPoint = "rich_panel_set_subtitle")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint PanelSetSubtitle(nint value, NativeRichStr subtitle);
    [LibraryImport(Library, EntryPoint = "rich_panel_set_box")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint PanelSetBox(nint value, uint box);
    [LibraryImport(Library, EntryPoint = "rich_panel_set_expand")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint PanelSetExpand(nint value, byte expand);
    [LibraryImport(Library, EntryPoint = "rich_panel_set_width")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint PanelSetWidth(nint value, uint width);
    [LibraryImport(Library, EntryPoint = "rich_panel_set_padding")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint PanelSetPadding(nint value, uint top, uint right, uint bottom, uint left);
    [LibraryImport(Library, EntryPoint = "rich_panel_set_style")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint PanelSetStyle(nint value, NativeRichStr style);
    [LibraryImport(Library, EntryPoint = "rich_panel_set_border_style")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint PanelSetBorderStyle(nint value, NativeRichStr style);
    [LibraryImport(Library, EntryPoint = "rich_panel_finish")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint PanelFinish(nint* value, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_panel_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void PanelFree(nint value);

    [LibraryImport(Library, EntryPoint = "rich_table_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableNew(nint* output);
    [LibraryImport(Library, EntryPoint = "rich_table_finish")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableFinish(nint* value, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_table_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void TableFree(nint value);
    [LibraryImport(Library, EntryPoint = "rich_table_set_title")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableSetTitle(nint value, NativeRichStr title);
    [LibraryImport(Library, EntryPoint = "rich_table_set_caption")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableSetCaption(nint value, NativeRichStr caption);
    [LibraryImport(Library, EntryPoint = "rich_table_set_title_markup")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableSetTitleMarkup(nint value, nint console, NativeRichStr markup);
    [LibraryImport(Library, EntryPoint = "rich_table_set_caption_markup")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableSetCaptionMarkup(nint value, nint console, NativeRichStr markup);
    [LibraryImport(Library, EntryPoint = "rich_table_set_box")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableSetBox(nint value, uint box);
    [LibraryImport(Library, EntryPoint = "rich_table_set_show_header")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableSetShowHeader(nint value, byte show);
    [LibraryImport(Library, EntryPoint = "rich_table_set_show_lines")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableSetShowLines(nint value, byte show);
    [LibraryImport(Library, EntryPoint = "rich_table_set_show_edge")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableSetShowEdge(nint value, byte show);
    [LibraryImport(Library, EntryPoint = "rich_table_set_expand")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableSetExpand(nint value, byte expand);
    [LibraryImport(Library, EntryPoint = "rich_table_set_padding")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableSetPadding(nint value, uint left, uint right);
    [LibraryImport(Library, EntryPoint = "rich_table_set_style")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableSetStyle(nint value, NativeRichStr style);
    [LibraryImport(Library, EntryPoint = "rich_table_add_column")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableAddColumn(nint value, NativeRichStr header);
    [LibraryImport(Library, EntryPoint = "rich_table_add_column_renderable")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableAddColumnRenderable(nint value, nint header);
    [LibraryImport(Library, EntryPoint = "rich_table_add_row_strs")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableAddRowStrings(nint value, NativeRichStr* cells, uint count);
    [LibraryImport(Library, EntryPoint = "rich_table_add_row_renderables")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TableAddRowRenderables(nint value, nint* cells, uint count);

    [LibraryImport(Library, EntryPoint = "rich_tree_new")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TreeNew(NativeRichStr label, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_tree_new_renderable")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TreeNewRenderable(nint label, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_tree_add")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TreeAdd(nint value, ulong parentId, NativeRichStr label, ulong* output);
    [LibraryImport(Library, EntryPoint = "rich_tree_add_renderable")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TreeAddRenderable(nint value, ulong parentId, nint label, ulong* output);
    [LibraryImport(Library, EntryPoint = "rich_tree_set_style")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TreeSetStyle(nint value, NativeRichStr style);
    [LibraryImport(Library, EntryPoint = "rich_tree_set_guide_style")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TreeSetGuideStyle(nint value, NativeRichStr style);
    [LibraryImport(Library, EntryPoint = "rich_tree_set_hide_root")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TreeSetHideRoot(nint value, byte hide);
    [LibraryImport(Library, EntryPoint = "rich_tree_finish")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial uint TreeFinish(nint* value, nint* output);
    [LibraryImport(Library, EntryPoint = "rich_tree_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    internal static partial void TreeFree(nint value);
}
