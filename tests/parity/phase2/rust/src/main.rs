mod console;
mod markup;
mod text;
mod text_wrap;
mod wrap;

fn main() {
    let module = std::env::args().nth(1).unwrap_or_else(|| "all".into());

    match module.as_str() {
        "console" => console::run(),
        "text" => text::run(),
        "markup" => markup::run(),
        "wrap" => wrap::run(),
        "text_wrap" => text_wrap::run(),
        "all" => {
            console::run();
            text::run();
            markup::run();
            wrap::run();
            text_wrap::run();
        }
        _ => {
            eprintln!("Unknown module: {}", module);
            std::process::exit(1);
        }
    }
}
