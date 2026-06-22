//! Phase 5 — Layout C ABI (Rule, Columns, Align, Padding).
//!
//! Implemented by the `ffi-phase5` lane. All functions here are
//! `#[unsafe(no_mangle)] pub extern "C"` and reuse the Phase-1 plumbing in
//! `crate::common` and the `crate::RichRenderable` composition currency.
//! cbindgen picks them up from this module automatically.

use std::ffi::{CStr, c_char, c_int, c_uint};
use std::panic::{AssertUnwindSafe, catch_unwind};

use rich_rs::{Align, AlignMethod, Columns, Padding, Rule};

use crate::RichRenderable;
use crate::common::{align_method_from_int, parse_style, vertical_align_method_from_int};

// ===========================================================================
// Phase 5 — Rule
// ===========================================================================

/// Opaque `Rule` (horizontal divider) builder handle. Build it, optionally
/// configure it, then either `rich_rule_finish` it into a `RichRenderable` or
/// `rich_rule_free` it.
///
/// The inner `Option<Rule>` lets consuming builder setters take/replace the
/// value and lets `rich_rule_finish` move it out.
pub struct RichRule(Option<Rule>);

/// Create a new `Rule` with default settings (no title, "─" line, centered).
///
/// Returns NULL on allocation failure. Free with `rich_rule_finish` or
/// `rich_rule_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_rule_new() -> *mut RichRule {
    catch_unwind(AssertUnwindSafe(|| {
        Box::into_raw(Box::new(RichRule(Some(Rule::new()))))
    }))
    .unwrap_or(std::ptr::null_mut())
}

/// Set the rule's title (supports BBCode-style markup). `title` must be valid
/// NUL-terminated UTF-8. A NULL/invalid `title` is a no-op. Does not consume
/// the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_rule_set_title(rule: *mut RichRule, title: *const c_char) {
    if rule.is_null() || title.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle; caller guarantees a valid NUL-terminated string.
        let r = unsafe { &mut *rule };
        let Some(s) = (unsafe { CStr::from_ptr(title) }).to_str().ok() else {
            return;
        };
        if let Some(v) = r.0.take() {
            r.0 = Some(v.with_title(s));
        }
    }));
}

/// Set the characters used to draw the rule line (e.g. "═" or "-="). `characters`
/// must be valid NUL-terminated UTF-8. A NULL/invalid value is a no-op. Does not
/// consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_rule_set_characters(rule: *mut RichRule, characters: *const c_char) {
    if rule.is_null() || characters.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle; caller guarantees a valid NUL-terminated string.
        let r = unsafe { &mut *rule };
        let Some(s) = (unsafe { CStr::from_ptr(characters) }).to_str().ok() else {
            return;
        };
        if let Some(v) = r.0.take() {
            r.0 = Some(v.with_characters(s));
        }
    }));
}

/// Set the rule's line style (e.g. `"bold red"`). Parsed with `Style::parse`;
/// a NULL/invalid/unparseable `style` is a no-op. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_rule_set_style(rule: *mut RichRule, style: *const c_char) {
    if rule.is_null() || style.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle; caller guarantees a valid NUL-terminated string.
        let r = unsafe { &mut *rule };
        let Some(s) = (unsafe { CStr::from_ptr(style) }).to_str().ok() else {
            return;
        };
        if let Some(parsed) = parse_style(s)
            && let Some(v) = r.0.take()
        {
            r.0 = Some(v.with_style(parsed));
        }
    }));
}

/// Set the title alignment: 0 = Left, 1 = Center, 2 = Right. Out-of-range
/// values are a no-op. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_rule_set_align(rule: *mut RichRule, align: c_int) {
    if rule.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_rule_new`.
        let r = unsafe { &mut *rule };
        if let Some(method) = align_method_from_int(align)
            && let Some(v) = r.0.take()
        {
            r.0 = Some(v.with_align(method));
        }
    }));
}

/// CONSUME a `RichRule`, erasing it into a `RichRenderable`. The `RichRule*` is
/// invalid afterward. Returns NULL if `rule` is NULL or was already finished.
/// The returned `RichRenderable*` must be freed with `rich_renderable_free`
/// (unless passed into a consuming container).
#[unsafe(no_mangle)]
pub extern "C" fn rich_rule_finish(rule: *mut RichRule) -> *mut RichRenderable {
    if rule.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_rule_new`.
        let boxed = unsafe { Box::from_raw(rule) };
        let inner = boxed.0?;
        Some(Box::into_raw(Box::new(RichRenderable(Box::new(inner)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Free a `RichRule` that was created but never `rich_rule_finish`ed.
/// NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_rule_free(rule: *mut RichRule) {
    if rule.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_rule_new`.
        drop(unsafe { Box::from_raw(rule) });
    }));
}

// ===========================================================================
// Phase 5 — Columns
// ===========================================================================

/// Opaque `Columns` (flow-layout) builder handle. Build it, add children and
/// configure it, then either `rich_columns_finish` it into a `RichRenderable`
/// or `rich_columns_free` it.
pub struct RichColumns(Option<Columns>);

/// Create a new empty `Columns`. Returns NULL on allocation failure. Free with
/// `rich_columns_finish` or `rich_columns_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_columns_new() -> *mut RichColumns {
    catch_unwind(AssertUnwindSafe(|| {
        Box::into_raw(Box::new(RichColumns(Some(Columns::empty()))))
    }))
    .unwrap_or(std::ptr::null_mut())
}

/// CONSUME a `RichRenderable` and append it as a column child. The
/// `RichRenderable*` is invalid afterward (it is freed even if `columns` is
/// NULL, so no leak). A NULL `renderable` is a no-op. Does not consume the
/// `columns` handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_columns_add(columns: *mut RichColumns, renderable: *mut RichRenderable) {
    if renderable.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in a `*_finish` function.
        let child = unsafe { Box::from_raw(renderable) };
        if columns.is_null() {
            return; // child dropped here — no leak.
        }
        // SAFETY: non-NULL handle from `rich_columns_new`.
        let c = unsafe { &mut *columns };
        if let Some(v) = c.0.as_mut() {
            v.add(child.0);
        }
    }));
}

