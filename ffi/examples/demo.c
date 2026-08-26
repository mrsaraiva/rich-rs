#define _POSIX_C_SOURCE 200809L

#include "rich.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static RichStr str_ref(const char *value) {
    RichStr result;
    result.data = (const uint8_t *)value;
    result.len = (uint64_t)strlen(value);
    return result;
}

static void check_result(RichResult result, const char *operation) {
    if (result != RICH_OK) {
        fprintf(stderr, "%s failed with RichResult %u\n", operation, (unsigned)result);
        exit(EXIT_FAILURE);
    }
}

#define CHECK(call) check_result((call), #call)

static RichRenderable *finish_text(RichText *text) {
    RichRenderable *result = NULL;
    CHECK(rich_text_finish(&text, &result));
    return result;
}

static RichRenderable *plain_text(const char *value) {
    RichText *text = NULL;
    CHECK(rich_text_new(str_ref(value), &text));
    return finish_text(text);
}

static RichRenderable *styled_text(const char *value, const char *style) {
    RichText *text = NULL;
    CHECK(rich_text_new(str_ref(value), &text));
    CHECK(rich_text_set_style(text, str_ref(style)));
    return finish_text(text);
}

static RichRenderable *markup_text(RichConsole *console, const char *value) {
    RichText *text = NULL;
    CHECK(rich_text_new_markup(console, str_ref(value), &text));
    return finish_text(text);
}

static RichRenderable *finish_table(RichTable *table) {
    RichRenderable *result = NULL;
    CHECK(rich_table_finish(&table, &result));
    return result;
}

static RichTable *new_two_column_table(void) {
    RichTable *table = NULL;
    CHECK(rich_table_new(&table));
    CHECK(rich_table_set_box(table, RICH_BOX_NONE));
    CHECK(rich_table_set_show_header(table, 0));
    CHECK(rich_table_set_expand(table, 1));
    CHECK(rich_table_set_pad_edge(table, 0));
    CHECK(rich_table_set_padding(table, 1, 1));
    RichTableColumnOptions options = {0};
    options.flags = RICH_COLUMN_HAS_RATIO;
    options.ratio = 1;
    CHECK(rich_table_add_column_options(table, NULL, &options));
    CHECK(rich_table_add_column_options(table, NULL, &options));
    return table;
}

static void add_feature(RichTable *card, const char *name, RichRenderable *demo) {
    RichRenderable *label = styled_text(name, "bold red");
    const RichRenderable *cells[2] = {label, demo};
    CHECK(rich_table_add_row_renderables(card, cells, 2));
    rich_renderable_free(label);
    rich_renderable_free(demo);
}

static void add_blank(RichTable *card) {
    RichRenderable *label = styled_text("             ", "bold red");
    RichRenderable *empty = plain_text("");
    const RichRenderable *cells[2] = {label, empty};
    CHECK(rich_table_add_row_renderables(card, cells, 2));
    rich_renderable_free(label);
    rich_renderable_free(empty);
}

static double hue_to_rgb(double p, double q, double value) {
    if (value < 0.0) {
        value += 1.0;
    }
    if (value > 1.0) {
        value -= 1.0;
    }
    if (value < 1.0 / 6.0) {
        return p + (q - p) * 6.0 * value;
    }
    if (value < 0.5) {
        return q;
    }
    if (value < 2.0 / 3.0) {
        return p + (q - p) * (2.0 / 3.0 - value) * 6.0;
    }
    return p;
}

static void hls_to_rgb(double h, double l, uint8_t *red, uint8_t *green, uint8_t *blue) {
    const double q = l < 0.5 ? l * (1.0 + 1.0) : l + 1.0 - l * 1.0;
    const double p = 2.0 * l - q;
    *red = (uint8_t)(hue_to_rgb(p, q, h + 1.0 / 3.0) * 255.0);
    *green = (uint8_t)(hue_to_rgb(p, q, h) * 255.0);
    *blue = (uint8_t)(hue_to_rgb(p, q, h - 1.0 / 3.0) * 255.0);
}

