// smoke_phase3.cpp — proves the Phase 3 Panel C ABI links, renders, and obeys
// the ownership + plain/styled contracts from the C ABI spec.
//
// Build (from ffi/, after `cargo build`):
//   g++ -std=c++17 -Iinclude examples/smoke_phase3.cpp \
//       target/debug/librich_ffi.a -lpthread -ldl -lm -o /home/msaraiva/scratch/rich-rs/wt-phase3/p3
//   /home/msaraiva/scratch/rich-rs/wt-phase3/p3
#include <cassert>
#include <cstdio>
#include <cstring>
#include <string>

#include "rich.h"

// Count ESC (0x1b) bytes in a rendered string.
static size_t count_esc(const char *s) {
    size_t n = 0;
    for (const char *p = s; *p; ++p) {
        if (static_cast<unsigned char>(*p) == 0x1b) ++n;
    }
    return n;
}

// Build a Panel around a styled markup Text child, render it through `con`,
// and return the captured output as a std::string. Each call performs a full
// create/finish/free cycle with no leaks.
static std::string render_panel(RichConsole *con) {
    // Child renderable: a styled markup Text (so the styled path emits ANSI).
    RichText *txt = rich_text_new_markup(con, "[bold green]tunnel up[/]");
    assert(txt && "rich_text_new_markup returned NULL");
    RichRenderable *content = rich_text_finish(txt);  // txt now invalid
    assert(content && "rich_text_finish returned NULL");

    // Panel CONSUMES content.
    RichPanel *panel = rich_panel_new(content);  // content now invalid
    assert(panel && "rich_panel_new returned NULL");

    rich_panel_set_title(panel, "Status");
    rich_panel_set_subtitle(panel, "wg0");
    rich_panel_set_box(panel, 2);                  // DOUBLE
    rich_panel_set_padding(panel, 0, 1, 0, 1);     // top,right,bottom,left
    rich_panel_set_border_style(panel, "bold cyan");
    rich_panel_set_style(panel, "on grey15");
    rich_panel_set_width(panel, 40);

    RichRenderable *r = rich_panel_finish(panel);  // panel now invalid
    assert(r && "rich_panel_finish returned NULL");

    char *out = rich_console_render(con, r);        // borrows r
    assert(out && "rich_console_render returned NULL");
    std::string result(out);
    rich_string_free(out);

    rich_renderable_free(r);  // r was NOT consumed by a container; free it.
    return result;
}

int main() {
    RichConsole *con = rich_console_new();
    if (!con) {
        std::fprintf(stderr, "failed to create console\n");
        return 1;
    }
    rich_console_set_size(con, 80, 24);

    // (a) Styled path: force_terminal(true) with an actual styled element
    //     MUST contain ESC 0x1b.
    rich_console_set_force_terminal(con, true);
    std::string styled = render_panel(con);
    size_t styled_esc = count_esc(styled.c_str());
    std::printf("styled ESC count = %zu\n", styled_esc);
    assert(styled_esc > 0 && "styled path produced no ANSI escapes");

    // (b) Plain path: force_terminal(false) MUST have ZERO 0x1b bytes.
    rich_console_set_force_terminal(con, false);
    std::string plain = render_panel(con);
    size_t plain_esc = count_esc(plain.c_str());
    std::printf("plain ESC count  = %zu\n", plain_esc);
    assert(plain_esc == 0 && "plain path leaked ANSI escapes");

    // (c) Ownership: a Panel created and freed WITHOUT finish (no leak), and a
    //     content RichRenderable consumed by rich_panel_new is NOT freed again.
    {
        RichText *txt = rich_text_new("child");
        assert(txt);
        RichRenderable *content = rich_text_finish(txt);
        assert(content);
        RichPanel *p = rich_panel_new(content);  // consumes content
        assert(p);
        // content is now invalid — do NOT rich_renderable_free(content).
        rich_panel_set_title(p, "discarded");
        rich_panel_free(p);  // freed without finish; no leak.
    }

    // NULL-safety: every entry point is a no-op / sentinel on NULL.
    rich_panel_free(nullptr);
    rich_panel_set_title(nullptr, "x");
    rich_panel_set_style(nullptr, "red");
    assert(rich_panel_new(nullptr) == nullptr);
    assert(rich_panel_finish(nullptr) == nullptr);

    rich_console_free(con);

    std::printf("smoke_phase3: OK\n");
    return 0;
}
