use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::Arc;

use rich_rs::r#box::{self, Box as RichBox};
use rich_rs::{
    AlignMethod, ConsoleOptions, JustifyMethod, Measurement, Renderable, Segments, Style,
    VerticalAlignMethod,
};

use crate::{
    RICH_ERROR_INVALID_ARGUMENT, RICH_ERROR_INVALID_STATE, RICH_ERROR_INVALID_UTF8,
    RICH_ERROR_NULL_ARGUMENT, RICH_ERROR_PANIC, RICH_ERROR_RENDER, RICH_OK, RichBuffer,
    RichRenderable, RichResult, RichStr,
};

pub(crate) fn ffi_call(f: impl FnOnce() -> Result<(), RichResult>) -> RichResult {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => RICH_OK,
        Ok(Err(error)) => error,
        Err(_) => RICH_ERROR_PANIC,
    }
}

pub(crate) unsafe fn init_out<T>(out: *mut *mut T) -> Result<(), RichResult> {
    if out.is_null() {
        return Err(RICH_ERROR_NULL_ARGUMENT);
    }
    unsafe { *out = ptr::null_mut() };
    Ok(())
}

pub(crate) unsafe fn init_scalar_out<T: Default>(out: *mut T) -> Result<(), RichResult> {
    if out.is_null() {
        return Err(RICH_ERROR_NULL_ARGUMENT);
    }
    unsafe { *out = T::default() };
    Ok(())
}

pub(crate) unsafe fn required_ref<'a, T>(ptr: *const T) -> Result<&'a T, RichResult> {
    if ptr.is_null() {
        Err(RICH_ERROR_NULL_ARGUMENT)
    } else {
        Ok(unsafe { &*ptr })
    }
}

pub(crate) unsafe fn required_mut<'a, T>(ptr: *mut T) -> Result<&'a mut T, RichResult> {
    if ptr.is_null() {
        Err(RICH_ERROR_NULL_ARGUMENT)
    } else {
        Ok(unsafe { &mut *ptr })
    }
}

pub(crate) unsafe fn rich_str<'a>(value: RichStr) -> Result<&'a str, RichResult> {
    if value.data.is_null() {
        return if value.len == 0 {
            Ok("")
        } else {
            Err(RICH_ERROR_NULL_ARGUMENT)
        };
    }
    if value.len > isize::MAX as u64 {
        return Err(RICH_ERROR_INVALID_ARGUMENT);
    }
    let len = usize::try_from(value.len).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?;
    let bytes = unsafe { std::slice::from_raw_parts(value.data, len) };
    std::str::from_utf8(bytes).map_err(|_| RICH_ERROR_INVALID_UTF8)
}

pub(crate) fn strict_bool(value: u8) -> Result<bool, RichResult> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(RICH_ERROR_INVALID_ARGUMENT),
    }
}

#[cfg(test)]
thread_local! {
    static FORCE_BOUNDARY_PANIC: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn force_next_boundary_panic() {
    FORCE_BOUNDARY_PANIC.with(|value| value.set(true));
}

pub(crate) fn maybe_test_boundary_panic() {
    #[cfg(test)]
    FORCE_BOUNDARY_PANIC.with(|value| {
        if value.replace(false) {
            panic!("forced FFI boundary panic");
        }
    });
}

pub(crate) fn count(value: u32) -> Result<usize, RichResult> {
    usize::try_from(value).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)
}

pub(crate) unsafe fn init_buffer(out: *mut RichBuffer) -> Result<(), RichResult> {
    if out.is_null() {
        return Err(RICH_ERROR_NULL_ARGUMENT);
    }
    unsafe { *out = RichBuffer::default() };
    Ok(())
}

pub(crate) unsafe fn write_buffer(out: *mut RichBuffer, bytes: Vec<u8>) -> Result<(), RichResult> {
    if bytes.is_empty() {
        return Ok(());
    }
    let len = u64::try_from(bytes.len()).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?;
    let boxed = bytes.into_boxed_slice();
    let data = Box::into_raw(boxed) as *mut u8;
    unsafe { *out = RichBuffer { data, len } };
    Ok(())
}

pub(crate) fn parse_style(value: &str) -> Result<Style, RichResult> {
    let parsed = Style::parse(value).ok_or(RICH_ERROR_INVALID_ARGUMENT)?;
    if parsed.is_null() {
        Err(RICH_ERROR_INVALID_ARGUMENT)
    } else {
        Ok(parsed)
    }
}

pub(crate) fn render_to_bytes(
    console: &mut crate::RichConsole,
    renderable: &(dyn Renderable + Send + Sync),
) -> Result<Vec<u8>, RichResult> {
    console.inner.clear_captured();
    console
        .inner
        .print(renderable, None, None, None, false, "")
        .map_err(|_| RICH_ERROR_RENDER)?;
    Ok(console.inner.get_captured().into_bytes())
}

pub(crate) struct SharedRenderable(pub(crate) Arc<dyn Renderable + Send + Sync>);

impl Renderable for SharedRenderable {
    fn render(&self, console: &rich_rs::Console, options: &ConsoleOptions) -> Segments {
        self.0.render(console, options)
    }

    fn measure(&self, console: &rich_rs::Console, options: &ConsoleOptions) -> Measurement {
        self.0.measure(console, options)
    }
}

pub(crate) fn shared_box(renderable: &RichRenderable) -> Box<dyn Renderable + Send + Sync> {
    Box::new(SharedRenderable(Arc::clone(&renderable.0)))
}

pub(crate) fn new_renderable(value: impl Renderable + 'static) -> *mut RichRenderable {
    Box::into_raw(Box::new(RichRenderable(Arc::new(value))))
}

