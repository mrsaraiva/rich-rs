// Generate include/rich.h from the public ABI on every build.
// Best-effort: a cbindgen hiccup logs a warning but never blocks the link.
fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let config = cbindgen::Config::from_file(format!("{crate_dir}/cbindgen.toml"))
        .unwrap_or_default();

    match cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
    {
        Ok(bindings) => {
            bindings.write_to_file(format!("{crate_dir}/include/rich.h"));
        }
        Err(e) => {
            println!("cargo:warning=cbindgen failed to generate rich.h: {e}");
        }
    }

    // Rerun on ANY source change, not just lib.rs — Wave-2 moves exported fns into
    // per-renderable modules, and the generated header must not silently drift.
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");
}
