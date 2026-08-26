//! Low-level import of the generated rich-ffi C ABI.
//!
//! Most consumers should import `rich.zig`; this module is intentionally a
//! thin, advanced escape hatch whose declarations come only from `rich.h`.

pub const raw = @cImport({
    @cInclude("rich.h");
});

comptime {
    if (@sizeOf(raw.RichResult) != @sizeOf(u32))
        @compileError("rich-ffi RichResult must be exactly 32 bits");
    if (@sizeOf(raw.RichStr) != 16 or @alignOf(raw.RichStr) != @alignOf(u64))
        @compileError("rich-ffi RichStr layout does not match the ABI");
    if (@sizeOf(raw.RichBuffer) != 16 or @alignOf(raw.RichBuffer) != @alignOf(u64))
        @compileError("rich-ffi RichBuffer layout does not match the ABI");
    if (@offsetOf(raw.RichStr, "data") != 0 or @offsetOf(raw.RichStr, "len") != 8)
        @compileError("rich-ffi RichStr field offsets do not match the ABI");
    if (@offsetOf(raw.RichBuffer, "data") != 0 or @offsetOf(raw.RichBuffer, "len") != 8)
        @compileError("rich-ffi RichBuffer field offsets do not match the ABI");
    if (raw.RICH_FFI_ABI_VERSION != 2)
        @compileError("rich-ffi header has an unsupported ABI version");

    if (@sizeOf(raw.RichRenderOptions) != 24 or
        @offsetOf(raw.RichRenderOptions, "reserved") != 17)
        @compileError("rich-ffi RichRenderOptions layout does not match ABI v2");
    if (@sizeOf(raw.RichMeasurement) != 8)
        @compileError("rich-ffi RichMeasurement layout does not match ABI v2");
    if (@sizeOf(raw.RichTableColumnOptions) != 40 or
        @offsetOf(raw.RichTableColumnOptions, "style") != 24)
        @compileError("rich-ffi RichTableColumnOptions layout does not match ABI v2");

    // Exhaustive ABI-v2 inventory: a removed/renamed declaration is a compile
    // error and the explicit count prevents silently omitting new functions.
    const functions = .{
        raw.rich_ffi_abi_version,
        raw.rich_ffi_version,
        raw.rich_buffer_free,
        raw.rich_console_new,
        raw.rich_console_free,
        raw.rich_console_set_force_terminal,
        raw.rich_console_set_size,
        raw.rich_console_set_color_system,
        raw.rich_console_set_markup_enabled,
        raw.rich_console_set_emoji_enabled,
        raw.rich_console_width,
        raw.rich_console_render_markup,
        raw.rich_console_render,
        raw.rich_renderable_free,
        raw.rich_text_new,
        raw.rich_text_new_markup,
        raw.rich_text_set_style,
        raw.rich_text_finish,
        raw.rich_text_free,
        raw.rich_style_parse,
        raw.rich_style_free,
        raw.rich_syntax_new,
        raw.rich_syntax_from_path,
        raw.rich_syntax_set_theme,
        raw.rich_syntax_set_line_numbers,
        raw.rich_syntax_set_word_wrap,
        raw.rich_syntax_set_indent_guides,
        raw.rich_syntax_set_line_range,
        raw.rich_syntax_finish,
        raw.rich_syntax_free,
        raw.rich_pretty_new,
        raw.rich_pretty_set_indent_guides,
        raw.rich_pretty_finish,
        raw.rich_pretty_free,
        raw.rich_markdown_new,
        raw.rich_markdown_set_code_theme,
        raw.rich_markdown_set_hyperlinks,
        raw.rich_markdown_set_justify,
        raw.rich_markdown_finish,
        raw.rich_markdown_free,
        raw.rich_json_new,
        raw.rich_json_finish,
        raw.rich_json_free,
        raw.rich_custom_renderable_new,
        raw.rich_segment_sink_push,
        raw.rich_segment_sink_line,
        raw.rich_rule_new,
        raw.rich_rule_set_title,
        raw.rich_rule_set_characters,
        raw.rich_rule_set_style,
        raw.rich_rule_set_align,
        raw.rich_rule_finish,
        raw.rich_rule_free,
        raw.rich_columns_new,
        raw.rich_columns_add,
        raw.rich_columns_add_str,
        raw.rich_columns_set_equal,
        raw.rich_columns_set_expand,
        raw.rich_columns_set_padding,
        raw.rich_columns_finish,
        raw.rich_columns_free,
        raw.rich_align_new,
        raw.rich_align_center,
        raw.rich_align_set_vertical,
        raw.rich_align_set_width,
        raw.rich_align_finish,
        raw.rich_align_free,
        raw.rich_padding_new,
        raw.rich_padding_set_style,
        raw.rich_padding_set_expand,
        raw.rich_padding_finish,
        raw.rich_padding_free,
        raw.rich_progress_new,
        raw.rich_progress_add_task,
        raw.rich_progress_update,
        raw.rich_progress_render_frame,
        raw.rich_progress_free,
        raw.rich_status_new,
        raw.rich_status_render_frame,
        raw.rich_status_free,
        raw.rich_spinner_new,
        raw.rich_spinner_render_frame,
        raw.rich_spinner_free,
        raw.rich_panel_new,
        raw.rich_panel_set_title,
        raw.rich_panel_set_subtitle,
        raw.rich_panel_set_box,
        raw.rich_panel_set_expand,
        raw.rich_panel_set_width,
        raw.rich_panel_set_padding,
        raw.rich_panel_set_style,
        raw.rich_panel_set_border_style,
        raw.rich_panel_finish,
        raw.rich_panel_free,
        raw.rich_table_new,
        raw.rich_table_new_grid,
        raw.rich_table_finish,
        raw.rich_table_free,
        raw.rich_table_set_title,
        raw.rich_table_set_caption,
        raw.rich_table_set_title_markup,
        raw.rich_table_set_caption_markup,
        raw.rich_table_set_box,
        raw.rich_table_set_pad_edge,
        raw.rich_table_set_row_styles,
        raw.rich_table_set_show_header,
        raw.rich_table_set_show_lines,
        raw.rich_table_set_show_edge,
        raw.rich_table_set_expand,
        raw.rich_table_set_padding,
        raw.rich_table_set_style,
        raw.rich_table_add_column,
        raw.rich_table_add_column_renderable,
        raw.rich_table_add_column_options,
        raw.rich_table_add_row_strs,
        raw.rich_table_add_row_renderables,
        raw.rich_tree_new,
        raw.rich_tree_new_renderable,
        raw.rich_tree_add,
        raw.rich_tree_add_renderable,
        raw.rich_tree_set_style,
        raw.rich_tree_set_guide_style,
        raw.rich_tree_set_hide_root,
        raw.rich_tree_finish,
        raw.rich_tree_free,
    };
    if (functions.len != 125) @compileError("rich-ffi ABI v2 must expose exactly 125 functions");
}
