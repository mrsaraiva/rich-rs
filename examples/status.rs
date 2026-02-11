//! Status example
//!
//! Run with: `cargo run --example status`
//!
//! Demonstrates:
//! - `Console::status()` for a transient spinner
//! - `Status::update()` while work is in progress
//! - `rich_rs::log!` for timestamped progress messages

use std::thread::sleep;
use std::time::Duration;

use rich_rs::{Console, Text};

fn main() -> std::io::Result<()> {
    let mut console = Console::new();

    let tasks: Vec<String> = (1..=10).map(|n| format!("task {}", n)).collect();
    let mut status = console.status("[bold green]Working on tasks...", None, None, None, None);
    status.start()?;

    for (index, task) in tasks.iter().enumerate() {
        status.update(&format!("[bold green]Working on {}...", task))?;
        sleep(Duration::from_secs(1));
        rich_rs::log!(
            console,
            &Text::plain(format!("{} complete ({}/{})", task, index + 1, tasks.len()))
        )?;
    }

    status.stop()?;
    rich_rs::log!(console, &Text::plain("All tasks complete"))?;

    Ok(())
}
