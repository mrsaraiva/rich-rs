// smoke_phase1.cpp — proves the Phase 1 C ABI: RichRenderable, the render
// pipeline, and the RichText / RichStyle handles, exercising the §3.3
// ownership table (every handle created is finished or freed exactly once).
//
// Build (from ffi/, after `cargo build`):
//   g++ -std=c++17 -Iinclude examples/smoke_phase1.cpp \
//       target/debug/librich_ffi.a -lpthread -ldl -lm -o /home/.../p1
//   /home/.../p1
#include <cassert>
#include <cstdio>
#include <cstring>
#include <string>

#include "rich.h"

// Count ESC (0x1b) bytes in a buffer.
static int count_esc(const std::string& s) {
    int n = 0;
    for (unsigned char c : s) {
        if (c == 0x1b) ++n;
    }
    return n;
}

// Render a RichText built from `markup`, finished, under the given terminal
// mode. BORROWS the renderable for the render, then frees both renderable and
// the returned string per the ownership table.
static std::string render_markup_via_text(RichConsole* con, const char* markup,
                                           bool force_terminal) {
    rich_console_set_force_terminal(con, force_terminal);

    RichText* t = rich_text_new_markup(con, markup);   // new -> must finish/free
    assert(t != nullptr);
    RichRenderable* r = rich_text_finish(t);           // CONSUMES t; t now invalid
    assert(r != nullptr);

    char* out = rich_console_render(con, r);           // BORROWS r -> char*
    assert(out != nullptr);
    std::string s(out);
    rich_string_free(out);                             // free the char*
    rich_renderable_free(r);                           // free the renderable (not consumed)
    return s;
}

int main() {
    RichConsole* con = rich_console_new();
    assert(con != nullptr);
    rich_console_set_size(con, 80, 24);

    const char* markup = "[bold green]OK[/] tunnel [cyan]wg0[/] up";

    // (a) Styled path under force_terminal(true) CONTAINS at least one ESC byte.
    std::string styled = render_markup_via_text(con, markup, true);
    int styled_esc = count_esc(styled);
    std::printf("styled ESC count: %d\n", styled_esc);
    assert(styled_esc > 0 && "force_terminal(true) must emit ANSI (ESC bytes)");

    // (b) Plain path under force_terminal(false) contains ZERO ESC bytes
    //     (the daemon/plain contract).
    std::string plain = render_markup_via_text(con, markup, false);
    int plain_esc = count_esc(plain);
    std::printf("plain ESC count: %d\n", plain_esc);
    assert(plain_esc == 0 && "force_terminal(false) must be plain (zero ESC bytes)");

    // (c) The bytes equal rich_console_render_markup for the same input,
    //     under both terminal modes (the Text path and the markup path agree).
    rich_console_set_force_terminal(con, true);
    char* direct_styled_c = rich_console_render_markup(con, markup);
    assert(direct_styled_c != nullptr);
    std::string direct_styled(direct_styled_c);
    rich_string_free(direct_styled_c);

    rich_console_set_force_terminal(con, false);
    char* direct_plain_c = rich_console_render_markup(con, markup);
    assert(direct_plain_c != nullptr);
    std::string direct_plain(direct_plain_c);
    rich_string_free(direct_plain_c);

    assert(styled == direct_styled &&
           "Text-finish-render must equal render_markup (styled)");
    assert(plain == direct_plain &&
           "Text-finish-render must equal render_markup (plain)");
    std::printf("consistency: Text path == render_markup path (styled & plain)\n");

    // (d) rich_style_parse: valid parses, nonsense returns NULL.
    RichStyle* good = rich_style_parse("bold red");
    assert(good != nullptr && "\"bold red\" must parse");
    rich_style_free(good);

    RichStyle* bad = rich_style_parse("nonsense-xyz");
    assert(bad == nullptr && "\"nonsense-xyz\" must NOT parse (NULL)");
    rich_style_free(bad);  // NULL is a no-op
    std::printf("style parse: \"bold red\" ok, \"nonsense-xyz\" -> NULL\n");

    // (e) Ownership coverage: plain Text path (new -> set_style -> finish ->
    //     render -> free), and a Text built+freed without finishing.
    {
        RichText* pt = rich_text_new("plain line");          // new
        assert(pt != nullptr);
        rich_text_set_style(pt, "bold");                     // mutate in place
        rich_text_set_style(pt, "nonsense-xyz");             // invalid -> no-op
        RichRenderable* pr = rich_text_finish(pt);           // CONSUMES pt
        assert(pr != nullptr);
        rich_console_set_force_terminal(con, true);
        char* po = rich_console_render(con, pr);             // BORROWS pr
        assert(po != nullptr);
        assert(count_esc(std::string(po)) > 0 && "\"bold\" base style must emit ANSI");
        rich_string_free(po);
        rich_renderable_free(pr);                            // free
    }
    {
        RichText* unused = rich_text_new("never finished");  // new
        assert(unused != nullptr);
        rich_text_free(unused);                              // free without finishing
    }

    // NULL-safety spot checks (must not crash; sentinels returned).
    assert(rich_text_new(nullptr) == nullptr);
    assert(rich_text_finish(nullptr) == nullptr);
    assert(rich_console_render(nullptr, nullptr) == nullptr);
    rich_renderable_free(nullptr);   // no-op
    rich_string_free(nullptr);       // no-op

    rich_console_free(con);

    std::printf("smoke_phase1: ALL ASSERTIONS PASSED\n");
    return 0;
}
