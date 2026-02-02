use rich_rs::{ColorSystem, Column, Console, ConsoleOptions, JustifyMethod, Row, Table, Text};

#[test]
fn fixed_feature_column_width_is_respected() {
    let mut console = Console::capture_with_options(ConsoleOptions {
        is_terminal: true,
        color_system: Some(ColorSystem::TrueColor),
        max_width: 80,
        ..Default::default()
    });
    console.set_size(80, 10);

    let mut table = Table::grid().with_padding(1, 1).with_pad_edge(true);
    table.add_column(Column::new().no_wrap(true).justify(JustifyMethod::Center).width(12));
    table.add_column(Column::new());
    table.add_row(Row::new(vec![
        Box::new(Text::plain("Colors")),
        Box::new(Text::plain("X")),
    ]));

    let _ = console.print(&table, None, None, None, false, "");
    let out = console.get_captured();

    // Expect: 1 left pad + 3 center pad = 4 spaces, then "Colors", then 3 center + 1 right pad = 4.
    // So "X" begins at index 14.
    let plain = strip_ansi(&out);
    let line = plain.lines().find(|l| l.contains("Colors")).expect("missing row");
    assert_eq!(
        line.find('X'),
        Some(14),
        "expected second column to start at 14, got {line:?}"
    );
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            // consume CSI sequence
            let _ = chars.next();
            while let Some(c) = chars.next() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

