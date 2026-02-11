//! Console encoding example
//!
//! Run with: `cargo run --example console_encoding`
//!
//! Demonstrates reading and updating console encoding with
//! `encoding()` / `set_encoding()`, and how it affects render fallback.

use rich_rs::{Console, ProgressBar};

fn main() -> std::io::Result<()> {
    let mut console = Console::capture();

    // ProgressBar uses box-drawing glyphs for UTF encodings and ASCII fallback otherwise.
    let mut bar = ProgressBar::new();
    bar.total = Some(100.0);
    bar.completed = 42.0;
    bar.width = Some(16);

    println!("default encoding: {}", console.encoding());
    console.print(&bar, None, None, None, false, "\n")?;
    let utf_bar = console.get_captured();
    console.clear_captured();

    console.set_encoding("ascii");
    println!("updated encoding: {}", console.encoding());
    console.print(&bar, None, None, None, false, "\n")?;
    let ascii_bar = console.get_captured();

    println!("utf render : {:?}", utf_bar.trim_end());
    println!("ascii render: {:?}", ascii_bar.trim_end());

    Ok(())
}