static RichResult color_render(
    void *context,
    const RichRenderOptions *options,
    RichSegmentSink *sink) {
    (void)context;
    for (uint32_t y = 0; y < 5; ++y) {
        for (uint32_t x = 0; x < options->max_width; ++x) {
            const double h = (double)x / (double)options->max_width;
            const double l1 = 0.1 + ((double)y / 5.0) * 0.7;
            const double l2 = l1 + 0.7 / 10.0;
            uint8_t r1;
            uint8_t g1;
            uint8_t b1;
            uint8_t r2;
            uint8_t g2;
            uint8_t b2;
            hls_to_rgb(h, l1, &r1, &g1, &b1);
            hls_to_rgb(h, l2, &r2, &g2, &b2);
            char style[96];
            const int count = snprintf(style, sizeof(style),
                "rgb(%u,%u,%u) on rgb(%u,%u,%u)",
                (unsigned)r2,
                (unsigned)g2,
                (unsigned)b2,
                (unsigned)r1,
                (unsigned)g1,
                (unsigned)b1);
            if (count < 0 || (size_t)count >= sizeof(style)) return RICH_ERROR_RENDER;
            RichResult result = rich_segment_sink_push(sink, str_ref("▄"), str_ref(style));
            if (result != RICH_OK) return result;
        }
        RichResult result = rich_segment_sink_line(sink);
        if (result != RICH_OK) return result;
    }
    return RICH_OK;
}

static RichResult color_measure(
    void *context,
    const RichRenderOptions *options,
    RichMeasurement *measurement) {
    (void)context;
    measurement->minimum = 1;
    measurement->maximum = options->max_width;
    return RICH_OK;
}

static void color_drop(void *context) {
    free(context);
}

static RichRenderable *make_color_demo(RichConsole *console) {

    RichRenderable *checklist = markup_text(console,
        "✓ [bold green]4-bit color[/]\n"
        "✓ [bold blue]8-bit color[/]\n"
        "✓ [bold magenta]Truecolor (16.7 million)[/]\n"
        "✓ [bold yellow]Dumb terminals[/]\n"
        "✓ [bold cyan]Automatic color conversion[/]");
    void *context = malloc(1);
    if (context == NULL) {
        fputs("failed to allocate ColorBox context\n", stderr);
        exit(EXIT_FAILURE);
    }
    RichCustomRenderableVTable vtable = {color_render, color_measure, color_drop};
    RichRenderable *gradient = NULL;
    RichResult custom_result = rich_custom_renderable_new(context, &vtable, &gradient);
    if (custom_result != RICH_OK) free(context);
    check_result(custom_result, "rich_custom_renderable_new");

    RichTable *table = NULL;
    CHECK(rich_table_new(&table));
    CHECK(rich_table_set_box(table, RICH_BOX_NONE));
    CHECK(rich_table_set_show_header(table, 0));
    CHECK(rich_table_set_show_edge(table, 0));
    CHECK(rich_table_set_expand(table, 0));
    CHECK(rich_table_set_pad_edge(table, 0));
    const RichRenderable *cells[2] = {checklist, gradient};
    CHECK(rich_table_add_row_renderables(table, cells, 2));
    rich_renderable_free(checklist);
    rich_renderable_free(gradient);
    return finish_table(table);
}

