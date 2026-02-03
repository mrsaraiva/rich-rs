use std::env;
use rich_rs::{
    Console, ConsoleOptions, LiveOptions, Progress, TaskID, Text,
};
use rich_rs::{Segment, Segments, Style};
use rich_rs::Renderable;

fn hex_utf8(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.as_bytes() {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn color_repr(color: Option<rich_rs::SimpleColor>) -> String {
    match color {
        None => "-".to_string(),
        Some(rich_rs::SimpleColor::Default) => "-".to_string(),
        Some(rich_rs::SimpleColor::Standard(n)) => format!("n{n}"),
        Some(rich_rs::SimpleColor::EightBit(n)) => format!("n{n}"),
        Some(rich_rs::SimpleColor::Rgb { r, g, b }) => format!("rgb({r},{g},{b})"),
    }
}

fn style_repr(style: Option<Style>) -> String {
    let Some(style) = style else {
        return "-".to_string();
    };
    let fg = color_repr(style.color);
    let bg = color_repr(style.bgcolor);
    let mut flags: Vec<&str> = Vec::new();
    if style.bold == Some(true) {
        flags.push("bold");
    }
    if style.dim == Some(true) {
        flags.push("dim");
    }
    if style.italic == Some(true) {
        flags.push("italic");
    }
    if style.underline == Some(true) {
        flags.push("underline");
    }
    if style.blink == Some(true) {
        flags.push("blink");
    }
    if style.reverse == Some(true) {
        flags.push("reverse");
    }
    if style.strike == Some(true) {
        flags.push("strike");
    }
    let attrs = if flags.is_empty() {
        "-".to_string()
    } else {
        flags.join(",")
    };
    format!("fg={fg};bg={bg};attrs={attrs}")
}

fn dump_text(label: &str, text: &Text) {
    println!("CASE|{label}");
    println!("TEXT|{}", hex_utf8(text.plain_text()));
    println!("BASE|{}", style_repr(text.base_style()));
    for span in text.spans() {
        println!(
            "SPAN|{}|{}|{}",
            span.start,
            span.end,
            style_repr(Some(span.style))
        );
    }
}

fn dump_segments(label: &str, segments: Segments) {
    println!("CASE|{label}");
    let simplified = Segment::simplify(segments);
    println!("COUNT|{}", simplified.len());
    for seg in simplified.iter() {
        if let Some(control) = seg.control {
            println!("CTL|{control:?}");
            continue;
        }
        println!(
            "SEG|{}|{}",
            hex_utf8(seg.text.as_ref()),
            style_repr(seg.style)
        );
    }
}

fn run_ansi() {
    dump_text("bold_then_reset", &Text::from_ansi("\x1b[1mBold\x1b[0m Normal"));
    dump_text("truecolor_fg", &Text::from_ansi("\x1b[38;2;255;0;0mRed\x1b[0m"));
    dump_text("persist_across_lines", &Text::from_ansi("\x1b[31mred\nstill"));
    dump_text("carriage_return", &Text::from_ansi("abc\rdef"));
}

fn run_progress() {
    let live_options = LiveOptions {
        auto_refresh: false,
        refresh_per_second: 10.0,
        ..Default::default()
    };

    // disable=true: ensure no background refresh / terminal behavior is required for rendering.
    let progress = Progress::new_default(live_options, true, false, false);

    let _t1: TaskID = progress.add_task("Download", true, Some(100.0), 25.0, true);
    let _t2: TaskID = progress.add_task("Process", true, Some(100.0), 90.0, true);

    let options = ConsoleOptions {
        size: (80, 24),
        max_width: 80,
        max_height: 24,
        is_terminal: false,
        color_system: None,
        ..Default::default()
    };

    let temp_console = Console::with_options(options.clone());
    let segments = progress.render(&temp_console, &options);
    dump_segments("progress_default_columns", segments);
}

fn main() {
    let module = env::args().nth(1).unwrap_or_else(|| "all".to_string());
    match module.as_str() {
        "ansi" => run_ansi(),
        "progress" => run_progress(),
        "all" => {
            run_ansi();
            run_progress();
        }
        other => {
            eprintln!("Unknown module: {other}");
            std::process::exit(2);
        }
    }
}
