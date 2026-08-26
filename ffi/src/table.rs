use rich_rs::{Table, Text};

use crate::common::{
    box_from_id, count, destroy, ffi_call, finish, init_out, parse_style, required_mut,
    required_ref, rich_str, shared_box, strict_bool,
};
use crate::{
    RICH_ERROR_INVALID_ARGUMENT, RICH_ERROR_INVALID_STATE, RichConsole, RichRenderable, RichResult,
    RichStr,
};

/// Owning table builder. String/renderable rows are validated completely before
/// mutation; renderable cells are borrowed and retained through shared ownership.
/// Successful `rich_table_finish` consumes and NULLs the builder.
pub struct RichTable(pub(crate) Option<Table>);

fn table_mut(table: *mut RichTable) -> Result<&'static mut Table, RichResult> {
    unsafe { required_mut(table)? }
        .0
        .as_mut()
        .ok_or(RICH_ERROR_INVALID_STATE)
}

fn map_table(table: *mut RichTable, f: impl FnOnce(Table) -> Table) -> Result<(), RichResult> {
    let table = unsafe { required_mut(table)? };
    let value = table.0.take().ok_or(RICH_ERROR_INVALID_STATE)?;
    table.0 = Some(f(value));
    Ok(())
}

#[unsafe(no_mangle)]
/// Create an empty table builder through `out_table`.
pub extern "C" fn rich_table_new(out_table: *mut *mut RichTable) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_table)? };
        unsafe { *out_table = Box::into_raw(Box::new(RichTable(Some(Table::new())))) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Consume a live table on success, NULLing `*table`, and return an owned renderable.
pub extern "C" fn rich_table_finish(
    table: *mut *mut RichTable,
    out_renderable: *mut *mut RichRenderable,
) -> RichResult {
    unsafe { finish(table, out_renderable, |v| v.0.is_some(), |v| v.0) }
}

#[unsafe(no_mangle)]
/// Destroy an unfinished table. NULL is accepted.
pub extern "C" fn rich_table_free(table: *mut RichTable) {
    unsafe { destroy(table) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_title(table: *mut RichTable, title: RichStr) -> RichResult {
    ffi_call(|| {
        let title = unsafe { rich_str(title)? };
        table_mut(table)?.set_title(Some(Text::plain(title)));
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_caption(table: *mut RichTable, caption: RichStr) -> RichResult {
    ffi_call(|| {
        let caption = unsafe { rich_str(caption)? };
        table_mut(table)?.set_caption(Some(Text::plain(caption)));
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_title_markup(
    table: *mut RichTable,
    console: *mut RichConsole,
    markup: RichStr,
) -> RichResult {
    ffi_call(|| {
        let markup = unsafe { rich_str(markup)? };
        let console = unsafe { required_mut(console)? };
        let value = console.inner.render_str(markup, None, None, None, None);
        table_mut(table)?.set_title(Some(value));
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_caption_markup(
    table: *mut RichTable,
    console: *mut RichConsole,
    markup: RichStr,
) -> RichResult {
    ffi_call(|| {
        let markup = unsafe { rich_str(markup)? };
        let console = unsafe { required_mut(console)? };
        let value = console.inner.render_str(markup, None, None, None, None);
        table_mut(table)?.set_caption(Some(value));
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_box(table: *mut RichTable, box_id: u32) -> RichResult {
    ffi_call(|| {
        let drawing = box_from_id(box_id)?;
        table_mut(table)?.set_box(Some(drawing));
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_show_header(table: *mut RichTable, show: u8) -> RichResult {
    ffi_call(|| {
        let show = strict_bool(show)?;
        map_table(table, |table| table.with_show_header(show))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_show_lines(table: *mut RichTable, show: u8) -> RichResult {
    ffi_call(|| {
        let show = strict_bool(show)?;
        map_table(table, |table| table.with_show_lines(show))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_show_edge(table: *mut RichTable, show: u8) -> RichResult {
    ffi_call(|| {
        let show = strict_bool(show)?;
        map_table(table, |table| table.with_show_edge(show))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_expand(table: *mut RichTable, expand: u8) -> RichResult {
    ffi_call(|| {
        let expand = strict_bool(expand)?;
        map_table(table, |table| table.with_expand(expand))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_padding(
    table: *mut RichTable,
    left: u32,
    right: u32,
) -> RichResult {
    ffi_call(|| {
        let left = usize::try_from(left).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?;
        let right = usize::try_from(right).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?;
        map_table(table, |table| table.with_padding(left, right))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_set_style(table: *mut RichTable, style: RichStr) -> RichResult {
    ffi_call(|| {
        let style = parse_style(unsafe { rich_str(style)? })?;
        map_table(table, |table| table.with_style(style))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_add_column(table: *mut RichTable, header: RichStr) -> RichResult {
    ffi_call(|| {
        let header = unsafe { rich_str(header)? };
        table_mut(table)?.add_column_str(header);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_add_column_renderable(
    table: *mut RichTable,
    header: *const RichRenderable,
) -> RichResult {
    ffi_call(|| {
        let header = unsafe { required_ref(header)? };
        table_mut(table)?.add_column_renderable(shared_box(header));
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_add_row_strs(
    table: *mut RichTable,
    cells: *const RichStr,
    cell_count: u32,
) -> RichResult {
    ffi_call(|| {
        let count = count(cell_count)?;
        if count == 0 || cells.is_null() {
            return Err(if cells.is_null() {
                crate::RICH_ERROR_NULL_ARGUMENT
            } else {
                RICH_ERROR_INVALID_ARGUMENT
            });
        }
        let cells = unsafe { std::slice::from_raw_parts(cells, count) };
        let values: Vec<&str> = cells
            .iter()
            .copied()
            .map(|cell| unsafe { rich_str(cell) })
            .collect::<Result<_, _>>()?;
        table_mut(table)?.add_row_strs(&values);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_table_add_row_renderables(
    table: *mut RichTable,
    cells: *const *const RichRenderable,
    cell_count: u32,
) -> RichResult {
    ffi_call(|| {
        let count = count(cell_count)?;
        if count == 0 || cells.is_null() {
            return Err(if cells.is_null() {
                crate::RICH_ERROR_NULL_ARGUMENT
            } else {
                RICH_ERROR_INVALID_ARGUMENT
            });
        }
        let cells = unsafe { std::slice::from_raw_parts(cells, count) };
        let values: Vec<Box<dyn rich_rs::Renderable + Send + Sync>> = cells
            .iter()
            .copied()
            .map(|cell| unsafe { required_ref(cell) }.map(shared_box))
            .collect::<Result<_, _>>()?;
        table_mut(table)?.add_row_renderables(values);
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn renderable_array_validation_is_atomic() {
        let mut table = ptr::null_mut();
        assert_eq!(rich_table_new(&mut table), crate::RICH_OK);
        let cells = [ptr::null()];
        assert_eq!(
            rich_table_add_row_renderables(table, cells.as_ptr(), 1),
            crate::RICH_ERROR_NULL_ARGUMENT
        );
        unsafe { assert_eq!((*table).0.as_ref().unwrap().row_count(), 0) };
        rich_table_free(table);
    }

    #[test]
    fn string_array_validation_is_atomic() {
        let mut table = ptr::null_mut();
        assert_eq!(rich_table_new(&mut table), crate::RICH_OK);
        let valid = b"valid";
        let invalid = [0xff_u8];
        let cells = [
            RichStr {
                data: valid.as_ptr(),
                len: valid.len() as u64,
            },
            RichStr {
                data: invalid.as_ptr(),
                len: invalid.len() as u64,
            },
        ];
        assert_eq!(
            rich_table_add_row_strs(table, cells.as_ptr(), cells.len() as u32),
            crate::RICH_ERROR_INVALID_UTF8
        );
        unsafe { assert_eq!((*table).0.as_ref().unwrap().row_count(), 0) };
        rich_table_free(table);
    }
}
