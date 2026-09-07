//! Stable C ABI foundation for rich-rs language bindings.
//!
//! All handles are opaque and must be freed with their matching destructor.
//! Non-destructor operations return a [`RichResult`]. Raw non-NULL handles must
//! still be live values created by this library; validation cannot make forged,
//! stale, or concurrently misused pointers safe.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::Arc;

use rich_rs::{ColorSystem, Console, Renderable, Style, Text};

mod common;
mod content;
mod custom;
mod layout;
mod live;
mod panel;
mod table;
mod trace;
mod tree;

use common::{
    destroy, ffi_call, finish, guard_ordinary_call, init_buffer, init_out, init_scalar_out,
    maybe_test_boundary_panic, parse_style, render_to_bytes, required_mut, required_ref, rich_str,
    strict_bool, write_buffer,
};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Stable result category returned by every fallible ABI operation.
pub struct RichResult(pub u32);

pub const RICH_OK: RichResult = RichResult(0);
pub const RICH_ERROR_NULL_ARGUMENT: RichResult = RichResult(1);
pub const RICH_ERROR_INVALID_UTF8: RichResult = RichResult(2);
pub const RICH_ERROR_INVALID_ARGUMENT: RichResult = RichResult(3);
pub const RICH_ERROR_INVALID_STATE: RichResult = RichResult(4);
pub const RICH_ERROR_IO: RichResult = RichResult(5);
pub const RICH_ERROR_RENDER: RichResult = RichResult(6);
pub const RICH_ERROR_PANIC: RichResult = RichResult(7);
pub const RICH_FFI_ABI_VERSION: u32 = 2;

pub const RICH_COLOR_NONE: u32 = 0;
pub const RICH_COLOR_STANDARD: u32 = 1;
pub const RICH_COLOR_EIGHT_BIT: u32 = 2;
pub const RICH_COLOR_TRUECOLOR: u32 = 3;
pub const RICH_COLOR_WINDOWS: u32 = 4;

pub const RICH_ALIGN_LEFT: u32 = 0;
pub const RICH_ALIGN_CENTER: u32 = 1;
pub const RICH_ALIGN_RIGHT: u32 = 2;
pub const RICH_VERTICAL_TOP: u32 = 0;
pub const RICH_VERTICAL_MIDDLE: u32 = 1;
pub const RICH_VERTICAL_BOTTOM: u32 = 2;
pub const RICH_JUSTIFY_DEFAULT: u32 = 0;
pub const RICH_JUSTIFY_LEFT: u32 = 1;
pub const RICH_JUSTIFY_CENTER: u32 = 2;
pub const RICH_JUSTIFY_RIGHT: u32 = 3;
pub const RICH_JUSTIFY_FULL: u32 = 4;
pub const RICH_BOX_ROUNDED: u32 = 0;
pub const RICH_BOX_HEAVY: u32 = 1;
pub const RICH_BOX_DOUBLE: u32 = 2;
pub const RICH_BOX_ASCII: u32 = 3;
pub const RICH_BOX_MINIMAL: u32 = 4;
pub const RICH_BOX_SQUARE: u32 = 5;
pub const RICH_BOX_SIMPLE: u32 = 6;
pub const RICH_BOX_HEAVY_HEAD: u32 = 7;
pub const RICH_BOX_HEAVY_EDGE: u32 = 8;
pub const RICH_BOX_DOUBLE_EDGE: u32 = 9;
pub const RICH_BOX_HORIZONTALS: u32 = 10;
pub const RICH_BOX_MINIMAL_HEAVY_HEAD: u32 = 11;
pub const RICH_BOX_SIMPLE_HEAVY: u32 = 12;
pub const RICH_BOX_MARKDOWN: u32 = 13;
pub const RICH_BOX_NONE: u32 = 14;
pub const RICH_COLUMN_HAS_JUSTIFY: u32 = 1 << 0;
pub const RICH_COLUMN_HAS_VERTICAL: u32 = 1 << 1;
pub const RICH_COLUMN_HAS_WIDTH: u32 = 1 << 2;
pub const RICH_COLUMN_HAS_MIN_WIDTH: u32 = 1 << 3;
pub const RICH_COLUMN_HAS_RATIO: u32 = 1 << 4;
pub const RICH_COLUMN_NO_WRAP: u32 = 1 << 5;
pub const RICH_TREE_ROOT_ID: u64 = 0;

