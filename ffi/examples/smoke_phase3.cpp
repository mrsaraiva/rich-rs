#include "rich.hpp"
#include <cassert>

int main() {
    rich::Console console;
    console.set_force_terminal(false);
    auto content = rich::Text("tunnel up").finish();
    rich::Panel panel(content);
    panel.set_title("status").set_border_style("bold cyan").set_padding(0, 1, 0, 1);
    auto result = panel.finish();
    assert(console.render(result).find("tunnel up") != std::string::npos);
    assert(console.render(content) == "tunnel up");
}
