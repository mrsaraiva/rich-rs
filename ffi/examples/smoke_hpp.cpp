// smoke_hpp.cpp — exercises the rich.hpp RAII wrappers across the full ABI.
//
// Build (from ffi/, after cargo build):
//   g++ -std=c++17 -Iinclude examples/smoke_hpp.cpp \
//       target/debug/librich_ffi.a -lpthread -ldl -lm -o /tmp/hpp && /tmp/hpp
#include <cassert>
#include <cstdio>
#include <string>
#include <utility>
#include <vector>

#include "rich.hpp"

static bool has_esc(const std::string& s) { return s.find('\x1b') != std::string::npos; }

int main() {
    rich::Console con;
    assert(con);
    con.set_force_terminal(true).set_size(80, 24);

    // ── Table with a markup title + string rows, composed and rendered ──
    {
        rich::Table t;
        t.set_title_markup(con, "[bold green]Peers[/]")
            .set_box(rich::Box::Rounded)
            .add_column("Peer")
            .add_column("Endpoint")
            .add_row({"wg0", "10.0.0.1:51820"})
            .add_row({"wg1", "10.0.0.2:51820"});
        rich::Renderable r = t.finish();
        std::string styled = con.render(r);
        assert(has_esc(styled) && "styled table must contain ANSI");
        assert(!styled.empty());
    }

    // ── Panel wrapping a styled Text (consumes the Renderable) ──
    {
        rich::Panel p(rich::Text::markup(con, "[bold red]tunnel down[/]").finish());
        p.set_title("status").set_box(rich::Box::Heavy);
        std::string styled = con.render(p.finish());
        assert(has_esc(styled));
    }

    // ── Tree with nested nodes ──
    {
        rich::Tree tree("root");
        auto child = tree.add("interfaces");
        child.add("wg0");
        child.add("wg1");
        tree.add("routes");
        std::string out = con.render(tree.finish());
        assert(!out.empty());
    }

    // ── Layout: Columns of renderables, Align, Padding ──
    {
        rich::Columns cols;
        cols.add(rich::Text::markup(con, "[cyan]A[/]").finish())
            .add(rich::Text::markup(con, "[magenta]B[/]").finish())
            .set_equal(true);
        std::string out = con.render(cols.finish());
        assert(has_esc(out));

        rich::Padding pad(rich::Text::markup(con, "[yellow]x[/]").finish(), 1, 2, 1, 2);
        assert(has_esc(con.render(pad.finish())));

        auto centered = rich::AlignBox::center(rich::Text::markup(con, "[blue]c[/]").finish());
        assert(!con.render(centered.finish()).empty());
    }

    // ── Content: Syntax + Json (valid + invalid) ──
    {
        rich::Syntax syn("fn main() {}", "rust");
        syn.set_line_numbers(true);
        assert(has_esc(con.render(syn.finish())));

        rich::Json ok(R"({"a":1})", 2, true, false);
        assert(ok && "valid JSON yields a handle");
        rich::Json bad("{not json", 2, true, false);
        assert(!bad && "invalid JSON yields an empty handle");
    }

    // ── Live: Progress frame (styled has ANSI, plain has none) ──
    {
        rich::Progress prog;
        std::uint64_t id = prog.add_task("download", 100.0);
        prog.update(id, 50.0);
        std::string styled = prog.render_frame(con);
        assert(has_esc(styled) && "styled progress frame must contain ANSI");

        con.set_force_terminal(false);
        std::string plain = prog.render_frame(con);
        assert(!has_esc(plain) && "plain progress frame must have zero ANSI");
        con.set_force_terminal(true);

        // Styled spinner via a style override demonstrates the styled path too.
        rich::Spinner sp("dots");
        assert(sp && "known spinner name yields a handle");
        rich::Spinner bad("__no_such_spinner__");
        assert(!bad && "unknown spinner name yields an empty handle");
        (void)sp.render_frame(con, 0.5);
    }

    // ── Plain-path contract for a full renderable ──
    {
        con.set_force_terminal(false);
        rich::Table t;
        t.set_title_markup(con, "[bold]X[/]").add_column("c").add_row({"v"});
        std::string plain = con.render(t.finish());
        assert(!has_esc(plain) && "plain table must have zero ANSI");
        con.set_force_terminal(true);
    }

    std::printf("smoke_hpp: ALL RAII ASSERTIONS PASSED\n");
    return 0;
}
