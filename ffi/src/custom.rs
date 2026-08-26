use std::cell::RefCell;
use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rich_rs::{ColorSystem, ConsoleOptions, Measurement, Renderable, Segment, Segments};

use crate::common::{init_out, parse_style, required_ref, rich_str};
use crate::{
    RICH_COLOR_EIGHT_BIT, RICH_COLOR_NONE, RICH_COLOR_STANDARD, RICH_COLOR_TRUECOLOR,
    RICH_COLOR_WINDOWS, RICH_ERROR_INVALID_STATE, RICH_ERROR_PANIC, RICH_ERROR_RENDER, RICH_OK,
    RichRenderable, RichResult, RichStr,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Stable rendering constraints borrowed by a custom-renderable callback.
pub struct RichRenderOptions {
    pub min_width: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub color_system: u32,
    pub is_terminal: u8,
    pub reserved: [u8; 7],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Width bounds written by a custom measurement callback.
pub struct RichMeasurement {
    pub minimum: u32,
    pub maximum: u32,
}

pub type RichCustomRenderFn = unsafe extern "C" fn(
    context: *mut c_void,
    options: *const RichRenderOptions,
    sink: *mut RichSegmentSink,
) -> RichResult;
pub type RichCustomMeasureFn = unsafe extern "C" fn(
    context: *mut c_void,
    options: *const RichRenderOptions,
    out_measurement: *mut RichMeasurement,
) -> RichResult;
pub type RichCustomDropFn = unsafe extern "C" fn(context: *mut c_void);

#[repr(C)]
#[derive(Clone, Copy)]
/// Nullable synchronous callback pointers copied by `rich_custom_renderable_new`.
/// Construction requires render; measure/drop follow the documented NULL rules.
pub struct RichCustomRenderableVTable {
    pub render: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            options: *const RichRenderOptions,
            sink: *mut RichSegmentSink,
        ) -> RichResult,
    >,
    pub measure: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            options: *const RichRenderOptions,
            out_measurement: *mut RichMeasurement,
        ) -> RichResult,
    >,
    pub drop: Option<unsafe extern "C" fn(context: *mut c_void)>,
}

/// Ephemeral, thread-bound segment staging area. Its pointer is valid only
/// during the render callback that receives it.
pub struct RichSegmentSink {
    segments: Segments,
    first_error: Option<RichResult>,
}

#[derive(Default)]
struct InvocationState {
    first_error: Option<RichResult>,
    callback_active: bool,
    adapter_id: usize,
    active_sink: *mut RichSegmentSink,
    sink_call_active: bool,
}

thread_local! {
    static INVOCATION: RefCell<Option<InvocationState>> = const { RefCell::new(None) };
}

pub(crate) struct InvocationGuard;

impl InvocationGuard {
    pub(crate) fn enter() -> Result<Self, RichResult> {
        INVOCATION.with(|slot| {
            let mut slot = slot.borrow_mut();
            if slot.is_some() {
                return Err(RICH_ERROR_INVALID_STATE);
            }
            *slot = Some(InvocationState::default());
            Ok(Self)
        })
    }

    pub(crate) fn error(&self) -> Option<RichResult> {
        INVOCATION.with(|slot| slot.borrow().as_ref().and_then(|state| state.first_error))
    }
}

impl Drop for InvocationGuard {
    fn drop(&mut self) {
        INVOCATION.with(|slot| *slot.borrow_mut() = None);
    }
}

pub(crate) fn callback_active() -> bool {
    INVOCATION.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|state| state.callback_active)
    })
}

pub(crate) fn ordinary_call_forbidden() -> bool {
    INVOCATION.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|state| state.callback_active && !state.sink_call_active)
    })
}

fn normalize_callback_result(result: RichResult) -> RichResult {
    if result.0 <= RICH_ERROR_PANIC.0 {
        result
    } else {
        RICH_ERROR_RENDER
    }
}

fn record_error(error: RichResult) {
    let error = normalize_callback_result(error);
    INVOCATION.with(|slot| {
        if let Some(state) = slot.borrow_mut().as_mut()
            && state.first_error.is_none()
        {
            state.first_error = Some(error);
        }
    });
}

fn has_error() -> bool {
    INVOCATION.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_none_or(|state| state.first_error.is_some())
    })
}

struct CallbackGuard;

