//! Screenshot Generator for README
//!
//! Run with: `cargo run --example screenshots`
//!
//! Generates SVG screenshots for all README examples using the MONOKAI theme.

use std::io::Stdout;

use rich_rs::r#box::{ROUNDED, SIMPLE};
use rich_rs::markdown::Markdown;
use rich_rs::{
    Column, Columns, Console, ConsoleOptions, JustifyMethod, MONOKAI, Measurement, Panel, Pretty,
    Renderable, Row, Segment, Segments, SimpleColor, Style, Syntax, Table, Text, Tree,
};

const IMG_DIR: &str = "imgs";

fn main() -> std::io::Result<()> {
    std::fs::create_dir_all(IMG_DIR)?;

    println!("Generating screenshots...");

    generate_features()?;
    generate_hello_world()?;
    generate_markup()?;
    generate_table()?;
    generate_syntax()?;
    generate_markdown()?;
    generate_tree()?;
    generate_panel()?;
    generate_pretty()?;
    generate_columns()?;

    println!("Generated all screenshots in {}/", IMG_DIR);
    Ok(())
}

/// Create a recording console with fixed width
fn recording_console(width: usize) -> Console<Stdout> {
    let options = ConsoleOptions {
        size: (width, 50),
        max_width: width,
        min_width: width,
        is_terminal: true,
        color_system: Some(rich_rs::ColorSystem::TrueColor),
        ..Default::default()
    };
    let mut console = Console::with_options(options);
    console.set_record(true);
    console.set_force_terminal(Some(true));
    console
}

/// Save console to SVG with MONOKAI theme
fn save_svg(console: &mut Console<Stdout>, filename: &str, title: &str) -> std::io::Result<()> {
    console.save_svg(
        &format!("{}/{}", IMG_DIR, filename),
        title,
        Some(&MONOKAI),
        true,
        0.61,
        None,
    )
}

// ============================================================================
// ColorBox - A gradient display showing TrueColor support
// ============================================================================

struct ColorBox;

impl ColorBox {
    fn hls_to_rgb(h: f64, l: f64, s: f64) -> (u8, u8, u8) {
        if s == 0.0 {
            let v = (l * 255.0) as u8;
            return (v, v, v);
        }

        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;

        let hue_to_rgb = |p: f64, q: f64, mut t: f64| -> f64 {
            if t < 0.0 {
                t += 1.0;
            }
            if t > 1.0 {
                t -= 1.0;
            }
            if t < 1.0 / 6.0 {
                return p + (q - p) * 6.0 * t;
            }
            if t < 1.0 / 2.0 {
                return q;
            }
            if t < 2.0 / 3.0 {
                return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
            }
            p
        };

        let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
        let g = hue_to_rgb(p, q, h);
        let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

        ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
    }
}

impl Renderable for ColorBox {
    fn render(&self, _console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let mut segments = Segments::new();
        let width = options.max_width;

        for y in 0..5 {
            for x in 0..width {
                let h = x as f64 / width as f64;
                let l1 = 0.1 + (y as f64 / 5.0) * 0.7;
                let l2 = l1 + 0.7 / 10.0;

                let (r1, g1, b1) = Self::hls_to_rgb(h, l1, 1.0);
                let (r2, g2, b2) = Self::hls_to_rgb(h, l2, 1.0);

                let bgcolor = SimpleColor::Rgb {
                    r: r1,
                    g: g1,
                    b: b1,
                };
                let color = SimpleColor::Rgb {
                    r: r2,
                    g: g2,
                    b: b2,
                };

                let style = Style::new().with_color(color).with_bgcolor(bgcolor);

                segments.push(Segment::styled("▄", style));
            }
            segments.push(Segment::line());
        }

        segments
    }

    fn measure(&self, _console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        Measurement::new(1, options.max_width)
    }
}

// ============================================================================
// Screenshots
// ============================================================================

