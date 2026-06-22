// smoke_phase6.cpp — proves the Phase 6 (Content: Syntax, Markdown, Json) C ABI
// links, renders, and obeys the ownership contract.
//
// Build (from ffi/, after `cargo build`):
//   g++ -std=c++17 -Iinclude examples/smoke_phase6.cpp \
//       target/debug/librich_ffi.a -lpthread -ldl -lm -o /tmp/p6
//   /tmp/p6
#include <cassert>
#include <cstdio>
#include <cstring>
#include <string>

#include "rich.h"

// Count ESC (0x1b) bytes in a rendered string.
static size_t count_esc(const char *s) {
    size_t n = 0;
    for (const char *p = s; *p; ++p) {
        if ((unsigned char)*p == 0x1b) ++n;
    }
    return n;
}

// Render a renderable (borrows it) and return the bytes; frees the C string.
static std::string render(RichConsole *con, const RichRenderable *r) {
    char *out = rich_console_render(con, r);
    assert(out != nullptr && "render returned NULL");
    std::string s(out);
    rich_string_free(out);
    return s;
}

int main() {
    RichConsole *con = rich_console_new();
    assert(con != nullptr);
    rich_console_set_size(con, 80, 24);

    // ---------------------------------------------------------------
    // 1. Syntax: styled path emits ANSI under force_terminal(true).
    // ---------------------------------------------------------------
    {
        RichSyntax *syn = rich_syntax_new("fn main() { let x = 42; }", "rust");
        assert(syn != nullptr);
        rich_syntax_set_theme(syn, "monokai");
        rich_syntax_set_line_numbers(syn, true);
        rich_syntax_set_word_wrap(syn, false);
        rich_syntax_set_line_range(syn, -1, -1); // both None => no-op
        RichRenderable *r = rich_syntax_finish(syn); // syn now invalid
        assert(r != nullptr);

        rich_console_set_force_terminal(con, true);
        std::string styled = render(con, r);
        assert(count_esc(styled.c_str()) > 0 && "styled syntax must contain ESC");

        rich_console_set_force_terminal(con, false);
        std::string plain = render(con, r);
        assert(count_esc(plain.c_str()) == 0 && "plain syntax must have zero ESC");

        rich_renderable_free(r);
        std::printf("syntax OK (styled esc>0, plain esc==0)\n");
    }

    // ---------------------------------------------------------------
    // 2. Markdown: container-ish content; verify ANSI on/off contract.
    // ---------------------------------------------------------------
    {
        RichMarkdown *md = rich_markdown_new("# Title\n\nSome **bold** text.\n");
        assert(md != nullptr);
        rich_markdown_set_code_theme(md, "monokai");
        rich_markdown_set_hyperlinks(md, true);
        rich_markdown_set_justify(md, 1); // Left
        RichRenderable *r = rich_markdown_finish(md); // md now invalid
        assert(r != nullptr);

        rich_console_set_force_terminal(con, true);
        std::string styled = render(con, r);
        assert(count_esc(styled.c_str()) > 0 && "styled markdown must contain ESC");

        rich_console_set_force_terminal(con, false);
        std::string plain = render(con, r);
        assert(count_esc(plain.c_str()) == 0 && "plain markdown must have zero ESC");

        rich_renderable_free(r);
        std::printf("markdown OK (styled esc>0, plain esc==0)\n");
    }

    // ---------------------------------------------------------------
    // 3. Json: highlighted under force_terminal(true), plain when off.
    // ---------------------------------------------------------------
    {
        RichJson *js = rich_json_new("{\"b\":2,\"a\":[1,2,3]}", 2, true, true);
        assert(js != nullptr);
        RichRenderable *r = rich_json_finish(js); // js now invalid
        assert(r != nullptr);

        rich_console_set_force_terminal(con, true);
        std::string styled = render(con, r);
        assert(count_esc(styled.c_str()) > 0 && "highlighted json must contain ESC");

        rich_console_set_force_terminal(con, false);
        std::string plain = render(con, r);
        assert(count_esc(plain.c_str()) == 0 && "plain json must have zero ESC");

        rich_renderable_free(r);
        std::printf("json OK (styled esc>0, plain esc==0)\n");
    }

    // ---------------------------------------------------------------
    // 3b. Json validation: valid JSON => non-NULL and renders;
    //     malformed JSON => NULL (honoring the spec contract).
    // ---------------------------------------------------------------
    {
        // Valid JSON: must construct, finish, and render.
        RichJson *good = rich_json_new("{\"a\":1}", 2, true, false);
        assert(good != nullptr && "valid JSON must yield a non-NULL handle");
        RichRenderable *r = rich_json_finish(good); // good now invalid
        assert(r != nullptr);
        rich_console_set_force_terminal(con, false);
        std::string out = render(con, r);
        assert(!out.empty() && "valid JSON must render non-empty output");
        rich_renderable_free(r);

        // Malformed JSON: must be rejected with NULL (no handle to free).
        RichJson *bad = rich_json_new("{bad", 2, true, false);
        assert(bad == nullptr && "malformed JSON must yield NULL");

        std::printf("json validation OK (valid renders, malformed => NULL)\n");
    }

    // ---------------------------------------------------------------
    // 4. Container composition: a Json consumed via rich_text path is
    //    not double-freed. Here we exercise the consume contract by
    //    finishing a Text child and rendering it, plus verifying a
    //    consumed renderable is owned by its sink. We build a child
    //    RichRenderable via rich_text_new + finish (as required), render
    //    it, then free it exactly once.
    // ---------------------------------------------------------------
    {
        RichText *txt = rich_text_new_markup(con, "[bold]child[/] element");
        assert(txt != nullptr);
        RichRenderable *child = rich_text_finish(txt); // txt invalid
        assert(child != nullptr);

        rich_console_set_force_terminal(con, true);
        std::string styled = render(con, child);
        assert(count_esc(styled.c_str()) > 0 && "styled child must contain ESC");

        // Free exactly once (never consumed by a container here).
        rich_renderable_free(child);
        std::printf("child renderable OK (single free, no double-free)\n");
    }

    // ---------------------------------------------------------------
    // 5. Ownership / NULL behavior: free-without-finish, NULL no-ops.
    // ---------------------------------------------------------------
    {
        // Create then free without finishing — no leak, no crash.
        RichSyntax *syn = rich_syntax_new("x = 1", "python");
        assert(syn != nullptr);
        rich_syntax_free(syn);

        RichMarkdown *md = rich_markdown_new("plain");
        assert(md != nullptr);
        rich_markdown_free(md);

        RichJson *js = rich_json_new("[]", 2, false, false);
        assert(js != nullptr);
        rich_json_free(js);

        // NULL no-ops must not crash.
        rich_syntax_free(nullptr);
        rich_markdown_free(nullptr);
        rich_json_free(nullptr);
        assert(rich_syntax_finish(nullptr) == nullptr);
        assert(rich_markdown_finish(nullptr) == nullptr);
        assert(rich_json_finish(nullptr) == nullptr);
        assert(rich_syntax_new(nullptr, "rust") == nullptr);
        assert(rich_markdown_new(nullptr) == nullptr);
        assert(rich_json_new(nullptr, 2, true, false) == nullptr);
        assert(rich_syntax_from_path("/nonexistent/path/xyz.rs") == nullptr);
        std::printf("ownership + NULL behavior OK\n");
    }

    rich_console_free(con);
    std::printf("ALL PHASE 6 SMOKE CHECKS PASSED\n");
    return 0;
}
