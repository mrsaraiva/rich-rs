#include "rich.hpp"
#include <cassert>

int main() {
    rich::Console console;
    console.set_force_terminal(false);
    const auto plain = console.render_markup("[bold green]ready[/]");
    assert(plain == "ready");
    console.set_force_terminal(true);
    assert(console.render_markup("[bold green]ready[/]").find("\x1b[") != std::string::npos);
}
