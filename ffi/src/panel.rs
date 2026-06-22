//! Phase 3 — Panel C ABI.
//!
//! Implemented by the `ffi-phase3` lane. All functions here are
//! `#[unsafe(no_mangle)] pub extern "C"` and reuse the Phase-1 plumbing in
//! `crate::common` and the `crate::RichRenderable` composition currency.
//! cbindgen picks them up from this module automatically.

use std::ffi::{CStr, c_char, c_int, c_uint};
use std::panic::{AssertUnwindSafe, catch_unwind};

use rich_rs::Panel;

use crate::common::{box_ids, parse_style};
use crate::RichRenderable;

/// Opaque `Panel` builder handle. Wraps a content `Renderable` in a styled
/// border box.
///
/// Build it from a `RichRenderable` content handle (which is CONSUMED), apply
/// any setters, then either `rich_panel_finish` it into a `RichRenderable` or
/// `rich_panel_free` it.
///
/// The inner `Option<Panel>` lets `rich_panel_finish` and the consuming
/// `with_*` builders take the value by move.
pub struct RichPanel(Option<Panel>);

/// Create a `Panel` wrapping the given content renderable.
///
/// CONSUMES `content`: the `RichRenderable*` is moved into the panel and is
/// invalid afterward (do NOT free it). Returns NULL if `content` is NULL.
/// Free the result with `rich_panel_finish` or `rich_panel_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_new(content: *mut RichRenderable) -> *mut RichPanel {
    if content.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in a `*_finish` function; consumed here.
        let inner = unsafe { Box::from_raw(content) }.0;
        Box::into_raw(Box::new(RichPanel(Some(Panel::new(inner)))))
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Set the panel's title (rendered into the top border). `title` is parsed as
/// plain text. A NULL/invalid-UTF-8 `title` is a no-op. Does not consume the
/// handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_title(panel: *mut RichPanel, title: *const c_char) {
    if panel.is_null() || title.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle; caller guarantees a valid NUL-terminated string.
        let p = unsafe { &mut *panel };
        let Some(s) = (unsafe { CStr::from_ptr(title) }).to_str().ok() else {
            return;
        };
        if let Some(v) = p.0.take() {
            p.0 = Some(v.with_title(s.to_owned()));
        }
    }));
}

/// Set the panel's subtitle (rendered into the bottom border). `subtitle` is
/// parsed as plain text. A NULL/invalid-UTF-8 `subtitle` is a no-op. Does not
/// consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_subtitle(panel: *mut RichPanel, subtitle: *const c_char) {
    if panel.is_null() || subtitle.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle; caller guarantees a valid NUL-terminated string.
        let p = unsafe { &mut *panel };
        let Some(s) = (unsafe { CStr::from_ptr(subtitle) }).to_str().ok() else {
            return;
        };
        if let Some(v) = p.0.take() {
            p.0 = Some(v.with_subtitle(s.to_owned()));
        }
    }));
}

/// Set the panel's box-drawing style by integer id (see `box_ids` in the
/// generated header: 0 = ROUNDED, 1 = HEAVY, 2 = DOUBLE, 3 = ASCII, ...).
/// An out-of-range id is a no-op (leaves the box unchanged). Does not consume
/// the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_box(panel: *mut RichPanel, box_id: c_int) {
    if panel.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_panel_new`.
        let p = unsafe { &mut *panel };
        let Some(b) = box_ids::from_int(box_id) else {
            return;
        };
        if let Some(v) = p.0.take() {
            p.0 = Some(v.with_box(b));
        }
    }));
}

/// Set whether the panel expands to the full available width (`true`) or fits
/// its content (`false`). Does not consume the handle. NULL handle is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_expand(panel: *mut RichPanel, expand: bool) {
    if panel.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_panel_new`.
        let p = unsafe { &mut *panel };
        if let Some(v) = p.0.take() {
            p.0 = Some(v.with_expand(expand));
        }
    }));
}

/// Set a fixed panel width in cells. Does not consume the handle. NULL handle
/// is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_width(panel: *mut RichPanel, width: c_uint) {
    if panel.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_panel_new`.
        let p = unsafe { &mut *panel };
        if let Some(v) = p.0.take() {
            p.0 = Some(v.with_width(width as usize));
        }
    }));
}

/// Set the panel's interior padding in cells, in CSS order
/// (top, right, bottom, left). Does not consume the handle. NULL handle is a
/// no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_padding(
    panel: *mut RichPanel,
    top: c_uint,
    right: c_uint,
    bottom: c_uint,
    left: c_uint,
) {
    if panel.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_panel_new`.
        let p = unsafe { &mut *panel };
        if let Some(v) = p.0.take() {
            p.0 = Some(v.with_padding((
                top as usize,
                right as usize,
                bottom as usize,
                left as usize,
            )));
        }
    }));
}

/// Set the panel's content/background style (e.g. `"on grey15"`).
///
/// Parses `style` with `Style::parse`. A NULL/invalid/unparseable `style` is a
/// no-op (the panel is left unchanged). Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_style(panel: *mut RichPanel, style: *const c_char) {
    if panel.is_null() || style.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle; caller guarantees a valid NUL-terminated string.
        let p = unsafe { &mut *panel };
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

/// Set the panel's border style (e.g. `"bold cyan"`).
///
/// Parses `style` with `Style::parse`. A NULL/invalid/unparseable `style` is a
/// no-op (the panel is left unchanged). Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_border_style(panel: *mut RichPanel, style: *const c_char) {
    if panel.is_null() || style.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle; caller guarantees a valid NUL-terminated string.
        let p = unsafe { &mut *panel };
        let Some(s) = (unsafe { CStr::from_ptr(style) }).to_str().ok() else {
            return;
        };
        if let Some(parsed) = parse_style(s)
            && let Some(v) = p.0.take()
        {
            p.0 = Some(v.with_border_style(parsed));
        }
    }));
}

/// CONSUME a `RichPanel`, erasing it into the type-erased `RichRenderable`
/// composition currency. The `RichPanel*` is invalid afterward.
///
/// Returns NULL if `panel` is NULL or was already finished. The returned
/// `RichRenderable*` must be freed with `rich_renderable_free` (unless it is
/// passed into a consuming container).
#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_finish(panel: *mut RichPanel) -> *mut RichRenderable {
    if panel.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_panel_new`.
        let boxed = unsafe { Box::from_raw(panel) };
        let inner = boxed.0?;
        Some(Box::into_raw(Box::new(RichRenderable(Box::new(inner)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Free a `RichPanel` that was created but never `rich_panel_finish`ed.
/// NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_free(panel: *mut RichPanel) {
    if panel.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_panel_new`.
        drop(unsafe { Box::from_raw(panel) });
    }));
}