impl CallbackGuard {
    fn enter(adapter_id: usize, active_sink: *mut RichSegmentSink) -> Result<Self, RichResult> {
        INVOCATION.with(|slot| {
            let mut slot = slot.borrow_mut();
            let state = slot.as_mut().ok_or(RICH_ERROR_INVALID_STATE)?;
            if state.callback_active || state.first_error.is_some() {
                return Err(RICH_ERROR_INVALID_STATE);
            }
            state.callback_active = true;
            state.adapter_id = adapter_id;
            state.active_sink = active_sink;
            Ok(Self)
        })
    }
}

impl Drop for CallbackGuard {
    fn drop(&mut self) {
        INVOCATION.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut() {
                state.active_sink = ptr::null_mut();
                state.adapter_id = 0;
                state.callback_active = false;
            }
        });
    }
}

struct DropCallbackGuard {
    installed_scope: bool,
}

impl DropCallbackGuard {
    fn enter(adapter_id: usize) -> Option<Self> {
        INVOCATION.with(|slot| {
            let mut slot = slot.borrow_mut();
            match slot.as_mut() {
                Some(state) if state.callback_active => None,
                Some(state) => {
                    state.callback_active = true;
                    state.adapter_id = adapter_id;
                    Some(Self {
                        installed_scope: false,
                    })
                }
                None => {
                    *slot = Some(InvocationState {
                        callback_active: true,
                        adapter_id,
                        ..InvocationState::default()
                    });
                    Some(Self {
                        installed_scope: true,
                    })
                }
            }
        })
    }
}

impl Drop for DropCallbackGuard {
    fn drop(&mut self) {
        INVOCATION.with(|slot| {
            if self.installed_scope {
                *slot.borrow_mut() = None;
            } else if let Some(state) = slot.borrow_mut().as_mut() {
                state.adapter_id = 0;
                state.callback_active = false;
            }
        });
    }
}

struct SinkCallGuard;

impl SinkCallGuard {
    fn enter(sink: *mut RichSegmentSink) -> Result<Self, RichResult> {
        if sink.is_null() {
            return Err(RICH_ERROR_INVALID_STATE);
        }
        INVOCATION.with(|slot| {
            let mut slot = slot.borrow_mut();
            let state = slot.as_mut().ok_or(RICH_ERROR_INVALID_STATE)?;
            if !state.callback_active || state.active_sink != sink || state.sink_call_active {
                return Err(RICH_ERROR_INVALID_STATE);
            }
            state.sink_call_active = true;
            Ok(Self)
        })
    }
}

impl Drop for SinkCallGuard {
    fn drop(&mut self) {
        INVOCATION.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut() {
                state.sink_call_active = false;
            }
        });
    }
}

fn poison_sink(sink: *mut RichSegmentSink, error: RichResult) -> RichResult {
    let error = normalize_callback_result(error);
    let sink = unsafe { &mut *sink };
    if sink.first_error.is_none() {
        sink.first_error = Some(error);
    }
    sink.first_error.unwrap_or(error)
}

fn color_system_id(value: Option<ColorSystem>) -> u32 {
    match value {
        None => RICH_COLOR_NONE,
        Some(ColorSystem::Standard) => RICH_COLOR_STANDARD,
        Some(ColorSystem::EightBit) => RICH_COLOR_EIGHT_BIT,
        Some(ColorSystem::TrueColor) => RICH_COLOR_TRUECOLOR,
        Some(ColorSystem::Windows) => RICH_COLOR_WINDOWS,
    }
}

fn render_options(options: &ConsoleOptions) -> Result<RichRenderOptions, RichResult> {
    Ok(RichRenderOptions {
        min_width: u32::try_from(options.min_width).map_err(|_| RICH_ERROR_RENDER)?,
        max_width: u32::try_from(options.max_width).map_err(|_| RICH_ERROR_RENDER)?,
        max_height: u32::try_from(options.max_height).map_err(|_| RICH_ERROR_RENDER)?,
        color_system: color_system_id(options.color_system),
        is_terminal: u8::from(options.is_terminal),
        reserved: [0; 7],
    })
}

struct CustomRenderable {
    context: *mut c_void,
    vtable: RichCustomRenderableVTable,
    callback_lock: Mutex<()>,
    owns_context: AtomicBool,
}

// The foreign contract requires the context and callbacks to remain valid for
// the adapter's lifetime. Calls are serialized and are never dispatched by Rust.
unsafe impl Send for CustomRenderable {}
unsafe impl Sync for CustomRenderable {}

