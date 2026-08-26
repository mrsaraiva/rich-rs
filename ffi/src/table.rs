use rich_rs::{Column, Style, Table, Text};

use crate::common::{
    box_from_id, count, destroy, ffi_call, finish, init_out, parse_style, required_mut,
    required_ref, rich_str, shared_box, strict_bool,
};
use crate::{
    RICH_BOX_NONE, RICH_COLUMN_HAS_JUSTIFY, RICH_COLUMN_HAS_MIN_WIDTH, RICH_COLUMN_HAS_RATIO,
    RICH_COLUMN_HAS_VERTICAL, RICH_COLUMN_HAS_WIDTH, RICH_COLUMN_NO_WRAP,
    RICH_ERROR_INVALID_ARGUMENT, RICH_ERROR_INVALID_STATE, RICH_ERROR_NULL_ARGUMENT, RichConsole,
    RichRenderable, RichResult, RichStr,
};

const RICH_COLUMN_FLAGS: u32 = RICH_COLUMN_HAS_JUSTIFY
    | RICH_COLUMN_HAS_VERTICAL
    | RICH_COLUMN_HAS_WIDTH
    | RICH_COLUMN_HAS_MIN_WIDTH
    | RICH_COLUMN_HAS_RATIO
    | RICH_COLUMN_NO_WRAP;

