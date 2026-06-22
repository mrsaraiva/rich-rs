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

use rich_rs::{ColorSystem, Console, Renderable, Style, Text};

mod common;
use common::{parse_style, render_to_cstring};

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

// ===========================================================================
// Phase 1 — RichRenderable: the type-erased composition currency
// ===========================================================================

/// Opaque, type-erased renderable — the universal composition currency.
///
/// Anything that can be rendered or nested inside a container (Panel, Table,
/// Tree, Align, ...) is moved through one of these. A `RichRenderable*` is
/// produced by a typed builder's `*_finish` function (e.g. `rich_text_finish`)
/// and either rendered with `rich_console_render`, passed into a consuming
/// container constructor, or freed with `rich_renderable_free`.
pub struct RichRenderable(Box<dyn Renderable + Send + Sync>);

/// Render a `RichRenderable` to an owned, NUL-terminated C string.
///
/// BORROWS `renderable`: the handle stays valid and the caller still owns it
/// (free it later with `rich_renderable_free` unless it was consumed by a
/// container). Honors the console's current force-terminal / color / width
/// settings, so `set_force_terminal(false)` yields plain text with zero ANSI.
/// No trailing newline is appended.
///
/// Returns NULL if `console`/`renderable` is NULL or rendering fails. A
/// non-NULL result MUST be freed with `rich_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_console_render(
    console: *mut RichConsole,
    renderable: *const RichRenderable,
) -> *mut c_char {
    if console.is_null() || renderable.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handles from `rich_console_new` / a `*_finish` fn.
        let con = unsafe { &mut *console };
        let r = unsafe { &*renderable };
        render_to_cstring(con, r.0.as_ref())
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Free a `RichRenderable` created by a `*_finish` function but never passed
/// into a consuming container. NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_renderable_free(renderable: *mut RichRenderable) {
    if renderable.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in a `*_finish` function.
        drop(unsafe { Box::from_raw(renderable) });
    }));
}

// ===========================================================================
// Phase 1 — RichText: styled-text builder
// ===========================================================================

/// Opaque `Text` builder handle. Build it, optionally style it, then either
/// `rich_text_finish` it into a `RichRenderable` or `rich_text_free` it.
///
/// The inner `Option<Text>` lets `rich_text_finish` take the value by move.
pub struct RichText(Option<Text>);

/// Create a `RichText` from plain text (no markup parsing).
///
/// `text` must be valid NUL-terminated UTF-8. Returns NULL if `text` is NULL
/// or not valid UTF-8. Free with `rich_text_finish` or `rich_text_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_text_new(text: *const c_char) -> *mut RichText {
    if text.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees a valid NUL-terminated string.
        let s = unsafe { CStr::from_ptr(text) }.to_str().ok()?;
        Some(Box::into_raw(Box::new(RichText(Some(Text::plain(s))))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Create a `RichText` by parsing console markup (e.g. `[bold red]hi[/]`).
///
/// Honors the console's markup/emoji/highlight settings via `Console::render_str`.
/// `markup` must be valid NUL-terminated UTF-8. Returns NULL if either pointer
/// is NULL or `markup` is not valid UTF-8. Free with `rich_text_finish` or
/// `rich_text_free`. The `console` handle is borrowed (not consumed).
#[unsafe(no_mangle)]
pub extern "C" fn rich_text_new_markup(
    console: *mut RichConsole,
    markup: *const c_char,
) -> *mut RichText {
    if console.is_null() || markup.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle; caller guarantees a valid NUL-terminated string.
        let con = unsafe { &mut *console };
        let s = unsafe { CStr::from_ptr(markup) }.to_str().ok()?;
        let text = con.inner.render_str(s, None, None, None, None);
        Some(Box::into_raw(Box::new(RichText(Some(text)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Apply a base style (e.g. `"bold red on white"`) to a `RichText`.
///
/// Parses `style` with `Style::parse` and sets it as the text's base style via
/// `Text::set_base_style`. A NULL/invalid/unparseable `style` is a no-op (the
/// text is left unchanged). Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_text_set_style(text: *mut RichText, style: *const c_char) {
    if text.is_null() || style.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle; caller guarantees a valid NUL-terminated string.
        let t = unsafe { &mut *text };
        let Some(s) = (unsafe { CStr::from_ptr(style) }).to_str().ok() else {
            return;
        };
        if let Some(parsed) = parse_style(s)
            && let Some(inner) = t.0.as_mut()
        {
            inner.set_base_style(Some(parsed));
        }
    }));
}

/// CONSUME a `RichText`, erasing it into the type-erased `RichRenderable`
/// composition currency. The `RichText*` is invalid afterward.
///
/// Returns NULL if `text` is NULL or was already finished. The returned
/// `RichRenderable*` must be freed with `rich_renderable_free` (unless it is
/// passed into a consuming container).
#[unsafe(no_mangle)]
pub extern "C" fn rich_text_finish(text: *mut RichText) -> *mut RichRenderable {
    if text.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in a `rich_text_new*` function.
        let boxed = unsafe { Box::from_raw(text) };
        let inner = boxed.0?;
        Some(Box::into_raw(Box::new(RichRenderable(Box::new(inner)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Free a `RichText` that was created but never `rich_text_finish`ed.
/// NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_text_free(text: *mut RichText) {
    if text.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in a `rich_text_new*` function.
        drop(unsafe { Box::from_raw(text) });
    }));
}

// ===========================================================================
// Phase 1 — RichStyle: parsed-style handle
// ===========================================================================

/// Opaque parsed-style handle wrapping a `rich_rs::Style`. Created by
/// `rich_style_parse`, freed by `rich_style_free`. Held opaque for
/// forward-compat even though `Style` is `Copy` underneath.
// The inner Style is not yet read in Phase 1 (parse + free only). Wave-2
// phases (e.g. style-taking setters) will consume it; landed now so the
// handle type and its free fn are stable across the campaign.
#[allow(dead_code)]
pub struct RichStyle(Style);

/// Parse a style string (e.g. `"bold red on white"`) into a `RichStyle`.
///
/// `style` must be valid NUL-terminated UTF-8. Returns NULL if `style` is NULL,
/// not valid UTF-8, or produces no actual style (empty/whitespace input, or
/// input whose tokens are all unrecognized — e.g. `"nonsense-xyz"`). A non-NULL
/// result MUST be freed with `rich_style_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_style_parse(style: *const c_char) -> *mut RichStyle {
    if style.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees a valid NUL-terminated string.
        let s = unsafe { CStr::from_ptr(style) }.to_str().ok()?;
        let parsed = parse_style(s)?;
        Some(Box::into_raw(Box::new(RichStyle(parsed))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Free a `RichStyle` created by `rich_style_parse`. NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_style_free(style: *mut RichStyle) {
    if style.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_style_parse`.
        drop(unsafe { Box::from_raw(style) });
    }));
}
