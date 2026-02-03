//! Rich-rs Demo
//!
//! Run with: `cargo run --example demo`
//!
//! This demonstrates the major features of rich-rs, mirroring the Python Rich demo.

use std::io::Stdout;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

use rich_rs::r#box::SIMPLE;
use rich_rs::markdown::Markdown;
use rich_rs::{
    Column, Console, ConsoleOptions, ControlType, JustifyMethod, Measurement, Panel, Pretty,
    Renderable, Row, Segment, Segments, SimpleColor, Style, Syntax, Table, Text,
    VerticalAlignMethod,
};

// ============================================================================
// ColorBox - A gradient display showing TrueColor support
// ============================================================================

/// A renderable that displays a colorful gradient.
struct ColorBox;

impl ColorBox {
    /// Convert HLS (Hue, Lightness, Saturation) to RGB.
    /// H is in [0, 1], L is in [0, 1], S is in [0, 1].
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
// OSC 8 Hyperlinks (used in the footer panel)
// ============================================================================

#[derive(Debug, Clone)]
struct Hyperlink {
    id: Arc<str>,
    url: Arc<str>,
    text: Arc<str>,
    style: Option<Style>,
}

impl Hyperlink {
    fn next_id() -> Arc<str> {
        static NEXT: once_cell::sync::Lazy<AtomicU32> = once_cell::sync::Lazy::new(|| {
            let seed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_micros();
            AtomicU32::new(seed % 1_000_000)
        });

        let id = NEXT.fetch_add(1, Ordering::Relaxed) % 1_000_000;
        Arc::<str>::from(id.to_string())
    }

    fn new(url: impl Into<Arc<str>>, text: impl Into<Arc<str>>, style: Option<Style>) -> Self {
        Self {
            id: Self::next_id(),
            url: url.into(),
            text: text.into(),
            style,
        }
    }
}

impl Renderable for Hyperlink {
    fn render(&self, console: &Console<Stdout>, _options: &ConsoleOptions) -> Segments {
        if !console.is_terminal() || console.is_dumb_terminal() {
            if let Some(style) = self.style {
                return Segments::from(Segment::styled(self.text.to_string(), style));
            }
            return Segments::from(Segment::new(self.text.to_string()));
        }

        let mut segments = Segments::new();
        segments.push(Segment::control(ControlType::HyperlinkStart {
            url: self.url.clone(),
            id: Some(self.id.clone()),
        }));
        if let Some(style) = self.style {
            segments.push(Segment::styled(self.text.to_string(), style));
        } else {
            segments.push(Segment::new(self.text.to_string()));
        }
        segments.push(Segment::control(ControlType::HyperlinkEnd));
        segments
    }
}

#[derive(Debug, Clone)]
struct ThanksIntro;

impl Renderable for ThanksIntro {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let mut segments = Segments::new();

        segments.push(Segment::new("We hope you enjoy using Rich!"));
        segments.push(Segment::line());
        segments.push(Segment::line());

        segments.push(Segment::new("Rich is maintained with "));
        let heart = Text::from_markup("[red]:heart:[/]", true).unwrap();
        segments.extend(heart.render(console, options));
        segments.push(Segment::new(" by "));

        let textualize = Hyperlink::new("https://www.textualize.io", "Textualize.io", None);
        segments.extend(textualize.render(console, options));

        segments.push(Segment::line());
        segments.push(Segment::line());
        segments.push(Segment::new("- Will McGugan"));

        segments
    }

    fn measure(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        // Default `Measurement::from_segments` assumes single-line, so compute a
        // multi-line measurement by splitting and taking the union.
        let lines = Segment::split_lines(self.render(console, options).into_iter());

        let mut measurement = Measurement::new(0, 0);
        for line in lines {
            let mut segs = Segments::new();
            segs.extend(line);
            measurement = measurement.union(&Measurement::from_segments(&segs));
        }

        measurement
    }
}

// ============================================================================
// Demo Test Card
// ============================================================================

