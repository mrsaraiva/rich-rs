use rich_rs::{
    ColorSystem, Column, Console, ConsoleOptions, JustifyMethod, Row, Table, Text,
    VerticalAlignMethod,
};

#[test]
fn multiline_centered_text_does_not_shift_right_per_line() {
    let mut console = Console::capture_with_options(ConsoleOptions {
        is_terminal: true,
        color_system: Some(ColorSystem::TrueColor),
        max_width: 80,
        ..Default::default()
    });
    console.set_size(80, 10);

    let mut table = Table::grid().with_padding(1, 1).with_pad_edge(true);
    table.add_column(
        Column::new()
            .no_wrap(true)
            .justify(JustifyMethod::Center)
            .vertical(VerticalAlignMethod::Top)
            .width(12),
    );
    table.add_column(Column::new());

    table.add_row(Row::new(vec![
        Box::new(Text::plain("Asian\nlanguage\nsupport")),
        Box::new(Text::plain("X")),
    ]));

    let _ = console.print(&table, None, None, None, false, "");
    let plain = strip_ansi(&console.get_captured());

    let asian = plain
        .lines()
        .find(|l| l.contains("Asian"))
        .expect("missing Asian line");
    let language = plain
        .lines()
        .find(|l| l.contains("language"))
        .expect("missing language line");
    let support = plain
        .lines()
        .find(|l| l.contains("support"))
        .expect("missing support line");

    let a = asian.find('A').expect("missing 'A'");
    let l = language.find('l').expect("missing 'l'");
    let s = support.find('s').expect("missing 's'");

    // In a centered 12-cell content column (plus 1 left padding), "Asian" (len 5)
    // starts 1 cell to the right of "language"/"support" (len 8/7).
    assert_eq!(
        a.saturating_sub(l),
        1,
        "expected 'Asian' to start 1 cell right of 'language', got:\n{asian:?}\n{language:?}"
    );
    assert_eq!(
        l, s,
        "expected 'language' and 'support' to start at same column, got:\n{language:?}\n{support:?}"
    );
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
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
