#include "rich.h"

#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static RichStr rich_str(const char *value) {
    RichStr result = {(const uint8_t *)value, (uint64_t)strlen(value)};
    return result;
}

typedef struct {
    uint32_t height;
} CustomContext;

static RichResult render_custom(
    void *context,
    const RichRenderOptions *options,
    RichSegmentSink *sink) {
    const CustomContext *custom = (const CustomContext *)context;
    assert(options->max_width > 0);
    for (uint32_t y = 0; y < custom->height; ++y) {
        for (uint32_t x = 0; x < options->max_width; ++x) {
            char style[64];
            assert(snprintf(style, sizeof(style), "rgb(%u,%u,%u)",
                            (unsigned)(x % 255), (unsigned)(y % 255), 127u) > 0);
            assert(rich_segment_sink_push(sink, rich_str("▄"), rich_str(style)) == RICH_OK);
        }
        assert(rich_segment_sink_line(sink) == RICH_OK);
    }
    return RICH_OK;
}

static RichResult measure_custom(
    void *context,
    const RichRenderOptions *options,
    RichMeasurement *measurement) {
    (void)context;
    measurement->minimum = 1;
    measurement->maximum = options->max_width;
    return RICH_OK;
}

static void drop_custom(void *context) {
    free(context);
}

static size_t count_bytes(const RichBuffer *buffer, const char *needle) {
    const size_t needle_len = strlen(needle);
    size_t count = 0;
    for (size_t offset = 0; offset + needle_len <= (size_t)buffer->len; ++offset) {
        if (memcmp(buffer->data + offset, needle, needle_len) == 0) ++count;
    }
    return count;
}

static void probe_colorbox(uint32_t width, uint32_t height) {
    RichConsole *console = NULL;
    assert(rich_console_new(&console) == RICH_OK);
    assert(rich_console_set_size(console, width, 20) == RICH_OK);
    assert(rich_console_set_force_terminal(console, 1) == RICH_OK);
    assert(rich_console_set_color_system(console, RICH_COLOR_TRUECOLOR) == RICH_OK);
    CustomContext *context = (CustomContext *)malloc(sizeof(*context));
    assert(context != NULL);
    context->height = height;
    RichCustomRenderableVTable vtable = {render_custom, measure_custom, drop_custom};
    RichRenderable *renderable = NULL;
    RichResult result = rich_custom_renderable_new(context, &vtable, &renderable);
    if (result != RICH_OK) free(context);
    assert(result == RICH_OK);
    RichBuffer output = {NULL, 0};
    assert(rich_console_render(console, renderable, &output) == RICH_OK);
    assert(count_bytes(&output, "▄") == (size_t)width * height);
    assert(count_bytes(&output, "\x1b[38;2") > 0);
    rich_buffer_free(&output);
    rich_renderable_free(renderable);
    rich_console_free(console);
}

static RichBuffer render_pretty_probe(RichConsole *console, const char *input, uint8_t guides) {
    RichPretty *pretty = NULL;
    RichRenderable *renderable = NULL;
    RichBuffer output = {NULL, 0};
    assert(rich_pretty_new(rich_str(input), &pretty) == RICH_OK);
    assert(rich_pretty_set_indent_guides(pretty, guides) == RICH_OK);
    assert(rich_pretty_finish(&pretty, &renderable) == RICH_OK);
    assert(rich_console_render(console, renderable, &output) == RICH_OK);
    rich_renderable_free(renderable);
    return output;
}

static RichBuffer render_syntax_probe(RichConsole *console, uint8_t guides) {
    RichSyntax *syntax = NULL;
    RichRenderable *renderable = NULL;
    RichBuffer output = {NULL, 0};
    assert(rich_syntax_new(
        rich_str("fn main() {\n    if true {\n        run();\n    }\n}"),
        rich_str("rust"), &syntax) == RICH_OK);
    assert(rich_syntax_set_indent_guides(syntax, guides) == RICH_OK);
    assert(rich_syntax_finish(&syntax, &renderable) == RICH_OK);
    assert(rich_console_render(console, renderable, &output) == RICH_OK);
    rich_renderable_free(renderable);
    return output;
}

