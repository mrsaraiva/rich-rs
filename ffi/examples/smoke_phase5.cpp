#include "rich.hpp"
#include <cassert>

int main() {
    rich::Console console;
    console.set_force_terminal(false).set_size(40, 10);
    auto child = rich::Text("child").finish();
    rich::Padding padding(child, 1, 2, 1, 2);
    auto padded = padding.set_expand(false).finish();
    auto aligned = rich::AlignBox::center(padded).set_width(30).finish();
    rich::Rule rule;
    auto divider = rule.set_title("section").set_align(rich::Align::Center).finish();
    rich::Columns columns;
    columns.add(aligned).add(divider).add(child);
    assert(console.render(columns.finish()).find("child") != std::string::npos);
}
