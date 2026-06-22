//! Phase 2 — Table C ABI.
//!
//! Implemented by the `ffi-phase2` lane. All functions here are
//! `#[unsafe(no_mangle)] pub extern "C"` and reuse the Phase-1 plumbing in
//! `crate::common` and the `crate::RichRenderable` composition currency.
//! cbindgen picks them up from this module automatically.

use std::ffi::{CStr, c_char, c_int, c_uint};
use std::panic::{AssertUnwindSafe, catch_unwind};

use rich_rs::{Table, Text};

use crate::RichRenderable;
use crate::common::{box_ids, parse_style};

/// Opaque `Table` builder handle. Build it with setters / `add_column*` /
/// `add_row*`, then either `rich_table_finish` it into a `RichRenderable` or
/// `rich_table_free` it.
///
/// The inner `Option<Table>` lets `rich_table_finish` take the value by move
/// and lets the consuming `with_*` builders take/replace it in place.
pub struct RichTable(Option<Table>);

/// Create an empty `Table`. Returns NULL on allocation failure.
///
/// Free with `rich_table_finish` (which consumes it into a `RichRenderable`)
/// or `rich_table_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_new() -> *mut RichTable {
    catch_unwind(|| Box::into_raw(Box::new(RichTable(Some(Table::new())))))
        .unwrap_or(std::ptr::null_mut())
}

/// CONSUME a `RichTable`, erasing it into the type-erased `RichRenderable`
/// composition currency. The `RichTable*` is invalid afterward.
///
/// Returns NULL if `table` is NULL or was already finished. The returned
/// `RichRenderable*` must be freed with `rich_renderable_free` (unless it is
/// passed into a consuming container).
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_finish(table: *mut RichTable) -> *mut RichRenderable {
    if table.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_table_new`.
        let boxed = unsafe { Box::from_raw(table) };
        let inner = boxed.0?;
        Some(Box::into_raw(Box::new(RichRenderable(Box::new(inner)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Free a `RichTable` that was created but never `rich_table_finish`ed.
/// NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_free(table: *mut RichTable) {
    if table.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_table_new`.
        drop(unsafe { Box::from_raw(table) });
    }));
}

/// Shared helper: NULL-check + panic-guard a `&mut Table` mutation.
///
/// Skips silently if `table` is NULL or has already been finished (the inner
/// `Option<Table>` is empty).
fn with_table(table: *mut RichTable, f: impl FnOnce(&mut Table)) {
    if table.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_table_new`.
        let t = unsafe { &mut *table };
        if let Some(inner) = t.0.as_mut() {
            f(inner);
        }
    }));
}

/// Shared helper for the consuming `with_*` builders: take the inner `Table`,
/// run a consuming transform, and replace it. No-op if NULL / already finished.
fn map_table(table: *mut RichTable, f: impl FnOnce(Table) -> Table) {
    if table.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_table_new`.
        let t = unsafe { &mut *table };
        if let Some(inner) = t.0.take() {
            t.0.replace(f(inner));
        }
    }));
}

/// Set the table title (parsed as plain text). NULL/non-UTF-8 is a no-op.
/// Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_title(table: *mut RichTable, title: *const c_char) {
    if title.is_null() {
        return;
    }
    with_table(table, |t| {
        // SAFETY: caller guarantees a valid NUL-terminated string.
        if let Ok(s) = unsafe { CStr::from_ptr(title) }.to_str() {
            t.set_title(Some(Text::plain(s)));
        }
    });
}

/// Set the table caption (parsed as plain text). NULL/non-UTF-8 is a no-op.
/// Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_caption(table: *mut RichTable, caption: *const c_char) {
    if caption.is_null() {
        return;
    }
    with_table(table, |t| {
        // SAFETY: caller guarantees a valid NUL-terminated string.
        if let Ok(s) = unsafe { CStr::from_ptr(caption) }.to_str() {
            t.set_caption(Some(Text::plain(s)));
        }
    });
}

/// Set the box-drawing style by stable int id (see `box_ids::from_int`).
/// An out-of-range id is a no-op (the box is left unchanged).
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_box(table: *mut RichTable, box_id: c_int) {
    with_table(table, |t| {
        if let Some(b) = box_ids::from_int(box_id) {
            t.set_box(Some(b));
        }
    });
}

/// Show or hide the header row.
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_show_header(table: *mut RichTable, show: bool) {
    map_table(table, |t| t.with_show_header(show));
}

