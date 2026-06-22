//! C ABI bindings for rich-rs.
//!
//! Designed for C/C++ consumers (e.g. the SafeTunnel CLI and the `--foreground`
//! daemon). The model is an **opaque handle** + explicit lifetime functions:
//!
//! ```c
//! RichConsole *con = rich_console_new();
//! rich_console_set_force_terminal(con, isatty(fileno(stdout))); // plain when piped
//! char *out = rich_console_render_markup(con, "[bold green]✓[/] tunnel up");
//! if (out) { fputs(out, stdout); rich_string_free(out); }
//! rich_console_free(con);
//! ```
//!
//! ## Invariants the caller must uphold
//! * Every `*_new` handle is freed exactly once with `rich_console_free`.
//! * Every non-NULL `char*` returned is freed exactly once with `rich_string_free`.
//! * String inputs are valid NUL-terminated UTF-8.
//!
//! ## Safety design
//! Every entry point is NULL-checked and wrapped in `catch_unwind`, so a panic
//! inside rich-rs surfaces as a NULL / no-op return instead of unwinding across
//! the FFI boundary (which would abort the host process).

// Every function here is a C ABI boundary: its only callers are C/C++, the
// pointer-validity contract is documented per-function and in include/rich.h,
// and each entry point NULL-checks + panic-guards before any deref. Marking
// individual fns `unsafe` would be inconsistent (the trivial setters deref via a
// helper) and gives the C/C++ consumers nothing, since cbindgen ignores it.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::ffi::{CStr, CString, c_char, c_int, c_uint};
use std::panic::{AssertUnwindSafe, catch_unwind};

use rich_rs::{ColorSystem, Console};

/// Opaque console handle. C/C++ only ever sees a `RichConsole*`.
///
/// Backed by a capture console: rendering writes ANSI (or plain) text to an
/// in-memory buffer that `rich_console_render_markup` hands back as an owned
/// string. The caller decides where it goes (stdout, a log line, etc.).
pub struct RichConsole {
    inner: Console<Vec<u8>>,
    /// Palette to use *when styling is on*. rich-rs emits ANSI whenever
    /// `color_system` is `Some`, regardless of terminal state, so we hold the
    /// desired palette here and only push it onto the console while
    /// `force_terminal` is true. That makes `set_force_terminal(false)` produce
    /// genuinely plain text (the daemon-vs-`--foreground` contract).
    desired_color: Option<ColorSystem>,
}

/// Create a new console. Returns NULL on allocation/init failure.
///
/// Defaults: markup + emoji enabled, truecolor, and `force_terminal = true` so
/// ANSI is emitted into the captured string. A consumer that may be piped should
/// immediately call `rich_console_set_force_terminal` with its `isatty` result.
#[unsafe(no_mangle)]
pub extern "C" fn rich_console_new() -> *mut RichConsole {
    catch_unwind(|| {
        let mut inner = Console::capture();
        let desired_color = Some(ColorSystem::TrueColor);
        inner.set_force_terminal(Some(true));
        inner.set_color_system(desired_color);
        inner.set_markup_enabled(true);
        inner.set_emoji_enabled(true);
        Box::into_raw(Box::new(RichConsole {
            inner,
            desired_color,
        }))
    })
    .unwrap_or(std::ptr::null_mut())
}

/// Free a console created by `rich_console_new`. NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_console_free(console: *mut RichConsole) {
    if console.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `console` came from `Box::into_raw` in `rich_console_new`.
        drop(unsafe { Box::from_raw(console) });
    }));
}

/// Force (or unforce) terminal/ANSI output.
///
/// Pass the result of `isatty()` here: `true` emits ANSI styling, `false` emits
/// plain text. This is the switch for SafeTunnel's daemon — styled under
/// `--foreground` on a TTY, plain when writing to journald/syslog.
#[unsafe(no_mangle)]
pub extern "C" fn rich_console_set_force_terminal(console: *mut RichConsole, force: bool) {
    with_console(console, |c| {
        c.inner.set_force_terminal(Some(force));
        // Gate the palette on the terminal flag so `false` => plain text.
        c.inner
            .set_color_system(if force { c.desired_color } else { None });
    });
}

