//! Parity tests for Phase 2 modules.

mod markup;

fn main() {
    let module = std::env::args().nth(1).unwrap_or_else(|| "all".into());

    match module.as_str() {
        "markup" | "all" => markup::run(),
        _ => eprintln!("Unknown module: {}", module),
    }
}