static RichRenderable *make_text_examples(RichConsole *console) {
    static const char lorem[] =
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. "
        "Quisque in metus sed sapien ultricies pretium a at justo. "
        "Maecenas luctus velit et auctor maximus.";
    RichTable *table = NULL;
    CHECK(rich_table_new_grid(&table));
    CHECK(rich_table_set_padding(table, 1, 1));
    CHECK(rich_table_set_pad_edge(table, 0));
    for (size_t index = 0; index < 4; ++index) {
        RichTableColumnOptions options = {0};
        options.flags = RICH_COLUMN_HAS_RATIO | RICH_COLUMN_HAS_JUSTIFY;
        options.ratio = 1;
        options.justify = (uint32_t[]){RICH_JUSTIFY_LEFT, RICH_JUSTIFY_CENTER,
                                      RICH_JUSTIFY_RIGHT, RICH_JUSTIFY_FULL}[index];
        CHECK(rich_table_add_column_options(table, NULL, &options));
    }
    RichRenderable *paragraphs[4] = {
        styled_text(lorem, "green"),
        styled_text(lorem, "yellow"),
        styled_text(lorem, "blue"),
        styled_text(lorem, "red"),
    };
    const RichRenderable *borrowed[4] = {
        paragraphs[0], paragraphs[1], paragraphs[2], paragraphs[3],
    };
    CHECK(rich_table_add_row_renderables(table, borrowed, 4));
    for (size_t index = 0; index < 4; ++index) {
        rich_renderable_free(paragraphs[index]);
    }
    RichRenderable *lorem_table = finish_table(table);

    RichTable *section = NULL;
    CHECK(rich_table_new_grid(&section));
    CHECK(rich_table_set_padding(section, 0, 0));
    RichTableColumnOptions section_column = {0};
    CHECK(rich_table_add_column_options(section, NULL, &section_column));
    RichRenderable *intro = markup_text(
        console,
        "Word wrap text. Justify [green]left[/], [yellow]center[/], "
        "[blue]right[/] or [red]full[/].\n");
    const RichRenderable *intro_cell[1] = {intro};
    CHECK(rich_table_add_row_renderables(section, intro_cell, 1));
    rich_renderable_free(intro);
    const RichStr blank[1] = {str_ref("")};
    CHECK(rich_table_add_row_strs(section, blank, 1));
    const RichRenderable *lorem_cell[1] = {lorem_table};
    CHECK(rich_table_add_row_renderables(section, lorem_cell, 1));
    rich_renderable_free(lorem_table);
    return finish_table(section);
}

static RichRenderable *make_movie_table(RichConsole *console) {
    RichTable *table = NULL;
    CHECK(rich_table_new(&table));
    CHECK(rich_table_set_box(table, RICH_BOX_SIMPLE));
    CHECK(rich_table_set_show_edge(table, 0));
    CHECK(rich_table_set_expand(table, 0));
    RichStr row_styles[2] = {str_ref(""), str_ref("dim")};
    CHECK(rich_table_set_row_styles(table, row_styles, 2));

    const char *headers[4] = {
        "[green]Date", "[blue]Title", "[cyan]Production Budget", "[magenta]Box Office",
    };
    static const char *column_colors[4] = {"green", "blue", "cyan", "magenta"};
    for (size_t index = 0; index < 4; ++index) {
        RichRenderable *header = markup_text(console, headers[index]);
        RichTableColumnOptions options = {0};
        options.style = str_ref(column_colors[index]);
        options.flags = index == 0
            ? RICH_COLUMN_HAS_MIN_WIDTH | RICH_COLUMN_NO_WRAP
            : index == 1
                ? RICH_COLUMN_HAS_RATIO
                : RICH_COLUMN_HAS_JUSTIFY | RICH_COLUMN_HAS_MIN_WIDTH | RICH_COLUMN_NO_WRAP;
        options.min_width = index == 0 ? 12 : index == 2 ? 17 : index == 3 ? 14 : 0;
        options.ratio = index == 1 ? 1 : 0;
        options.justify = index >= 2 ? RICH_JUSTIFY_RIGHT : 0;
        CHECK(rich_table_add_column_options(table, header, &options));
        rich_renderable_free(header);
    }

    static const char *movies[][4] = {
        {"Dec 20, 2019", "Star Wars: The Rise of Skywalker", "$275,000,000", "$375,126,118"},
        {"May 25, 2018", "[b]Solo[/]: A Star Wars Story", "$275,000,000", "$393,151,347"},
        {"Dec 15, 2017", "Star Wars Ep. VIII: The Last Jedi", "$262,000,000", "[bold]$1,332,539,889[/]"},
        {"May 19, 1999", "Star Wars Ep. [b]I[/b]: [i]The phantom Menace[/i]", "$115,000,000", "$1,027,044,677"},
    };
    for (size_t row_index = 0; row_index < sizeof(movies) / sizeof(movies[0]); ++row_index) {
        RichRenderable *owned[4] = {NULL, NULL, NULL, NULL};
        const RichRenderable *borrowed[4];
        for (size_t cell_index = 0; cell_index < 4; ++cell_index) {
            owned[cell_index] = markup_text(console, movies[row_index][cell_index]);
            borrowed[cell_index] = owned[cell_index];
        }
        CHECK(rich_table_add_row_renderables(table, borrowed, 4));
        for (size_t cell_index = 0; cell_index < 4; ++cell_index) {
            rich_renderable_free(owned[cell_index]);
        }
    }
    return finish_table(table);
}

