//! Phase 7 — Live widgets C ABI (Progress, Status, Spinner).
//!
//! Implemented by the `ffi-live` lane. These widgets are animated/stateful, so
//! the ABI is **frame-based and caller-driven**: a `*_render_frame` function
//! returns one frame as an owned string and the C/C++ caller owns the loop and
//! the terminal cursor. No background thread ever calls back across the FFI.
//!
//! Concretely: there is **no `start()` / `stop()` / background refresh** across
//! this boundary. The C/C++ caller drives its own animation loop — on each tick
//! it mutates state (`rich_progress_update`, ...) and calls the matching
//! `*_render_frame` to obtain ONE frame as an owned C string, which it positions
//! on the terminal itself (cursor save/restore, line clears, etc.). This keeps
//! the FFI single-threaded and panic-safe: rich-rs never spawns a thread that
//! unwinds back into C.
//!
//! Functions here are `#[unsafe(no_mangle)] pub extern "C"` and reuse the
//! Phase-1 plumbing in `crate::common`. cbindgen picks them up automatically.

use std::ffi::{CStr, c_char, c_double};
use std::panic::{AssertUnwindSafe, catch_unwind};

use rich_rs::{LiveOptions, Progress, Spinner, Status, TaskID};

use crate::RichConsole;
use crate::common::render_to_cstring;

// ===========================================================================
// Phase 7 — RichProgress: a multi-task progress display, rendered frame by frame
// ===========================================================================

/// Opaque `Progress` handle — a multi-task progress display.
///
/// Built with `rich_progress_new` (default columns: description, bar,
/// percentage, time remaining). Tasks are added with `rich_progress_add_task`
/// and advanced with `rich_progress_update`. Each `rich_progress_render_frame`
/// renders the CURRENT state of all tasks to one owned string; the C/C++ caller
/// owns the animation loop and the terminal cursor (see the module contract —
/// no background thread crosses the FFI). Free with `rich_progress_free`.
///
/// `Progress` uses interior mutability (a `Mutex` over its task state), so the
/// task-mutating functions take a shared handle and never need exclusive access.
pub struct RichProgress(Progress);

/// Create a `Progress` with Rich's default columns (description, bar,
/// percentage, time remaining). Returns NULL on allocation/init failure.
///
/// The returned handle MUST be freed with `rich_progress_free`. Auto-refresh is
/// disabled: this ABI never starts a background thread — the caller renders each
/// frame explicitly via `rich_progress_render_frame`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_progress_new() -> *mut RichProgress {
    catch_unwind(|| {
        // No background refresh thread ever crosses the FFI: the caller drives
        // the loop and calls `rich_progress_render_frame` per tick.
        let live_options = LiveOptions {
            auto_refresh: false,
            ..Default::default()
        };
        // disable = false, expand = false, show_speed = false.
        let progress = Progress::new_default(live_options, false, false, false);
        Box::into_raw(Box::new(RichProgress(progress)))
    })
    .unwrap_or(std::ptr::null_mut())
}

