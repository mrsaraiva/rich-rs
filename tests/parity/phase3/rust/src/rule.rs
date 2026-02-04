//! Rule module parity tests.

use rich_rs::{AlignMethod, Console, ConsoleOptions, Rule};

fn render_rule(rule: &Rule, width: usize) -> String {
    let mut console = Console::capture_with_options(ConsoleOptions {
        max_width: width,
        is_terminal: true,
        color_system: None,
        ..Default::default()
    });
    let _ = console.print(rule, None, None, None, false, "");
    let mut out = console.get_captured();
    while out.ends_with('\n') {
        out.pop();
    }
    out
}

pub fn run() {
    println!("=== Rule without title ===");

    let result = render_rule(&Rule::new(), 40);
    println!("Rule(width=40) -> \"{}\"", result);

    let result = render_rule(&Rule::new(), 20);
    println!("Rule(width=20) -> \"{}\"", result);

    println!("\n=== Rule with centered title ===");

    let result = render_rule(&Rule::new().with_title("Title"), 40);
    println!("Rule(\"Title\", width=40) -> \"{}\"", result);

    let result = render_rule(&Rule::new().with_title("Hello"), 30);
    println!("Rule(\"Hello\", width=30) -> \"{}\"", result);

    println!("\n=== Rule with left-aligned title ===");

    let result = render_rule(
        &Rule::new()
            .with_title("Left")
            .with_align(AlignMethod::Left),
        30,
    );
    println!("Rule(\"Left\", align=left, width=30) -> \"{}\"", result);

    println!("\n=== Rule with right-aligned title ===");

    let result = render_rule(
        &Rule::new()
            .with_title("Right")
            .with_align(AlignMethod::Right),
        30,
    );
    println!("Rule(\"Right\", align=right, width=30) -> \"{}\"", result);

    println!("\n=== Rule with custom characters ===");

    let result = render_rule(&Rule::new().with_characters("="), 20);
    println!("Rule(characters=\"=\", width=20) -> \"{}\"", result);

    let result = render_rule(&Rule::new().with_title("Test").with_characters("*"), 20);
    println!("Rule(\"Test\", characters=\"*\", width=20) -> \"{}\"", result);

    let result = render_rule(
        &Rule::new().with_title("Multi").with_characters("+-"),
        30,
    );
    println!("Rule(\"Multi\", characters=\"+-\", width=30) -> \"{}\"", result);

    println!("\n=== Rule with narrow width ===");

    let result = render_rule(&Rule::new().with_title("Very Long Title"), 15);
    println!("Rule(\"Very Long Title\", width=15) -> \"{}\"", result);

    let result = render_rule(&Rule::new().with_title("X"), 10);
    println!("Rule(\"X\", width=10) -> \"{}\"", result);

    println!("\n=== AlignMethod parsing ===");

    println!(
        "AlignMethod::parse(\"left\") -> {:?}",
        AlignMethod::parse("left")
    );
    println!(
        "AlignMethod::parse(\"center\") -> {:?}",
        AlignMethod::parse("center")
    );
    println!(
        "AlignMethod::parse(\"right\") -> {:?}",
        AlignMethod::parse("right")
    );
    println!(
        "AlignMethod::parse(\"invalid\") -> {:?}",
        AlignMethod::parse("invalid")
    );
}
