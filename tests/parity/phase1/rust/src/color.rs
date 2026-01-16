use rich_rs::{Color, ColorSystem, ColorType};

fn format_color_type(ct: ColorType) -> &'static str {
    match ct {
        ColorType::Default => "DEFAULT",
        ColorType::Standard => "STANDARD",
        ColorType::EightBit => "EIGHT_BIT",
        ColorType::TrueColor => "TRUECOLOR",
        ColorType::Windows => "WINDOWS",
    }
}

fn format_triplet(c: &Color) -> String {
    match &c.triplet {
        Some(t) => format!("ColorTriplet(red={}, green={}, blue={})", t.red, t.green, t.blue),
        None => "None".to_string(),
    }
}

pub fn run() {
    println!("=== Color Parsing ===");

    // Named colors
    let c = Color::parse("red").unwrap();
    println!("parse(\"red\") -> type={}, number={}", format_color_type(c.color_type), c.number.unwrap());

    let c = Color::parse("blue").unwrap();
    println!("parse(\"blue\") -> type={}, number={}", format_color_type(c.color_type), c.number.unwrap());

    let c = Color::parse("green").unwrap();
    println!("parse(\"green\") -> type={}, number={}", format_color_type(c.color_type), c.number.unwrap());

    // Hex colors
    let c = Color::parse("#ff0000").unwrap();
    println!("parse(\"#ff0000\") -> type={}, triplet={}", format_color_type(c.color_type), format_triplet(&c));

    let c = Color::parse("#00ff00").unwrap();
    println!("parse(\"#00ff00\") -> type={}, triplet={}", format_color_type(c.color_type), format_triplet(&c));

    // RGB function
    let c = Color::parse("rgb(255,128,0)").unwrap();
    println!("parse(\"rgb(255,128,0)\") -> type={}, triplet={}", format_color_type(c.color_type), format_triplet(&c));

    // Color number
    let c = Color::parse("color(196)").unwrap();
    println!("parse(\"color(196)\") -> type={}, number={}", format_color_type(c.color_type), c.number.unwrap());

    // Default
    let c = Color::parse("default").unwrap();
    println!("parse(\"default\") -> type={}", format_color_type(c.color_type));

    println!("\n=== ANSI Codes (foreground) ===");

    let c = Color::parse("red").unwrap();
    let codes = c.get_ansi_codes(true);
    println!("Standard red -> {}", codes.join(";"));

    let c = Color::parse("color(196)").unwrap();
    let codes = c.get_ansi_codes(true);
    println!("EightBit(196) -> {}", codes.join(";"));

    let c = Color::parse("#ff0000").unwrap();
    let codes = c.get_ansi_codes(true);
    println!("TrueColor(255,0,0) -> {}", codes.join(";"));

    println!("\n=== ANSI Codes (background) ===");

    let c = Color::parse("red").unwrap();
    let codes = c.get_ansi_codes(false);
    println!("Standard red bg -> {}", codes.join(";"));

    let c = Color::parse("#ff0000").unwrap();
    let codes = c.get_ansi_codes(false);
    println!("TrueColor(255,0,0) bg -> {}", codes.join(";"));

    println!("\n=== Color Downgrade ===");

    let c = Color::parse("#ff0000").unwrap();
    let downgraded = c.downgrade(ColorSystem::EightBit);
    println!("#ff0000 -> EIGHT_BIT: type={}, number={}", format_color_type(downgraded.color_type), downgraded.number.unwrap());

    let c = Color::parse("color(196)").unwrap();
    let downgraded = c.downgrade(ColorSystem::Standard);
    println!("color(196) -> STANDARD: type={}, number={}", format_color_type(downgraded.color_type), downgraded.number.unwrap());
}
