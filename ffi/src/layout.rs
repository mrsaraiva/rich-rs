//! Phase 5 — Layout C ABI (Rule, Columns, Align, Padding).
//!
//! Implemented by the `ffi-phase5` lane. All functions here are
//! `#[unsafe(no_mangle)] pub extern "C"` and reuse the Phase-1 plumbing in
//! `crate::common` and the `crate::RichRenderable` composition currency.
//! cbindgen picks them up from this module automatically.