fn generate_features() -> std::io::Result<()> {
    let mut console = recording_console(88);

    // Main grid table
    let mut table = Table::grid()
        .with_padding(1, 1)
        .with_pad_edge(true)
        .with_title("Rich features");

    table.add_column(
        Column::new()
            .no_wrap(true)
            .justify(JustifyMethod::Center)
            .width(12)
            .style(
                Style::new()
                    .with_bold(true)
                    .with_color(SimpleColor::Standard(1)),
            ),
    );
    table.add_column(Column::new());

    // Colors Section
    let mut color_table = Table::new()
        .with_box(None)
        .with_expand(false)
        .with_show_header(false)
        .with_show_edge(false)
        .with_pad_edge(false);

    let color_text = Text::from_markup(
        "✓ [bold green]4-bit color[/]\n\
         ✓ [bold blue]8-bit color[/]\n\
         ✓ [bold magenta]Truecolor (16.7 million)[/]\n\
         ✓ [bold yellow]Dumb terminals[/]\n\
         ✓ [bold cyan]Automatic color conversion",
        true,
    )
    .unwrap();

    let color_cells: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(color_text), Box::new(ColorBox)];
    color_table.add_row(Row::new(color_cells));

    let colors_cells: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("Colors")), Box::new(color_table)];
    table.add_row(Row::new(colors_cells));

    // Blank row
    table.add_row(Row::new(vec![
        Box::new(Text::plain("")) as Box<dyn Renderable + Send + Sync>,
        Box::new(Text::plain("")),
    ]));

    // Styles Section
    let styles_text = Text::from_markup(
        "All ansi styles: [bold]bold[/], [dim]dim[/], [italic]italic[/italic], \
         [underline]underline[/], [strike]strikethrough[/], [reverse]reverse[/], \
         and even [blink]blink[/].",
        true,
    )
    .unwrap();

    table.add_row(Row::new(vec![
        Box::new(Text::plain("Styles")) as Box<dyn Renderable + Send + Sync>,
        Box::new(styles_text),
    ]));

    // Blank row
    table.add_row(Row::new(vec![
        Box::new(Text::plain("")) as Box<dyn Renderable + Send + Sync>,
        Box::new(Text::plain("")),
    ]));

    // Markup Section
    let markup_text = Text::from_markup(
        "[bold magenta]Rich[/] supports a simple [i]bbcode[/i]-like [b]markup[/b] for \
         [yellow]color[/], [underline]style[/], and emoji! \
         :+1: :apple: :ant: :bear: :baguette_bread: :bus: ",
        true,
    )
    .unwrap();

    table.add_row(Row::new(vec![
        Box::new(Text::plain("Markup")) as Box<dyn Renderable + Send + Sync>,
        Box::new(markup_text),
    ]));

    // Blank row
    table.add_row(Row::new(vec![
        Box::new(Text::plain("")) as Box<dyn Renderable + Send + Sync>,
        Box::new(Text::plain("")),
    ]));

    // Tables Section
    let mut movie_table = Table::new()
        .with_show_edge(false)
        .with_show_header(true)
        .with_expand(false)
        .with_box(Some(SIMPLE))
        .with_row_styles(vec![Style::default(), Style::new().with_dim(true)]);

    movie_table.add_column(
        Column::with_header(Box::new(Text::from_markup("[green]Date", false).unwrap()))
            .style(Style::new().with_color(SimpleColor::Standard(2)))
            .no_wrap(true),
    );
    movie_table.add_column(
        Column::with_header(Box::new(Text::from_markup("[blue]Title", false).unwrap()))
            .style(Style::new().with_color(SimpleColor::Standard(4))),
    );
    movie_table.add_column(
        Column::with_header(Box::new(
            Text::from_markup("[magenta]Box Office", false).unwrap(),
        ))
        .style(Style::new().with_color(SimpleColor::Standard(5)))
        .justify(JustifyMethod::Right),
    );

    let movies = vec![
        vec![
            "Dec 20, 2019",
            "Star Wars: The Rise of Skywalker",
            "$375,126,118",
        ],
        vec![
            "May 25, 2018",
            "[b]Solo[/]: A Star Wars Story",
            "$393,151,347",
        ],
        vec![
            "Dec 15, 2017",
            "Star Wars Ep. VIII: The Last Jedi",
            "[bold]$1,332,539,889[/bold]",
        ],
    ];

    for movie in movies {
        let row_cells: Vec<Box<dyn Renderable + Send + Sync>> = movie
            .into_iter()
            .map(|cell| {
                let text = Text::from_markup(cell, false).unwrap_or_else(|_| Text::plain(cell));
                Box::new(text) as Box<dyn Renderable + Send + Sync>
            })
            .collect();
        movie_table.add_row(Row::new(row_cells));
    }

    table.add_row(Row::new(vec![
        Box::new(Text::plain("Tables")) as Box<dyn Renderable + Send + Sync>,
        Box::new(movie_table),
    ]));

    // Blank row
    table.add_row(Row::new(vec![
        Box::new(Text::plain("")) as Box<dyn Renderable + Send + Sync>,
        Box::new(Text::plain("")),
    ]));

    // +more! Section
    let more_text =
        Text::plain("Progress bars, syntax highlighting, markdown, tracebacks, and more...");

    table.add_row(Row::new(vec![
        Box::new(Text::plain("+more!")) as Box<dyn Renderable + Send + Sync>,
        Box::new(more_text),
    ]));

    console
        .print(&table, None, None, None, false, "\n")
        .unwrap();
    save_svg(&mut console, "features.svg", "rich-rs Features")
}

