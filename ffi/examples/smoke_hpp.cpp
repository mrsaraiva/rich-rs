#include "rich.hpp"
#include <cassert>
#include <memory>

class TestRenderer final : public rich::CustomRenderer {
public:
    void render(rich::RenderOptions options, rich::SegmentSink& sink) override {
        assert(options.max_width > 0);
        if (fail) throw std::runtime_error("callback failure");
        retained_sink = sink;
        if (move_target != nullptr && move_source != nullptr) {
            try { *move_target = std::move(*move_source); }
            catch (const rich::Exception& error) {
                move_rejected = error.code() == RICH_ERROR_INVALID_STATE;
            }
        }
        if (tree_target != nullptr && tree_source != nullptr) {
            try { *tree_target = std::move(*tree_source); }
            catch (const rich::Exception& error) {
                tree_move_rejected = error.code() == RICH_ERROR_INVALID_STATE;
            }
        }
        sink.push("custom", "bold green");
        sink.line();
    }
    rich::Measurement measure(rich::RenderOptions options) override {
        return rich::Measurement{1, options.max_width};
    }
    bool fail = false;
    std::optional<rich::SegmentSink> retained_sink;
    rich::Renderable* move_target = nullptr;
    rich::Renderable* move_source = nullptr;
    bool move_rejected = false;
    rich::Tree* tree_target = nullptr;
    rich::Tree* tree_source = nullptr;
    bool tree_move_rejected = false;
};

class ProbeRenderer final : public rich::CustomRenderer {
public:
    explicit ProbeRenderer(std::uint32_t value) : height(value) {}
    void render(rich::RenderOptions options, rich::SegmentSink& sink) override {
        for (std::uint32_t y = 0; y < height; ++y) {
            for (std::uint32_t x = 0; x < options.max_width; ++x) {
                sink.push("▄", "rgb(" + std::to_string(x % 255) + "," +
                    std::to_string(y % 255) + ",127)");
            }
            sink.line();
        }
    }
private:
    std::uint32_t height;
};

std::size_t occurrences(const std::string& value, std::string_view needle) {
    std::size_t count = 0;
    for (std::size_t offset = 0; (offset = value.find(needle, offset)) != std::string::npos;
         offset += needle.size()) {
        ++count;
    }
    return count;
}

void probe_colorbox(std::uint32_t width, std::uint32_t height) {
    rich::Console console;
    console.set_size(width, 20)
        .set_force_terminal(true)
        .set_color_system(rich::ColorSystem::TrueColor);
    auto renderable = rich::custom_renderable(std::make_shared<ProbeRenderer>(height));
    const auto output = console.render(renderable);
    assert(occurrences(output, "▄") == width * height);
    assert(output.find("\x1b[38;2") != std::string::npos);
}

std::string render_table_probe(int mode) {
    rich::Console console;
    console.set_size(24, 20).set_force_terminal(true);
    rich::Table table;
    table.set_expand(true);
    if (mode == 1) table.set_box(rich::Box::None);
    if (mode == 2) table.set_pad_edge(false);
    if (mode == 3) table.set_row_styles({"red"});
    rich::ColumnOptions first;
    rich::ColumnOptions second;
    if (mode == 4) { first.ratio = 3; second.ratio = 1; }
    if (mode == 5) first.width = 5;
    if (mode == 6) first.min_width = 15;
    if (mode == 7) first.justify = rich::Justify::Right;
    if (mode == 8) first.no_wrap = true;
    table.add_column(first).add_column(second)
        .add_row({"alpha beta gamma delta", "omega"});
    return console.render(table.finish());
}