static int buffers_equal(const RichBuffer *left, const RichBuffer *right) {
    return left->len == right->len &&
           (left->len == 0 || memcmp(left->data, right->data, (size_t)left->len) == 0);
}

static RichBuffer render_table_probe(int mode) {
    RichConsole *console = NULL;
    RichTable *table = NULL;
    RichRenderable *renderable = NULL;
    RichBuffer output = {NULL, 0};
    assert(rich_console_new(&console) == RICH_OK);
    assert(rich_console_set_size(console, 24, 20) == RICH_OK);
    assert(rich_console_set_force_terminal(console, 1) == RICH_OK);
    assert(rich_table_new(&table) == RICH_OK);
    assert(rich_table_set_expand(table, 1) == RICH_OK);
    if (mode == 1) assert(rich_table_set_box(table, RICH_BOX_NONE) == RICH_OK);
    if (mode == 2) assert(rich_table_set_pad_edge(table, 0) == RICH_OK);
    if (mode == 3) {
        RichStr styles[] = {rich_str("red")};
        assert(rich_table_set_row_styles(table, styles, 1) == RICH_OK);
    }
    RichTableColumnOptions first = {0};
    RichTableColumnOptions second = {0};
    if (mode == 4) {
        first.flags = second.flags = RICH_COLUMN_HAS_RATIO;
        first.ratio = 3;
        second.ratio = 1;
    } else if (mode == 5) {
        first.flags = RICH_COLUMN_HAS_WIDTH;
        first.width = 5;
    } else if (mode == 6) {
        first.flags = RICH_COLUMN_HAS_MIN_WIDTH;
        first.min_width = 15;
    } else if (mode == 7) {
        first.flags = RICH_COLUMN_HAS_JUSTIFY;
        first.justify = RICH_JUSTIFY_RIGHT;
    } else if (mode == 8) {
        first.flags = RICH_COLUMN_NO_WRAP;
    }
    assert(rich_table_add_column_options(table, NULL, &first) == RICH_OK);
    assert(rich_table_add_column_options(table, NULL, &second) == RICH_OK);
    RichStr row[] = {rich_str("alpha beta gamma delta"), rich_str("omega")};
    assert(rich_table_add_row_strs(table, row, 2) == RICH_OK);
    assert(rich_table_finish(&table, &renderable) == RICH_OK);
    assert(rich_console_render(console, renderable, &output) == RICH_OK);
    rich_renderable_free(renderable);
    rich_console_free(console);
    return output;
}

