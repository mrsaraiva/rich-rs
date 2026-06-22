// Phase 2 smoke test — Table C ABI.
//
// Exercises the full create/finish/free lifecycle and the consume contracts
// (RichRenderable headers/cells must NOT be double-freed), then asserts the
// styled-vs-plain ANSI behaviour.
//
// Build + run:
//   g++ -std=c++17 -Iinclude examples/smoke_phase2.cpp \
//       target/debug/librich_ffi.a -lpthread -ldl -lm -o ../p2 && ../p2

#include <cassert>
#include <cstring>
#include <cstdio>

#include "rich.h"

// Does the C string contain an ESC (0x1b) byte?
static bool has_esc(const char *s) {
    for (const char *p = s; *p; ++p) {
        if (*p == 0x1b) return true;
    }
    return false;
}

int main() {
    // --- Build a styled table containing a child RichRenderable cell --------
    RichConsole *con = rich_console_new();
    assert(con != nullptr);
    rich_console_set_force_terminal(con, true); // styled

    RichTable *t = rich_table_new();
    assert(t != nullptr);

    rich_table_set_title(t, "Peers");
    rich_table_set_caption(t, "live");
    rich_table_set_box(t, 0);            // ROUNDED
    rich_table_set_box(t, 9999);         // out-of-range: no-op
    rich_table_set_show_header(t, true);
    rich_table_set_show_lines(t, true);
    rich_table_set_show_edge(t, true);
    rich_table_set_expand(t, false);
    rich_table_set_padding(t, 1, 1);
    rich_table_set_style(t, "on grey23");
    rich_table_set_style(t, "nonsense-xyz"); // invalid: no-op

    // A plain string column plus a renderable (markup) column header.
    rich_table_add_column(t, "Peer");
    RichText *hdr = rich_text_new_markup(con, "[bold green]Status[/]");
    assert(hdr != nullptr);
    RichRenderable *hdr_r = rich_text_finish(hdr); // hdr now invalid
    assert(hdr_r != nullptr);
    rich_table_add_column_renderable(t, hdr_r);    // CONSUMES hdr_r

    // A plain-string row.
    const char *row0[2] = {"alice", "up"};
    rich_table_add_row_strs(t, row0, 2);

    // A renderable row: one styled (ANSI) cell + one plain cell.
    RichText *c0 = rich_text_new_markup(con, "[bold red]bob[/]");
    RichText *c1 = rich_text_new("down");
    assert(c0 != nullptr && c1 != nullptr);
    RichRenderable *cells[2] = {rich_text_finish(c0), rich_text_finish(c1)};
    assert(cells[0] != nullptr && cells[1] != nullptr);
    rich_table_add_row_renderables(t, cells, 2);   // CONSUMES both cells

    // --- Finish + render under force_terminal(true): expect ANSI -----------
    RichRenderable *r = rich_table_finish(t);      // t now invalid
    assert(r != nullptr);

    char *styled = rich_console_render(con, r);
    assert(styled != nullptr);
    assert(has_esc(styled) && "styled output must contain ESC 0x1b");
    rich_string_free(styled);

    // --- Render the SAME renderable under force_terminal(false): no ANSI ----
    rich_console_set_force_terminal(con, false);
    char *plain = rich_console_render(con, r);
    assert(plain != nullptr);
    assert(!has_esc(plain) && "plain output must have zero ESC 0x1b bytes");
    rich_string_free(plain);

    // r still owned by us (render borrows): free it once.
    rich_renderable_free(r);

    // --- NULL / lifecycle hygiene ------------------------------------------
    rich_table_free(nullptr);                 // no-op
    rich_renderable_free(nullptr);            // no-op
    rich_table_set_title(nullptr, "x");       // no-op
    rich_table_add_column(nullptr, "x");      // no-op

    // Create + free without finishing (no leak path).
    RichTable *t2 = rich_table_new();
    assert(t2 != nullptr);
    rich_table_add_column(t2, "only");
    rich_table_free(t2);

    rich_console_free(con);

    printf("smoke_phase2 OK\n");
    return 0;
}
