// Phase 5 — Layout (Rule, Columns, Align, Padding) C ABI smoke test.
//
// Exercises each of the four layout widgets: builds it (with a child
// RichRenderable where it is a container), finishes it, and renders it through
// a console. Asserts:
//   (a) under force_terminal(true) a styled element produces ANSI (ESC 0x1b),
//   (b) under force_terminal(false) the output has ZERO 0x1b bytes,
//   (c) full create/finish/free with no leaks, and that a consumed
//       RichRenderable is NOT double-freed.
//
// Build + run (from ffi/):
//   cargo build
//   g++ -std=c++17 -Iinclude examples/smoke_phase5.cpp \
//       target/debug/librich_ffi.a -lpthread -ldl -lm -o /tmp/p5 && /tmp/p5

#include "rich.h"

#include <cassert>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>

// Count ESC (0x1b) bytes in a rendered, owned C string; frees it.
static int count_esc_and_free(char *s) {
    if (!s) return -1;
    int n = 0;
    for (const char *p = s; *p; ++p) {
        if (*p == 0x1b) ++n;
    }
    rich_string_free(s);
    return n;
}

// Make a fresh styled child renderable (bold red markup) -> RichRenderable*.
static RichRenderable *styled_child(RichConsole *con) {
    RichText *t = rich_text_new_markup(con, "[bold red]hello[/]");
    assert(t != nullptr);
    RichRenderable *r = rich_text_finish(t); // t consumed
    assert(r != nullptr);
    return r;
}

int main() {
    RichConsole *con = rich_console_new();
    assert(con != nullptr);
    rich_console_set_size(con, 40, 10);

    // ---------------------------------------------------------------------
    // Rule: styled title, custom characters/style/align.
    // ---------------------------------------------------------------------
    {
        rich_console_set_force_terminal(con, true);
        RichRule *rule = rich_rule_new();
        assert(rule != nullptr);
        rich_rule_set_title(rule, "[bold]Section[/]");
        rich_rule_set_characters(rule, "=");
        rich_rule_set_style(rule, "bold red");
        rich_rule_set_align(rule, 1); // center
        RichRenderable *r = rich_rule_finish(rule); // rule consumed
        assert(r != nullptr);

        int styled = count_esc_and_free(rich_console_render(con, r));
        assert(styled > 0 && "Rule should emit ANSI when force_terminal=true");

        rich_console_set_force_terminal(con, false);
        int plain = count_esc_and_free(rich_console_render(con, r));
        assert(plain == 0 && "Rule must be plain when force_terminal=false");

        rich_renderable_free(r); // r never consumed by a container
        printf("Rule: styled=%d plain=%d OK\n", styled, plain);
    }

    // ---------------------------------------------------------------------
    // Columns: container that CONSUMES a child RichRenderable + a string.
    // ---------------------------------------------------------------------
    {
        rich_console_set_force_terminal(con, true);
        RichColumns *cols = rich_columns_new();
        assert(cols != nullptr);
        rich_columns_add(cols, styled_child(con)); // CONSUMES child
        rich_columns_add_str(cols, "second");
        rich_columns_set_equal(cols, true);
        rich_columns_set_expand(cols, false);
        rich_columns_set_padding(cols, 0, 2);
        RichRenderable *r = rich_columns_finish(cols); // cols consumed
        assert(r != nullptr);

        int styled = count_esc_and_free(rich_console_render(con, r));
        assert(styled > 0 && "Columns should emit ANSI for a styled child");

        rich_console_set_force_terminal(con, false);
        int plain = count_esc_and_free(rich_console_render(con, r));
        assert(plain == 0 && "Columns must be plain when force_terminal=false");

        rich_renderable_free(r);
        printf("Columns: styled=%d plain=%d OK\n", styled, plain);
    }

    // ---------------------------------------------------------------------
    // Align: CONSUMES content; rich_align_new + rich_align_center.
    // ---------------------------------------------------------------------
    {
        rich_console_set_force_terminal(con, true);
        RichAlign *al = rich_align_new(styled_child(con), 2); // right; CONSUMES
        assert(al != nullptr);
        rich_align_set_vertical(al, 1); // middle
        rich_align_set_width(al, 30);
        RichRenderable *r = rich_align_finish(al); // al consumed
        assert(r != nullptr);

        int styled = count_esc_and_free(rich_console_render(con, r));
        assert(styled > 0 && "Align should emit ANSI for a styled child");

        rich_console_set_force_terminal(con, false);
        int plain = count_esc_and_free(rich_console_render(con, r));
        assert(plain == 0 && "Align must be plain when force_terminal=false");
        rich_renderable_free(r);

        // rich_align_center variant.
        rich_console_set_force_terminal(con, true);
        RichAlign *al2 = rich_align_center(styled_child(con)); // CONSUMES
        assert(al2 != nullptr);
        RichRenderable *r2 = rich_align_finish(al2);
        assert(r2 != nullptr);
        int styled2 = count_esc_and_free(rich_console_render(con, r2));
        assert(styled2 > 0);
        rich_renderable_free(r2);
        printf("Align: styled=%d plain=%d center_styled=%d OK\n",
               styled, plain, styled2);
    }

    // ---------------------------------------------------------------------
    // Padding: CONSUMES content; style + expand.
    // ---------------------------------------------------------------------
    {
        rich_console_set_force_terminal(con, true);
        RichPadding *pad =
            rich_padding_new(styled_child(con), 1, 2, 1, 2); // CONSUMES
        assert(pad != nullptr);
        rich_padding_set_style(pad, "on blue");
        rich_padding_set_expand(pad, false);
        RichRenderable *r = rich_padding_finish(pad); // pad consumed
        assert(r != nullptr);

        int styled = count_esc_and_free(rich_console_render(con, r));
        assert(styled > 0 && "Padding should emit ANSI for a styled child");

        rich_console_set_force_terminal(con, false);
        int plain = count_esc_and_free(rich_console_render(con, r));
        assert(plain == 0 && "Padding must be plain when force_terminal=false");

        rich_renderable_free(r);
        printf("Padding: styled=%d plain=%d OK\n", styled, plain);
    }

    // ---------------------------------------------------------------------
    // Lifecycle: create-then-free without finishing (no leak / no crash).
    // ---------------------------------------------------------------------
    {
        rich_rule_free(rich_rule_new());
        rich_columns_free(rich_columns_new());
        rich_padding_free(rich_padding_new(styled_child(con), 0, 0, 0, 0));
        rich_align_free(rich_align_center(styled_child(con)));

        // NULL-safety: every free is a no-op on NULL.
        rich_rule_free(nullptr);
        rich_columns_free(nullptr);
        rich_align_free(nullptr);
        rich_padding_free(nullptr);

        // rich_columns_add with NULL columns still consumes (frees) the child.
        rich_columns_add(nullptr, styled_child(con));
        printf("Lifecycle: free-without-finish + NULL-safety OK\n");
    }

    rich_console_free(con);
    printf("ALL PHASE 5 SMOKE CHECKS PASSED\n");
    return 0;
}