/// Add a task to the progress display, returning its task id.
///
/// `description` must be valid NUL-terminated UTF-8. `total` is the total amount
/// of work (e.g. `100.0`); pass a non-positive/`0` value to still create a task
/// (it is forwarded as-is). The task is started immediately and made visible.
///
/// Returns the new task id as an `unsigned long long` (maps to rich-rs
/// `TaskID(usize)`). Returns `0` if `progress`/`description` is NULL or
/// `description` is not valid UTF-8 — note `0` is also the FIRST legitimately
/// assigned task id, so this sentinel is only meaningful for distinguishing a
/// NULL-argument early-out, not for in-band error detection.
#[unsafe(no_mangle)]
pub extern "C" fn rich_progress_add_task(
    progress: *mut RichProgress,
    description: *const c_char,
    total: c_double,
) -> u64 {
    if progress.is_null() || description.is_null() {
        return 0;
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_progress_new`; caller guarantees a
        // valid NUL-terminated string.
        let p = unsafe { &*progress };
        let desc = unsafe { CStr::from_ptr(description) }.to_str().ok()?;
        // start = true, completed = 0.0, visible = true.
        let id = p.0.add_task(desc, true, Some(total), 0.0, true);
        Some(id.0 as u64)
    }));
    result.ok().flatten().unwrap_or(0)
}

/// Set a task's completed amount to `completed`.
///
/// `task_id` is a value previously returned by `rich_progress_add_task`. An
/// unknown id is a silent no-op (matching rich-rs `Progress::update`). Does not
/// trigger a refresh (the caller renders frames explicitly). NULL `progress`
/// is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_progress_update(
    progress: *mut RichProgress,
    task_id: u64,
    completed: c_double,
) {
    if progress.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from `rich_progress_new`.
        let p = unsafe { &*progress };
        p.0.update(
            TaskID(task_id as usize),
            None,             // total: unchanged
            Some(completed),  // completed: set absolute
            None,             // advance: none
            None,             // description: unchanged
            None,             // visible: unchanged
            false,            // refresh: caller drives frames
            None,             // fields: unchanged
        );
    }));
}

/// Render the CURRENT progress state to an owned, NUL-terminated C string.
///
/// Renders every visible task's row (the tasks table) through `console`,
/// honoring its force-terminal / color / width settings: with
/// `set_force_terminal(true)` the frame contains ANSI escapes; with `false` it
/// is plain text with zero escapes. No trailing newline is appended — the caller
/// owns line endings and cursor positioning.
///
/// BORROWS both handles (neither is consumed). Returns NULL if
/// `progress`/`console` is NULL or rendering fails. A non-NULL result MUST be
/// freed with `rich_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_progress_render_frame(
    progress: *mut RichProgress,
    console: *mut RichConsole,
) -> *mut c_char {
    if progress.is_null() || console.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handles from `rich_progress_new` / `rich_console_new`.
        let p = unsafe { &*progress };
        let con = unsafe { &mut *console };
        render_to_cstring(con, &p.0)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Free a `Progress` created by `rich_progress_new`. NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_progress_free(progress: *mut RichProgress) {
    if progress.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_progress_new`.
        drop(unsafe { Box::from_raw(progress) });
    }));
}

// ===========================================================================
// Phase 7 — RichStatus: a spinner + status message, rendered frame by frame
// ===========================================================================

/// Opaque `Status` handle — a spinner animation with a status message.
///
/// Built with `rich_status_new`. Each `rich_status_render_frame` renders one
/// frame (spinner glyph + message) to an owned string; the spinner advances
/// according to wall-clock time elapsed since creation, so successive frames
/// animate. The C/C++ caller owns the loop and cursor (no thread crosses the
/// FFI). Free with `rich_status_free`.
///
/// The inner `Option<Status>` lets the handle hold the value by move for a
/// uniform new/free idiom (the option is always `Some` while live).
pub struct RichStatus(Option<Status>);

/// Create a `Status` with the given message (uses the default "dots" spinner).
///
/// `message` may contain console markup (e.g. `"[bold green]Working..."`). It
/// must be valid NUL-terminated UTF-8. Returns NULL if `message` is NULL or not
/// valid UTF-8. The returned handle MUST be freed with `rich_status_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_status_new(message: *const c_char) -> *mut RichStatus {
    if message.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees a valid NUL-terminated string.
        let msg = unsafe { CStr::from_ptr(message) }.to_str().ok()?;
        Some(Box::into_raw(Box::new(RichStatus(Some(Status::new(msg))))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Render the CURRENT status frame (spinner glyph + message) to an owned C
/// string through `console`.
///
/// Honors the console's force-terminal / color / width settings: styled (with
/// ANSI escapes) when forced, plain (zero escapes) otherwise. No trailing
/// newline is appended. BORROWS both handles (neither consumed).
///
/// Returns NULL if `status`/`console` is NULL, the status was somehow empty, or
/// rendering fails. A non-NULL result MUST be freed with `rich_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_status_render_frame(
    status: *mut RichStatus,
    console: *mut RichConsole,
) -> *mut c_char {
    if status.is_null() || console.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handles from `rich_status_new` / `rich_console_new`.
        let s = unsafe { &*status };
        let con = unsafe { &mut *console };
        let inner = s.0.as_ref()?;
        Some(render_to_cstring(con, inner))
    }));
    match result {
        Ok(Some(ptr)) => ptr,
        _ => std::ptr::null_mut(),
    }
}

/// Free a `Status` created by `rich_status_new`. NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_status_free(status: *mut RichStatus) {
    if status.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_status_new`.
        drop(unsafe { Box::from_raw(status) });
    }));
}

// ===========================================================================
// Phase 7 — RichSpinner: a standalone spinner, rendered at an explicit time
// ===========================================================================

/// Opaque `Spinner` handle — a single terminal spinner animation.
///
/// Built with `rich_spinner_new` (NULL if the spinner name is unknown). Each
/// `rich_spinner_render_frame` renders the glyph for an explicit elapsed time
/// supplied by the caller, so the caller controls the animation phase exactly
/// (no internal clock crosses the FFI). Free with `rich_spinner_free`.
///
/// The inner `Option<Spinner>` mirrors the other live-widget handles' move idiom.
pub struct RichSpinner(Option<Spinner>);

/// Create a `Spinner` by name (e.g. `"dots"`, `"line"`, `"earth"`).
///
/// `name` must be valid NUL-terminated UTF-8. Returns NULL if `name` is NULL,
/// not valid UTF-8, or NOT a known spinner (rich-rs `Spinner::new` returns an
/// error for unknown names). The returned handle MUST be freed with
/// `rich_spinner_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_spinner_new(name: *const c_char) -> *mut RichSpinner {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees a valid NUL-terminated string.
        let n = unsafe { CStr::from_ptr(name) }.to_str().ok()?;
        // Spinner::new returns Err for unknown names -> NULL.
        let spinner = Spinner::new(n).ok()?;
        Some(Box::into_raw(Box::new(RichSpinner(Some(spinner)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Render the spinner glyph for `time_seconds` of elapsed time to an owned C
/// string through `console`.
///
/// `time_seconds` is the elapsed time (from the caller's chosen epoch) used to
/// pick the animation frame; advancing it across calls animates the spinner.
/// Honors the console's force-terminal / color / width settings: styled (ANSI)
/// when forced, plain (zero escapes) otherwise. No trailing newline is appended.
/// BORROWS both handles (neither consumed).
///
/// Returns NULL if `spinner`/`console` is NULL or rendering fails. A non-NULL
/// result MUST be freed with `rich_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_spinner_render_frame(
    spinner: *mut RichSpinner,
    console: *mut RichConsole,
    time_seconds: c_double,
) -> *mut c_char {
    if spinner.is_null() || console.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handles from `rich_spinner_new` / `rich_console_new`.
        let s = unsafe { &*spinner };
        let con = unsafe { &mut *console };
        let inner = s.0.as_ref()?;
        // Render the glyph for the caller-supplied elapsed time (epoch 0.0).
        let frame = inner.render_at(time_seconds, Some(0.0), None);
        Some(render_to_cstring(con, &frame))
    }));
    match result {
        Ok(Some(ptr)) => ptr,
        _ => std::ptr::null_mut(),
    }
}

/// Free a `Spinner` created by `rich_spinner_new`. NULL is a no-op.
#[unsafe(no_mangle)]
pub extern "C" fn rich_spinner_free(spinner: *mut RichSpinner) {
    if spinner.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in `rich_spinner_new`.
        drop(unsafe { Box::from_raw(spinner) });
    }));
}