#[repr(C)]
#[derive(Clone, Copy)]
/// Atomic configured-column input. Zero initialization selects every Column
/// default and `{NULL,0}` style means the default style.
pub struct RichTableColumnOptions {
    pub flags: u32,
    pub justify: u32,
    pub vertical: u32,
    pub width: u32,
    pub min_width: u32,
    pub ratio: u32,
    pub style: RichStr,
}

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
/// Create an exact `Table::grid()` builder through `out_table`.
pub extern "C" fn rich_table_new_grid(out_table: *mut *mut RichTable) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_table)? };
        unsafe { *out_table = Box::into_raw(Box::new(RichTable(Some(Table::grid())))) };
        crate::trace::event("table_grid", None);
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
        if box_id == RICH_BOX_NONE {
            table_mut(table)?.set_box(None);
            crate::trace::event("table_none", None);
            return Ok(());
        }
        let drawing = box_from_id(box_id)?;
        table_mut(table)?.set_box(Some(drawing));
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Enable or disable padding on the outermost table cells.
pub extern "C" fn rich_table_set_pad_edge(table: *mut RichTable, pad: u8) -> RichResult {
    ffi_call(|| {
        let pad = strict_bool(pad)?;
        table_mut(table)?.set_pad_edge(pad);
        crate::trace::event("table_pad_edge", None);
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Atomically replace alternating row styles. `{NULL,0}` clears the list;
/// every input string is copied and parsed before the table is mutated.
pub extern "C" fn rich_table_set_row_styles(
    table: *mut RichTable,
    styles: *const RichStr,
    style_count: u32,
) -> RichResult {
    ffi_call(|| {
        let count = count(style_count)?;
        if count == 0 {
            table_mut(table)?.set_row_styles(Vec::new());
            crate::trace::event("table_row_styles", None);
            return Ok(());
        }
        if styles.is_null() {
            return Err(RICH_ERROR_NULL_ARGUMENT);
        }
        let styles = unsafe { std::slice::from_raw_parts(styles, count) };
        let parsed = styles
            .iter()
            .copied()
            .map(|style| {
                let style = unsafe { rich_str(style)? };
                if style.is_empty() {
                    Ok(Style::default())
                } else {
                    parse_style(style)
                }
            })
            .collect::<Result<Vec<Style>, RichResult>>()?;
        table_mut(table)?.set_row_styles(parsed);
        crate::trace::event("table_row_styles", None);
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
/// Validate and add one fully configured column atomically. `header` may be
/// NULL; a non-NULL renderable is borrowed and retained through shared ownership.
pub extern "C" fn rich_table_add_column_options(
    table: *mut RichTable,
    header: *const RichRenderable,
    options: *const RichTableColumnOptions,
) -> RichResult {
    ffi_call(|| {
        let options = *unsafe { required_ref(options)? };
        if options.flags & !RICH_COLUMN_FLAGS != 0 {
            return Err(RICH_ERROR_INVALID_ARGUMENT);
        }
        if options.flags & RICH_COLUMN_HAS_JUSTIFY == 0 && options.justify != 0
            || options.flags & RICH_COLUMN_HAS_VERTICAL == 0 && options.vertical != 0
            || options.flags & RICH_COLUMN_HAS_WIDTH == 0 && options.width != 0
            || options.flags & RICH_COLUMN_HAS_MIN_WIDTH == 0 && options.min_width != 0
            || options.flags & RICH_COLUMN_HAS_RATIO == 0 && options.ratio != 0
        {
            return Err(RICH_ERROR_INVALID_ARGUMENT);
        }

        let justify = if options.flags & RICH_COLUMN_HAS_JUSTIFY != 0 {
            Some(crate::common::justify_from_id(options.justify)?)
        } else {
            None
        };
        let vertical = if options.flags & RICH_COLUMN_HAS_VERTICAL != 0 {
            Some(crate::common::vertical_from_id(options.vertical)?)
        } else {
            None
        };
        let width = if options.flags & RICH_COLUMN_HAS_WIDTH != 0 {
            if options.width == 0 {
                return Err(RICH_ERROR_INVALID_ARGUMENT);
            }
            Some(usize::try_from(options.width).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?)
        } else {
            None
        };
        let min_width = if options.flags & RICH_COLUMN_HAS_MIN_WIDTH != 0 {
            if options.min_width == 0 {
                return Err(RICH_ERROR_INVALID_ARGUMENT);
            }
            Some(usize::try_from(options.min_width).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?)
        } else {
            None
        };
        let ratio = if options.flags & RICH_COLUMN_HAS_RATIO != 0 {
            if options.ratio == 0 {
                return Err(RICH_ERROR_INVALID_ARGUMENT);
            }
            Some(usize::try_from(options.ratio).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?)
        } else {
            None
        };
        let style = if options.style.data.is_null() && options.style.len == 0 {
            Style::default()
        } else {
            parse_style(unsafe { rich_str(options.style)? })?
        };

        let mut column = if header.is_null() {
            Column::new()
        } else {
            Column::with_header(shared_box(unsafe { required_ref(header)? }))
        };
        column = column.style(style);
        if let Some(justify) = justify {
            column = column.justify(justify);
        }
        if let Some(vertical) = vertical {
            column = column.vertical(vertical);
        }
        if let Some(width) = width {
            column = column.width(width);
        }
        if let Some(min_width) = min_width {
            column = column.min_width(min_width);
        }
        if let Some(ratio) = ratio {
            column = column.ratio(ratio);
        }
        if options.flags & RICH_COLUMN_NO_WRAP != 0 {
            column = column.no_wrap(true);
        }

        table_mut(table)?.add_column(column);
        crate::trace::event("table_column_options", None);
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

    fn s(value: &str) -> RichStr {
        RichStr {
            data: value.as_ptr(),
            len: value.len() as u64,
        }
    }

    fn default_options() -> RichTableColumnOptions {
        RichTableColumnOptions {
            flags: 0,
            justify: 0,
            vertical: 0,
            width: 0,
            min_width: 0,
            ratio: 0,
            style: RichStr {
                data: ptr::null(),
                len: 0,
            },
        }
    }

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

    #[test]
    fn grid_none_and_configured_column_validation_are_atomic() {
        let mut table = ptr::null_mut();
        assert_eq!(rich_table_new_grid(&mut table), crate::RICH_OK);
        assert_eq!(rich_table_set_box(table, RICH_BOX_NONE), crate::RICH_OK);
        assert_eq!(
            rich_table_set_box(table, RICH_BOX_NONE + 1),
            RICH_ERROR_INVALID_ARGUMENT
        );
        assert_eq!(
            rich_table_set_pad_edge(table, 2),
            RICH_ERROR_INVALID_ARGUMENT
        );

        let mut options = default_options();
        assert_eq!(
            rich_table_add_column_options(table, ptr::null(), &options),
            crate::RICH_OK
        );
        assert_eq!(unsafe { (*table).0.as_ref().unwrap().column_count() }, 1);

        options.flags = 1 << 31;
        assert_eq!(
            rich_table_add_column_options(table, ptr::null(), &options),
            RICH_ERROR_INVALID_ARGUMENT
        );
        assert_eq!(unsafe { (*table).0.as_ref().unwrap().column_count() }, 1);

        for (flag, field) in [
            (RICH_COLUMN_HAS_WIDTH, 0_u8),
            (RICH_COLUMN_HAS_MIN_WIDTH, 1_u8),
            (RICH_COLUMN_HAS_RATIO, 2_u8),
        ] {
            options = default_options();
            options.flags = flag;
            match field {
                0 => options.width = 0,
                1 => options.min_width = 0,
                _ => options.ratio = 0,
            }
            assert_eq!(
                rich_table_add_column_options(table, ptr::null(), &options),
                RICH_ERROR_INVALID_ARGUMENT
            );
        }

        options = default_options();
        options.justify = 1;
        assert_eq!(
            rich_table_add_column_options(table, ptr::null(), &options),
            RICH_ERROR_INVALID_ARGUMENT
        );
        options.flags = RICH_COLUMN_HAS_JUSTIFY;
        options.justify = 99;
        assert_eq!(
            rich_table_add_column_options(table, ptr::null(), &options),
            RICH_ERROR_INVALID_ARGUMENT
        );
        options = default_options();
        options.flags = RICH_COLUMN_HAS_VERTICAL;
        options.vertical = 99;
        assert_eq!(
            rich_table_add_column_options(table, ptr::null(), &options),
            RICH_ERROR_INVALID_ARGUMENT
        );
        assert_eq!(unsafe { (*table).0.as_ref().unwrap().column_count() }, 1);
        rich_table_free(table);
    }

    #[test]
    fn configured_column_rejects_bad_text_and_retains_borrowed_header() {
        let mut table = ptr::null_mut();
        assert_eq!(rich_table_new(&mut table), crate::RICH_OK);
        assert_eq!(rich_table_set_box(table, RICH_BOX_NONE), crate::RICH_OK);
        let mut options = default_options();
        let invalid = [0xff_u8];
        options.style = RichStr {
            data: invalid.as_ptr(),
            len: 1,
        };
        assert_eq!(
            rich_table_add_column_options(table, ptr::null(), &options),
            crate::RICH_ERROR_INVALID_UTF8
        );
        options.style = s("definitely-not-a-style");
        assert_eq!(
            rich_table_add_column_options(table, ptr::null(), &options),
            RICH_ERROR_INVALID_ARGUMENT
        );
        assert_eq!(unsafe { (*table).0.as_ref().unwrap().column_count() }, 0);

        let mut text = ptr::null_mut();
        assert_eq!(crate::rich_text_new(s("Header"), &mut text), crate::RICH_OK);
        let mut header = ptr::null_mut();
        assert_eq!(
            crate::rich_text_finish(&mut text, &mut header),
            crate::RICH_OK
        );
        options = default_options();
        options.flags = RICH_COLUMN_HAS_WIDTH
            | RICH_COLUMN_HAS_MIN_WIDTH
            | RICH_COLUMN_HAS_RATIO
            | RICH_COLUMN_HAS_JUSTIFY
            | RICH_COLUMN_HAS_VERTICAL
            | RICH_COLUMN_NO_WRAP;
        options.width = 12;
        options.min_width = 4;
        options.ratio = 1;
        options.justify = crate::RICH_JUSTIFY_CENTER;
        options.vertical = crate::RICH_VERTICAL_MIDDLE;
        options.style = s("bold red");
        assert_eq!(
            rich_table_add_column_options(table, header, &options),
            crate::RICH_OK
        );
        crate::rich_renderable_free(header);

        let cells = [s("value")];
        assert_eq!(
            rich_table_add_row_strs(table, cells.as_ptr(), 1),
            crate::RICH_OK
        );
        let mut renderable = ptr::null_mut();
        assert_eq!(
            rich_table_finish(&mut table, &mut renderable),
            crate::RICH_OK
        );
        let mut console = ptr::null_mut();
        assert_eq!(crate::rich_console_new(&mut console), crate::RICH_OK);
        assert_eq!(
            crate::rich_console_set_force_terminal(console, 0),
            crate::RICH_OK
        );
        let mut output = crate::RichBuffer::default();
        assert_eq!(
            crate::rich_console_render(console, renderable, &mut output),
            crate::RICH_OK
        );
        let rendered = unsafe { std::slice::from_raw_parts(output.data, output.len as usize) };
        assert!(rendered.windows(6).any(|part| part == b"Header"));
        assert!(rendered.windows(5).any(|part| part == b"value"));
        crate::rich_buffer_free(&mut output);
        crate::rich_console_free(console);
        crate::rich_renderable_free(renderable);
    }

    #[test]
    fn row_style_replacement_is_atomic_and_empty_style_is_default() {
        let mut table = ptr::null_mut();
        assert_eq!(rich_table_new(&mut table), crate::RICH_OK);
        let initial = [s("red")];
        assert_eq!(
            rich_table_set_row_styles(table, initial.as_ptr(), 1),
            crate::RICH_OK
        );
        let invalid = [s("blue"), s("not-a-style")];
        assert_eq!(
            rich_table_set_row_styles(table, invalid.as_ptr(), 2),
            RICH_ERROR_INVALID_ARGUMENT
        );
        let defaults = [RichStr {
            data: ptr::null(),
            len: 0,
        }];
        assert_eq!(
            rich_table_set_row_styles(table, defaults.as_ptr(), 1),
            crate::RICH_OK
        );
        assert_eq!(
            rich_table_set_row_styles(table, ptr::null(), 1),
            RICH_ERROR_NULL_ARGUMENT
        );
        assert_eq!(
            rich_table_set_row_styles(table, ptr::null(), 0),
            crate::RICH_OK
        );
        rich_table_free(table);
    }
}
