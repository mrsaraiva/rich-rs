//! Phase 6 — Content C ABI (Syntax, Markdown, Json).
//!
//! Implemented by the `ffi-phase6` lane. All functions here are
//! `#[unsafe(no_mangle)] pub extern "C"` and reuse the Phase-1 plumbing in
//! `crate::common` and the `crate::RichRenderable` composition currency.
//! cbindgen picks them up from this module automatically.

use std::ffi::{CStr, c_char, c_int, c_uint};
use std::panic::{AssertUnwindSafe, catch_unwind};

use rich_rs::markdown::Markdown;
use rich_rs::{Json, Syntax};

use crate::RichRenderable;
use crate::common::justify_method_from_int;

// ===========================================================================
// Phase 6 — RichSyntax: source-code syntax highlighting
// ===========================================================================

/// Opaque `Syntax` builder handle. Build it, optionally configure it, then
/// either `rich_syntax_finish` it into a `RichRenderable` or `rich_syntax_free`
/// it.
///
/// The inner `Option<Syntax>` lets the consuming builder setters take/replace
/// the value and lets `rich_syntax_finish` move it out.
pub struct RichSyntax(Option<rich_rs::Syntax>);

/// Create a `RichSyntax` from a code string and an explicit lexer name.
///
/// `code` and `lexer` must be valid NUL-terminated UTF-8 (e.g. `lexer = "rust"`,
/// `"python"`, `"json"`). Returns NULL if either pointer is NULL or not valid
/// UTF-8. Free with `rich_syntax_finish` or `rich_syntax_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_syntax_new(code: *const c_char, lexer: *const c_char) -> *mut RichSyntax {
    if code.is_null() || lexer.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees valid NUL-terminated strings.
        let code = unsafe { CStr::from_ptr(code) }.to_str().ok()?;
        let lexer = unsafe { CStr::from_ptr(lexer) }.to_str().ok()?;
        Some(Box::into_raw(Box::new(RichSyntax(Some(Syntax::new(
            code, lexer,
        ))))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Create a `RichSyntax` by reading a source file from disk; the lexer is
/// auto-detected from the file extension.
///
/// `path` must be valid NUL-terminated UTF-8. Returns NULL if `path` is NULL,
/// not valid UTF-8, or the file cannot be read (any IO error). Free with
/// `rich_syntax_finish` or `rich_syntax_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_syntax_from_path(path: *const c_char) -> *mut RichSyntax {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees a valid NUL-terminated string.
        let path = unsafe { CStr::from_ptr(path) }.to_str().ok()?;
        // IO error => None => NULL handle.
        let syntax = Syntax::from_path(path).ok()?;
        Some(Box::into_raw(Box::new(RichSyntax(Some(syntax)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Set the syntax-highlighting theme by name (e.g. `"monokai"`, `"ansi_dark"`).
///
/// `theme` must be valid NUL-terminated UTF-8. A NULL/invalid `theme` is a
/// no-op. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_syntax_set_theme(syntax: *mut RichSyntax, theme: *const c_char) {
    if syntax.is_null() || theme.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle; caller guarantees a valid NUL-terminated string.
        let s = unsafe { &mut *syntax };
        let Some(theme) = (unsafe { CStr::from_ptr(theme) }).to_str().ok() else {
            return;
        };
        if let Some(v) = s.0.take() {
            s.0 = Some(v.with_theme(theme));
        }
    }));
}

/// Enable or disable line-number gutter display. No-op if `syntax` is NULL.
/// Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_syntax_set_line_numbers(syntax: *mut RichSyntax, enabled: bool) {
    if syntax.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from a `rich_syntax_new*` function.
        let s = unsafe { &mut *syntax };
        if let Some(v) = s.0.take() {
            s.0 = Some(v.with_line_numbers(enabled));
        }
    }));
}

/// Enable or disable word wrapping of long lines. No-op if `syntax` is NULL.
/// Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_syntax_set_word_wrap(syntax: *mut RichSyntax, enabled: bool) {
    if syntax.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from a `rich_syntax_new*` function.
        let s = unsafe { &mut *syntax };
        if let Some(v) = s.0.take() {
            s.0 = Some(v.with_word_wrap(enabled));
        }
    }));
}

