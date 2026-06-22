// smoke_phase4.cpp — proves the Phase 4 Tree C ABI: the RichTree owning handle,
// recursive RichTreeNode borrows, renderable labels (container composition),
// the styled/plain ANSI contract, and the §3.3 ownership table.
//
// Build (from ffi/, after `cargo build`):
//   g++ -std=c++17 -Iinclude examples/smoke_phase4.cpp \
//       target/debug/librich_ffi.a -lpthread -ldl -lm -o /home/.../p4
//   /home/.../p4
#include <cassert>
#include <cstdio>
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

// Build a styled Tree (root + nested children) under the given terminal mode and
// render it. The Tree is a container: its root label is a *finished*
// RichRenderable (built via rich_text_new_markup) which the Tree CONSUMES — so
// it must NOT be freed afterward (double-free check). Returns the rendered bytes.
static std::string render_tree(RichConsole* con, bool force_terminal) {
    rich_console_set_force_terminal(con, force_terminal);

    // Container case: root label is a styled child RichRenderable.
    RichText* root_label = rich_text_new_markup(con, "[bold cyan]project[/]");
    assert(root_label != nullptr);
    RichRenderable* root_r = rich_text_finish(root_label);  // CONSUMES root_label
    assert(root_r != nullptr);

    RichTree* tree = rich_tree_new_renderable(root_r);      // CONSUMES root_r
    assert(tree != nullptr);
    // root_r and root_label are now invalid: do NOT free them (no double free).

    // Style the guides so a styled element is guaranteed in the output.
    rich_tree_set_guide_style(tree, "bold magenta");
    rich_tree_set_style(tree, "green");
    rich_tree_set_hide_root(tree, false);
    rich_tree_set_style(tree, "nonsense-xyz");   // invalid -> no-op

    // Plain-text children.
    RichTreeNode* src = rich_tree_add(tree, "src");
    assert(src != nullptr);
    rich_tree_add(tree, "README.md");

    // Grandchildren on the "src" sub-node (recursive borrow).
    RichTreeNode* lib = rich_tree_node_add(src, "lib.rs");
    assert(lib != nullptr);
    rich_tree_node_add(src, "main.rs");

    // A renderable-labelled grandchild (consumed by the node).
    RichText* mod_label = rich_text_new_markup(con, "[italic]mod.rs[/]");
    assert(mod_label != nullptr);
    RichRenderable* mod_r = rich_text_finish(mod_label);    // CONSUMES mod_label
    assert(mod_r != nullptr);
    RichTreeNode* mod_node = rich_tree_node_add_renderable(src, mod_r);  // CONSUMES mod_r
    assert(mod_node != nullptr);
    // mod_r is now invalid: do NOT free it.

    // Great-grandchild, to prove deeper nesting works.
    rich_tree_node_add(lib, "deep.rs");

    RichRenderable* r = rich_tree_finish(tree);   // CONSUMES tree; nodes now dangling
    assert(r != nullptr);

    char* out = rich_console_render(con, r);      // BORROWS r -> char*
    assert(out != nullptr);
    std::string s(out);
    rich_string_free(out);                        // free the char*
    rich_renderable_free(r);                       // free the renderable (not consumed)
    return s;
}

int main() {
    RichConsole* con = rich_console_new();
    assert(con != nullptr);
    rich_console_set_size(con, 80, 24);

    // (a) Styled path under force_terminal(true) CONTAINS at least one ESC byte
    //     (the styled guides / styled root label emit ANSI).
    std::string styled = render_tree(con, true);
    int styled_esc = count_esc(styled);
    std::printf("styled ESC count: %d\n", styled_esc);
    assert(styled_esc > 0 && "force_terminal(true) must emit ANSI (ESC bytes)");
    // Sanity: the tree structure rendered (guides + labels present).
    assert(styled.find("project") != std::string::npos);
    assert(styled.find("lib.rs") != std::string::npos);
    assert(styled.find("mod.rs") != std::string::npos);
    assert(styled.find("deep.rs") != std::string::npos);

    // (b) Plain path under force_terminal(false) contains ZERO ESC bytes.
    std::string plain = render_tree(con, false);
    int plain_esc = count_esc(plain);
    std::printf("plain ESC count: %d\n", plain_esc);
    assert(plain_esc == 0 && "force_terminal(false) must be plain (zero ESC bytes)");
    assert(plain.find("project") != std::string::npos);
    assert(plain.find("README.md") != std::string::npos);

    // (c) Plain-label constructor + full create/finish/free, no leaks.
    {
        RichTree* t = rich_tree_new("root");        // new
        assert(t != nullptr);
        rich_tree_add(t, "a");
        RichTreeNode* b = rich_tree_add(t, "b");
        assert(b != nullptr);
        rich_tree_node_add(b, "b1");
        RichRenderable* r = rich_tree_finish(t);    // CONSUMES t
        assert(r != nullptr);
        rich_console_set_force_terminal(con, false);
        char* o = rich_console_render(con, r);      // BORROWS r
        assert(o != nullptr);
        assert(count_esc(std::string(o)) == 0);
        rich_string_free(o);
        rich_renderable_free(r);                    // free
    }

    // (d) A Tree built then freed without finishing (ownership table).
    {
        RichTree* unused = rich_tree_new("never finished");  // new
        assert(unused != nullptr);
        rich_tree_add(unused, "child");                      // borrow, never used
        rich_tree_free(unused);                              // free without finishing
    }

    // (e) A Tree built from a renderable label then freed without finishing —
    //     proves the consumed label is dropped with the tree (no leak).
    {
        RichText* lbl = rich_text_new("erased root");
        assert(lbl != nullptr);
        RichRenderable* lr = rich_text_finish(lbl);          // CONSUMES lbl
        assert(lr != nullptr);
        RichTree* t = rich_tree_new_renderable(lr);          // CONSUMES lr
        assert(t != nullptr);
        rich_tree_free(t);                                   // free without finishing
    }

    // NULL-safety spot checks (must not crash; sentinels returned).
    assert(rich_tree_new(nullptr) == nullptr);
    assert(rich_tree_new_renderable(nullptr) == nullptr);
    assert(rich_tree_add(nullptr, "x") == nullptr);
    assert(rich_tree_node_add(nullptr, "x") == nullptr);
    assert(rich_tree_finish(nullptr) == nullptr);
    rich_tree_set_style(nullptr, "bold");   // no-op
    rich_tree_set_hide_root(nullptr, true); // no-op
    rich_tree_free(nullptr);                // no-op

    rich_console_free(con);

    std::printf("smoke_phase4: ALL ASSERTIONS PASSED\n");
    return 0;
}
