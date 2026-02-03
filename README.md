# rich-rs

Rich text and beautiful formatting for the terminal — a Rust port of Python's [Rich](https://github.com/Textualize/rich) library.

![Demo](https://raw.githubusercontent.com/Textualize/rich/master/imgs/features.png)

## Features

**rich-rs** provides everything you need for beautiful terminal output:

- **Colors & Styles** — 16, 256, and TrueColor support with automatic terminal detection
- **Text** — Word wrapping, justification, markup parsing (`[bold red]Hello[/]`)
- **Tables** — Full-featured tables with borders, alignment, and styling
- **Syntax Highlighting** — Code highlighting via syntect with multiple themes
- **Markdown** — CommonMark + GFM rendering with syntax-highlighted code blocks
- **Progress Bars** — Multi-task progress display with spinners and ETA
- **Live Display** — Real-time updating content with cursor management
- **Trees** — Hierarchical data with guide lines
- **Panels** — Bordered boxes with titles
- **Tracebacks** — Beautiful panic backtraces with source context

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
rich-rs = "1.0"
```

### Basic Usage

```rust
use rich_rs::{Console, Text, Style};

fn main() {
    let mut console = Console::new();

    // Simple styled text
    console.print_styled("Hello, World!", Style::parse("bold green").unwrap()).unwrap();

    // Rich markup
    let text = Text::from_markup("[bold blue]Hello[/] [italic]World[/]!", false).unwrap();
    console.print(&text, None, None, None, false, "\n").unwrap();
}
```

### Tables

```rust
use rich_rs::{Console, Table};

fn main() {
    let mut console = Console::new();

    let mut table = Table::new();
    table.add_column_with_header("Name");
    table.add_column_with_header("Age");
    table.add_row(vec!["Alice", "30"]);
    table.add_row(vec!["Bob", "25"]);

    console.print(&table, None, None, None, false, "\n").unwrap();
}
```

### Progress Bars

```rust
use rich_rs::progress::{Progress, ProgressIteratorExt};

fn main() {
    // Simple iterator progress
    for item in (0..100).progress() {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Or with custom configuration
    let progress = Progress::new();
    // ... add tasks, update progress
}
```

### Syntax Highlighting

```rust
use rich_rs::{Console, Syntax};

fn main() {
    let mut console = Console::new();

    let code = r#"
fn main() {
    println!("Hello, World!");
}
"#;

    let syntax = Syntax::new(code, "rust")
        .with_line_numbers(true)
        .with_theme("base16-ocean.dark");

    console.print(&syntax, None, None, None, false, "\n").unwrap();
}
```

### Live Display

```rust
use rich_rs::{Console, Live, Text};
use std::time::Duration;

fn main() {
    let console = Console::new();
    let mut live = Live::new(console);

    live.start().unwrap();
    for i in 0..10 {
        live.update(Text::plain(&format!("Count: {}", i)));
        std::thread::sleep(Duration::from_millis(500));
    }
    live.stop().unwrap();
}
```

## Demo

See all features in action:

```bash
cargo run --example demo
```

## Feature Parity with Python Rich

rich-rs provides complete feature parity with Python Rich's core rendering capabilities:

| Category | Status |
|----------|--------|
| Colors (16/256/TrueColor) | ✅ Complete |
| Styles & Markup | ✅ Complete |
| Text (wrapping, justify) | ✅ Complete |
| Tables | ✅ Complete |
| Panels | ✅ Complete |
| Trees | ✅ Complete |
| Syntax Highlighting | ✅ Complete |
| Markdown | ✅ Complete |
| Pretty Printing | ✅ Complete |
| Tracebacks | ✅ Complete |
| Progress Bars | ✅ Complete |
| Live Display | ✅ Complete |
| Spinners | ✅ Complete |
| Emoji | ✅ Complete |

### Not Included (Python-specific or niche)

- `logging.py` — Python logging integration (use `tracing` crate for Rust)
- `jupyter.py` — Jupyter notebook support
- `prompt.py` — Interactive prompts (see `dialoguer` crate)
- `pager.py` — Less-style paging

## Minimum Supported Rust Version

Rust 2024 edition (1.85+)

## License

MIT License — see [LICENSE](LICENSE) for details.

## Acknowledgments

- [Textualize](https://github.com/Textualize) for creating the original Python Rich library
- [syntect](https://github.com/trishume/syntect) for syntax highlighting
- [crossterm](https://github.com/crossterm-rs/crossterm) for cross-platform terminal support
