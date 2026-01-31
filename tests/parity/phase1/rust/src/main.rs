mod cells;
mod color;
mod measure;
mod segment;
mod style;

fn main() {
    let module = std::env::args().nth(1).unwrap_or_else(|| "all".into());

    match module.as_str() {
        "color" => color::run(),
        "cells" => cells::run(),
        "style" => style::run(),
        "segment" => segment::run(),
        "measure" => measure::run(),
        "all" => {
            color::run();
            cells::run();
            style::run();
            segment::run();
            measure::run();
        }
        _ => eprintln!("Unknown module: {}", module),
    }
}