#[repr(C)]
#[derive(Clone, Copy)]
/// Borrowed UTF-8 bytes. The library never stores `data` beyond the call.
pub struct RichStr {
    pub data: *const u8,
    pub len: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// Rust-allocated output bytes released and cleared by `rich_buffer_free`.
pub struct RichBuffer {
    pub data: *mut u8,
    pub len: u64,
}

impl Default for RichBuffer {
    fn default() -> Self {
        Self {
            data: ptr::null_mut(),
            len: 0,
        }
    }
}

/// Owning capture-console handle. Setters return a result and rendering writes
/// owned UTF-8 to `RichBuffer` without appending a newline.
pub struct RichConsole {
    pub(crate) inner: Console<Vec<u8>>,
    desired_color: Option<ColorSystem>,
}

/// Owning handle to an Arc-backed renderable. Container inputs borrow this
/// handle and retain their own shared reference; free releases one reference.
pub struct RichRenderable(pub(crate) Arc<dyn Renderable + Send + Sync>);
/// Owning text builder consumed only by successful `rich_text_finish`.
pub struct RichText(pub(crate) Option<Text>);
/// Owning parsed-style handle used to validate style syntax.
pub struct RichStyle {
    _value: Style,
}

pub use custom::{
    RichCustomDropFn, RichCustomMeasureFn, RichCustomRenderFn, RichCustomRenderableVTable,
    RichMeasurement, RichRenderOptions, RichSegmentSink,
};

#[unsafe(no_mangle)]
/// Return the native ABI version. This infallible value must equal
/// `RICH_FFI_ABI_VERSION` before ordinary calls are made.
pub extern "C" fn rich_ffi_abi_version() -> u32 {
    if custom::callback_active() {
        0
    } else {
        RICH_FFI_ABI_VERSION
    }
}

#[unsafe(no_mangle)]
/// Write the synchronized Cargo package version as an owned UTF-8 buffer.
pub extern "C" fn rich_ffi_version(out_version: *mut RichBuffer) -> RichResult {
    ffi_call(|| {
        unsafe { init_buffer(out_version)? };
        unsafe { write_buffer(out_version, env!("CARGO_PKG_VERSION").as_bytes().to_vec()) }
    })
}

#[unsafe(no_mangle)]
/// Free a produced buffer and reset it to `{NULL, 0}`. NULL is accepted.
pub extern "C" fn rich_buffer_free(buffer: *mut RichBuffer) {
    if custom::callback_active() {
        return;
    }
    if buffer.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        let value = &mut *buffer;
        if !value.data.is_null() {
            if let Ok(len) = usize::try_from(value.len) {
                let slice = ptr::slice_from_raw_parts_mut(value.data, len);
                drop(Box::<[u8]>::from_raw(slice));
            }
        }
        *value = RichBuffer::default();
    }));
}

