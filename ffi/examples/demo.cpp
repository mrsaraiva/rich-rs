#include "rich.hpp"

#include <chrono>
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include <memory>
#include <sstream>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace {

using rich::Box;
using rich::Console;
using rich::Markdown;
using rich::Panel;
using rich::Renderable;
using rich::Syntax;
using rich::Table;
using rich::Text;

Renderable markup(Console& console, std::string_view value) {
    auto text = Text::markup(console, value);
    return text.finish();
}

Renderable styled(std::string_view value, std::string_view style) {
    Text text(value);
    text.set_style(style);
    return text.finish();
}

void add_feature(Table& card, std::string_view name, Renderable demo) {
    auto label = Text(name).finish();
    card.add_row(std::vector<const Renderable*>{&label, &demo});
}

void add_blank(Table& card) {
    auto label = Text("").finish();
    Text empty_text("");
    auto empty = empty_text.finish();
    card.add_row(std::vector<const Renderable*>{&label, &empty});
}

double hue_to_rgb(double p, double q, double value) {
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

struct Rgb {
    unsigned red;
    unsigned green;
    unsigned blue;
};

Rgb hls_to_rgb(double h, double l) {
    const double q = l < 0.5 ? l * (1.0 + 1.0) : l + 1.0 - l * 1.0;
    const double p = 2.0 * l - q;
    return Rgb{
        static_cast<unsigned>(hue_to_rgb(p, q, h + 1.0 / 3.0) * 255.0),
        static_cast<unsigned>(hue_to_rgb(p, q, h) * 255.0),
        static_cast<unsigned>(hue_to_rgb(p, q, h - 1.0 / 3.0) * 255.0),
    };
}

class ColorBox final : public rich::CustomRenderer {
public:
    void render(rich::RenderOptions options, rich::SegmentSink& sink) override {
        for (std::uint32_t y = 0; y < 5; ++y) {
            for (std::uint32_t x = 0; x < options.max_width; ++x) {
            const double h = static_cast<double>(x) / static_cast<double>(options.max_width);
            const double l1 = 0.1 + (static_cast<double>(y) / 5.0) * 0.7;
            const auto background = hls_to_rgb(h, l1);
            const auto foreground = hls_to_rgb(h, l1 + 0.7 / 10.0);
                std::ostringstream style;
                style << "rgb(" << foreground.red << ',' << foreground.green << ','
                      << foreground.blue << ") on rgb(" << background.red << ','
                      << background.green << ',' << background.blue << ')';
                sink.push("▄", style.str());
            }
            sink.line();
        }
    }
};

Renderable make_color_demo(Console& console) {

    auto checklist = markup(console,
        "✓ [bold green]4-bit color[/]\n"
        "✓ [bold blue]8-bit color[/]\n"
        "✓ [bold magenta]Truecolor (16.7 million)[/]\n"
        "✓ [bold yellow]Dumb terminals[/]\n"
        "✓ [bold cyan]Automatic color conversion[/]");
    auto swatches = rich::custom_renderable(std::make_shared<ColorBox>());
    Table table;
    table.set_box(Box::None)
        .set_show_header(false)
        .set_show_edge(false)
        .set_expand(false)
        .set_pad_edge(false)
        .add_row(std::vector<const Renderable*>{&checklist, &swatches});
    return table.finish();
}

Renderable make_text_examples(Console& console) {
    const std::string lorem =
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. "
        "Quisque in metus sed sapien ultricies pretium a at justo. "
        "Maecenas luctus velit et auctor maximus.";
    auto left = styled(lorem, "green");
    auto center = styled(lorem, "yellow");
    auto right = styled(lorem, "blue");
    auto full = styled(lorem, "red");
    Table table = Table::grid();
    table.set_padding(1, 1).set_pad_edge(false);
    for (auto justify : {rich::Justify::Left, rich::Justify::Center,
                         rich::Justify::Right, rich::Justify::Full}) {
        rich::ColumnOptions column;
        column.ratio = 1;
        column.justify = justify;
        table.add_column(column);
    }
    table.add_row(std::vector<const Renderable*>{&left, &center, &right, &full});
    auto lorem_table = table.finish();

    auto intro = markup(console,
        "Word wrap text. Justify [green]left[/], [yellow]center[/], "
        "[blue]right[/] or [red]full[/].\n");
    Text blank_text("");
    auto blank = blank_text.finish();
    Table section = Table::grid();
    section
        .set_padding(0, 0)
        .add_column(rich::ColumnOptions{})
        .add_row(std::vector<const Renderable*>{&intro})
        .add_row(std::vector<const Renderable*>{&blank})
        .add_row(std::vector<const Renderable*>{&lorem_table});
    return section.finish();
}

Renderable make_movie_table(Console& console) {
    Table table;
    table.set_box(Box::Simple)
        .set_show_edge(false)
        .set_expand(false)
        .set_row_styles({"", "dim"});

    auto date_header = markup(console, "[green]Date");
    auto title_header = markup(console, "[blue]Title");
    auto budget_header = markup(console, "[cyan]Production Budget");
    auto office_header = markup(console, "[magenta]Box Office");
    rich::ColumnOptions date;
    date.style = "green"; date.min_width = 12; date.no_wrap = true;
    table.add_column(date, std::cref(date_header));
    rich::ColumnOptions title;
    title.style = "blue"; title.ratio = 1;
    table.add_column(title, std::cref(title_header));
    rich::ColumnOptions budget;
    budget.style = "cyan"; budget.justify = rich::Justify::Right;
    budget.min_width = 17; budget.no_wrap = true;
    table.add_column(budget, std::cref(budget_header));
    rich::ColumnOptions office;
    office.style = "magenta"; office.justify = rich::Justify::Right;
    office.min_width = 14; office.no_wrap = true;
    table.add_column(office, std::cref(office_header));

    const std::vector<std::vector<std::string>> movies{
        {"Dec 20, 2019", "Star Wars: The Rise of Skywalker", "$275,000,000", "$375,126,118"},
        {"May 25, 2018", "[b]Solo[/]: A Star Wars Story", "$275,000,000", "$393,151,347"},
        {"Dec 15, 2017", "Star Wars Ep. VIII: The Last Jedi", "$262,000,000", "[bold]$1,332,539,889[/]"},
        {"May 19, 1999", "Star Wars Ep. [b]I[/b]: [i]The phantom Menace[/i]", "$115,000,000", "$1,027,044,677"},
    };
    for (std::size_t row_index = 0; row_index < movies.size(); ++row_index) {
        std::vector<Renderable> owned;
        owned.reserve(movies[row_index].size());
        for (std::size_t cell_index = 0;
             cell_index < movies[row_index].size();
             ++cell_index) {
            owned.push_back(markup(console, movies[row_index][cell_index]));
        }
        std::vector<const Renderable*> borrowed;
        borrowed.reserve(owned.size());
        for (const auto& cell : owned) {
            borrowed.push_back(&cell);
        }
        table.add_row(borrowed);
    }
    return table.finish();
}

Renderable make_syntax_pretty_comparison() {
    const std::string code =
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
    const std::string pretty_data =
        "{'foo': [3.1427, ('Paul Atreides', 'Vladimir Harkonnen', 'Thufir Hawat')], "
        "'atomic': (False, True, None)}";

    Syntax syntax(code, "python3");
    syntax.set_line_numbers(true).set_indent_guides(true);
    auto syntax_renderable = syntax.finish();
    auto pretty_renderable = rich::Pretty(pretty_data).set_indent_guides(true).finish();

    Table comparison;
    comparison.set_box(Box::None)
        .set_show_header(false)
        .set_expand(true)
        .set_pad_edge(false)
        .set_padding(1, 1);
    rich::ColumnOptions column;
    column.ratio = 1;
    comparison.add_column(column).add_column(column)
        .add_row(std::vector<const Renderable*>{&syntax_renderable, &pretty_renderable});
    return comparison.finish();
}

Renderable make_markdown_comparison() {
    const std::string source =
        "# Markdown\n\n"
        "Supports much of the *markdown* __syntax__!\n\n"
        "- Headers\n"
        "- Basic formatting: **bold**, *italic*, " "\x60" "code" "\x60" "\n"
        "- Block quotes\n"
        "- Lists, and more...\n";

    auto raw = styled(source, "cyan");
    auto rendered = Markdown(source).finish();
    Table comparison;
    comparison.set_box(Box::None)
        .set_show_header(false)
        .set_expand(true)
        .set_pad_edge(false);
    rich::ColumnOptions column;
    column.ratio = 1;
    comparison.add_column(column).add_column(column)
        .add_row(std::vector<const Renderable*>{&raw, &rendered});
    return comparison.finish();
}

Renderable make_test_card(Console& console) {
    Table card = Table::grid();
    card.set_title_markup(console, "[italic]Rich features[/]")
        .set_padding(1, 1)
        .set_pad_edge(true);
    rich::ColumnOptions label;
    label.width = 12;
    label.justify = rich::Justify::Center;
    label.vertical = rich::VAlign::Top;
    label.style = "bold red";
    label.no_wrap = true;
    card.add_column(label).add_column(rich::ColumnOptions{});

    add_feature(card, "Colors", make_color_demo(console));
    add_blank(card);

    add_feature(card, "Styles", markup(console,
        "All ansi styles: [bold]bold[/], [dim]dim[/], [italic]italic[/italic], "
        "[underline]underline[/], [strike]strikethrough[/], [reverse]reverse[/], "
        "and even [blink]blink[/]."));
    add_blank(card);

    add_feature(card, "Text", make_text_examples(console));
    add_blank(card);

    add_feature(card, "Asian\nlanguage\nsupport", markup(console,
        "🇨🇳  该库支持中文，日文和韩文文本！\n"
        "🇯🇵  ライブラリは中国語、日本語、韓国語のテキストをサポートしています\n"
        "🇰🇷  이 라이브러리는 중국어, 일본어 및 한국어 텍스트를 지원합니다"));
    add_blank(card);

    add_feature(card, "Markup", markup(console,
        "[bold magenta]Rich[/] supports a simple [i]bbcode[/i]-like [b]markup[/b] for "
        "[yellow]color[/], [underline]style[/], and emoji! "
        ":+1: :apple: :ant: :bear: :baguette_bread: :bus:"));
    add_blank(card);

    add_feature(card, "Tables", make_movie_table(console));
    add_blank(card);

    add_feature(card, "Syntax\nhighlighting\n&\npretty\nprinting",
                make_syntax_pretty_comparison());
    add_blank(card);

    add_feature(card, "Markdown", make_markdown_comparison());
    add_blank(card);
    add_blank(card);

    Text more("Progress bars, columns, styled logging handler, tracebacks, etc...");
    add_feature(card, "+more!", more.finish());
    return card.finish();
}

Renderable make_sponsor_panel(Console& console) {
    auto body = markup(console,
        "[b magenta]Hope you enjoy using Rich-rs![/]\n\n"
        "Consider sponsoring the upstream port to ensure it's maintained.\n\n"
        "[cyan]https://github.com/sponsors/willmcgugan[/cyan] | [b]Rust port: [/b]"
        "[green]Marcos Saraiva - https://github.com/mrsaraiva/rich-rs[/green]");
    Panel panel(body);
    panel.set_title("Help ensure Rich is maintained")
        .set_border_style("green")
        .set_padding(1, 2, 1, 2);
    return panel.finish();
}

void write_all(const std::string& output) {
    std::cout.write(output.data(), static_cast<std::streamsize>(output.size()));
    if (!std::cout) {
        throw std::runtime_error("failed to write rendered UTF-8");
    }
}

} // namespace

