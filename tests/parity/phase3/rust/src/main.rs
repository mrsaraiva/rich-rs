mod align;
mod r#box;
mod padding;
mod rule;

fn main() {
    let module = std::env::args().nth(1).unwrap_or_else(|| "all".into());

    match module.as_str() {
        "align" => align::run(),
        "box" => r#box::run(),
        "padding" => padding::run(),
        "rule" => rule::run(),
        "all" => {
            align::run();
            r#box::run();
            padding::run();
            rule::run();
        }
        _ => {
            eprintln!("Unknown module: {}", module);
            std::process::exit(1);
        }
    }
}