fn generate_hello_world() -> std::io::Result<()> {
    let mut console = recording_console(50);

    let text = Text::from_markup("Hello, [bold magenta]World[/]!", false).unwrap();
    console.print(&text, None, None, None, false, "\n").unwrap();

    save_svg(&mut console, "hello_world.svg", "Hello World")
}

fn generate_markup() -> std::io::Result<()> {
    let mut console = recording_console(60);

    let line1 = Text::from_markup("[bold red]Error:[/] Something went wrong", false).unwrap();
    let line2 = Text::from_markup("[link=https://example.com]Click here[/link]", false).unwrap();
    let line3 = Text::from_markup(":warning: [yellow]Warning[/] :warning:", true).unwrap();

    console
        .print(&line1, None, None, None, false, "\n")
        .unwrap();
    console
        .print(&line2, None, None, None, false, "\n")
        .unwrap();
    console
        .print(&line3, None, None, None, false, "\n")
        .unwrap();

    save_svg(&mut console, "markup.svg", "Markup Examples")
}

fn generate_table() -> std::io::Result<()> {
    let mut console = recording_console(50);

    let mut table = Table::new();
    table.add_column(
        Column::with_header(Box::new(Text::styled(
            "Name",
            Style::new().with_color(SimpleColor::Standard(6)), // cyan
        )))
        .style(Style::new().with_color(SimpleColor::Standard(6))),
    );
    table.add_column(
        Column::with_header(Box::new(Text::styled(
            "Age",
            Style::new().with_color(SimpleColor::Standard(5)), // magenta
        )))
        .style(Style::new().with_color(SimpleColor::Standard(5)))
        .justify(JustifyMethod::Right),
    );
    table.add_column(
        Column::with_header(Box::new(Text::styled(
            "City",
            Style::new().with_color(SimpleColor::Standard(2)), // green
        )))
        .style(Style::new().with_color(SimpleColor::Standard(2))),
    );

    table.add_row_strs(&["Alice", "30", "New York"]);
    table.add_row_strs(&["Bob", "25", "Los Angeles"]);
    table.add_row_strs(&["Charlie", "35", "Chicago"]);

    console
        .print(&table, None, None, None, false, "\n")
        .unwrap();
    save_svg(&mut console, "table.svg", "Table Example")
}

fn generate_syntax() -> std::io::Result<()> {
    let mut console = recording_console(55);

    let code = r#"fn main() {
    let greeting = "Hello, World!";
    println!("{}", greeting);
}
"#;

    let syntax = Syntax::new(code, "rust")
        .with_line_numbers(true)
        .with_theme("base16-ocean.dark");

    console
        .print(&syntax, None, None, None, false, "\n")
        .unwrap();
    save_svg(&mut console, "syntax.svg", "Syntax Highlighting")
}