int main() {
    probe_colorbox(4, 1);
    probe_colorbox(9, 1);
    probe_colorbox(4, 3);
    probe_colorbox(9, 3);

    rich::Console console;
    console.set_size(24, 24).set_force_terminal(false);

    const auto pretty_off = console.render(rich::Pretty(
        "Demo { nested: [Alpha { value: 1 }, Beta { value: 2 }] }").finish());
    const auto pretty_on = console.render(rich::Pretty(
        "Demo { nested: [Alpha { value: 1 }, Beta { value: 2 }] }")
        .set_indent_guides(true).finish());
    const auto pretty_other = console.render(rich::Pretty(
        "Other { nested: [Gamma { value: 3 }, Delta { value: 4 }] }")
        .set_indent_guides(true).finish());
    assert(pretty_off != pretty_on);
    assert(pretty_on != pretty_other);

    const auto syntax_off = console.render(rich::Syntax(
        "fn main() {\n    if true {\n        run();\n    }\n}", "rust").finish());
    const auto syntax_on = console.render(rich::Syntax(
        "fn main() {\n    if true {\n        run();\n    }\n}", "rust")
        .set_indent_guides(true).finish());
    assert(syntax_off != syntax_on);

    const auto table_baseline = render_table_probe(0);
    for (int mode = 1; mode <= 8; ++mode) assert(table_baseline != render_table_probe(mode));
    console.set_size(80, 24);
    auto content = rich::Text::markup(console, "[bold]shared[/]").finish();

    rich::Panel panel(content);
    panel.set_title("borrowed").set_box(rich::Box::Rounded);
    auto panel_renderable = panel.finish();

    rich::Columns columns;
    columns.add(content).add(panel_renderable).set_padding(0, 1);
    auto nested = columns.finish();
    assert(console.render(nested).find("shared") != std::string::npos);
    assert(console.render(content).find("shared") != std::string::npos);

    auto grid = rich::Table::grid();
    grid.set_box(rich::Box::None)
        .set_pad_edge(false)
        .set_row_styles({"", "dim"});
    rich::ColumnOptions options;
    options.justify = rich::Justify::Right;
    options.vertical = rich::VAlign::Middle;
    options.width = 12;
    options.min_width = 3;
    options.ratio = 1;
    options.style = "cyan";
    options.no_wrap = true;
    grid.add_column(options).add_row({"configured"});
    auto configured = grid.finish();
    assert(console.render(configured).find("configured") != std::string::npos);

    auto pretty = rich::Pretty("Demo { nested: [1, 2, 3] }")
        .set_indent_guides(true)
        .finish();
    assert(console.render(pretty).find("Demo") != std::string::npos);

    auto syntax = rich::Syntax("if true {\n    value();\n}", "rust")
        .set_indent_guides(true)
        .finish();
    assert(console.render(syntax).find("value") != std::string::npos);

    auto renderer = std::make_shared<TestRenderer>();
    auto move_target = rich::Text("move target").finish();
    auto move_source = rich::Text("move source").finish();
    renderer->move_target = &move_target;
    renderer->move_source = &move_source;
    rich::Tree tree_move_target("tree target");
    rich::Tree tree_move_source("tree source");
    renderer->tree_target = &tree_move_target;
    renderer->tree_source = &tree_move_source;
    auto custom = rich::custom_renderable(renderer);
    assert(console.render(custom).find("custom") != std::string::npos);
    assert(renderer->move_rejected);
    assert(renderer->tree_move_rejected);
    assert(console.render(move_target).find("move target") != std::string::npos);
    assert(console.render(move_source).find("move source") != std::string::npos);
    assert(console.render(tree_move_target.finish()).find("tree target") != std::string::npos);
    assert(console.render(tree_move_source.finish()).find("tree source") != std::string::npos);
    bool stale_sink_caught = false;
    try { renderer->retained_sink->line(); }
    catch (const rich::Exception& error) {
        stale_sink_caught = error.code() == RICH_ERROR_INVALID_STATE;
    }
    assert(stale_sink_caught);
    renderer->fail = true;
    bool callback_caught = false;
    try { (void)console.render(custom); }
    catch (const std::runtime_error& error) {
        callback_caught = std::string(error.what()) == "callback failure";
    }
    assert(callback_caught);

    rich::Tree tree("root");
    auto stale_after_finish = tree.add("child");
    rich::Tree moved_tree(std::move(tree));
    stale_after_finish.add("valid-after-owner-move");
    auto tree_renderable = moved_tree.finish();
    assert(console.render(tree_renderable).find("valid-after-owner-move") != std::string::npos);
    bool stale_finish_caught = false;
    try { stale_after_finish.add("invalid"); }
    catch (const rich::Exception& error) {
        stale_finish_caught = error.code() == RICH_ERROR_INVALID_STATE;
    }
    assert(stale_finish_caught);

    std::unique_ptr<rich::TreeNode> stale_after_destruction;
    {
        rich::Tree scoped_tree("root");
        stale_after_destruction = std::make_unique<rich::TreeNode>(scoped_tree.add("child"));
    }
    bool stale_destruction_caught = false;
    try { stale_after_destruction->add("invalid"); }
    catch (const rich::Exception& error) {
        stale_destruction_caught = error.code() == RICH_ERROR_INVALID_STATE;
    }
    assert(stale_destruction_caught);

    bool caught = false;
    try { rich::Style invalid("not-a-style"); }
    catch (const rich::Exception& error) {
        caught = error.code() == RICH_ERROR_INVALID_ARGUMENT;
    }
    assert(caught);
    assert(!rich::version().empty());
}