int main(int argc, char** argv) {
    try {
        if (argc > 2 || (argc == 2 && std::string_view(argv[1]) != "--parity")) {
            std::cerr << "usage: demo [--parity]\n";
            return EXIT_FAILURE;
        }
        Console console;
        console.set_force_terminal(true)
            .set_color_system(rich::ColorSystem::TrueColor)
            .set_size(120, 72);

        auto card = make_test_card(console);
        const auto cold_start = std::chrono::steady_clock::now();
        static_cast<void>(console.render(card));
        const auto warm_start = std::chrono::steady_clock::now();
        static_cast<void>(console.render(card));
        const auto finish = std::chrono::steady_clock::now();

        const auto cold_ms =
            std::chrono::duration<double, std::milli>(warm_start - cold_start).count();
        const auto warm_ms =
            std::chrono::duration<double, std::milli>(finish - warm_start).count();

        write_all(console.render(card));
        write_all("\n");
        std::ostringstream timing;
        timing << std::fixed << std::setprecision(1)
               << "[dim]rendered in [not dim][cyan]" << cold_ms
               << "[/cyan]ms[/] (cold cache)";
        write_all(console.render_markup(timing.str()));
        write_all("\n");

        timing.str("");
        timing.clear();
        timing << std::fixed << std::setprecision(1)
               << "[dim]rendered in [not dim][cyan]" << warm_ms
               << "[/cyan]ms[/] (warm cache)";
        write_all(console.render_markup(timing.str()));
        write_all("\n\n");

        auto sponsor = make_sponsor_panel(console);
        write_all(console.render(sponsor));
        std::cout.put('\n');
        return EXIT_SUCCESS;
    } catch (const rich::Exception& error) {
        std::cerr << "rich-ffi call failed with RichResult " << error.code() << '\n';
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
    }
    return EXIT_FAILURE;
}