fn generate_markdown() -> std::io::Result<()> {
    let mut console = recording_console(60);

    let md = r#"# Hello World

This is **bold** and *italic*.

- Item one
- Item two
- Item three

```rust
fn main() {
    println!("Hello!");
}
```
"#;

    let markdown = Markdown::new(md);
    console
        .print(&markdown, None, None, None, false, "\n")
        .unwrap();
    save_svg(&mut console, "markdown.svg", "Markdown Rendering")
}

fn generate_tree() -> std::io::Result<()> {
    let mut console = recording_console(40);

    let mut tree = Tree::new(Box::new(
        Text::from_markup("[bold]:open_file_folder: Project[/]", true).unwrap(),
    ));

    // Add src directory with children
    let src = tree.add(Box::new(
        Text::from_markup("[blue]:open_file_folder: src[/]", true).unwrap(),
    ));
    src.add(Box::new(
        Text::from_markup("[green]:page_facing_up: main.rs[/]", true).unwrap(),
    ));
    src.add(Box::new(
        Text::from_markup("[green]:page_facing_up: lib.rs[/]", true).unwrap(),
    ));

    // Add tests directory with children
    let tests = tree.add(Box::new(
        Text::from_markup("[blue]:open_file_folder: tests[/]", true).unwrap(),
    ));
    tests.add(Box::new(
        Text::from_markup("[green]:page_facing_up: integration_test.rs[/]", true).unwrap(),
    ));

    tree.add(Box::new(
        Text::from_markup("[yellow]:page_facing_up: Cargo.toml[/]", true).unwrap(),
    ));
    tree.add(Box::new(
        Text::from_markup("[cyan]:page_facing_up: README.md[/]", true).unwrap(),
    ));

    console.print(&tree, None, None, None, false, "\n").unwrap();
    save_svg(&mut console, "tree.svg", "Tree View")
}

fn generate_panel() -> std::io::Result<()> {
    let mut console = recording_console(60);

    let content = Text::from_markup(
        "This is a [bold cyan]Panel[/]!\n\n\
         Panels are great for highlighting important content\n\
         with a border and optional title.",
        true,
    )
    .unwrap();

    let panel = Panel::new(Box::new(content))
        .with_title("Information")
        .with_box(ROUNDED)
        .with_border_style(Style::new().with_color(SimpleColor::Standard(4))); // blue

    console
        .print(&panel, None, None, None, false, "\n")
        .unwrap();
    save_svg(&mut console, "panel.svg", "Panel Example")
}

fn generate_pretty() -> std::io::Result<()> {
    let mut console = recording_console(60);

    let data = r#"{
        "name": "rich-rs",
        "version": "1.0.0",
        "features": ["tables", "syntax", "markdown"],
        "authors": [
            {"name": "Alice", "role": "maintainer"},
            {"name": "Bob", "role": "contributor"}
        ]
    }"#;

    let pretty = Pretty::from_str(data).with_indent_guides(true);
    console
        .print(&pretty, None, None, None, false, "\n")
        .unwrap();
    save_svg(&mut console, "pretty.svg", "Pretty Printing")
}

fn generate_columns() -> std::io::Result<()> {
    let mut console = recording_console(50);

    let items: Vec<Box<dyn Renderable + Send + Sync>> = (1..=9)
        .map(|i| {
            let color = SimpleColor::Standard((i % 7 + 1) as u8);
            Box::new(Text::styled(
                &format!("Item {}", i),
                Style::new().with_color(color),
            )) as Box<dyn Renderable + Send + Sync>
        })
        .collect();

    let columns = Columns::new(items).with_equal(true).with_width(15);
    console
        .print(&columns, None, None, None, false, "\n")
        .unwrap();
    save_svg(&mut console, "columns.svg", "Columns Layout")
}