fn make_test_card() -> Table {
    // Main grid table
    let mut table = Table::grid()
        .with_padding(1, 1)
        .with_pad_edge(true)
        .with_title("Rich features");

    // Add columns: Feature name and Demonstration
    table.add_column(
        Column::new()
            .no_wrap(true)
            .justify(JustifyMethod::Center)
            .vertical(VerticalAlignMethod::Top)
            // Match Python Rich demo: feature labels are centered in a 12-cell content column
            // (plus 1 cell padding on each side from the table).
            .width(12)
            .style(
                Style::new()
                    .with_bold(true)
                    .with_color(SimpleColor::Standard(1)),
            ), // bold red
    );
    table.add_column(Column::new());

    // ─────────────────────────────────────────────────────────────────────────
    // Colors Section
    // ─────────────────────────────────────────────────────────────────────────
    let mut color_table = Table::new()
        .with_box(None)
        .with_expand(false)
        .with_show_header(false)
        .with_show_edge(false)
        .with_pad_edge(false);

    // Note: No explicit columns - let add_row auto-create them like Python Rich does.
    // This ensures the ColorBox gets the same width as in Python.

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

    // Add blank row for spacing (like Python Rich demo)
    let blank_row: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("")), Box::new(Text::plain(""))];
    table.add_row(Row::new(blank_row));

    // ─────────────────────────────────────────────────────────────────────────
    // Styles Section
    // ─────────────────────────────────────────────────────────────────────────
    let styles_text = Text::from_markup(
        "All ansi styles: [bold]bold[/], [dim]dim[/], [italic]italic[/italic], \
         [underline]underline[/], [strike]strikethrough[/], [reverse]reverse[/], \
         and even [blink]blink[/].",
        true,
    )
    .unwrap();

    let styles_cells: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("Styles")), Box::new(styles_text)];
    table.add_row(Row::new(styles_cells));

    // Blank row
    let blank_row: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("")), Box::new(Text::plain(""))];
    table.add_row(Row::new(blank_row));

    // ─────────────────────────────────────────────────────────────────────────
    // Text Section (Word Wrap with Justification)
    // ─────────────────────────────────────────────────────────────────────────
    let lorem = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                 Quisque in metus sed sapien ultricies pretium a at justo. \
                 Maecenas luctus velit et auctor maximus.";

    let mut lorem_table = Table::grid().with_padding(1, 1).with_pad_edge(false);

    lorem_table.add_column(Column::new().ratio(1).justify(JustifyMethod::Left));
    lorem_table.add_column(Column::new().ratio(1).justify(JustifyMethod::Center));
    lorem_table.add_column(Column::new().ratio(1).justify(JustifyMethod::Right));
    lorem_table.add_column(Column::new().ratio(1).justify(JustifyMethod::Full));

    let lorem_cells: Vec<Box<dyn Renderable + Send + Sync>> = vec![
        Box::new(Text::styled(
            lorem,
            Style::new().with_color(SimpleColor::Standard(2)),
        )), // green
        Box::new(Text::styled(
            lorem,
            Style::new().with_color(SimpleColor::Standard(3)),
        )), // yellow
        Box::new(Text::styled(
            lorem,
            Style::new().with_color(SimpleColor::Standard(4)),
        )), // blue
        Box::new(Text::styled(
            lorem,
            Style::new().with_color(SimpleColor::Standard(1)),
        )), // red
    ];
    lorem_table.add_row(Row::new(lorem_cells));

    let text_intro = Text::from_markup(
        "Word wrap text. Justify [green]left[/], [yellow]center[/], [blue]right[/] or [red]full[/].\n",
        true,
    )
    .unwrap();

    // Create a combined text section
    let mut text_section = Table::grid().with_padding(0, 0);
    text_section.add_column(Column::new());
    let text_section_cells: Vec<Box<dyn Renderable + Send + Sync>> = vec![Box::new(text_intro)];
    text_section.add_row(Row::new(text_section_cells));
    // Match Python Rich demo spacing: one blank row after the intro line.
    text_section.add_row(Row::new(vec![Box::new(Text::plain(""))]));
    let text_section_cells2: Vec<Box<dyn Renderable + Send + Sync>> = vec![Box::new(lorem_table)];
    text_section.add_row(Row::new(text_section_cells2));

    let text_cells: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("Text")), Box::new(text_section)];
    table.add_row(Row::new(text_cells));

    // Blank row
    let blank_row: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("")), Box::new(Text::plain(""))];
    table.add_row(Row::new(blank_row));

    // ─────────────────────────────────────────────────────────────────────────
    // Asian Language Support
    // ─────────────────────────────────────────────────────────────────────────
    let asian_text = Text::from_markup(
        ":flag_for_china:  该库支持中文，日文和韩文文本！\n\
         :flag_for_japan:  ライブラリは中国語、日本語、韓国語のテキストをサポートしています\n\
         :flag_for_south_korea:  이 라이브러리는 중국어, 일본어 및 한국어 텍스트를 지원합니다",
        true,
    )
    .unwrap();

    let asian_cells: Vec<Box<dyn Renderable + Send + Sync>> = vec![
        Box::new(Text::plain("Asian\nlanguage\nsupport")),
        Box::new(asian_text),
    ];
    table.add_row(Row::new(asian_cells));

    // Blank row
    let blank_row: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("")), Box::new(Text::plain(""))];
    table.add_row(Row::new(blank_row));

    // ─────────────────────────────────────────────────────────────────────────
    // Markup Section
    // ─────────────────────────────────────────────────────────────────────────
    let markup_text = Text::from_markup(
        "[bold magenta]Rich[/] supports a simple [i]bbcode[/i]-like [b]markup[/b] for \
         [yellow]color[/], [underline]style[/], and emoji! \
         :+1: :apple: :ant: :bear: :baguette_bread: :bus: ",
        true,
    )
    .unwrap();

    let markup_cells: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("Markup")), Box::new(markup_text)];
    table.add_row(Row::new(markup_cells));

    // Blank row
    let blank_row: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("")), Box::new(Text::plain(""))];
    table.add_row(Row::new(blank_row));

    // ─────────────────────────────────────────────────────────────────────────
    // Tables Section
    // ─────────────────────────────────────────────────────────────────────────
    let mut movie_table = Table::new()
        .with_show_edge(false)
        .with_show_header(true)
        .with_expand(false)
        .with_box(Some(SIMPLE))
        .with_row_styles(vec![Style::default(), Style::new().with_dim(true)]);

    movie_table.add_column(
        Column::with_header(Box::new(Text::from_markup("[green]Date", false).unwrap()))
            .style(Style::new().with_color(SimpleColor::Standard(2)))
            .no_wrap(true)
            .min_width(12),
    );
    movie_table.add_column(
        Column::with_header(Box::new(Text::from_markup("[blue]Title", false).unwrap()))
            .style(Style::new().with_color(SimpleColor::Standard(4)))
            .ratio(1),
    );
    movie_table.add_column(
        Column::with_header(Box::new(
            Text::from_markup("[cyan]Production Budget", false).unwrap(),
        ))
        .style(Style::new().with_color(SimpleColor::Standard(6)))
        .justify(JustifyMethod::Right)
        .no_wrap(true)
        .min_width(17),
    );
    movie_table.add_column(
        Column::with_header(Box::new(
            Text::from_markup("[magenta]Box Office", false).unwrap(),
        ))
        .style(Style::new().with_color(SimpleColor::Standard(5)))
        .justify(JustifyMethod::Right)
        .no_wrap(true)
        .min_width(14),
    );

    // Movie data rows
    let movies: Vec<Vec<&str>> = vec![
        vec![
            "Dec 20, 2019",
            "Star Wars: The Rise of Skywalker",
            "$275,000,000",
            "$375,126,118",
        ],
        vec![
            "May 25, 2018",
            "[b]Solo[/]: A Star Wars Story",
            "$275,000,000",
            "$393,151,347",
        ],
        vec![
            "Dec 15, 2017",
            "Star Wars Ep. VIII: The Last Jedi",
            "$262,000,000",
            "[bold]$1,332,539,889[/bold]",
        ],
        vec![
            "May 19, 1999",
            "Star Wars Ep. [b]I[/b]: [i]The Phantom Menace",
            "$115,000,000",
            "$1,027,044,677",
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

    let tables_cells: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("Tables")), Box::new(movie_table)];
    table.add_row(Row::new(tables_cells));

    // Blank row
    let blank_row: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("")), Box::new(Text::plain(""))];
    table.add_row(Row::new(blank_row));

    // ─────────────────────────────────────────────────────────────────────────
    // Syntax Highlighting & Pretty Printing
    // ─────────────────────────────────────────────────────────────────────────
    let code = r#"def iter_last(values: Iterable[T]) -> Iterable[Tuple[bool, T]]:
    """Iterate and generate a tuple with a flag for last value."""
    iter_values = iter(values)
    try:
        previous_value = next(iter_values)
    except StopIteration:
        return
    for value in iter_values:
        yield False, previous_value
        previous_value = value
    yield True, previous_value"#;

    let syntax = Syntax::new(code, "python3")
        .with_line_numbers(true)
        .with_indent_guides(true);

    let pretty_data = "{'foo': [3.1427, ('Paul Atreides', 'Vladimir Harkonnen', 'Thufir Hawat')], 'atomic': (False, True, None)}";
    let pretty = Pretty::from_str(pretty_data).with_indent_guides(true);

    // Side-by-side comparison table with padding between columns
    let mut comparison_table = Table::new()
        .with_show_header(false)
        .with_pad_edge(false)
        .with_box(None)
        .with_padding(1, 1) // Add horizontal padding between columns
        .with_expand(true);

    comparison_table.add_column(Column::new().ratio(1));
    comparison_table.add_column(Column::new().ratio(1));

    let syntax_pretty_cells: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(syntax), Box::new(pretty)];
    comparison_table.add_row(Row::new(syntax_pretty_cells));

    let syntax_cells: Vec<Box<dyn Renderable + Send + Sync>> = vec![
        Box::new(Text::plain("Syntax\nhighlighting\n&\npretty\nprinting")),
        Box::new(comparison_table),
    ];
    table.add_row(Row::new(syntax_cells));

    // Blank row
    let blank_row: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("")), Box::new(Text::plain(""))];
    table.add_row(Row::new(blank_row));

    // ─────────────────────────────────────────────────────────────────────────
    // Markdown Section
    // ─────────────────────────────────────────────────────────────────────────
    let markdown_source = r#"# Markdown