#[unsafe(no_mangle)]
/// Create an owning console handle through `out_console`.
pub extern "C" fn rich_console_new(out_console: *mut *mut RichConsole) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_console)? };
        let mut inner = Console::capture();
        let desired_color = Some(ColorSystem::TrueColor);
        inner.set_force_terminal(Some(true));
        inner.set_color_system(desired_color);
        inner.set_markup_enabled(true);
        inner.set_emoji_enabled(true);
        unsafe {
            *out_console = Box::into_raw(Box::new(RichConsole {
                inner,
                desired_color,
            }));
        }
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Destroy a console handle. NULL is a no-op.
pub extern "C" fn rich_console_free(console: *mut RichConsole) {
    unsafe { destroy(console) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_console_set_force_terminal(
    console: *mut RichConsole,
    force: u8,
) -> RichResult {
    ffi_call(|| {
        let force = strict_bool(force)?;
        let console = unsafe { required_mut(console)? };
        console.inner.set_force_terminal(Some(force));
        console
            .inner
            .set_color_system(if force { console.desired_color } else { None });
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_console_set_size(
    console: *mut RichConsole,
    width: u32,
    height: u32,
) -> RichResult {
    ffi_call(|| {
        let width = usize::try_from(width).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?;
        let height = usize::try_from(height).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?;
        unsafe { required_mut(console)? }
            .inner
            .set_size(width, height);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_console_set_color_system(
    console: *mut RichConsole,
    system: u32,
) -> RichResult {
    ffi_call(|| {
        guard_ordinary_call()?;
        let color = match system {
            RICH_COLOR_NONE => None,
            RICH_COLOR_STANDARD => Some(ColorSystem::Standard),
            RICH_COLOR_EIGHT_BIT => Some(ColorSystem::EightBit),
            RICH_COLOR_TRUECOLOR => Some(ColorSystem::TrueColor),
            RICH_COLOR_WINDOWS => Some(ColorSystem::Windows),
            _ => return Err(RICH_ERROR_INVALID_ARGUMENT),
        };
        let console = unsafe { required_mut(console)? };
        console.desired_color = color;
        if console.inner.is_terminal() {
            console.inner.set_color_system(color);
        }
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_console_set_markup_enabled(
    console: *mut RichConsole,
    enabled: u8,
) -> RichResult {
    ffi_call(|| {
        let enabled = strict_bool(enabled)?;
        unsafe { required_mut(console)? }
            .inner
            .set_markup_enabled(enabled);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_console_set_emoji_enabled(
    console: *mut RichConsole,
    enabled: u8,
) -> RichResult {
    ffi_call(|| {
        let enabled = strict_bool(enabled)?;
        unsafe { required_mut(console)? }
            .inner
            .set_emoji_enabled(enabled);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_console_width(
    console: *const RichConsole,
    out_width: *mut u32,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_scalar_out(out_width)? };
        maybe_test_boundary_panic();
        let width = unsafe { required_ref(console)? }.inner.width();
        unsafe { *out_width = u32::try_from(width).map_err(|_| RICH_ERROR_RENDER)? };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Render length-delimited markup to an owned output buffer. Rendering errors
/// are `RICH_ERROR_RENDER`; a caught panic is `RICH_ERROR_PANIC`.
pub extern "C" fn rich_console_render_markup(
    console: *mut RichConsole,
    markup: RichStr,
    out_rendered: *mut RichBuffer,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_buffer(out_rendered)? };
        maybe_test_boundary_panic();
        let markup = unsafe { rich_str(markup)? };
        let console = unsafe { required_mut(console)? };
        let text = console.inner.render_str(markup, None, None, None, None);
        let bytes = render_to_bytes(console, &text)?;
        unsafe { write_buffer(out_rendered, bytes) }
    })
}

#[unsafe(no_mangle)]
/// Borrow and render a reusable renderable to an owned output buffer.
pub extern "C" fn rich_console_render(
    console: *mut RichConsole,
    renderable: *const RichRenderable,
    out_rendered: *mut RichBuffer,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_buffer(out_rendered)? };
        let invocation = custom::InvocationGuard::enter()?;
        let renderable = unsafe { required_ref(renderable)? };
        let console = unsafe { required_mut(console)? };
        let bytes = render_to_bytes(console, renderable.0.as_ref())?;
        if let Some(error) = invocation.error() {
            return Err(error);
        }
        unsafe { write_buffer(out_rendered, bytes) }
    })
}

#[unsafe(no_mangle)]
/// Release one renderable handle/reference. NULL is a no-op.
pub extern "C" fn rich_renderable_free(renderable: *mut RichRenderable) {
    unsafe { destroy(renderable) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_text_new(text: RichStr, out_text: *mut *mut RichText) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_text)? };
        let text = unsafe { rich_str(text)? };
        unsafe { *out_text = Box::into_raw(Box::new(RichText(Some(Text::plain(text))))) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_text_new_markup(
    console: *mut RichConsole,
    markup: RichStr,
    out_text: *mut *mut RichText,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_text)? };
        let markup = unsafe { rich_str(markup)? };
        let console = unsafe { required_mut(console)? };
        let text = console.inner.render_str(markup, None, None, None, None);
        unsafe { *out_text = Box::into_raw(Box::new(RichText(Some(text)))) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_text_set_style(text: *mut RichText, style: RichStr) -> RichResult {
    ffi_call(|| {
        let style = parse_style(unsafe { rich_str(style)? })?;
        let text = unsafe { required_mut(text)? };
        let value = text.0.as_mut().ok_or(RICH_ERROR_INVALID_STATE)?;
        value.set_base_style(Some(style));
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Consume a live text builder on success and NULL `*text`; failures before
/// ownership is taken leave the builder live.
pub extern "C" fn rich_text_finish(
    text: *mut *mut RichText,
    out_renderable: *mut *mut RichRenderable,
) -> RichResult {
    unsafe { finish(text, out_renderable, |v| v.0.is_some(), |v| v.0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_text_free(text: *mut RichText) {
    unsafe { destroy(text) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_style_parse(style: RichStr, out_style: *mut *mut RichStyle) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_style)? };
        let style = parse_style(unsafe { rich_str(style)? })?;
        unsafe { *out_style = Box::into_raw(Box::new(RichStyle { _value: style })) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_style_free(style: *mut RichStyle) {
    unsafe { destroy(style) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(value: &str) -> RichStr {
        RichStr {
            data: value.as_ptr(),
            len: value.len() as u64,
        }
    }

    fn take_buffer(mut buffer: RichBuffer) -> String {
        let bytes = if buffer.len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(buffer.data, buffer.len as usize) }.to_vec()
        };
        rich_buffer_free(&mut buffer);
        String::from_utf8(bytes).expect("FFI output must be UTF-8")
    }

    fn render(console: *mut RichConsole, renderable: *const RichRenderable) -> String {
        let mut buffer = RichBuffer::default();
        assert_eq!(
            rich_console_render(console, renderable, &mut buffer),
            RICH_OK
        );
        take_buffer(buffer)
    }

    #[test]
    fn result_values_and_metadata_are_stable() {
        assert_eq!(RICH_OK.0, 0);
        assert_eq!(RICH_ERROR_NULL_ARGUMENT.0, 1);
        assert_eq!(RICH_ERROR_INVALID_UTF8.0, 2);
        assert_eq!(RICH_ERROR_INVALID_ARGUMENT.0, 3);
        assert_eq!(RICH_ERROR_INVALID_STATE.0, 4);
        assert_eq!(RICH_ERROR_IO.0, 5);
        assert_eq!(RICH_ERROR_RENDER.0, 6);
        assert_eq!(RICH_ERROR_PANIC.0, 7);
        assert_eq!(rich_ffi_abi_version(), 2);
        let root_manifest = include_str!("../../Cargo.toml");
        let root_version = root_manifest
            .lines()
            .find_map(|line| {
                line.strip_prefix("version = \"")
                    .and_then(|v| v.strip_suffix('"'))
            })
            .unwrap();
        assert_eq!(env!("CARGO_PKG_VERSION"), root_version);
    }

    #[test]
    fn buffer_round_trip_and_double_free() {
        let mut buffer = RichBuffer::default();
        assert_eq!(rich_ffi_version(&mut buffer), RICH_OK);
        assert_eq!(
            unsafe { std::slice::from_raw_parts(buffer.data, buffer.len as usize) },
            env!("CARGO_PKG_VERSION").as_bytes()
        );
        rich_buffer_free(&mut buffer);
        assert!(buffer.data.is_null());
        rich_buffer_free(&mut buffer);
    }

    #[test]
    fn successful_finish_nulls_builder_and_renderable_is_reusable() {
        let mut text = ptr::null_mut();
        assert_eq!(rich_text_new(s("hello"), &mut text), RICH_OK);
        let mut renderable = ptr::null_mut();
        assert_eq!(rich_text_finish(&mut text, &mut renderable), RICH_OK);
        assert!(text.is_null());
        let mut console = ptr::null_mut();
        assert_eq!(rich_console_new(&mut console), RICH_OK);
        let mut first = RichBuffer::default();
        let mut second = RichBuffer::default();
        assert_eq!(
            rich_console_render(console, renderable, &mut first),
            RICH_OK
        );
        assert_eq!(
            rich_console_render(console, renderable, &mut second),
            RICH_OK
        );
        rich_buffer_free(&mut first);
        rich_buffer_free(&mut second);
        rich_renderable_free(renderable);
        rich_console_free(console);
    }

    #[test]
    fn containers_borrow_and_retain_renderables() {
        let mut text = ptr::null_mut();
        assert_eq!(rich_text_new(s("shared"), &mut text), RICH_OK);
        let mut renderable = ptr::null_mut();
        assert_eq!(rich_text_finish(&mut text, &mut renderable), RICH_OK);

        let mut panel = ptr::null_mut();
        assert_eq!(
            crate::panel::rich_panel_new(renderable, &mut panel),
            RICH_OK
        );
        let mut columns = ptr::null_mut();
        assert_eq!(crate::layout::rich_columns_new(&mut columns), RICH_OK);
        assert_eq!(
            crate::layout::rich_columns_add(columns, renderable),
            RICH_OK
        );
        assert_eq!(
            crate::layout::rich_columns_add(columns, renderable),
            RICH_OK
        );

        let mut console = ptr::null_mut();
        assert_eq!(rich_console_new(&mut console), RICH_OK);
        assert_eq!(rich_console_set_force_terminal(console, 0), RICH_OK);
        let mut output = RichBuffer::default();
        assert_eq!(
            rich_console_render(console, renderable, &mut output),
            RICH_OK
        );
        assert_eq!(
            unsafe { std::slice::from_raw_parts(output.data, output.len as usize) },
            b"shared"
        );
        rich_buffer_free(&mut output);

        crate::panel::rich_panel_free(panel);
        crate::layout::rich_columns_free(columns);
        rich_renderable_free(renderable);
        rich_console_free(console);
    }

    #[test]
    fn every_family_emits_utf8_and_shared_composition_remains_reusable() {
        let mut console = ptr::null_mut();
        assert_eq!(rich_console_new(&mut console), RICH_OK);
        assert_eq!(rich_console_set_force_terminal(console, 0), RICH_OK);
        assert_eq!(rich_console_set_size(console, 80, 24), RICH_OK);

        let mut text = ptr::null_mut();
        assert_eq!(rich_text_new(s("héllo ✓"), &mut text), RICH_OK);
        let mut shared = ptr::null_mut();
        assert_eq!(rich_text_finish(&mut text, &mut shared), RICH_OK);
        assert_eq!(render(console, shared), "héllo ✓");

        let mut renderables = Vec::new();

        let mut table = ptr::null_mut();
        assert_eq!(crate::table::rich_table_new(&mut table), RICH_OK);
        assert_eq!(
            crate::table::rich_table_add_column(table, s("col")),
            RICH_OK
        );
        let cells = [shared as *const RichRenderable];
        assert_eq!(
            crate::table::rich_table_add_row_renderables(table, cells.as_ptr(), 1),
            RICH_OK
        );
        let mut value = ptr::null_mut();
        assert_eq!(
            crate::table::rich_table_finish(&mut table, &mut value),
            RICH_OK
        );
        renderables.push(value);

        let mut panel = ptr::null_mut();
        assert_eq!(crate::panel::rich_panel_new(shared, &mut panel), RICH_OK);
        value = ptr::null_mut();
        assert_eq!(
            crate::panel::rich_panel_finish(&mut panel, &mut value),
            RICH_OK
        );
        renderables.push(value);

        let mut tree = ptr::null_mut();
        assert_eq!(
            crate::tree::rich_tree_new_renderable(shared, &mut tree),
            RICH_OK
        );
        let mut node_id = 0;
        assert_eq!(
            crate::tree::rich_tree_add_renderable(tree, RICH_TREE_ROOT_ID, shared, &mut node_id),
            RICH_OK
        );
        value = ptr::null_mut();
        assert_eq!(
            crate::tree::rich_tree_finish(&mut tree, &mut value),
            RICH_OK
        );
        renderables.push(value);

        let mut rule = ptr::null_mut();
        assert_eq!(crate::layout::rich_rule_new(&mut rule), RICH_OK);
        assert_eq!(
            crate::layout::rich_rule_set_title(rule, s("séction")),
            RICH_OK
        );
        value = ptr::null_mut();
        assert_eq!(
            crate::layout::rich_rule_finish(&mut rule, &mut value),
            RICH_OK
        );
        renderables.push(value);

        let mut columns = ptr::null_mut();
        assert_eq!(crate::layout::rich_columns_new(&mut columns), RICH_OK);
        assert_eq!(crate::layout::rich_columns_add(columns, shared), RICH_OK);
        assert_eq!(crate::layout::rich_columns_add(columns, shared), RICH_OK);
        value = ptr::null_mut();
        assert_eq!(
            crate::layout::rich_columns_finish(&mut columns, &mut value),
            RICH_OK
        );
        renderables.push(value);

        let mut align = ptr::null_mut();
        assert_eq!(
            crate::layout::rich_align_new(shared, RICH_ALIGN_CENTER, &mut align),
            RICH_OK
        );
        value = ptr::null_mut();
        assert_eq!(
            crate::layout::rich_align_finish(&mut align, &mut value),
            RICH_OK
        );
        renderables.push(value);

        let mut padding = ptr::null_mut();
        assert_eq!(
            crate::layout::rich_padding_new(shared, 0, 1, 0, 1, &mut padding),
            RICH_OK
        );
        value = ptr::null_mut();
        assert_eq!(
            crate::layout::rich_padding_finish(&mut padding, &mut value),
            RICH_OK
        );
        renderables.push(value);

        let mut syntax = ptr::null_mut();
        assert_eq!(
            crate::content::rich_syntax_new(s("let café = \"✓\";"), s("rust"), &mut syntax),
            RICH_OK
        );
        value = ptr::null_mut();
        assert_eq!(
            crate::content::rich_syntax_finish(&mut syntax, &mut value),
            RICH_OK
        );
        renderables.push(value);

        let mut markdown = ptr::null_mut();
        assert_eq!(
            crate::content::rich_markdown_new(s("# Héllo ✓"), &mut markdown),
            RICH_OK
        );
        value = ptr::null_mut();
        assert_eq!(
            crate::content::rich_markdown_finish(&mut markdown, &mut value),
            RICH_OK
        );
        renderables.push(value);

        let mut json = ptr::null_mut();
        assert_eq!(
            crate::content::rich_json_new(s("{\"é\":\"✓\"}"), 2, 0, 0, &mut json),
            RICH_OK
        );
        value = ptr::null_mut();
        assert_eq!(
            crate::content::rich_json_finish(&mut json, &mut value),
            RICH_OK
        );
        renderables.push(value);

        for &renderable in &renderables {
            assert!(!render(console, renderable).is_empty());
        }
        assert_eq!(render(console, shared), "héllo ✓");

        let mut progress = ptr::null_mut();
        assert_eq!(crate::live::rich_progress_new(&mut progress), RICH_OK);
        let mut task_id = u64::MAX;
        assert_eq!(
            crate::live::rich_progress_add_task(progress, s("tâche ✓"), 10.0, &mut task_id),
            RICH_OK
        );
        let mut buffer = RichBuffer::default();
        assert_eq!(
            crate::live::rich_progress_render_frame(progress, console, &mut buffer),
            RICH_OK
        );
        assert!(take_buffer(buffer).contains("tâche ✓"));

        let mut status = ptr::null_mut();
        assert_eq!(
            crate::live::rich_status_new(s("état ✓"), &mut status),
            RICH_OK
        );
        buffer = RichBuffer::default();
        assert_eq!(
            crate::live::rich_status_render_frame(status, console, &mut buffer),
            RICH_OK
        );
        assert!(take_buffer(buffer).contains("état ✓"));

        let mut spinner = ptr::null_mut();
        assert_eq!(
            crate::live::rich_spinner_new(s("dots"), &mut spinner),
            RICH_OK
        );
        buffer = RichBuffer::default();
        assert_eq!(
            crate::live::rich_spinner_render_frame(spinner, console, 0.5, &mut buffer),
            RICH_OK
        );
        assert!(!take_buffer(buffer).is_empty());

        crate::live::rich_progress_free(progress);
        crate::live::rich_status_free(status);
        crate::live::rich_spinner_free(spinner);
        for renderable in renderables {
            rich_renderable_free(renderable);
        }
        rich_renderable_free(shared);
        rich_console_free(console);
    }

    #[test]
    fn failed_finish_keeps_builder_live() {
        let mut text = ptr::null_mut();
        assert_eq!(rich_text_new(s("hello"), &mut text), RICH_OK);
        assert_eq!(
            rich_text_finish(&mut text, ptr::null_mut()),
            RICH_ERROR_NULL_ARGUMENT
        );
        assert!(!text.is_null());
        rich_text_free(text);
    }

    #[test]
    fn errors_initialize_scalar_and_buffer_outputs() {
        let mut width = 99;
        assert_eq!(
            rich_console_width(ptr::null(), &mut width),
            RICH_ERROR_NULL_ARGUMENT
        );
        assert_eq!(width, 0);
        let mut buffer = RichBuffer {
            data: ptr::dangling_mut(),
            len: 99,
        };
        assert_eq!(
            rich_console_render_markup(ptr::null_mut(), s("x"), &mut buffer),
            RICH_ERROR_NULL_ARGUMENT
        );
        assert!(buffer.data.is_null());
        assert_eq!(buffer.len, 0);
    }

    #[test]
    fn exported_calls_initialize_outputs_before_forced_panic() {
        let mut console = ptr::null_mut();
        assert_eq!(rich_console_new(&mut console), RICH_OK);

        let mut width = 99;
        crate::common::force_next_boundary_panic();
        assert_eq!(rich_console_width(console, &mut width), RICH_ERROR_PANIC);
        assert_eq!(width, 0);

        let mut buffer = RichBuffer {
            data: ptr::dangling_mut(),
            len: 99,
        };
        crate::common::force_next_boundary_panic();
        assert_eq!(
            rich_console_render_markup(console, s("hello"), &mut buffer),
            RICH_ERROR_PANIC
        );
        assert!(buffer.data.is_null());
        assert_eq!(buffer.len, 0);

        rich_console_free(console);
    }

    #[test]
    fn all_destructor_families_accept_null() {
        rich_buffer_free(ptr::null_mut());
        rich_console_free(ptr::null_mut());
        rich_renderable_free(ptr::null_mut());
        rich_text_free(ptr::null_mut());
        rich_style_free(ptr::null_mut());
        crate::table::rich_table_free(ptr::null_mut());
        crate::panel::rich_panel_free(ptr::null_mut());
        crate::tree::rich_tree_free(ptr::null_mut());
        crate::layout::rich_rule_free(ptr::null_mut());
        crate::layout::rich_columns_free(ptr::null_mut());
        crate::layout::rich_align_free(ptr::null_mut());
        crate::layout::rich_padding_free(ptr::null_mut());
        crate::content::rich_syntax_free(ptr::null_mut());
        crate::content::rich_markdown_free(ptr::null_mut());
        crate::content::rich_json_free(ptr::null_mut());
        crate::live::rich_progress_free(ptr::null_mut());
        crate::live::rich_status_free(ptr::null_mut());
        crate::live::rich_spinner_free(ptr::null_mut());
    }

    #[test]
    fn terminal_mode_controls_ansi() {
        let mut console = ptr::null_mut();
        assert_eq!(rich_console_new(&mut console), RICH_OK);
        let mut out = RichBuffer::default();
        assert_eq!(rich_console_set_force_terminal(console, 0), RICH_OK);
        assert_eq!(
            rich_console_render_markup(console, s("[bold]x[/]"), &mut out),
            RICH_OK
        );
        let plain = unsafe { std::slice::from_raw_parts(out.data, out.len as usize) };
        assert!(!plain.windows(2).any(|w| w == b"\x1b["));
        assert!(!plain.windows(2).any(|w| w == b"\x1b]"));
        rich_buffer_free(&mut out);
        assert_eq!(rich_console_set_force_terminal(console, 1), RICH_OK);
        assert_eq!(
            rich_console_render_markup(console, s("[bold]x[/]"), &mut out),
            RICH_OK
        );
        let ansi = unsafe { std::slice::from_raw_parts(out.data, out.len as usize) };
        assert!(ansi.windows(2).any(|w| w == b"\x1b["));
        rich_buffer_free(&mut out);
        rich_console_free(console);
    }
}
