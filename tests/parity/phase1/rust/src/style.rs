use rich_rs::{ColorSystem, SimpleColor, Style};

fn color_name(color: &SimpleColor) -> String {
    // Reverse lookup standard ANSI colors to names
    match color {
        SimpleColor::Standard(1) => "red".to_string(),
        SimpleColor::Standard(4) => "blue".to_string(),
        SimpleColor::Standard(2) => "green".to_string(),
        SimpleColor::Standard(n) => format!("color({})", n),
        SimpleColor::EightBit(n) => format!("color({})", n),
        SimpleColor::Rgb { r, g, b } => format!("rgb({},{},{})", r, g, b),
        SimpleColor::Default => "default".to_string(),
    }
}

fn format_color(style: &Style) -> String {
    match &style.color {
        Some(c) => color_name(c),
        None => "None".to_string(),
    }
}

fn bool_py(value: bool) -> &'static str {
    if value {
        "True"
    } else {
        "False"
    }
}

fn bool_py_lower(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn escape_ansi(s: &str) -> String {
    format!("\"{}\"", s.replace('\x1b', "\\x1b"))
}

pub fn run() {
    println!("=== Style Parsing ===");

    let s = Style::parse("bold").unwrap();
    println!("parse(\"bold\") -> bold={}", bool_py(s.bold.unwrap()));

    let s = Style::parse("italic").unwrap();
    println!("parse(\"italic\") -> italic={}", bool_py(s.italic.unwrap()));

    let s = Style::parse("bold italic").unwrap();
    println!(
        "parse(\"bold italic\") -> bold={}, italic={}",
        bool_py(s.bold.unwrap()),
        bool_py(s.italic.unwrap())
    );

    let s = Style::parse("bold red").unwrap();
    println!(
        "parse(\"bold red\") -> bold={}, color={}",
        bool_py(s.bold.unwrap()),
        format_color(&s)
    );

    let s = Style::parse("bold red on blue").unwrap();
    let bgcolor = match &s.bgcolor {
        Some(c) => color_name(c),
        None => "None".to_string(),
    };
    println!(
        "parse(\"bold red on blue\") -> bold={}, color={}, bgcolor={}",
        bool_py(s.bold.unwrap()),
        format_color(&s),
        bgcolor
    );

    let s = Style::parse("underline strike").unwrap();
    println!(
        "parse(\"underline strike\") -> underline={}, strike={}",
        bool_py(s.underline.unwrap()),
        bool_py(s.strike.unwrap())
    );

    println!("\n=== Style Combination ===");

    let s1 = Style::parse("bold").unwrap();
    let s2 = Style::parse("italic").unwrap();
    let combined = s1 + s2;
    println!(
        "bold + italic -> bold={}, italic={}",
        bool_py(combined.bold.unwrap()),
        bool_py(combined.italic.unwrap())
    );

    let s1 = Style::parse("bold red").unwrap();
    let s2 = Style::parse("blue").unwrap();
    let combined = s1 + s2;
    println!(
        "(bold red) + blue -> bold={}, color={}",
        bool_py(combined.bold.unwrap()),
        format_color(&combined)
    );

    println!("\n=== ANSI Rendering ===");

    let s = Style::parse("bold").unwrap();
    let rendered = s.render("X", ColorSystem::TrueColor);
    println!("Style(bold).render(\"X\") -> {}", escape_ansi(&rendered));

    let s = Style::parse("italic").unwrap();
    let rendered = s.render("X", ColorSystem::TrueColor);
    println!("Style(italic).render(\"X\") -> {}", escape_ansi(&rendered));

    let s = Style::parse("bold italic").unwrap();
    let rendered = s.render("X", ColorSystem::TrueColor);
    println!("Style(bold italic).render(\"X\") -> {}", escape_ansi(&rendered));

    let s = Style::parse("red").unwrap();
    let rendered = s.render("X", ColorSystem::TrueColor);
    println!("Style(red).render(\"X\") -> {}", escape_ansi(&rendered));

    let s = Style::parse("bold red").unwrap();
    let rendered = s.render("X", ColorSystem::TrueColor);
    println!("Style(bold red).render(\"X\") -> {}", escape_ansi(&rendered));

    let s = Style::parse("on blue").unwrap();
    let rendered = s.render("X", ColorSystem::TrueColor);
    println!("Style(on blue).render(\"X\") -> {}", escape_ansi(&rendered));

    println!("\n=== Null Style ===");

    let s = Style::new();
    println!("Style() is null -> {}", bool_py_lower(s.is_null()));
    println!("Style().render(\"X\") -> {}", escape_ansi(&s.render("X", ColorSystem::TrueColor)));

    let s = Style::parse("bold").unwrap();
    println!("Style(bold) is null -> {}", bool_py_lower(s.is_null()));
}