Supports much of the *markdown* __syntax__!

- Headers
- Basic formatting: **bold**, *italic*, `code`
- Block quotes
- Lists, and more...
"#;

    let markdown_raw = Text::from_markup(&format!("[cyan]{}", markdown_source), false)
        .unwrap_or_else(|_| Text::plain(markdown_source));

    let markdown_rendered = Markdown::new(markdown_source);

    // Side-by-side comparison
    let mut md_comparison = Table::new()
        .with_show_header(false)
        .with_pad_edge(false)
        .with_box(None)
        .with_expand(true);

    md_comparison.add_column(Column::new().ratio(1));
    md_comparison.add_column(Column::new().ratio(1));

    let md_cells: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(markdown_raw), Box::new(markdown_rendered)];
    md_comparison.add_row(Row::new(md_cells));

    let markdown_cells: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("Markdown")), Box::new(md_comparison)];
    table.add_row(Row::new(markdown_cells));

    // Blank row
    let blank_row: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("")), Box::new(Text::plain(""))];
    table.add_row(Row::new(blank_row));

    // ─────────────────────────────────────────────────────────────────────────
    // +more! Section
    // ─────────────────────────────────────────────────────────────────────────
    let more_text =
        Text::plain("Progress bars, columns, styled logging handler, tracebacks, etc...");

    let more_cells: Vec<Box<dyn Renderable + Send + Sync>> =
        vec![Box::new(Text::plain("+more!")), Box::new(more_text)];
    table.add_row(Row::new(more_cells));

    table
}

