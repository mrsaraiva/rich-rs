// smoke.cpp — proves the rich-rs C ABI links and renders from C++.
//
// Build (from ffi/, after `cargo build`):
//   g++ -std=c++17 -Iinclude examples/smoke.cpp \
//       target/debug/librich_ffi.a -lpthread -ldl -lm -o /tmp/rich_smoke
//   /tmp/rich_smoke
#include <cstdio>
#include <unistd.h>

#include "rich.hpp"

int main() {
    rich::Console con;
    if (!con) {
        std::fprintf(stderr, "failed to create console\n");
        return 1;
    }

    // Mirror SafeTunnel's daemon logic: style only on a real TTY.
    const bool tty = ::isatty(STDOUT_FILENO);
    con.set_force_terminal(tty).set_size(80, 24);

    std::printf("isatty(stdout) = %s, width = %u\n", tty ? "true" : "false", con.width());

    std::string styled = con.render_markup("[bold green]✓[/] tunnel [cyan]wg0[/] up");
    std::printf("%s\n", styled.c_str());

    // Forced styled, so we can see the escapes regardless of TTY.
    con.set_force_terminal(true);
    std::string forced = con.render_markup("[bold red]✗[/] handshake [yellow]timed out[/]");
    std::printf("forced-styled bytes: ");
    for (unsigned char c : forced) {
        if (c == 0x1b) std::printf("\\e");
        else std::printf("%c", c);
    }
    std::printf("\n");
    return 0;
}
