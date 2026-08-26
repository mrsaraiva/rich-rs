#include "rich.hpp"
#include <cassert>
#include <memory>

int main() {
    rich::Console console;
    console.set_size(80, 24).set_force_terminal(false);
    auto content = rich::Text::markup(console, "[bold]shared[/]").finish();

    rich::Panel panel(content);
    panel.set_title("borrowed").set_box(rich::Box::Rounded);
    auto panel_renderable = panel.finish();

    rich::Columns columns;
    columns.add(content).add(panel_renderable).set_padding(0, 1);
    auto nested = columns.finish();
    assert(console.render(nested).find("shared") != std::string::npos);
    assert(console.render(content).find("shared") != std::string::npos);

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
