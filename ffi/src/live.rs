//! Phase 7 — Live widgets C ABI (Progress, Status, Spinner).
//!
//! Implemented by the `ffi-live` lane. These widgets are animated/stateful, so
//! the ABI is **frame-based and caller-driven**: a `*_render_frame` function
//! returns one frame as an owned string and the C/C++ caller owns the loop and
//! the terminal cursor. No background thread ever calls back across the FFI.
//!
//! Functions here are `#[unsafe(no_mangle)] pub extern "C"` and reuse the
//! Phase-1 plumbing in `crate::common`. cbindgen picks them up automatically.