/// Append a plain-text column child. `text` must be valid NUL-terminated UTF-8.
/// A NULL/invalid `text` is a no-op. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_columns_add_str(columns: *mut RichColumns, text: *const c_char) {
    if columns.is_null() || text.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle; caller guarantees a valid NUL-terminated string.
        let c = unsafe { &mut *columns };
        let Some(s) = (unsafe { CStr::from_ptr(text) }).to_str().ok() else {
            return;
        };
        if let Some(v) = c.0.as_mut() {
            v.add_str(s);
        }
    }));
}

/// Set whether all columns share equal width. A NULL handle is a no-op. Does
/// not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_columns_set_equal(columns: *mut RichColumns, equal: bool) {
    if columns.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_columns_new`.
        let c = unsafe { &mut *columns };
        if let Some(v) = c.0.take() {
            c.0 = Some(v.with_equal(equal));
        }
    }));
}

/// Set whether columns expand to fill the available width. A NULL handle is a
/// no-op. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_columns_set_expand(columns: *mut RichColumns, expand: bool) {
    if columns.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_columns_new`.
        let c = unsafe { &mut *columns };
        if let Some(v) = c.0.take() {
            c.0 = Some(v.with_expand(expand));
        }
    }));
}

/// Set the inter-column padding as `(vertical, horizontal)` cell counts. A NULL
/// handle is a no-op. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_columns_set_padding(
    columns: *mut RichColumns,
    vertical: c_uint,
    horizontal: c_uint,
) {
    if columns.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_columns_new`.
        let c = unsafe { &mut *columns };
        if let Some(v) = c.0.take() {
            c.0 = Some(v.with_padding((vertical as usize, horizontal as usize)));
        }
    }));
}

/// CONSUME a `RichColumns`, erasing it into a `RichRenderable`. The
/// `RichColumns*` is invalid afterward. Returns NULL if `columns` is NULL or
/// was already finished. The returned `RichRenderable*` must be freed with
/// `rich_renderable_free` (unless passed into a consuming container).
#[unsafe(no_mangle)]
pub extern "C" fn rich_columns_finish(columns: *mut RichColumns) -> *mut RichRenderable {
    if columns.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_columns_new`.
        let boxed = unsafe { Box::from_raw(columns) };
        let inner = boxed.0?;
        Some(Box::into_raw(Box::new(RichRenderable(Box::new(inner)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Free a `RichColumns` that was created but never `rich_columns_finish`ed.
/// NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_columns_free(columns: *mut RichColumns) {
    if columns.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_columns_new`.
        drop(unsafe { Box::from_raw(columns) });
    }));
}

// ===========================================================================
// Phase 5 — Align
// ===========================================================================

/// Opaque `Align` builder handle. Wraps a child renderable with horizontal
/// (and optional vertical) alignment. Build it, configure it, then either
/// `rich_align_finish` it into a `RichRenderable` or `rich_align_free` it.
pub struct RichAlign(Option<Align>);

/// Create an `Align` wrapping `content` with horizontal alignment `h_align`
/// (0 = Left, 1 = Center, 2 = Right; any other value defaults to Left).
///
/// CONSUMES `content`: the `RichRenderable*` is invalid afterward. Returns NULL
/// if `content` is NULL. Free the result with `rich_align_finish` or
/// `rich_align_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_align_new(content: *mut RichRenderable, h_align: c_int) -> *mut RichAlign {
    if content.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in a `*_finish` function.
        let child = unsafe { Box::from_raw(content) };
        let method = align_method_from_int(h_align).unwrap_or(AlignMethod::Left);
        Box::into_raw(Box::new(RichAlign(Some(Align::new(child.0, method)))))
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Create an `Align` that centers `content` horizontally.
///
/// CONSUMES `content`: the `RichRenderable*` is invalid afterward. Returns NULL
/// if `content` is NULL. Free the result with `rich_align_finish` or
/// `rich_align_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_align_center(content: *mut RichRenderable) -> *mut RichAlign {
    if content.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in a `*_finish` function.
        let child = unsafe { Box::from_raw(content) };
        Box::into_raw(Box::new(RichAlign(Some(Align::center(child.0)))))
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Set vertical alignment: 0 = Top, 1 = Middle, 2 = Bottom. Out-of-range values
/// are a no-op. A NULL handle is a no-op. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_align_set_vertical(align: *mut RichAlign, vertical: c_int) {
    if align.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_align_new`/`rich_align_center`.
        let a = unsafe { &mut *align };
        if let Some(method) = vertical_align_method_from_int(vertical)
            && let Some(v) = a.0.take()
        {
            a.0 = Some(v.with_vertical(method));
        }
    }));
}

/// Set the fixed alignment width in cells. A NULL handle is a no-op. Does not
/// consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_align_set_width(align: *mut RichAlign, width: c_uint) {
    if align.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_align_new`/`rich_align_center`.
        let a = unsafe { &mut *align };
        if let Some(v) = a.0.take() {
            a.0 = Some(v.with_width(width as usize));
        }
    }));
}

/// CONSUME a `RichAlign`, erasing it into a `RichRenderable`. The `RichAlign*`
/// is invalid afterward. Returns NULL if `align` is NULL or was already
/// finished. The returned `RichRenderable*` must be freed with
/// `rich_renderable_free` (unless passed into a consuming container).
#[unsafe(no_mangle)]
pub extern "C" fn rich_align_finish(align: *mut RichAlign) -> *mut RichRenderable {
    if align.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_align_new`/`rich_align_center`.
        let boxed = unsafe { Box::from_raw(align) };
        let inner = boxed.0?;
        Some(Box::into_raw(Box::new(RichRenderable(Box::new(inner)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Free a `RichAlign` that was created but never `rich_align_finish`ed.
/// NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_align_free(align: *mut RichAlign) {
    if align.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_align_new`/`rich_align_center`.
        drop(unsafe { Box::from_raw(align) });
    }));
}

