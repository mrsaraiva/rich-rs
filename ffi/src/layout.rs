use rich_rs::{Align, Columns, Padding, Rule, cell_len};

use crate::common::{
    align_from_id, destroy, ffi_call, finish, init_out, parse_style, required_mut, required_ref,
    rich_str, shared_box, strict_bool, vertical_from_id,
};
use crate::{
    RICH_ALIGN_CENTER, RICH_ERROR_INVALID_ARGUMENT, RICH_ERROR_INVALID_STATE, RichRenderable,
    RichResult, RichStr,
};

macro_rules! map_builder {
    ($handle:expr, $field:tt, $error:expr, $transform:expr) => {{
        let handle = unsafe { required_mut($handle)? };
        let value = handle.$field.take().ok_or($error)?;
        handle.$field = Some(($transform)(value));
        Ok(())
    }};
}

/// Owning horizontal-rule builder. Setters reject invalid styles/enums;
/// successful finish consumes and NULLs the builder.
pub struct RichRule(Option<Rule>);

#[unsafe(no_mangle)]
/// Create a rule builder through `out_rule`.
pub extern "C" fn rich_rule_new(out_rule: *mut *mut RichRule) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_rule)? };
        unsafe { *out_rule = Box::into_raw(Box::new(RichRule(Some(Rule::new())))) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_rule_set_title(rule: *mut RichRule, title: RichStr) -> RichResult {
    ffi_call(|| {
        let title = unsafe { rich_str(title)? }.to_owned();
        map_builder!(rule, 0, RICH_ERROR_INVALID_STATE, |rule: Rule| rule
            .with_title(title))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_rule_set_characters(rule: *mut RichRule, characters: RichStr) -> RichResult {
    ffi_call(|| {
        let characters = unsafe { rich_str(characters)? }.to_owned();
        if cell_len(&characters) == 0 {
            return Err(RICH_ERROR_INVALID_ARGUMENT);
        }
        map_builder!(rule, 0, RICH_ERROR_INVALID_STATE, |rule: Rule| rule
            .with_characters(characters))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_rule_set_style(rule: *mut RichRule, style: RichStr) -> RichResult {
    ffi_call(|| {
        let style = parse_style(unsafe { rich_str(style)? })?;
        map_builder!(rule, 0, RICH_ERROR_INVALID_STATE, |rule: Rule| rule
            .with_style(style))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_rule_set_align(rule: *mut RichRule, align: u32) -> RichResult {
    ffi_call(|| {
        let align = align_from_id(align)?;
        map_builder!(rule, 0, RICH_ERROR_INVALID_STATE, |rule: Rule| rule
            .with_align(align))
    })
}

#[unsafe(no_mangle)]
/// Consume a live rule on success and write an owned renderable.
pub extern "C" fn rich_rule_finish(
    rule: *mut *mut RichRule,
    out: *mut *mut RichRenderable,
) -> RichResult {
    unsafe { finish(rule, out, |v| v.0.is_some(), |v| v.0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_rule_free(rule: *mut RichRule) {
    unsafe { destroy(rule) }
}

/// Owning columns builder. Added renderables are borrowed and retained, so the
/// caller's renderable handles remain reusable after composition.
pub struct RichColumns(Option<Columns>);

#[unsafe(no_mangle)]
/// Create an empty columns builder through `out_columns`.
pub extern "C" fn rich_columns_new(out_columns: *mut *mut RichColumns) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_columns)? };
        unsafe { *out_columns = Box::into_raw(Box::new(RichColumns(Some(Columns::empty())))) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_columns_add(
    columns: *mut RichColumns,
    renderable: *const RichRenderable,
) -> RichResult {
    ffi_call(|| {
        let renderable = unsafe { required_ref(renderable)? };
        let columns = unsafe { required_mut(columns)? }
            .0
            .as_mut()
            .ok_or(RICH_ERROR_INVALID_STATE)?;
        columns.add(shared_box(renderable));
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_columns_add_str(columns: *mut RichColumns, text: RichStr) -> RichResult {
    ffi_call(|| {
        let text = unsafe { rich_str(text)? };
        let columns = unsafe { required_mut(columns)? }
            .0
            .as_mut()
            .ok_or(RICH_ERROR_INVALID_STATE)?;
        columns.add_str(text);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_columns_set_equal(columns: *mut RichColumns, equal: u8) -> RichResult {
    ffi_call(|| {
        let equal = strict_bool(equal)?;
        map_builder!(columns, 0, RICH_ERROR_INVALID_STATE, |columns: Columns| {
            columns.with_equal(equal)
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_columns_set_expand(columns: *mut RichColumns, expand: u8) -> RichResult {
    ffi_call(|| {
        let expand = strict_bool(expand)?;
        map_builder!(columns, 0, RICH_ERROR_INVALID_STATE, |columns: Columns| {
            columns.with_expand(expand)
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_columns_set_padding(
    columns: *mut RichColumns,
    vertical: u32,
    horizontal: u32,
) -> RichResult {
    ffi_call(|| {
        let vertical = usize::try_from(vertical).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?;
        let horizontal = usize::try_from(horizontal).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?;
        map_builder!(columns, 0, RICH_ERROR_INVALID_STATE, |columns: Columns| {
            columns.with_padding((vertical, horizontal))
        })
    })
}

#[unsafe(no_mangle)]
/// Consume live columns on success and write an owned renderable.
pub extern "C" fn rich_columns_finish(
    columns: *mut *mut RichColumns,
    out: *mut *mut RichRenderable,
) -> RichResult {
    unsafe { finish(columns, out, |v| v.0.is_some(), |v| v.0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_columns_free(columns: *mut RichColumns) {
    unsafe { destroy(columns) }
}

/// Owning alignment builder around borrowed, internally retained content.
pub struct RichAlign(Option<Align>);

#[unsafe(no_mangle)]
/// Create an alignment builder; invalid alignment codes are argument errors.
pub extern "C" fn rich_align_new(
    content: *const RichRenderable,
    horizontal: u32,
    out_align: *mut *mut RichAlign,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_align)? };
        let horizontal = align_from_id(horizontal)?;
        let content = unsafe { required_ref(content)? };
        unsafe {
            *out_align = Box::into_raw(Box::new(RichAlign(Some(Align::new(
                shared_box(content),
                horizontal,
            )))))
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_align_center(
    content: *const RichRenderable,
    out_align: *mut *mut RichAlign,
) -> RichResult {
    rich_align_new(content, RICH_ALIGN_CENTER, out_align)
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_align_set_vertical(align: *mut RichAlign, vertical: u32) -> RichResult {
    ffi_call(|| {
        let vertical = vertical_from_id(vertical)?;
        map_builder!(align, 0, RICH_ERROR_INVALID_STATE, |align: Align| align
            .with_vertical(vertical))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_align_set_width(align: *mut RichAlign, width: u32) -> RichResult {
    ffi_call(|| {
        let width = usize::try_from(width).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?;
        map_builder!(align, 0, RICH_ERROR_INVALID_STATE, |align: Align| align
            .with_width(width))
    })
}

#[unsafe(no_mangle)]
/// Consume a live alignment builder on success and write an owned renderable.
pub extern "C" fn rich_align_finish(
    align: *mut *mut RichAlign,
    out: *mut *mut RichRenderable,
) -> RichResult {
    unsafe { finish(align, out, |v| v.0.is_some(), |v| v.0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_align_free(align: *mut RichAlign) {
    unsafe { destroy(align) }
}

/// Owning padding builder around borrowed, internally retained content.
pub struct RichPadding(Option<Padding>);

#[unsafe(no_mangle)]
/// Create a padding builder with fixed-width cell dimensions.
pub extern "C" fn rich_padding_new(
    content: *const RichRenderable,
    top: u32,
    right: u32,
    bottom: u32,
    left: u32,
    out_padding: *mut *mut RichPadding,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_padding)? };
        let pad = (
            usize::try_from(top).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?,
            usize::try_from(right).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?,
            usize::try_from(bottom).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?,
            usize::try_from(left).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?,
        );
        let content = unsafe { required_ref(content)? };
        unsafe {
            *out_padding = Box::into_raw(Box::new(RichPadding(Some(Padding::new(
                shared_box(content),
                pad,
            )))))
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_padding_set_style(padding: *mut RichPadding, style: RichStr) -> RichResult {
    ffi_call(|| {
        let style = parse_style(unsafe { rich_str(style)? })?;
        map_builder!(padding, 0, RICH_ERROR_INVALID_STATE, |padding: Padding| {
            padding.with_style(style)
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_padding_set_expand(padding: *mut RichPadding, expand: u8) -> RichResult {
    ffi_call(|| {
        let expand = strict_bool(expand)?;
        map_builder!(padding, 0, RICH_ERROR_INVALID_STATE, |padding: Padding| {
            padding.with_expand(expand)
        })
    })
}

#[unsafe(no_mangle)]
/// Consume a live padding builder on success and write an owned renderable.
pub extern "C" fn rich_padding_finish(
    padding: *mut *mut RichPadding,
    out: *mut *mut RichRenderable,
) -> RichResult {
    unsafe { finish(padding, out, |v| v.0.is_some(), |v| v.0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_padding_free(padding: *mut RichPadding) {
    unsafe { destroy(padding) }
}
