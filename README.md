# rich-rs

[![Crates.io](https://img.shields.io/crates/v/rich-rs.svg)](https://crates.io/crates/rich-rs)
[![Documentation](https://docs.rs/rich-rs/badge.svg)](https://docs.rs/rich-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Rich text and beautiful formatting for the terminal — a Rust port of Python's [Rich](https://github.com/Textualize/rich) library.

The rich-rs API makes it easy to add color and style to terminal output. Rich can also render pretty tables, progress bars, markdown, syntax highlighted source code, tracebacks, and more — out of the box.

![Features](https://github.com/textualize/rich/raw/master/imgs/features.png)

## Compatibility

rich-rs works on Linux, macOS, and Windows. True color / emoji works with modern terminals; legacy Windows console is limited to 16 colors.

**Minimum Supported Rust Version:** 1.85+ (Rust 2024 edition)

## Installing

Add to your `Cargo.toml`:

```toml
[dependencies]
rich-rs = "1.0"
```

Run the demo to see rich-rs in action:

```sh
cargo run --example demo
```

## Console Printing

To add rich output to your application, create a `Console` and use its print methods:

```rust
use rich_rs::{Console, Text};

let mut console = Console::new();

// Simple text
console.print_text("Hello, World!").unwrap();

// Styled text with markup
let text = Text::from_markup("[bold blue]Hello[/] [italic]World[/]!", false).unwrap();
console.print(&text, None, None, None, false, "\n").unwrap();
```

Rich will automatically word-wrap text to fit the terminal width and detect color support.

## Markup

Rich uses a BBCode-like markup syntax for inline styling:

```rust
console.print_markup("[bold red]Error:[/] Something went wrong").unwrap();
console.print_markup("[link=https://example.com]Click here[/link]").unwrap();
console.print_markup(":warning: [yellow]Warning[/] :warning:").unwrap();  // Emoji support
```

# Rich Library

Rich contains a number of builtin *renderables* you can use to create elegant output in your CLI.

Click the headings below for details:

<details>
<summary>Tables</summary>

Rich can render flexible tables with unicode box characters, borders, styles, and cell alignment.

```rust
use rich_rs::{Console, Table};

let mut console = Console::new();

let mut table = Table::new();
table.add_column_with_header("Name");
table.add_column_with_header("Age");
table.add_row(vec!["Alice", "30"]);
table.add_row(vec!["Bob", "25"]);

console.print(&table, None, None, None, false, "\n").unwrap();
```

Tables automatically resize columns to fit the terminal width, wrapping text as needed.

</details>

<details>
<summary>Progress Bars</summary>

Rich can render multiple flicker-free progress bars to track long-running tasks.

```rust
use rich_rs::progress::{Progress, ProgressIteratorExt};

// Simple iterator progress
for item in (0..100).progress() {
    std::thread::sleep(std::time::Duration::from_millis(50));
}
```

For more control, create a `Progress` instance with custom columns:

```rust
use rich_rs::progress::Progress;

let progress = Progress::new();
let task = progress.add_task("Downloading...", Some(100));
// ... update task progress
```

Built-in columns include percentage, file size, transfer speed, time elapsed, and time remaining.

</details>

<details>
<summary>Live Display</summary>

Rich can update content in-place for real-time displays.

```rust
use rich_rs::{Console, Live, Text};
use std::time::Duration;

let console = Console::new();
let mut live = Live::new(console);

live.start().unwrap();
for i in 0..10 {
    live.update(Text::plain(&format!("Count: {}", i)));
    std::thread::sleep(Duration::from_millis(500));
}
live.stop().unwrap();
```

Live display supports transient mode (clears on exit), alt-screen mode, and vertical overflow handling.

</details>

<details>
<summary>Syntax Highlighting</summary>

Rich uses [syntect](https://github.com/trishume/syntect) to implement syntax highlighting with multiple themes.

```rust
use rich_rs::{Console, Syntax};

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
```

Available themes include `base16-ocean.dark`, `Solarized (dark)`, `InspiredGitHub`, and more.

</details>

<details>
<summary>Markdown</summary>

Rich can render markdown with syntax-highlighted code blocks.

```rust
use rich_rs::{Console, markdown::Markdown};

let mut console = Console::new();

let md = "# Hello World\n\nThis is **bold** and *italic*.\n\n```rust\nfn main() {}\n```";
let markdown = Markdown::new(md);

console.print(&markdown, None, None, None, false, "\n").unwrap();
```

Supports CommonMark + GitHub Flavored Markdown including tables, task lists, and fenced code blocks.

</details>

<details>
<summary>Trees</summary>

Rich can render hierarchical data with guide lines.

```rust
use rich_rs::{Console, Tree};

let mut console = Console::new();

let mut tree = Tree::new("Root");
tree.add("Child 1");
let mut child2 = tree.add_tree("Child 2");
child2.add("Grandchild");

console.print(&tree, None, None, None, false, "\n").unwrap();
```

</details>

<details>
<summary>Panels</summary>

Rich can render content in bordered boxes with titles.

```rust
use rich_rs::{Console, Panel, Text};

let mut console = Console::new();

let panel = Panel::new(Text::plain("Hello, World!"))
    .with_title("Greeting");

console.print(&panel, None, None, None, false, "\n").unwrap();
```

</details>

<details>
<summary>Tracebacks</summary>

Rich can render beautiful panic backtraces with syntax-highlighted source context.

```rust
use rich_rs::traceback;

// Install as the default panic handler
traceback::install();

// Now panics will show beautiful tracebacks
```

Tracebacks show the call stack with source code snippets and local variable values.

</details>

<details>
<summary>Pretty Printing</summary>

Rich can pretty-print Rust data structures with syntax highlighting.

```rust
use rich_rs::{Console, Pretty};

let mut console = Console::new();

let data = vec![1, 2, 3, 4, 5];
let pretty = Pretty::new(&data);

console.print(&pretty, None, None, None, false, "\n").unwrap();
```

</details>

<details>
<summary>Prompts</summary>

Rich provides interactive prompts with validation and choices.

```rust
use rich_rs::prompt::{Prompt, Confirm, IntPrompt};

// Text prompt with choices
let color = Prompt::new("Favorite color?")
    .with_choices(&["red", "green", "blue"])
    .run()?;

// Numeric input
let age: i64 = IntPrompt::ask("How old are you?")?;

// Yes/No confirmation
if Confirm::ask("Continue?")? {
    println!("Proceeding...");
}
```

Supports password input, default values, and custom validation.

</details>

<details>
<summary>Columns</summary>

Rich can render content in neat columns with equal or optimal width.

```rust
use rich_rs::{Console, Columns, Text};

let mut console = Console::new();

let items: Vec<Text> = (1..=12)
    .map(|i| Text::plain(&format!("Item {}", i)))
    .collect();

let columns = Columns::new(items);
console.print(&columns, None, None, None, false, "\n").unwrap();
```

</details>

## Acknowledgments

- [Textualize](https://github.com/Textualize) for creating the original Python Rich library
- [syntect](https://github.com/trishume/syntect) for syntax highlighting
- [crossterm](https://github.com/crossterm-rs/crossterm) for cross-platform terminal support
- [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) for Markdown parsing

## License

MIT License — see [LICENSE](LICENSE) for details.