/// Set the render width and height in cells (controls wrapping).
#[unsafe(no_mangle)]
pub extern "C" fn rich_console_set_size(console: *mut RichConsole, width: c_uint, height: c_uint) {
    with_console(console, |c| {
        c.inner.set_size(width as usize, height as usize)
    });
}

/// Set the color system: 0 = none/monochrome, 1 = standard (16),
/// 2 = 8-bit (256), 3 = truecolor (24-bit), 4 = Windows legacy.
/// Out-of-range values are ignored.
#[unsafe(no_mangle)]
pub extern "C" fn rich_console_set_color_system(console: *mut RichConsole, system: c_int) {
    with_console(console, |c| {
        let cs = match system {
            0 => None,
            1 => Some(ColorSystem::Standard),
            2 => Some(ColorSystem::EightBit),
            3 => Some(ColorSystem::TrueColor),
            4 => Some(ColorSystem::Windows),
            _ => return,
        };
        c.desired_color = cs;
        // Only take effect now if styling is currently on; otherwise it applies
        // the next time `set_force_terminal(true)` is called.
        if c.inner.is_terminal() {
            c.inner.set_color_system(cs);
        }
    });
}

/// Enable or disable BBCode-style markup parsing (e.g. `[bold]...[/]`).
#[unsafe(no_mangle)]
pub extern "C" fn rich_console_set_markup_enabled(console: *mut RichConsole, enabled: bool) {
    with_console(console, |c| c.inner.set_markup_enabled(enabled));
}

/// Enable or disable `:emoji_name:` substitution.
#[unsafe(no_mangle)]
pub extern "C" fn rich_console_set_emoji_enabled(console: *mut RichConsole, enabled: bool) {
    with_console(console, |c| c.inner.set_emoji_enabled(enabled));
}

/// Current render width in cells. Returns 0 if `console` is NULL.
#[unsafe(no_mangle)]
pub extern "C" fn rich_console_width(console: *mut RichConsole) -> c_uint {
    if console.is_null() {
        return 0;
    }
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_console_new`.
        (unsafe { &*console }).inner.width() as c_uint
    }))
    .unwrap_or(0)
}

/// Render a markup string to an owned, NUL-terminated C string.
///
/// Honors the console's current markup/emoji/color/width settings. No trailing
/// newline is appended — the caller controls line endings. Returns NULL if
/// `console`/`markup` is NULL, `markup` is not valid UTF-8, or rendering fails.
///
/// The returned pointer MUST be freed with `rich_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_console_render_markup(
    console: *mut RichConsole,
    markup: *const c_char,
) -> *mut c_char {
    if console.is_null() || markup.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle; caller guarantees a valid NUL-terminated string.
        let rc = unsafe { &mut *console };
        let input = unsafe { CStr::from_ptr(markup) }.to_str().ok()?;

        // render_str respects the console's markup/emoji/highlight defaults.
        let text = rc.inner.render_str(input, None, None, None, None);
        rc.inner.clear_captured();
        // end = "" so we don't impose a trailing newline on the caller.
        rc.inner.print(&text, None, None, None, false, "").ok()?;
        let out = rc.inner.get_captured();
        CString::new(out).ok().map(CString::into_raw)
    }));
    match result {
        Ok(Some(ptr)) => ptr,
        _ => std::ptr::null_mut(),
    }
}

/// Free a string returned by `rich_console_render_markup`. NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_string_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `s` came from `CString::into_raw` in `rich_console_render_markup`.
        drop(unsafe { CString::from_raw(s) });
    }));
}

/// Shared helper: NULL-check + panic-guard a `&mut RichConsole` mutation.
fn with_console(console: *mut RichConsole, f: impl FnOnce(&mut RichConsole)) {
    if console.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_console_new`.
        f(unsafe { &mut *console });
    }));
}
