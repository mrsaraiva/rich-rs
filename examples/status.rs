//! Status example - Show a spinner with status text while working on tasks.
//!
//! This is a port of Python Rich's status example from `rich/status.py`.
//!
//! Run with: `cargo run --example status`

use std::thread::sleep;
use std::time::Duration;

use rich_rs::Console;
use rich_rs::status::Status;

fn main() -> std::io::Result<()> {
    let mut console = Console::new();
    console.print_text("")?; // blank line

    let tasks: Vec<String> = (1..=10).map(|n| format!("task {}", n)).collect();

    let mut status = Status::new("[bold green]Working on tasks...");
    status.start()?;

    for task in tasks {
        sleep(Duration::from_secs(1));
        // Note: Python Rich uses console.log() which we don't have yet.
        // We use println! instead, which works well because Status uses transient=true
        // so the spinner line is cleared before our println output.
        println!("{} complete", task);
    }

    status.stop()?;

    Ok(())
}
