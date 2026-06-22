//! Shared FFI plumbing reused across every renderable phase.
//!
//! This module holds the pieces that are *not* tied to a single widget:
//! the universal render helper ([`render_to_cstring`]) and the stable
//! `int` -> enum / `int` -> box maps that container ABIs (Table, Panel,
//! Rule, Align, ...) all share. They live here so Wave-2 phases can plug
//! into them without conflicting edits.

use std::ffi::{CString, c_char, c_int};

use rich_rs::r#box::{self, Box as RichBox};
use rich_rs::{AlignMethod, JustifyMethod, Renderable, Style, VerticalAlignMethod};

use crate::RichConsole;

/// Parse a style string, rejecting input that produces no actual style.
///
/// `rich_rs::Style::parse` is intentionally lenient — it never returns `None`
/// and silently ignores tokens that are neither known attributes/named-styles
/// nor valid colors (so `"nonsense-xyz"` parses to an empty/null style). The
/// FFI contract is stricter: a style string is "parseable" only if it yields a
/// non-null style. This helper enforces that, so `rich_style_parse` and
/// `rich_text_set_style` agree on what counts as a valid style.
///
/// Returns `None` for empty/whitespace-only input and for input whose tokens
/// were all unrecognized (the resulting style is null).
pub(crate) fn parse_style(s: &str) -> Option<Style> {
    let parsed = Style::parse(s)?;
    if parsed.is_null() {
        return None;
    }
    Some(parsed)
}

/// Render any `&dyn Renderable` through a console into an owned C string.
///
/// Clears the console's capture buffer, prints the renderable with no
/// trailing newline (`end = ""`), and returns the captured bytes as a
/// heap-allocated NUL-terminated string. Returns NULL if printing fails
/// or the output contains an interior NUL byte.
///
/// The returned pointer is owned by the caller and MUST be freed with
/// `rich_string_free`.
pub(crate) fn render_to_cstring(
    con: &mut RichConsole,
    r: &(dyn Renderable + Send + Sync),
) -> *mut c_char {
    con.inner.clear_captured();
    // end = "" so we never impose a trailing newline on the C caller.
    if con.inner.print(r, None, None, None, false, "").is_err() {
        return std::ptr::null_mut();
    }
    match CString::new(con.inner.get_captured()) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Map a stable `int` code to a [`rich_rs::box::Box`].
///
/// Wave-2 box-using ABIs (`rich_table_set_box`, `rich_panel_set_box`, ...)
/// all share this table so the integer contract is identical everywhere.
///
/// Codes (documented in the generated header):
/// * `0` = ROUNDED
/// * `1` = HEAVY
/// * `2` = DOUBLE
/// * `3` = ASCII
/// * `4` = MINIMAL
/// * `5` = SQUARE
/// * `6` = SIMPLE
/// * `7` = HEAVY_HEAD
/// * `8` = HEAVY_EDGE
/// * `9` = DOUBLE_EDGE
/// * `10` = HORIZONTALS
/// * `11` = MINIMAL_HEAVY_HEAD
/// * `12` = SIMPLE_HEAVY
/// * `13` = MARKDOWN
///
/// Any other value yields `None` (callers should treat that as "leave the
/// box unchanged").
pub(crate) mod box_ids {
    use super::{RichBox, r#box};

    /// Resolve an `int` box id to a concrete `Box`, or `None` if out of range.
    #[allow(dead_code)] // Consumed by Wave-2 (Table, Panel); landed now to avoid conflicts.
    pub(crate) fn from_int(id: i32) -> Option<RichBox> {
        let b = match id {
            0 => r#box::ROUNDED,
            1 => r#box::HEAVY,
            2 => r#box::DOUBLE,
            3 => r#box::ASCII,
            4 => r#box::MINIMAL,
            5 => r#box::SQUARE,
            6 => r#box::SIMPLE,
            7 => r#box::HEAVY_HEAD,
            8 => r#box::HEAVY_EDGE,
            9 => r#box::DOUBLE_EDGE,
            10 => r#box::HORIZONTALS,
            11 => r#box::MINIMAL_HEAVY_HEAD,
            12 => r#box::SIMPLE_HEAVY,
            13 => r#box::MARKDOWN,
            _ => return None,
        };
        Some(b)
    }
}

/// Map a stable `int` code to an [`AlignMethod`] (horizontal alignment).
///
/// Codes: `0` = Left, `1` = Center, `2` = Right. Any other value -> `None`.
#[allow(dead_code)] // Consumed by Wave-2 (Rule, Align); landed now to avoid conflicts.
pub(crate) fn align_method_from_int(id: c_int) -> Option<AlignMethod> {
    match id {
        0 => Some(AlignMethod::Left),
        1 => Some(AlignMethod::Center),
        2 => Some(AlignMethod::Right),
        _ => None,
    }
}

/// Map a stable `int` code to a [`VerticalAlignMethod`].
///
/// Codes: `0` = Top, `1` = Middle, `2` = Bottom. Any other value -> `None`.
#[allow(dead_code)] // Consumed by Wave-2 (Align); landed now to avoid conflicts.
pub(crate) fn vertical_align_method_from_int(id: c_int) -> Option<VerticalAlignMethod> {
    match id {
        0 => Some(VerticalAlignMethod::Top),
        1 => Some(VerticalAlignMethod::Middle),
        2 => Some(VerticalAlignMethod::Bottom),
        _ => None,
    }
}

/// Map a stable `int` code to a [`JustifyMethod`].
///
/// Codes: `0` = Default, `1` = Left, `2` = Center, `3` = Right, `4` = Full.
/// Any other value -> `None`.
#[allow(dead_code)] // Consumed by Wave-2 (Columns, Markdown); landed now to avoid conflicts.
pub(crate) fn justify_method_from_int(id: c_int) -> Option<JustifyMethod> {
    match id {
        0 => Some(JustifyMethod::Default),
        1 => Some(JustifyMethod::Left),
        2 => Some(JustifyMethod::Center),
        3 => Some(JustifyMethod::Right),
        4 => Some(JustifyMethod::Full),
        _ => None,
    }
}