// ===========================================================================
// Phase 5 — Padding
// ===========================================================================

/// Opaque `Padding` builder handle. Wraps a child renderable with cell padding
/// on each side. Build it, configure it, then either `rich_padding_finish` it
/// into a `RichRenderable` or `rich_padding_free` it.
pub struct RichPadding(Option<Padding>);

/// Create a `Padding` wrapping `content` with `top`/`right`/`bottom`/`left`
/// cell padding.
///
/// CONSUMES `content`: the `RichRenderable*` is invalid afterward. Returns NULL
/// if `content` is NULL. Free the result with `rich_padding_finish` or
/// `rich_padding_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_padding_new(
    content: *mut RichRenderable,
    top: c_uint,
    right: c_uint,
    bottom: c_uint,
    left: c_uint,
) -> *mut RichPadding {
    if content.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in a `*_finish` function.
        let child = unsafe { Box::from_raw(content) };
        let pad = (
            top as usize,
            right as usize,
            bottom as usize,
            left as usize,
        );
        Box::into_raw(Box::new(RichPadding(Some(Padding::new(child.0, pad)))))
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Set the padding background style (e.g. `"on blue"`). Parsed with
/// `Style::parse`; a NULL/invalid/unparseable `style` is a no-op. Does not
/// consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_padding_set_style(padding: *mut RichPadding, style: *const c_char) {
    if padding.is_null() || style.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle; caller guarantees a valid NUL-terminated string.
        let p = unsafe { &mut *padding };
        let Some(s) = (unsafe { CStr::from_ptr(style) }).to_str().ok() else {
            return;
        };
        if let Some(parsed) = parse_style(s)
            && let Some(v) = p.0.take()
        {
            p.0 = Some(v.with_style(parsed));
        }
    }));
}

/// Set whether the padding expands to fill the available width. A NULL handle
/// is a no-op. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_padding_set_expand(padding: *mut RichPadding, expand: bool) {
    if padding.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_padding_new`.
        let p = unsafe { &mut *padding };
        if let Some(v) = p.0.take() {
            p.0 = Some(v.with_expand(expand));
        }
    }));
}

/// CONSUME a `RichPadding`, erasing it into a `RichRenderable`. The
/// `RichPadding*` is invalid afterward. Returns NULL if `padding` is NULL or
/// was already finished. The returned `RichRenderable*` must be freed with
/// `rich_renderable_free` (unless passed into a consuming container).
#[unsafe(no_mangle)]
pub extern "C" fn rich_padding_finish(padding: *mut RichPadding) -> *mut RichRenderable {
    if padding.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_padding_new`.
        let boxed = unsafe { Box::from_raw(padding) };
        let inner = boxed.0?;
        Some(Box::into_raw(Box::new(RichRenderable(Box::new(inner)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Free a `RichPadding` that was created but never `rich_padding_finish`ed.
/// NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_padding_free(padding: *mut RichPadding) {
    if padding.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_padding_new`.
        drop(unsafe { Box::from_raw(padding) });
    }));
}