/// Restrict rendering to a line range (1-based, inclusive).
///
/// Pass `-1` for either `start` or `end` to leave that bound unset (`None`).
/// Negative values other than `-1` are treated as unset too. No-op if `syntax`
/// is NULL. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_syntax_set_line_range(syntax: *mut RichSyntax, start: c_int, end: c_int) {
    if syntax.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from a `rich_syntax_new*` function.
        let s = unsafe { &mut *syntax };
        let start = if start < 0 {
            None
        } else {
            Some(start as usize)
        };
        let end = if end < 0 { None } else { Some(end as usize) };
        if let Some(v) = s.0.take() {
            s.0 = Some(v.with_line_range(start, end));
        }
    }));
}

/// CONSUME a `RichSyntax`, erasing it into the type-erased `RichRenderable`
/// composition currency. The `RichSyntax*` is invalid afterward.
///
/// Returns NULL if `syntax` is NULL or was already finished. The returned
/// `RichRenderable*` must be freed with `rich_renderable_free` (unless it is
/// passed into a consuming container).
#[unsafe(no_mangle)]
pub extern "C" fn rich_syntax_finish(syntax: *mut RichSyntax) -> *mut RichRenderable {
    if syntax.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in a `rich_syntax_new*` function.
        let boxed = unsafe { Box::from_raw(syntax) };
        let inner = boxed.0?;
        Some(Box::into_raw(Box::new(RichRenderable(Box::new(inner)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Free a `RichSyntax` that was created but never `rich_syntax_finish`ed.
/// NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_syntax_free(syntax: *mut RichSyntax) {
    if syntax.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in a `rich_syntax_new*` function.
        drop(unsafe { Box::from_raw(syntax) });
    }));
}

// ===========================================================================
// Phase 6 — RichMarkdown: Markdown document rendering
// ===========================================================================

/// Opaque `Markdown` builder handle. Build it, optionally configure it, then
/// either `rich_markdown_finish` it into a `RichRenderable` or
/// `rich_markdown_free` it.
///
/// The inner `Option<Markdown>` lets the consuming builder setters take/replace
/// the value and lets `rich_markdown_finish` move it out.
pub struct RichMarkdown(Option<rich_rs::markdown::Markdown>);

/// Create a `RichMarkdown` from a Markdown source string.
///
/// `source` must be valid NUL-terminated UTF-8. Returns NULL if `source` is
/// NULL or not valid UTF-8. Free with `rich_markdown_finish` or
/// `rich_markdown_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_markdown_new(source: *const c_char) -> *mut RichMarkdown {
    if source.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees a valid NUL-terminated string.
        let source = unsafe { CStr::from_ptr(source) }.to_str().ok()?;
        Some(Box::into_raw(Box::new(RichMarkdown(Some(Markdown::new(
            source,
        ))))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Set the theme used for fenced code blocks (e.g. `"monokai"`).
///
/// `theme` must be valid NUL-terminated UTF-8. A NULL/invalid `theme` is a
/// no-op. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_markdown_set_code_theme(markdown: *mut RichMarkdown, theme: *const c_char) {
    if markdown.is_null() || theme.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle; caller guarantees a valid NUL-terminated string.
        let m = unsafe { &mut *markdown };
        let Some(theme) = (unsafe { CStr::from_ptr(theme) }).to_str().ok() else {
            return;
        };
        if let Some(v) = m.0.take() {
            m.0 = Some(v.with_code_theme(theme));
        }
    }));
}