static RichRenderable *make_syntax_json_comparison(void) {
    static const char code[] =
        "def iter_last(values: Iterable[T]) -> Iterable[Tuple[bool, T]]:\n"
        "    \"\"\"Iterate and generate a tuple with a flag for last value.\"\"\"\n"
        "    iter_values = iter(values)\n"
        "    try:\n"
        "        previous_value = next(iter_values)\n"
        "    except StopIteration:\n"
        "        return\n"
        "    for value in iter_values:\n"
        "        yield False, previous_value\n"
        "        previous_value = value\n"
        "    yield True, previous_value";
    static const char pretty_data[] =
        "{'foo': [3.1427, ('Paul Atreides', 'Vladimir Harkonnen', 'Thufir Hawat')], "
        "'atomic': (False, True, None)}";

    RichSyntax *syntax = NULL;
    RichRenderable *syntax_renderable = NULL;
    CHECK(rich_syntax_new(str_ref(code), str_ref("python3"), &syntax));
    CHECK(rich_syntax_set_line_numbers(syntax, 1));
    CHECK(rich_syntax_set_indent_guides(syntax, 1));
    CHECK(rich_syntax_finish(&syntax, &syntax_renderable));

    RichPretty *pretty = NULL;
    RichRenderable *pretty_renderable = NULL;
    CHECK(rich_pretty_new(str_ref(pretty_data), &pretty));
    CHECK(rich_pretty_set_indent_guides(pretty, 1));
    CHECK(rich_pretty_finish(&pretty, &pretty_renderable));

    RichTable *comparison = new_two_column_table();
    const RichRenderable *cells[2] = {syntax_renderable, pretty_renderable};
    CHECK(rich_table_add_row_renderables(comparison, cells, 2));
    rich_renderable_free(syntax_renderable);
    rich_renderable_free(pretty_renderable);
    return finish_table(comparison);
}

static RichRenderable *make_markdown_comparison(void) {
    static const char source[] =
        "# Markdown\n\n"
        "Supports much of the *markdown* __syntax__!\n\n"
        "- Headers\n"
        "- Basic formatting: **bold**, *italic*, " "\x60" "code" "\x60" "\n"
        "- Block quotes\n"
        "- Lists, and more...\n";

    RichRenderable *raw = styled_text(source, "cyan");
    RichMarkdown *markdown = NULL;
    RichRenderable *rendered = NULL;
    CHECK(rich_markdown_new(str_ref(source), &markdown));
    CHECK(rich_markdown_finish(&markdown, &rendered));

    RichTable *comparison = new_two_column_table();
    const RichRenderable *cells[2] = {raw, rendered};
    CHECK(rich_table_add_row_renderables(comparison, cells, 2));
    rich_renderable_free(raw);
    rich_renderable_free(rendered);
    return finish_table(comparison);
}

