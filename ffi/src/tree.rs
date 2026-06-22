//! Phase 4 — Tree C ABI.
//!
//! Implemented by the `ffi-phase4` lane. All functions here are
//! `#[unsafe(no_mangle)] pub extern "C"` and reuse the Phase-1 plumbing in
//! `crate::common` and the `crate::RichRenderable` composition currency.
//! cbindgen picks them up from this module automatically.
