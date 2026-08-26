// Generate include/rich.h from the public ABI on every build. Generation is
// deliberately fatal: compiling against a stale ABI header is never safe.
fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let config = cbindgen::Config::from_file(format!("{crate_dir}/cbindgen.toml"))
        .expect("failed to load cbindgen.toml");

    let bindings = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .expect("cbindgen failed to generate rich.h");
    bindings.write_to_file(format!("{crate_dir}/include/rich.h"));

    // Rerun on ANY source change, not just lib.rs — Wave-2 moves exported fns into
    // per-renderable modules, and the generated header must not silently drift.
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");
    // The regeneration script changes this nonce on every invocation so a
    // manually stale/deleted header is repaired even when Rust sources are unchanged.
    println!("cargo:rerun-if-env-changed=RICH_FFI_HEADER_NONCE");
}
