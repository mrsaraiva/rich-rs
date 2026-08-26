//! Caller-driven live widgets. No function starts a background refresh thread.

use rich_rs::{LiveOptions, Progress, Spinner, Status, TaskID};

use crate::common::{
    destroy, ffi_call, guard_ordinary_call, init_buffer, init_out, init_scalar_out,
    render_to_bytes, required_mut, required_ref, rich_str, write_buffer,
};
use crate::{
    RICH_ERROR_INVALID_ARGUMENT, RICH_ERROR_INVALID_STATE, RichBuffer, RichConsole, RichResult,
    RichStr,
};

/// Owning caller-driven progress state. No ABI call starts a refresh thread;
/// frame output is owned by the caller and released with `rich_buffer_free`.
pub struct RichProgress(Progress);

#[unsafe(no_mangle)]
/// Create progress state with auto-refresh disabled.
pub extern "C" fn rich_progress_new(out_progress: *mut *mut RichProgress) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_progress)? };
        let options = LiveOptions {
            auto_refresh: false,
            ..Default::default()
        };
        let progress = Progress::new_default(options, false, false, false);
        unsafe { *out_progress = Box::into_raw(Box::new(RichProgress(progress))) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Add a task and write its ID; task ID zero is a valid successful output.
pub extern "C" fn rich_progress_add_task(
    progress: *const RichProgress,
    description: RichStr,
    total: f64,
    out_task_id: *mut u64,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_scalar_out(out_task_id)? };
        if !total.is_finite() {
            return Err(RICH_ERROR_INVALID_ARGUMENT);
        }
        let description = unsafe { rich_str(description)? };
        let progress = unsafe { required_ref(progress)? };
        let id = progress
            .0
            .add_task(description, true, Some(total), 0.0, true);
        unsafe { *out_task_id = u64::try_from(id.0).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)? };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_progress_update(
    progress: *const RichProgress,
    task_id: u64,
    completed: f64,
) -> RichResult {
    ffi_call(|| {
        guard_ordinary_call()?;
        if !completed.is_finite() {
            return Err(RICH_ERROR_INVALID_ARGUMENT);
        }
        let task_id = usize::try_from(task_id).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?;
        let progress = unsafe { required_ref(progress)? };
        if !progress.0.task_ids().contains(&TaskID(task_id)) {
            return Err(RICH_ERROR_INVALID_STATE);
        }
        progress.0.update(
            TaskID(task_id),
            None,
            Some(completed),
            None,
            None,
            None,
            false,
            None,
        );
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Render one caller-driven progress frame to an owned UTF-8 buffer.
pub extern "C" fn rich_progress_render_frame(
    progress: *const RichProgress,
    console: *mut RichConsole,
    out_frame: *mut RichBuffer,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_buffer(out_frame)? };
        let progress_handle = unsafe { required_ref(progress)? };
        let console_handle = unsafe { required_mut(console)? };
        let bytes = render_to_bytes(console_handle, &progress_handle.0)?;
        unsafe { write_buffer(out_frame, bytes) }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_progress_free(progress: *mut RichProgress) {
    unsafe { destroy(progress) }
}

/// Owning caller-driven status widget. Rendering produces one owned frame and
/// never starts a foreign callback or background refresh.
pub struct RichStatus(Option<Status>);

#[unsafe(no_mangle)]
/// Create status state from borrowed UTF-8 copied by Rust.
pub extern "C" fn rich_status_new(
    message: RichStr,
    out_status: *mut *mut RichStatus,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_status)? };
        let message = unsafe { rich_str(message)? };
        unsafe { *out_status = Box::into_raw(Box::new(RichStatus(Some(Status::new(message))))) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_status_render_frame(
    status: *const RichStatus,
    console: *mut RichConsole,
    out_frame: *mut RichBuffer,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_buffer(out_frame)? };
        let status = unsafe { required_ref(status)? }
            .0
            .as_ref()
            .ok_or(RICH_ERROR_INVALID_STATE)?;
        let console = unsafe { required_mut(console)? };
        let bytes = render_to_bytes(console, status)?;
        unsafe { write_buffer(out_frame, bytes) }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_status_free(status: *mut RichStatus) {
    unsafe { destroy(status) }
}

/// Owning caller-driven spinner selected by name. Unknown names and non-finite
/// frame times are invalid arguments.
pub struct RichSpinner(Option<Spinner>);

#[unsafe(no_mangle)]
/// Create a spinner through `out_spinner`.
pub extern "C" fn rich_spinner_new(
    name: RichStr,
    out_spinner: *mut *mut RichSpinner,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_spinner)? };
        let name = unsafe { rich_str(name)? };
        let spinner = Spinner::new(name).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?;
        unsafe { *out_spinner = Box::into_raw(Box::new(RichSpinner(Some(spinner)))) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_spinner_render_frame(
    spinner: *const RichSpinner,
    console: *mut RichConsole,
    time_seconds: f64,
    out_frame: *mut RichBuffer,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_buffer(out_frame)? };
        if !time_seconds.is_finite() {
            return Err(RICH_ERROR_INVALID_ARGUMENT);
        }
        let spinner = unsafe { required_ref(spinner)? }
            .0
            .as_ref()
            .ok_or(RICH_ERROR_INVALID_STATE)?;
        let console = unsafe { required_mut(console)? };
        let frame = spinner.render_at(time_seconds, Some(0.0), None);
        let bytes = render_to_bytes(console, &frame)?;
        unsafe { write_buffer(out_frame, bytes) }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_spinner_free(spinner: *mut RichSpinner) {
    unsafe { destroy(spinner) }
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

    #[test]
    fn progress_zero_task_id_is_success_and_unknown_is_distinct() {
        let mut progress = ptr::null_mut();
        assert_eq!(rich_progress_new(&mut progress), crate::RICH_OK);
        let mut id = u64::MAX;
        assert_eq!(
            rich_progress_add_task(progress, s("work"), 10.0, &mut id),
            crate::RICH_OK
        );
        assert_eq!(id, 0);
        assert_eq!(rich_progress_update(progress, id, 1.0), crate::RICH_OK);
        assert_eq!(
            rich_progress_update(progress, 999, 1.0),
            RICH_ERROR_INVALID_STATE
        );
        rich_progress_free(progress);
    }
}