static RichRenderable *make_test_card(RichConsole *console) {
    RichTable *card = NULL;
    CHECK(rich_table_new_grid(&card));
    CHECK(rich_table_set_title_markup(card, console, str_ref("[italic]Rich features[/]")));
    CHECK(rich_table_set_padding(card, 1, 1));
    CHECK(rich_table_set_pad_edge(card, 1));
    RichTableColumnOptions label = {0};
    label.flags = RICH_COLUMN_HAS_WIDTH | RICH_COLUMN_HAS_JUSTIFY |
                  RICH_COLUMN_HAS_VERTICAL | RICH_COLUMN_NO_WRAP;
    label.width = 12;
    label.justify = RICH_JUSTIFY_CENTER;
    label.vertical = RICH_VERTICAL_TOP;
    label.style = str_ref("bold red");
    RichTableColumnOptions demo = {0};
    CHECK(rich_table_add_column_options(card, NULL, &label));
    CHECK(rich_table_add_column_options(card, NULL, &demo));

    add_feature(card, "Colors", make_color_demo(console));
    add_blank(card);

    add_feature(card, "Styles", markup_text(console,
        "All ansi styles: [bold]bold[/], [dim]dim[/], [italic]italic[/italic], "
        "[underline]underline[/], [strike]strikethrough[/], [reverse]reverse[/], "
        "and even [blink]blink[/]."));
    add_blank(card);

    add_feature(card, "Text", make_text_examples(console));
    add_blank(card);

    add_feature(card, "Asian\nlanguage\nsupport", markup_text(console,
        "🇨🇳  该库支持中文，日文和韩文文本！\n"
        "🇯🇵  ライブラリは中国語、日本語、韓国語のテキストをサポートしています\n"
        "🇰🇷  이 라이브러리는 중국어, 일본어 및 한국어 텍스트를 지원합니다"));
    add_blank(card);

    add_feature(card, "Markup", markup_text(console,
        "[bold magenta]Rich[/] supports a simple [i]bbcode[/i]-like [b]markup[/b] for "
        "[yellow]color[/], [underline]style[/], and emoji! "
        ":+1: :apple: :ant: :bear: :baguette_bread: :bus:"));
    add_blank(card);

    add_feature(card, "Tables", make_movie_table(console));
    add_blank(card);

    add_feature(card, "Syntax\nhighlighting\n&\npretty\nprinting",
                make_syntax_json_comparison());
    add_blank(card);

    add_feature(card, "Markdown", make_markdown_comparison());
    add_blank(card);
    add_blank(card);

    add_feature(card, "+more!", plain_text(
        "Progress bars, columns, styled logging handler, tracebacks, etc..."));
    return finish_table(card);
}

static double elapsed_ms(struct timespec start, struct timespec end) {
    return (double)(end.tv_sec - start.tv_sec) * 1000.0 +
           (double)(end.tv_nsec - start.tv_nsec) / 1000000.0;
}

static void discard_render(RichConsole *console, const RichRenderable *renderable) {
    RichBuffer output = {NULL, 0};
    CHECK(rich_console_render(console, renderable, &output));
    rich_buffer_free(&output);
}

static void write_render(RichConsole *console, const RichRenderable *renderable) {
    RichBuffer output = {NULL, 0};
    CHECK(rich_console_render(console, renderable, &output));
    if (output.len > 0 && fwrite(output.data, 1, (size_t)output.len, stdout) != (size_t)output.len) {
        fputs("failed to write rendered UTF-8\n", stderr);
        rich_buffer_free(&output);
        exit(EXIT_FAILURE);
    }
    rich_buffer_free(&output);
}

static void write_markup(RichConsole *console, const char *markup) {
    RichBuffer output = {NULL, 0};
    CHECK(rich_console_render_markup(console, str_ref(markup), &output));
    if (output.len > 0 && fwrite(output.data, 1, (size_t)output.len, stdout) != (size_t)output.len) {
        fputs("failed to write rendered UTF-8\n", stderr);
        rich_buffer_free(&output);
        exit(EXIT_FAILURE);
    }
    rich_buffer_free(&output);
}