int main(void) {
    assert(rich_ffi_abi_version() == RICH_FFI_ABI_VERSION);
    RichConsole *console = NULL;
    assert(rich_console_new(&console) == RICH_OK);
    assert(rich_console_set_force_terminal(console, 0) == RICH_OK);
    assert(rich_console_set_size(console, 24, 24) == RICH_OK);
    assert(rich_console_set_force_terminal(console, 2) == RICH_ERROR_INVALID_ARGUMENT);

    RichBuffer pretty_a_off = render_pretty_probe(
        console, "Demo { nested: [Alpha { value: 1 }, Beta { value: 2 }] }", 0);
    RichBuffer pretty_a_on = render_pretty_probe(
        console, "Demo { nested: [Alpha { value: 1 }, Beta { value: 2 }] }", 1);
    RichBuffer pretty_b_on = render_pretty_probe(
        console, "Other { nested: [Gamma { value: 3 }, Delta { value: 4 }] }", 1);
    assert(!buffers_equal(&pretty_a_off, &pretty_a_on));
    assert(!buffers_equal(&pretty_a_on, &pretty_b_on));
    rich_buffer_free(&pretty_a_off);
    rich_buffer_free(&pretty_a_on);
    rich_buffer_free(&pretty_b_on);

    RichBuffer syntax_off = render_syntax_probe(console, 0);
    RichBuffer syntax_on = render_syntax_probe(console, 1);
    assert(!buffers_equal(&syntax_off, &syntax_on));
    rich_buffer_free(&syntax_off);
    rich_buffer_free(&syntax_on);

    RichBuffer table_baseline = render_table_probe(0);
    for (int mode = 1; mode <= 8; ++mode) {
        RichBuffer changed = render_table_probe(mode);
        assert(!buffers_equal(&table_baseline, &changed));
        rich_buffer_free(&changed);
    }
    rich_buffer_free(&table_baseline);

    RichText *text = NULL;
    assert(rich_text_new(rich_str("nested"), &text) == RICH_OK);
    RichRenderable *content = NULL;
    assert(rich_text_finish(&text, &content) == RICH_OK && text == NULL);

    RichPanel *panel = NULL;
    assert(rich_panel_new(content, &panel) == RICH_OK);
    assert(rich_panel_set_title(panel, rich_str("C11")) == RICH_OK);
    RichRenderable *nested = NULL;
    assert(rich_panel_finish(&panel, &nested) == RICH_OK && panel == NULL);

    RichBuffer output = {NULL, 0};
    assert(rich_console_render(console, nested, &output) == RICH_OK);
    assert(output.data != NULL && output.len != 0);
    rich_buffer_free(&output);
    assert(output.data == NULL && output.len == 0);

    assert(rich_console_render(console, content, &output) == RICH_OK);
    assert(output.len == 6 && memcmp(output.data, "nested", 6) == 0);
    rich_buffer_free(&output);

    probe_colorbox(4, 1);
    probe_colorbox(9, 1);
    probe_colorbox(4, 3);
    probe_colorbox(9, 3);

    CustomContext *custom_context = (CustomContext *)malloc(sizeof(*custom_context));
    assert(custom_context != NULL);
    custom_context->height = 1;
    RichCustomRenderableVTable vtable = {render_custom, measure_custom, drop_custom};
    RichRenderable *custom = NULL;
    assert(rich_custom_renderable_new(custom_context, &vtable, &custom) == RICH_OK);

    RichTable *grid = NULL;
    assert(rich_table_new_grid(&grid) == RICH_OK);
    assert(rich_table_set_box(grid, RICH_BOX_NONE) == RICH_OK);
    assert(rich_table_set_pad_edge(grid, 0) == RICH_OK);
    RichStr row_styles[] = {rich_str(""), rich_str("dim")};
    assert(rich_table_set_row_styles(grid, row_styles, 2) == RICH_OK);
    RichTableColumnOptions options = {0};
    options.flags = RICH_COLUMN_HAS_JUSTIFY | RICH_COLUMN_HAS_RATIO | RICH_COLUMN_NO_WRAP;
    options.justify = RICH_JUSTIFY_RIGHT;
    options.ratio = 1;
    assert(rich_table_add_column_options(grid, NULL, &options) == RICH_OK);
    const RichRenderable *custom_cells[] = {custom};
    assert(rich_table_add_row_renderables(grid, custom_cells, 1) == RICH_OK);
    RichRenderable *grid_renderable = NULL;
    assert(rich_table_finish(&grid, &grid_renderable) == RICH_OK);
    rich_renderable_free(custom);
    assert(rich_console_render(console, grid_renderable, &output) == RICH_OK);
    assert(output.len > 0);
    rich_buffer_free(&output);

    RichPretty *pretty = NULL;
    assert(rich_pretty_new(rich_str("Demo { values: [1, 2] }"), &pretty) == RICH_OK);
    assert(rich_pretty_set_indent_guides(pretty, 1) == RICH_OK);
    RichRenderable *pretty_renderable = NULL;
    assert(rich_pretty_finish(&pretty, &pretty_renderable) == RICH_OK);
    assert(rich_console_render(console, pretty_renderable, &output) == RICH_OK);
    rich_buffer_free(&output);

    RichSyntax *syntax = NULL;
    assert(rich_syntax_new(rich_str("if true {\n    run();\n}"), rich_str("rust"), &syntax) == RICH_OK);
    assert(rich_syntax_set_indent_guides(syntax, 1) == RICH_OK);
    RichRenderable *syntax_renderable = NULL;
    assert(rich_syntax_finish(&syntax, &syntax_renderable) == RICH_OK);
    assert(rich_console_render(console, syntax_renderable, &output) == RICH_OK);
    rich_buffer_free(&output);

    rich_renderable_free(syntax_renderable);
    rich_renderable_free(pretty_renderable);
    rich_renderable_free(grid_renderable);
    rich_renderable_free(nested);
    rich_renderable_free(content);
    rich_console_free(console);
    return 0;
}
