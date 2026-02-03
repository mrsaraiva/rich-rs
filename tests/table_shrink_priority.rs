use rich_rs::{
    ColorSystem, Column, Console, ConsoleOptions, JustifyMethod, Measurement, Renderable, Row,
    Table, Text,
};
use std::io::Stdout;

struct MeasuredCell {
    width: usize,
}

impl Renderable for MeasuredCell {
    fn render(&self, _console: &Console<Stdout>, _options: &ConsoleOptions) -> rich_rs::Segments {
        // Content doesn't matter for the test; the measurement does.
        Text::plain("X").render(_console, _options)
    }

    fn measure(&self, _console: &Console<Stdout>, _options: &ConsoleOptions) -> Measurement {
        Measurement::new(self.width, self.width)
    }
}

#[test]
fn fixed_feature_column_width_survives_last_resort_shrink() {
    let mut console = Console::capture_with_options(ConsoleOptions {
        is_terminal: true,
        color_system: Some(ColorSystem::TrueColor),
        max_width: 40,
        ..Default::default()
    });
    console.set_size(40, 5);

    // Force a scenario where the table's measured minimum widths exceed max_width,
    // which triggers the "last resort" shrink path.
    let mut table = Table::grid().with_padding(1, 1).with_pad_edge(true);
    table.add_column(
        Column::new()
            .no_wrap(true)
            .justify(JustifyMethod::Center)
            .width(12),
    );
    table.add_column(Column::new());
    table.add_row(Row::new(vec![
        Box::new(Text::plain("Colors")),
        Box::new(MeasuredCell { width: 100 }),
    ]));

    let _ = console.print(&table, None, None, None, false, "");
    let out = console.get_captured();
    let plain = strip_ansi(&out);
    let line = plain
        .lines()
        .find(|l| l.contains("Colors"))
        .unwrap_or_else(|| panic!("missing row in output: {plain:?}"));

    // The fixed feature column should retain its 12-cell content width (plus 1 left pad),
    // so the second column begins at index 14 even when the table is forced to shrink.
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
