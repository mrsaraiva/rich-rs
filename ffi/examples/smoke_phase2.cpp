#include "rich.hpp"
#include <cassert>

int main() {
    rich::Console console;
    console.set_force_terminal(false);
    auto styled = rich::Text::markup(console, "[green]up[/]").finish();
    rich::Table table;
    table.add_column("peer").add_column(styled).add_row({"alice", "up"});
    auto renderable = table.finish();
    assert(console.render(renderable).find("alice") != std::string::npos);
    assert(console.render(styled).find("up") != std::string::npos);
}
