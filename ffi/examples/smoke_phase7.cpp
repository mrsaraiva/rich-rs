// smoke_phase7.cpp — proves the Phase 7 C ABI: the frame-based live widgets
// (Progress, Status, Spinner). Each widget is animated/stateful, but the ABI is
// caller-driven: the C++ side mutates state and asks for ONE frame at a time,
// owning the loop and the cursor. No background thread crosses the FFI.
//
// This exercises both the styled-ANSI path (force_terminal=true CONTAINS the ESC
// byte 0x1b) and the zero-escape plain path (force_terminal=false has ZERO ESC),
// plus the full create/render/free lifecycle for every handle (no leaks).
//
// Build (from ffi/, after `cargo build`):
//   g++ -std=c++17 -Iinclude examples/smoke_phase7.cpp \
//       target/debug/librich_ffi.a -lpthread -ldl -lm -o /home/.../sm
//   /home/.../sm
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

int main() {
    RichConsole* con = rich_console_new();
    assert(con != nullptr);
    rich_console_set_size(con, 80, 24);

    // ---------------------------------------------------------------------
    // Progress: add a task, advance it, then render one frame in each mode.
    // ---------------------------------------------------------------------
    RichProgress* prog = rich_progress_new();
    assert(prog != nullptr);

    unsigned long long task = rich_progress_add_task(prog, "download", 100.0);
    rich_progress_update(prog, task, 50.0);

    // (a) Styled frame: force_terminal(true) must emit ANSI (>= 1 ESC byte).
    rich_console_set_force_terminal(con, true);
    char* pframe_styled = rich_progress_render_frame(prog, con);
    assert(pframe_styled != nullptr);
    std::string prog_styled(pframe_styled);
    rich_string_free(pframe_styled);
    int prog_styled_esc = count_esc(prog_styled);
    std::printf("progress styled ESC count: %d\n", prog_styled_esc);
    assert(prog_styled_esc > 0 && "force_terminal(true) must emit ANSI (ESC bytes)");

    // (b) Plain frame: force_terminal(false) must contain ZERO ESC bytes.
    rich_console_set_force_terminal(con, false);
    char* pframe_plain = rich_progress_render_frame(prog, con);
    assert(pframe_plain != nullptr);
    std::string prog_plain(pframe_plain);
    rich_string_free(pframe_plain);
    int prog_plain_esc = count_esc(prog_plain);
    std::printf("progress plain ESC count: %d\n", prog_plain_esc);
    assert(prog_plain_esc == 0 && "force_terminal(false) must be plain (no ESC)");

    rich_progress_free(prog);

    // ---------------------------------------------------------------------
    // Status: a spinner + message, rendered in each mode.
    // ---------------------------------------------------------------------
    RichStatus* status = rich_status_new("Working");
    assert(status != nullptr);

    rich_console_set_force_terminal(con, true);
    char* sframe_styled = rich_status_render_frame(status, con);
    assert(sframe_styled != nullptr);
    std::string status_styled(sframe_styled);
    rich_string_free(sframe_styled);
    std::printf("status styled ESC count: %d\n", count_esc(status_styled));

    rich_console_set_force_terminal(con, false);
    char* sframe_plain = rich_status_render_frame(status, con);
    assert(sframe_plain != nullptr);
    std::string status_plain(sframe_plain);
    rich_string_free(sframe_plain);
    int status_plain_esc = count_esc(status_plain);
    std::printf("status plain ESC count: %d\n", status_plain_esc);
    assert(status_plain_esc == 0 && "status plain must have no ESC");

    rich_status_free(status);

    // ---------------------------------------------------------------------
    // Spinner: a known name must succeed; an unknown name must return NULL.
    // ---------------------------------------------------------------------
    RichSpinner* bad = rich_spinner_new("__nope__");
    assert(bad == nullptr && "unknown spinner name must yield NULL");

    RichSpinner* spin = rich_spinner_new("dots");
    assert(spin != nullptr && "'dots' is a real spinner name");

    // Render the spinner frame at an explicit elapsed time (0.5s).
    rich_console_set_force_terminal(con, true);
    char* spframe = rich_spinner_render_frame(spin, con, 0.5);
    assert(spframe != nullptr);
    std::string spin_frame(spframe);
    rich_string_free(spframe);
    std::printf("spinner frame@0.5s bytes: %zu (ESC=%d)\n",
                spin_frame.size(), count_esc(spin_frame));
    assert(!spin_frame.empty() && "spinner frame must be non-empty");

    // Plain spinner frame -> zero ESC.
    rich_console_set_force_terminal(con, false);
    char* spframe_plain = rich_spinner_render_frame(spin, con, 0.5);
    assert(spframe_plain != nullptr);
    std::string spin_plain(spframe_plain);
    rich_string_free(spframe_plain);
    assert(count_esc(spin_plain) == 0 && "spinner plain must have no ESC");

    rich_spinner_free(spin);

    // ---------------------------------------------------------------------
    // NULL-handling smoke: every free is a no-op on NULL; renders return NULL.
    // ---------------------------------------------------------------------
    rich_progress_free(nullptr);
    rich_status_free(nullptr);
    rich_spinner_free(nullptr);
    assert(rich_progress_render_frame(nullptr, con) == nullptr);
    assert(rich_status_render_frame(nullptr, con) == nullptr);
    assert(rich_spinner_render_frame(nullptr, con, 0.0) == nullptr);
    assert(rich_progress_add_task(nullptr, "x", 1.0) == 0);

    rich_console_free(con);

    std::printf("smoke_phase7: OK\n");
    return 0;
}