impl CustomRenderable {
    fn latch_options_error(&self, options: &ConsoleOptions) -> Option<RichRenderOptions> {
        if has_error() {
            return None;
        }
        match render_options(options) {
            Ok(value) => Some(value),
            Err(error) => {
                record_error(error);
                None
            }
        }
    }
}

impl Renderable for CustomRenderable {
    fn render(&self, _console: &rich_rs::Console, options: &ConsoleOptions) -> Segments {
        let Some(options) = self.latch_options_error(options) else {
            return Segments::new();
        };
        let mut sink = Box::new(RichSegmentSink {
            segments: Segments::new(),
            first_error: None,
        });
        let sink_ptr = &mut *sink as *mut RichSegmentSink;
        let adapter_id = self as *const Self as usize;
        let Ok(callback_guard) = CallbackGuard::enter(adapter_id, sink_ptr) else {
            record_error(RICH_ERROR_INVALID_STATE);
            return Segments::new();
        };
        let _lock = self
            .callback_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let callback = self
            .vtable
            .render
            .expect("validated custom render callback");
        crate::trace::event("custom_render", Some(options.max_width));
        let result = unsafe { callback(self.context, &options, sink_ptr) };
        drop(callback_guard);

        let error = sink
            .first_error
            .or_else(|| (result != RICH_OK).then(|| normalize_callback_result(result)));
        if let Some(error) = error {
            record_error(error);
            Segments::new()
        } else {
            sink.segments
        }
    }

    fn measure(&self, _console: &rich_rs::Console, options: &ConsoleOptions) -> Measurement {
        let Some(options) = self.latch_options_error(options) else {
            return Measurement::default();
        };
        if self.vtable.measure.is_none() {
            return Measurement::new(1, options.max_width as usize);
        }
        let adapter_id = self as *const Self as usize;
        let Ok(callback_guard) = CallbackGuard::enter(adapter_id, ptr::null_mut()) else {
            record_error(RICH_ERROR_INVALID_STATE);
            return Measurement::default();
        };
        let _lock = self
            .callback_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut measurement = RichMeasurement::default();
        let callback = self
            .vtable
            .measure
            .expect("checked custom measure callback");
        crate::trace::event("custom_measure", Some(options.max_width));
        let result = unsafe { callback(self.context, &options, &mut measurement) };
        drop(callback_guard);

        if result != RICH_OK {
            record_error(normalize_callback_result(result));
            return Measurement::default();
        }
        if measurement.minimum > measurement.maximum {
            record_error(RICH_ERROR_RENDER);
            return Measurement::default();
        }
        Measurement::new(measurement.minimum as usize, measurement.maximum as usize)
    }
}

impl Drop for CustomRenderable {
    fn drop(&mut self) {
        if !self.owns_context.load(Ordering::Acquire) {
            return;
        }
        let adapter_id = self as *const Self as usize;
        let Some(_guard) = DropCallbackGuard::enter(adapter_id) else {
            return;
        };
        let _lock = self
            .callback_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(callback) = self.vtable.drop {
            crate::trace::event("custom_drop", None);
            unsafe { callback(self.context) };
        }
    }
}

