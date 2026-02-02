use rich_rs::{Column, Console, Renderable, Row, Segment, SimpleColor, Style, Table, Text};

#[test]
fn table_vertical_padding_inherits_cell_background() {
    let mut console = Console::new();
    console.set_size(3, 10);

    let bg = Style::new().with_bgcolor(SimpleColor::Rgb { r: 1, g: 2, b: 3 });

    let mut table = Table::grid().with_padding(0, 0).with_pad_edge(false);
    table.add_column(Column::new().width(1));
    table.add_column(Column::new().width(2));

    // First cell is 2 lines tall; second cell is 1 line tall with a background style.
    table.add_row(Row::new(vec![
        Box::new(Text::plain("a\nb")),
        Box::new(Text::styled("x", bg)),
    ]));

    let rendered = table.render(&console, console.options());
    let lines = Segment::split_lines(rendered);
    assert_eq!(lines.len(), 2);

    // Line 1: second column should have a trailing padding space with background style.
    let has_bg_trailing_space = lines[0].iter().any(|seg| {
        seg.text.as_ref() == " "
            && seg.style
                .is_some_and(|s| s.bgcolor == Some(SimpleColor::Rgb { r: 1, g: 2, b: 3 }))
    });
    assert!(has_bg_trailing_space);

    // Line 2: second column should be a blank line padded to column width with background style.
    let has_bg_blank_line = lines[1].iter().any(|seg| {
        seg.text.as_ref() == "  "
            && seg.style
                .is_some_and(|s| s.bgcolor == Some(SimpleColor::Rgb { r: 1, g: 2, b: 3 }))
    });
    assert!(has_bg_blank_line);
}
