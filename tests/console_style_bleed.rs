use rich_rs::{ColorSystem, Column, Console, ConsoleOptions, Row, SimpleColor, Style, Table, Text};

#[test]
fn background_does_not_bleed_between_table_cells() {
    let mut console = Console::capture_with_options(ConsoleOptions {
        is_terminal: true,
        color_system: Some(ColorSystem::TrueColor),
        ..Default::default()
    });
    console.set_size(20, 5);

    let bg = Style::new().with_bgcolor(SimpleColor::Rgb { r: 1, g: 2, b: 3 });

    let mut table = Table::grid().with_padding(0, 0).with_pad_edge(false);
    table.add_column(Column::new().width(5));
    table.add_column(Column::new().width(5));
    table.add_row(Row::new(vec![
        Box::new(Text::styled("A", bg)),
        Box::new(Text::plain("B")),
    ]));

    let _ = console.print(&table, None, None, None, false, "");
    let out = console.get_captured();

    let a = out.find('A').expect("expected 'A' in output");
    let b = out.find('B').expect("expected 'B' in output");
    assert!(a < b, "expected 'A' to appear before 'B'");

    let between = &out[a..b];
    assert!(
        between.contains("\x1b[49")
            || between.contains("\x1b[0m")
            || between.contains("\x1b[39")
            || between.contains("\x1b[22"),
        "expected an ANSI reset between cells, got: {between:?}"
    );
}

#[test]
fn dim_does_not_bleed_into_plain_text() {
    let mut console = Console::capture_with_options(ConsoleOptions {
        is_terminal: true,
        color_system: Some(ColorSystem::TrueColor),
        ..Default::default()
    });
    console.set_size(20, 5);

    let dim = Style::new().with_dim(true);
    let mut text = Text::new();
    text.append("X", Some(dim));
    text.append("Y", None);

    let _ = console.print(&text, None, None, None, false, "");
    let out = console.get_captured();

    let x = out.find('X').expect("expected 'X' in output");
    let y = out.find('Y').expect("expected 'Y' in output");
    assert!(x < y, "expected 'X' to appear before 'Y'");

    let between = &out[x..y];
    assert!(
        between.contains("\x1b[22")
            || between.contains("\x1b[0m")
            || between.contains("\x1b[39")
            || between.contains("\x1b[49"),
        "expected an ANSI reset between dim and plain spans, got: {between:?}"
    );
}