static RichRenderable *make_sponsor_panel(RichConsole *console) {
    RichRenderable *body = markup_text(console,
        "[b magenta]Hope you enjoy using Rich-rs![/]\n\n"
        "Consider sponsoring the upstream port to ensure it's maintained.\n\n"
        "[cyan]https://github.com/sponsors/willmcgugan[/cyan] | [b]Rust port: [/b]"
        "[green]Marcos Saraiva - https://github.com/mrsaraiva/rich-rs[/green]");
    RichPanel *panel = NULL;
    RichRenderable *result = NULL;
    CHECK(rich_panel_new(body, &panel));
    rich_renderable_free(body);
    CHECK(rich_panel_set_title(panel, str_ref("Help ensure Rich is maintained")));
    CHECK(rich_panel_set_border_style(panel, str_ref("green")));
    CHECK(rich_panel_set_padding(panel, 1, 2, 1, 2));
    CHECK(rich_panel_finish(&panel, &result));
    return result;
}

int main(int argc, char **argv) {
    if (argc > 2 || (argc == 2 && strcmp(argv[1], "--parity") != 0)) {
        fputs("usage: demo [--parity]\n", stderr);
        return EXIT_FAILURE;
    }
    if (rich_ffi_abi_version() != RICH_FFI_ABI_VERSION) {
        fprintf(stderr, "rich FFI ABI mismatch: native=%u header=%u\n",
                (unsigned)rich_ffi_abi_version(), (unsigned)RICH_FFI_ABI_VERSION);
        return EXIT_FAILURE;
    }

    RichConsole *console = NULL;
    CHECK(rich_console_new(&console));
    CHECK(rich_console_set_force_terminal(console, 1));
    CHECK(rich_console_set_color_system(console, RICH_COLOR_TRUECOLOR));
    CHECK(rich_console_set_size(console, 120, 72));

    RichRenderable *card = make_test_card(console);
    struct timespec start;
    struct timespec middle;
    struct timespec end;
    if (clock_gettime(CLOCK_MONOTONIC, &start) != 0) {
        perror("clock_gettime");
        return EXIT_FAILURE;
    }
    discard_render(console, card);
    if (clock_gettime(CLOCK_MONOTONIC, &middle) != 0) {
        perror("clock_gettime");
        return EXIT_FAILURE;
    }
    discard_render(console, card);
    if (clock_gettime(CLOCK_MONOTONIC, &end) != 0) {
        perror("clock_gettime");
        return EXIT_FAILURE;
    }

    write_render(console, card);
    fputc('\n', stdout);
    char timing[256];
    int written = snprintf(timing, sizeof(timing),
        "[dim]rendered in [not dim][cyan]%.1f[/cyan]ms[/] (cold cache)",
        elapsed_ms(start, middle));
    if (written < 0 || (size_t)written >= sizeof(timing)) {
        fputs("failed to format timing output\n", stderr);
        return EXIT_FAILURE;
    }
    write_markup(console, timing);
    fputc('\n', stdout);

    written = snprintf(timing, sizeof(timing),
        "[dim]rendered in [not dim][cyan]%.1f[/cyan]ms[/] (warm cache)",
        elapsed_ms(middle, end));
    if (written < 0 || (size_t)written >= sizeof(timing)) {
        fputs("failed to format timing output\n", stderr);
        return EXIT_FAILURE;
    }
    write_markup(console, timing);
    fputs("\n\n", stdout);

    RichRenderable *sponsor = make_sponsor_panel(console);
    write_render(console, sponsor);
    fputc('\n', stdout);

    rich_renderable_free(sponsor);
    rich_renderable_free(card);
    rich_console_free(console);
    return EXIT_SUCCESS;
}