#[unsafe(no_mangle)]
/// Create an Arc-backed synchronous custom renderable. Render is required;
/// NULL measure selects `{1,max_width}`, and NULL drop requires NULL context.
/// Context ownership transfers only on success and is released by drop when
/// the last native Arc is gone.
pub extern "C" fn rich_custom_renderable_new(
    context: *mut c_void,
    vtable: *const RichCustomRenderableVTable,
    out_renderable: *mut *mut RichRenderable,
) -> RichResult {
    crate::common::ffi_call(|| {
        unsafe { init_out(out_renderable)? };
        let vtable = *unsafe { required_ref(vtable)? };
        if vtable.render.is_none() || (!context.is_null() && vtable.drop.is_none()) {
            return Err(crate::RICH_ERROR_INVALID_ARGUMENT);
        }

        let adapter = Arc::new(CustomRenderable {
            context,
            vtable,
            callback_lock: Mutex::new(()),
            owns_context: AtomicBool::new(false),
        });
        let renderable: Arc<dyn Renderable + Send + Sync> = adapter.clone();
        let handle = Box::new(RichRenderable(renderable));
        adapter.owns_context.store(true, Ordering::Release);
        unsafe { *out_renderable = Box::into_raw(handle) };
        crate::trace::event("custom_new", None);
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Stage copied UTF-8 text with a native-parsed style. `{NULL,0}` style is
/// unstyled. A failed push poisons the transactional sink with its first error.
pub extern "C" fn rich_segment_sink_push(
    sink: *mut RichSegmentSink,
    text: RichStr,
    style: RichStr,
) -> RichResult {
    let Ok(_sink_call) = SinkCallGuard::enter(sink) else {
        return RICH_ERROR_INVALID_STATE;
    };
    let result = catch_unwind(AssertUnwindSafe(|| {
        let sink = unsafe { &mut *sink };
        if let Some(error) = sink.first_error {
            return Err(error);
        }
        let text = unsafe { rich_str(text)? }.to_owned();
        let style = if style.data.is_null() && style.len == 0 {
            None
        } else {
            Some(parse_style(unsafe { rich_str(style)? })?)
        };
        sink.segments.push(match style {
            Some(style) => Segment::styled(text, style),
            None => Segment::new(text),
        });
        crate::trace::event("sink_push", None);
        Ok(())
    }));
    match result {
        Ok(Ok(())) => RICH_OK,
        Ok(Err(error)) => poison_sink(sink, error),
        Err(_) => poison_sink(sink, RICH_ERROR_PANIC),
    }
}

#[unsafe(no_mangle)]
/// Stage one native newline segment in the active transactional sink.
pub extern "C" fn rich_segment_sink_line(sink: *mut RichSegmentSink) -> RichResult {
    let Ok(_sink_call) = SinkCallGuard::enter(sink) else {
        return RICH_ERROR_INVALID_STATE;
    };
    let result = catch_unwind(AssertUnwindSafe(|| {
        let sink = unsafe { &mut *sink };
        if let Some(error) = sink.first_error {
            return Err(error);
        }
        sink.segments.push(Segment::line());
        crate::trace::event("sink_line", None);
        Ok(())
    }));
    match result {
        Ok(Ok(())) => RICH_OK,
        Ok(Err(error)) => poison_sink(sink, error),
        Err(_) => poison_sink(sink, RICH_ERROR_PANIC),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize};
    use std::thread;
    use std::time::Duration;

    #[derive(Default)]
    struct Observer {
        drops: AtomicUsize,
        active: AtomicUsize,
        max_active: AtomicUsize,
        callback_threads: Mutex<Vec<thread::ThreadId>>,
        drop_thread: Mutex<Option<thread::ThreadId>>,
        drop_overlapped: AtomicBool,
    }

    struct TestContext {
        observer: Arc<Observer>,
        render_result: AtomicU32,
        measure_result: AtomicU32,
        malformed_measurement: AtomicBool,
        poison_sink: AtomicBool,
        height: AtomicU32,
        last_width: AtomicU32,
        console: AtomicPtr<crate::RichConsole>,
        renderable: AtomicPtr<RichRenderable>,
        nested_result: AtomicU32,
        nested_width: AtomicU32,
        nested_version: AtomicU32,
        guarded_free_left_live: AtomicBool,
    }

    impl TestContext {
        fn new(observer: Arc<Observer>) -> Self {
            Self {
                observer,
                render_result: AtomicU32::new(RICH_OK.0),
                measure_result: AtomicU32::new(RICH_OK.0),
                malformed_measurement: AtomicBool::new(false),
                poison_sink: AtomicBool::new(false),
                height: AtomicU32::new(1),
                last_width: AtomicU32::new(0),
                console: AtomicPtr::new(ptr::null_mut()),
                renderable: AtomicPtr::new(ptr::null_mut()),
                nested_result: AtomicU32::new(RICH_OK.0),
                nested_width: AtomicU32::new(u32::MAX),
                nested_version: AtomicU32::new(u32::MAX),
                guarded_free_left_live: AtomicBool::new(false),
            }
        }
    }

    fn s(value: &str) -> RichStr {
        RichStr {
            data: value.as_ptr(),
            len: value.len() as u64,
        }
    }

    unsafe extern "C" fn test_render(
        context: *mut c_void,
        options: *const RichRenderOptions,
        sink: *mut RichSegmentSink,
    ) -> RichResult {
        let context = unsafe { &*(context as *const TestContext) };
        let options = unsafe { &*options };
        context
            .observer
            .callback_threads
            .lock()
            .unwrap()
            .push(thread::current().id());
        let active = context.observer.active.fetch_add(1, Ordering::SeqCst) + 1;
        context
            .observer
            .max_active
            .fetch_max(active, Ordering::SeqCst);
        if context.height.load(Ordering::SeqCst) == 99 {
            thread::sleep(Duration::from_millis(15));
        }
        context
            .last_width
            .store(options.max_width, Ordering::SeqCst);

        let configured = RichResult(context.render_result.load(Ordering::SeqCst));
        if configured != RICH_OK {
            context.observer.active.fetch_sub(1, Ordering::SeqCst);
            return configured;
        }

        if context.poison_sink.load(Ordering::SeqCst) {
            assert_eq!(
                rich_segment_sink_push(sink, s("kept private"), s("bad-style")),
                crate::RICH_ERROR_INVALID_ARGUMENT
            );
            assert_eq!(
                rich_segment_sink_line(sink),
                crate::RICH_ERROR_INVALID_ARGUMENT
            );
            context.observer.active.fetch_sub(1, Ordering::SeqCst);
            return RICH_OK;
        }

        let reentrant = context.renderable.load(Ordering::SeqCst);
        if !reentrant.is_null() {
            assert_eq!(
                crate::rich_console_set_color_system(ptr::null_mut(), u32::MAX),
                RICH_ERROR_INVALID_STATE
            );
            assert_eq!(
                crate::live::rich_progress_update(ptr::null(), u64::MAX, f64::NAN),
                RICH_ERROR_INVALID_STATE
            );
            let mut other_sink = RichSegmentSink {
                segments: Segments::new(),
                first_error: None,
            };
            assert_eq!(
                rich_segment_sink_line(&mut other_sink),
                RICH_ERROR_INVALID_STATE
            );
            let raw_sink = sink as usize;
            assert_eq!(
                thread::spawn(move || rich_segment_sink_line(raw_sink as *mut RichSegmentSink))
                    .join()
                    .unwrap(),
                RICH_ERROR_INVALID_STATE
            );
            context
                .nested_version
                .store(crate::rich_ffi_abi_version(), Ordering::SeqCst);
            let mut width = u32::MAX;
            let width_result =
                crate::rich_console_width(context.console.load(Ordering::SeqCst), &mut width);
            context.nested_width.store(width, Ordering::SeqCst);
            assert_eq!(width_result, RICH_ERROR_INVALID_STATE);

            let mut nested = crate::RichBuffer {
                data: ptr::dangling_mut(),
                len: 99,
            };
            let result = crate::rich_console_render(
                context.console.load(Ordering::SeqCst),
                reentrant,
                &mut nested,
            );
            context.nested_result.store(result.0, Ordering::SeqCst);
            assert!(nested.data.is_null());
            assert_eq!(nested.len, 0);

            let mut guarded_buffer = crate::RichBuffer {
                data: ptr::dangling_mut(),
                len: 17,
            };
            crate::rich_buffer_free(&mut guarded_buffer);
            assert_eq!(guarded_buffer.len, 17);
            assert!(!guarded_buffer.data.is_null());

            crate::rich_renderable_free(reentrant);
            crate::rich_console_free(context.console.load(Ordering::SeqCst));
            context.guarded_free_left_live.store(true, Ordering::SeqCst);
        }

        let height = context.height.load(Ordering::SeqCst).min(5);
        for y in 0..height {
            for x in 0..options.max_width {
                let h = x as f64 / options.max_width as f64;
                let l1 = 0.1 + (y as f64 / height as f64) * 0.7;
                let l2 = l1 + 0.7 / (2.0 * height as f64);
                let (r1, g1, b1) = hls_to_rgb(h, l1, 1.0);
                let (r2, g2, b2) = hls_to_rgb(h, l2, 1.0);
                let style = format!("rgb({r2},{g2},{b2}) on rgb({r1},{g1},{b1})");
                let result = rich_segment_sink_push(sink, s("▄"), s(&style));
                if result != RICH_OK {
                    context.observer.active.fetch_sub(1, Ordering::SeqCst);
                    return result;
                }
            }
            let result = rich_segment_sink_line(sink);
            if result != RICH_OK {
                context.observer.active.fetch_sub(1, Ordering::SeqCst);
                return result;
            }
        }
        context.observer.active.fetch_sub(1, Ordering::SeqCst);
        RICH_OK
    }

    unsafe extern "C" fn test_measure(
        context: *mut c_void,
        options: *const RichRenderOptions,
        out: *mut RichMeasurement,
    ) -> RichResult {
        let context = unsafe { &*(context as *const TestContext) };
        let options = unsafe { &*options };
        assert_eq!(
            rich_segment_sink_line(ptr::null_mut()),
            RICH_ERROR_INVALID_STATE
        );
        let configured = RichResult(context.measure_result.load(Ordering::SeqCst));
        if configured != RICH_OK {
            return configured;
        }
        unsafe {
            if context.malformed_measurement.load(Ordering::SeqCst) {
                *out = RichMeasurement {
                    minimum: 9,
                    maximum: 2,
                };
            } else {
                *out = RichMeasurement {
                    minimum: 1,
                    maximum: options.max_width,
                };
            }
        }
        RICH_OK
    }

    unsafe extern "C" fn test_drop(context: *mut c_void) {
        let context = unsafe { Box::from_raw(context as *mut TestContext) };
        context.observer.drop_overlapped.store(
            context.observer.active.load(Ordering::SeqCst) != 0,
            Ordering::SeqCst,
        );
        *context.observer.drop_thread.lock().unwrap() = Some(thread::current().id());
        context.observer.drops.fetch_add(1, Ordering::SeqCst);
    }

    fn hls_to_rgb(h: f64, l: f64, s: f64) -> (u8, u8, u8) {
        if s == 0.0 {
            let value = (l * 255.0) as u8;
            return (value, value, value);
        }
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;
        let channel = |p: f64, q: f64, mut t: f64| {
            if t < 0.0 {
                t += 1.0;
            }
            if t > 1.0 {
                t -= 1.0;
            }
            if t < 1.0 / 6.0 {
                p + (q - p) * 6.0 * t
            } else if t < 0.5 {
                q
            } else if t < 2.0 / 3.0 {
                p + (q - p) * (2.0 / 3.0 - t) * 6.0
            } else {
                p
            }
        };
        (
            (channel(p, q, h + 1.0 / 3.0) * 255.0) as u8,
            (channel(p, q, h) * 255.0) as u8,
            (channel(p, q, h - 1.0 / 3.0) * 255.0) as u8,
        )
    }

    fn vtable() -> RichCustomRenderableVTable {
        RichCustomRenderableVTable {
            render: Some(test_render),
            measure: Some(test_measure),
            drop: Some(test_drop),
        }
    }

    fn custom(context: Box<TestContext>, table: RichCustomRenderableVTable) -> *mut RichRenderable {
        let raw = Box::into_raw(context);
        let mut renderable = ptr::null_mut();
        assert_eq!(
            rich_custom_renderable_new(raw.cast(), &table, &mut renderable),
            RICH_OK
        );
        renderable
    }

    fn console(width: u32) -> *mut crate::RichConsole {
        let mut console = ptr::null_mut();
        assert_eq!(crate::rich_console_new(&mut console), RICH_OK);
        assert_eq!(crate::rich_console_set_size(console, width, 72), RICH_OK);
        assert_eq!(
            crate::rich_console_set_color_system(console, RICH_COLOR_TRUECOLOR),
            RICH_OK
        );
        console
    }

    fn render(
        console: *mut crate::RichConsole,
        renderable: *const RichRenderable,
    ) -> (RichResult, Vec<u8>) {
        let mut output = crate::RichBuffer::default();
        let result = crate::rich_console_render(console, renderable, &mut output);
        let bytes = if output.len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(output.data, output.len as usize) }.to_vec()
        };
        crate::rich_buffer_free(&mut output);
        (result, bytes)
    }

    #[test]
    fn constructor_nullability_and_context_transfer() {
        let mut output = ptr::dangling_mut();
        assert_eq!(
            rich_custom_renderable_new(ptr::null_mut(), ptr::null(), &mut output),
            crate::RICH_ERROR_NULL_ARGUMENT
        );
        assert!(output.is_null());

        let observer = Arc::new(Observer::default());
        let context = Box::new(TestContext::new(observer.clone()));
        let raw = Box::into_raw(context);
        let invalid = RichCustomRenderableVTable {
            render: None,
            measure: None,
            drop: Some(test_drop),
        };
        assert_eq!(
            rich_custom_renderable_new(raw.cast(), &invalid, &mut output),
            crate::RICH_ERROR_INVALID_ARGUMENT
        );
        assert!(output.is_null());
        assert_eq!(observer.drops.load(Ordering::SeqCst), 0);
        unsafe { drop(Box::from_raw(raw)) };

        let no_drop = RichCustomRenderableVTable {
            render: Some(test_render),
            measure: None,
            drop: None,
        };
        let context = Box::new(TestContext::new(observer.clone()));
        let raw = Box::into_raw(context);
        assert_eq!(
            rich_custom_renderable_new(raw.cast(), &no_drop, &mut output),
            crate::RICH_ERROR_INVALID_ARGUMENT
        );
        unsafe { drop(Box::from_raw(raw)) };

        assert_eq!(
            rich_custom_renderable_new(ptr::null_mut(), &no_drop, &mut output),
            RICH_OK
        );
        crate::rich_renderable_free(output);
    }

    #[test]
    fn colorbox_is_width_aware_and_context_drops_once() {
        for width in [1, 7, 120] {
            let observer = Arc::new(Observer::default());
            let context = Box::new(TestContext::new(observer.clone()));
            context.height.store(5, Ordering::SeqCst);
            let context_ptr = &*context as *const TestContext;
            let renderable = custom(context, vtable());
            let console = console(width);
            let (result, bytes) = render(console, renderable);
            assert_eq!(result, RICH_OK);
            assert_eq!(
                bytes
                    .windows(3)
                    .filter(|part| *part == "▄".as_bytes())
                    .count(),
                width as usize * 5
            );
            assert!(
                bytes
                    .windows(b"\x1b[38;2".len())
                    .any(|part| part == b"\x1b[38;2")
            );
            assert_eq!(
                unsafe { (*context_ptr).last_width.load(Ordering::SeqCst) },
                width
            );
            crate::rich_renderable_free(renderable);
            assert_eq!(observer.drops.load(Ordering::SeqCst), 1);
            assert!(!observer.drop_overlapped.load(Ordering::SeqCst));
            crate::rich_console_free(console);
        }
    }

    #[test]
    fn callback_and_sink_failures_are_transactional() {
        let observer = Arc::new(Observer::default());
        let context = Box::new(TestContext::new(observer.clone()));
        context.poison_sink.store(true, Ordering::SeqCst);
        let renderable = custom(context, vtable());
        let native_console = console(20);
        let (result, bytes) = render(native_console, renderable);
        assert_eq!(result, crate::RICH_ERROR_INVALID_ARGUMENT);
        assert!(bytes.is_empty());
        crate::rich_renderable_free(renderable);
        crate::rich_console_free(native_console);

        for callback_result in [crate::RICH_ERROR_IO.0, 99] {
            let context = Box::new(TestContext::new(observer.clone()));
            context
                .render_result
                .store(callback_result, Ordering::SeqCst);
            let renderable = custom(context, vtable());
            let console = console(20);
            let (result, bytes) = render(console, renderable);
            assert_eq!(
                result,
                if callback_result == crate::RICH_ERROR_IO.0 {
                    crate::RICH_ERROR_IO
                } else {
                    RICH_ERROR_RENDER
                }
            );
            assert!(bytes.is_empty());
            crate::rich_renderable_free(renderable);
            crate::rich_console_free(console);
        }

        let context = Box::new(TestContext::new(observer));
        context.malformed_measurement.store(true, Ordering::SeqCst);
        let renderable = custom(context, vtable());
        let mut table = ptr::null_mut();
        assert_eq!(crate::table::rich_table_new_grid(&mut table), RICH_OK);
        let cells = [renderable as *const RichRenderable];
        assert_eq!(
            crate::table::rich_table_add_row_renderables(table, cells.as_ptr(), 1),
            RICH_OK
        );
        let mut parent = ptr::null_mut();
        assert_eq!(
            crate::table::rich_table_finish(&mut table, &mut parent),
            RICH_OK
        );
        crate::rich_renderable_free(renderable);
        let console = console(20);
        let (result, bytes) = render(console, parent);
        assert_eq!(result, RICH_ERROR_RENDER);
        assert!(bytes.is_empty());
        crate::rich_renderable_free(parent);
        crate::rich_console_free(console);
    }

    #[test]
    fn callback_scope_rejects_reentry_and_initializes_outputs() {
        let observer = Arc::new(Observer::default());
        let context = Box::new(TestContext::new(observer));
        let context_ptr = &*context as *const TestContext;
        let renderable = custom(context, vtable());
        let console = console(8);
        unsafe {
            (*context_ptr).console.store(console, Ordering::SeqCst);
            (*context_ptr)
                .renderable
                .store(renderable, Ordering::SeqCst);
        }
        let (result, bytes) = render(console, renderable);
        assert_eq!(result, RICH_OK);
        assert!(!bytes.is_empty());
        unsafe {
            assert_eq!((*context_ptr).nested_version.load(Ordering::SeqCst), 0);
            assert_eq!((*context_ptr).nested_width.load(Ordering::SeqCst), 0);
            assert_eq!(
                (*context_ptr).nested_result.load(Ordering::SeqCst),
                RICH_ERROR_INVALID_STATE.0
            );
            assert!((*context_ptr).guarded_free_left_live.load(Ordering::SeqCst));
            (*context_ptr)
                .renderable
                .store(ptr::null_mut(), Ordering::SeqCst);
        }
        let (second, _) = render(console, renderable);
        assert_eq!(second, RICH_OK);
        crate::rich_renderable_free(renderable);
        crate::rich_console_free(console);
    }

    #[test]
    fn shared_adapter_callbacks_serialize_across_threads() {
        let observer = Arc::new(Observer::default());
        let context = Box::new(TestContext::new(observer.clone()));
        context.height.store(99, Ordering::SeqCst);
        let first = custom(
            context,
            RichCustomRenderableVTable {
                render: Some(test_render),
                measure: None,
                drop: Some(test_drop),
            },
        );
        let second = unsafe { Box::into_raw(Box::new(RichRenderable(Arc::clone(&(*first).0)))) };

        let handles = [first as usize, second as usize];
        let threads: Vec<_> = handles
            .into_iter()
            .map(|handle| {
                thread::spawn(move || {
                    let id = thread::current().id();
                    let console = console(6);
                    let (result, _) = render(console, handle as *mut RichRenderable);
                    assert_eq!(result, RICH_OK);
                    crate::rich_renderable_free(handle as *mut RichRenderable);
                    crate::rich_console_free(console);
                    id
                })
            })
            .collect();
        let caller_threads: Vec<_> = threads
            .into_iter()
            .map(|join| join.join().unwrap())
            .collect();
        assert_eq!(observer.max_active.load(Ordering::SeqCst), 1);
        let callback_threads = observer.callback_threads.lock().unwrap();
        assert!(
            caller_threads
                .iter()
                .all(|id| callback_threads.contains(id))
        );
        drop(callback_threads);
        assert_eq!(observer.drops.load(Ordering::SeqCst), 1);
        assert!(caller_threads.contains(observer.drop_thread.lock().unwrap().as_ref().unwrap()));
        assert!(!observer.drop_overlapped.load(Ordering::SeqCst));
    }

    #[test]
    fn last_release_during_an_active_callback_drops_after_quiescence() {
        let observer = Arc::new(Observer::default());
        let context = Box::new(TestContext::new(observer.clone()));
        context.height.store(99, Ordering::SeqCst);
        let retained = custom(
            context,
            RichCustomRenderableVTable {
                render: Some(test_render),
                measure: None,
                drop: Some(test_drop),
            },
        );
        let rendering =
            unsafe { Box::into_raw(Box::new(RichRenderable(Arc::clone(&(*retained).0)))) } as usize;

        let thread = thread::spawn(move || {
            let caller = thread::current().id();
            let console = console(6);
            let (result, _) = render(console, rendering as *mut RichRenderable);
            assert_eq!(result, RICH_OK);
            crate::rich_renderable_free(rendering as *mut RichRenderable);
            crate::rich_console_free(console);
            caller
        });
        while observer.active.load(Ordering::SeqCst) == 0 {
            thread::yield_now();
        }
        crate::rich_renderable_free(retained);
        assert_eq!(observer.drops.load(Ordering::SeqCst), 0);

        let caller = thread.join().unwrap();
        assert_eq!(observer.drops.load(Ordering::SeqCst), 1);
        assert_eq!(*observer.drop_thread.lock().unwrap(), Some(caller));
        assert!(!observer.drop_overlapped.load(Ordering::SeqCst));
    }
}