pub(crate) unsafe fn finish<B, T: Renderable + 'static>(
    builder: *mut *mut B,
    out: *mut *mut RichRenderable,
    is_live: impl FnOnce(&B) -> bool,
    take: impl FnOnce(B) -> Option<T>,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out)? };
        if builder.is_null() {
            return Err(RICH_ERROR_NULL_ARGUMENT);
        }
        let raw = unsafe { *builder };
        if raw.is_null() || !is_live(unsafe { &*raw }) {
            return Err(RICH_ERROR_INVALID_STATE);
        }
        let boxed = unsafe { Box::from_raw(raw) };
        unsafe { *builder = ptr::null_mut() };
        let value = take(*boxed).ok_or(RICH_ERROR_INVALID_STATE)?;
        unsafe { *out = new_renderable(value) };
        Ok(())
    })
}

pub(crate) unsafe fn destroy<T>(ptr: *mut T) {
    if ptr.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        drop(unsafe { Box::from_raw(ptr) });
    }));
}

pub(crate) fn box_from_id(id: u32) -> Result<RichBox, RichResult> {
    match id {
        0 => Ok(r#box::ROUNDED),
        1 => Ok(r#box::HEAVY),
        2 => Ok(r#box::DOUBLE),
        3 => Ok(r#box::ASCII),
        4 => Ok(r#box::MINIMAL),
        5 => Ok(r#box::SQUARE),
        6 => Ok(r#box::SIMPLE),
        7 => Ok(r#box::HEAVY_HEAD),
        8 => Ok(r#box::HEAVY_EDGE),
        9 => Ok(r#box::DOUBLE_EDGE),
        10 => Ok(r#box::HORIZONTALS),
        11 => Ok(r#box::MINIMAL_HEAVY_HEAD),
        12 => Ok(r#box::SIMPLE_HEAVY),
        13 => Ok(r#box::MARKDOWN),
        _ => Err(RICH_ERROR_INVALID_ARGUMENT),
    }
}

pub(crate) fn align_from_id(id: u32) -> Result<AlignMethod, RichResult> {
    match id {
        0 => Ok(AlignMethod::Left),
        1 => Ok(AlignMethod::Center),
        2 => Ok(AlignMethod::Right),
        _ => Err(RICH_ERROR_INVALID_ARGUMENT),
    }
}

pub(crate) fn vertical_from_id(id: u32) -> Result<VerticalAlignMethod, RichResult> {
    match id {
        0 => Ok(VerticalAlignMethod::Top),
        1 => Ok(VerticalAlignMethod::Middle),
        2 => Ok(VerticalAlignMethod::Bottom),
        _ => Err(RICH_ERROR_INVALID_ARGUMENT),
    }
}

pub(crate) fn justify_from_id(id: u32) -> Result<JustifyMethod, RichResult> {
    match id {
        0 => Ok(JustifyMethod::Default),
        1 => Ok(JustifyMethod::Left),
        2 => Ok(JustifyMethod::Center),
        3 => Ok(JustifyMethod::Right),
        4 => Ok(JustifyMethod::Full),
        _ => Err(RICH_ERROR_INVALID_ARGUMENT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_boolean_validation() {
        assert_eq!(strict_bool(0), Ok(false));
        assert_eq!(strict_bool(1), Ok(true));
        assert_eq!(strict_bool(2), Err(RICH_ERROR_INVALID_ARGUMENT));
    }

    #[test]
    fn borrowed_string_validation() {
        unsafe {
            assert_eq!(
                rich_str(RichStr {
                    data: ptr::null(),
                    len: 0
                }),
                Ok("")
            );
            assert_eq!(
                rich_str(RichStr {
                    data: ptr::null(),
                    len: 1
                }),
                Err(RICH_ERROR_NULL_ARGUMENT)
            );
            let valid = b"a\0b";
            assert_eq!(
                rich_str(RichStr {
                    data: valid.as_ptr(),
                    len: 3
                }),
                Ok("a\0b")
            );
            let invalid = [0xff];
            assert_eq!(
                rich_str(RichStr {
                    data: invalid.as_ptr(),
                    len: 1
                }),
                Err(RICH_ERROR_INVALID_UTF8)
            );
        }
    }

    #[test]
    fn outputs_are_initialized_on_error_and_panic() {
        let mut value = 99_u32;
        let error = ffi_call(|| {
            unsafe { init_scalar_out(&mut value)? };
            Err(RICH_ERROR_INVALID_ARGUMENT)
        });
        assert_eq!(error, RICH_ERROR_INVALID_ARGUMENT);
        assert_eq!(value, 0);

        value = 99;
        let panic = ffi_call(|| {
            unsafe { init_scalar_out(&mut value)? };
            panic!("boundary test");
        });
        assert_eq!(panic, RICH_ERROR_PANIC);
        assert_eq!(value, 0);
    }

    #[test]
    fn empty_buffer_stays_canonical() {
        let mut buffer = RichBuffer {
            data: ptr::dangling_mut(),
            len: 99,
        };
        unsafe {
            init_buffer(&mut buffer).unwrap();
            write_buffer(&mut buffer, Vec::new()).unwrap();
        }
        assert!(buffer.data.is_null());
        assert_eq!(buffer.len, 0);
    }

    #[test]
    fn oversized_string_length_is_rejected() {
        let byte = b'x';
        let value = RichStr {
            data: &byte,
            len: isize::MAX as u64 + 1,
        };
        assert_eq!(unsafe { rich_str(value) }, Err(RICH_ERROR_INVALID_ARGUMENT));
    }
}