/// Show or hide horizontal lines between rows.
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_show_lines(table: *mut RichTable, show: bool) {
    map_table(table, |t| t.with_show_lines(show));
}

/// Show or hide the outer edge/border.
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_show_edge(table: *mut RichTable, show: bool) {
    map_table(table, |t| t.with_show_edge(show));
}

/// Expand the table to the full available width.
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_expand(table: *mut RichTable, expand: bool) {
    map_table(table, |t| t.with_expand(expand));
}

/// Set left/right cell padding (top/bottom stay 0).
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_padding(table: *mut RichTable, left: c_uint, right: c_uint) {
    map_table(table, |t| t.with_padding(left as usize, right as usize));
}

/// Set the base table style from a style string (e.g. `"on grey23"`).
/// NULL/non-UTF-8/unparseable input is a no-op. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_style(table: *mut RichTable, style: *const c_char) {
    if style.is_null() {
        return;
    }
    // SAFETY: caller guarantees a valid NUL-terminated string.
    let Ok(s) = (unsafe { CStr::from_ptr(style) }).to_str() else {
        return;
    };
    let Some(parsed) = parse_style(s) else {
        return;
    };
    map_table(table, |t| t.with_style(parsed));
}

/// Add a column with a plain-text header. NULL/non-UTF-8 header is a no-op.
/// Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_add_column(table: *mut RichTable, header: *const c_char) {
    if header.is_null() {
        return;
    }
    with_table(table, |t| {
        // SAFETY: caller guarantees a valid NUL-terminated string.
        if let Ok(s) = unsafe { CStr::from_ptr(header) }.to_str() {
            t.add_column_str(s);
        }
    });
}

/// Add a column whose header is a `RichRenderable`. CONSUMES `header`: the
/// pointer is invalid afterward (do NOT free it). A NULL `header` is a no-op.
///
/// If `table` is NULL or already finished, the `header` is still consumed and
/// dropped (it cannot be returned to the caller).
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_add_column_renderable(
    table: *mut RichTable,
    header: *mut RichRenderable,
) {
    if header.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `header` came from `Box::into_raw` in a `*_finish` function.
        let boxed = unsafe { Box::from_raw(header) };
        with_table(table, |t| t.add_column_renderable(boxed.0));
    }));
}

/// Add a row of plain-text cells. `cells` is an array of `count` NUL-terminated
/// UTF-8 strings. NULL `cells`, a NULL/non-UTF-8 element, or `count == 0` is a
/// no-op. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_add_row_strs(
    table: *mut RichTable,
    cells: *const *const c_char,
    count: c_uint,
) {
    if cells.is_null() || count == 0 {
        return;
    }
    with_table(table, |t| {
        // SAFETY: caller guarantees `cells` points to `count` valid pointers.
        let slice = unsafe { std::slice::from_raw_parts(cells, count as usize) };
        let mut owned: Vec<&str> = Vec::with_capacity(slice.len());
        for &ptr in slice {
            if ptr.is_null() {
                return;
            }
            // SAFETY: each element is a valid NUL-terminated string.
            match unsafe { CStr::from_ptr(ptr) }.to_str() {
                Ok(s) => owned.push(s),
                Err(_) => return,
            }
        }
        t.add_row_strs(&owned);
    });
}

/// Add a row of `RichRenderable` cells. `cells` is an array of `count`
/// `RichRenderable*`. CONSUMES every cell pointer (do NOT free them). A NULL
/// `cells` or `count == 0` is a no-op; any NULL element is skipped.
///
/// All non-NULL cells are consumed regardless of whether `table` is valid; if
/// `table` is NULL or already finished, the cells are dropped.
#[unsafe(no_mangle)]
pub extern "C" fn rich_table_add_row_renderables(
    table: *mut RichTable,
    cells: *const *mut RichRenderable,
    count: c_uint,
) {
    if cells.is_null() || count == 0 {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees `cells` points to `count` valid pointers.
        let slice = unsafe { std::slice::from_raw_parts(cells, count as usize) };
        let mut owned: Vec<Box<dyn rich_rs::Renderable + Send + Sync>> =
            Vec::with_capacity(slice.len());
        for &ptr in slice {
            if ptr.is_null() {
                continue;
            }
            // SAFETY: each element came from `Box::into_raw` in a `*_finish` fn.
            let boxed = unsafe { Box::from_raw(ptr) };
            owned.push(boxed.0);
        }
        with_table(table, |t| t.add_row_renderables(owned));
    }));
}