/// Enable or disable rendering of links as terminal hyperlinks. No-op if
/// `markdown` is NULL. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_markdown_set_hyperlinks(markdown: *mut RichMarkdown, enabled: bool) {
    if markdown.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_markdown_new`.
        let m = unsafe { &mut *markdown };
        if let Some(v) = m.0.take() {
            m.0 = Some(v.with_hyperlinks(enabled));
        }
    }));
}

/// Set the justification for Markdown text blocks.
///
/// `justify` codes: `0` = Default, `1` = Left, `2` = Center, `3` = Right,
/// `4` = Full. Any other value is a no-op (justification left unchanged). No-op
/// if `markdown` is NULL. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_markdown_set_justify(markdown: *mut RichMarkdown, justify: c_int) {
    if markdown.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_markdown_new`.
        let m = unsafe { &mut *markdown };
        let Some(method) = justify_method_from_int(justify) else {
            return;
        };
        if let Some(v) = m.0.take() {
            m.0 = Some(v.with_justify(method));
        }
    }));
}

/// CONSUME a `RichMarkdown`, erasing it into the type-erased `RichRenderable`
/// composition currency. The `RichMarkdown*` is invalid afterward.
///
/// Returns NULL if `markdown` is NULL or was already finished. The returned
/// `RichRenderable*` must be freed with `rich_renderable_free` (unless it is
/// passed into a consuming container).
#[unsafe(no_mangle)]
pub extern "C" fn rich_markdown_finish(markdown: *mut RichMarkdown) -> *mut RichRenderable {
    if markdown.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_markdown_new`.
        let boxed = unsafe { Box::from_raw(markdown) };
        let inner = boxed.0?;
        Some(Box::into_raw(Box::new(RichRenderable(Box::new(inner)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Free a `RichMarkdown` that was created but never `rich_markdown_finish`ed.
/// NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_markdown_free(markdown: *mut RichMarkdown) {
    if markdown.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_markdown_new`.
        drop(unsafe { Box::from_raw(markdown) });
    }));
}

// ===========================================================================
// Phase 6 — RichJson: pretty-printed, highlighted JSON
// ===========================================================================

/// Opaque `Json` builder handle. Build it, then either `rich_json_finish` it
/// into a `RichRenderable` or `rich_json_free` it.
///
/// The inner `Option<Json>` lets `rich_json_finish` move the value out.
pub struct RichJson(Option<rich_rs::Json>);

/// Create a `RichJson` from a JSON string.
///
/// Re-formats `data` with `indent` spaces per level, optionally applies JSON
/// syntax `highlight`ing, and optionally `sort_keys` on objects.
///
/// `data` must be valid NUL-terminated UTF-8. Returns NULL if `data` is NULL or
/// not valid UTF-8.
///
/// NOTE: `rich_rs::Json::new` does NOT validate the input — if `data` is not
/// well-formed JSON it is rendered as-is (passed through unformatted) rather
/// than rejected. This function therefore returns a non-NULL handle for any
/// valid-UTF-8 `data`; it does not (and cannot, without a parser dependency)
/// signal invalid JSON by returning NULL. Free with `rich_json_finish` or
/// `rich_json_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_json_new(
    data: *const c_char,
    indent: c_uint,
    highlight: bool,
    sort_keys: bool,
) -> *mut RichJson {
    if data.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees a valid NUL-terminated string.
        let data = unsafe { CStr::from_ptr(data) }.to_str().ok()?;
        let json = Json::new(data, indent as usize, highlight, sort_keys);
        Some(Box::into_raw(Box::new(RichJson(Some(json)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// CONSUME a `RichJson`, erasing it into the type-erased `RichRenderable`
/// composition currency. The `RichJson*` is invalid afterward.
///
/// Returns NULL if `json` is NULL or was already finished. The returned
/// `RichRenderable*` must be freed with `rich_renderable_free` (unless it is
/// passed into a consuming container).
#[unsafe(no_mangle)]
pub extern "C" fn rich_json_finish(json: *mut RichJson) -> *mut RichRenderable {
    if json.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_json_new`.
        let boxed = unsafe { Box::from_raw(json) };
        let inner = boxed.0?;
        Some(Box::into_raw(Box::new(RichRenderable(Box::new(inner)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Free a `RichJson` that was created but never `rich_json_finish`ed.
/// NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_json_free(json: *mut RichJson) {
    if json.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_json_new`.
        drop(unsafe { Box::from_raw(json) });
    }));
}