// ============================================================================
// Main Entry Point
// ============================================================================

fn main() {
    // Build the test card
    let test_card = make_test_card();

    // Time two renders off-screen, similar to the upstream Python demo.
    let options = ConsoleOptions::from_terminal();
    let mut measure_console = Console::<Vec<u8>>::capture_with_options(ConsoleOptions {
        is_terminal: true,
        ..options.clone()
    });
    measure_console.set_force_terminal(Some(true));

    let start = Instant::now();
    let _ = measure_console.print(&test_card, None, None, None, false, "");
    let cold_time = start.elapsed();

    measure_console.clear_captured();
    let start = Instant::now();
    let _ = measure_console.print(&test_card, None, None, None, false, "");
    let warm_time = start.elapsed();

    // Print to stdout once
    let mut console = Console::new();
    let _ = console.print(&test_card, None, None, None, false, "");

    // Print timing info
    let _ = console.line(1);
    let timing_cold = Text::from_markup(
        &format!(
            "[dim]rendered in [not dim]{:.1}ms[/] (cold cache)",
            cold_time.as_secs_f64() * 1000.0
        ),
        false,
    )
    .unwrap();
    let timing_warm = Text::from_markup(
        &format!(
            "[dim]rendered in [not dim]{:.1}ms[/] (warm cache)",
            warm_time.as_secs_f64() * 1000.0
        ),
        false,
    )
    .unwrap();

    let _ = console.print(&timing_cold, None, None, None, false, "\n");
    let _ = console.print(&timing_warm, None, None, None, false, "\n");

    // "Thanks" panel (mirrors Python Rich demo output).
    let mut sponsor_message = Table::grid().with_padding(1, 1);
    sponsor_message.add_column(
        Column::new()
            .style(Style::new().with_color(SimpleColor::Standard(2))) // green
            .justify(JustifyMethod::Right),
    );
    sponsor_message.add_column(Column::new().no_wrap(true));

    let underline_blue = Style::parse("underline blue").unwrap_or_else(Style::new);
    let textualize_link = Hyperlink::new(
        "https://github.com/textualize",
        "https://github.com/textualize",
        Some(underline_blue),
    );
    let twitter_link = Hyperlink::new(
        "https://twitter.com/willmcgugan",
        "https://twitter.com/willmcgugan",
        Some(underline_blue),
    );

    sponsor_message.add_row(Row::new(vec![
        Box::new(Text::plain("Textualize")) as Box<dyn Renderable + Send + Sync>,
        Box::new(textualize_link),
    ]));
    sponsor_message.add_row_strs(&["", ""]);
    sponsor_message.add_row(Row::new(vec![
        Box::new(Text::plain("Twitter")) as Box<dyn Renderable + Send + Sync>,
        Box::new(twitter_link),
    ]));

    let intro_message = ThanksIntro;

    let mut message = Table::grid().with_padding(2, 2);
    message.add_column(Column::new());
    message.add_column(Column::new().no_wrap(true));
    message.add_row(Row::new(vec![
        Box::new(intro_message) as Box<dyn Renderable + Send + Sync>,
        Box::new(sponsor_message),
    ]));

    let title = Text::from_markup("[b red]Thanks for trying out Rich!", false).unwrap();

    let panel = Panel::fit(Box::new(message))
        .with_box(rich_rs::r#box::ROUNDED)
        .with_padding((1, 2))
        .with_title_text(title)
        .with_border_style(Style::parse("bright_blue").unwrap_or_else(Style::new));

    let centered = rich_rs::Align::center(Box::new(panel));
    let _ = console.print(&centered, None, None, None, false, "\n");
}
