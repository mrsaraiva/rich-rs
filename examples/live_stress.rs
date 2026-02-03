//! Live stress test example.
//!
//! Exercises:
//! - Printing regular output while a live region is active (should not corrupt live output).
//! - A nested live entry added/removed mid-stream.
//! - Frequent redraws.
//!
//! Run with:
//!   FORCE_COLOR=1 COLORTERM=truecolor cargo run --example live_stress

use std::thread::sleep;
use std::time::Duration;

use rich_rs::{Console, Control, Text, VerticalOverflowMethod};

fn main() -> std::io::Result<()> {
    let mut console = Console::new();

    if !console.is_terminal() || console.is_dumb_terminal() {
        console.print_text("live_stress: not a supported interactive terminal; printing once.")?;
        console.print(&Text::plain("LIVE (final)\n"), None, None, None, false, "")?;
        return Ok(());
    }

    let _ = console.show_cursor(false)?;

    let (root_id, _is_root) = console.live_start(
        Box::new(Text::from_markup("[bold green]Live[/] stress test", false).unwrap()),
        VerticalOverflowMethod::Ellipsis,
    );

    // Establish initial shape so subsequent prints will reposition.
    console.print(&Control::new(), None, None, None, false, "")?;

    let mut nested_id: Option<usize> = None;

    for tick in 0..80u32 {
        // Print regular output periodically (this should render above the live region).
        if tick % 7 == 0 {
            console.print_text(&format!("log line {tick}"))?;
        }

        // Update the root live renderable.
        let root_markup =
            format!("[bold green]Live[/] tick={tick}  [dim]printing while live is active[/]");
        let root = Text::from_markup(&root_markup, false)
            .unwrap_or_else(|_| Text::plain(format!("Live tick={tick}")));
        console.live_update(root_id, Box::new(root));

        // Add a nested entry, update it for a while, then remove it.
        if tick == 15 {
            let (id, _is_root) = console.live_start(
                Box::new(Text::from_markup("[bold cyan]Nested[/] started", false).unwrap()),
                VerticalOverflowMethod::Ellipsis,
            );
            nested_id = Some(id);
        }

        if let Some(id) = nested_id {
            let nested_markup =
                format!("[bold cyan]Nested[/] tick={tick}  [dim](will stop at tick=55)[/]");
            let nested = Text::from_markup(&nested_markup, false)
                .unwrap_or_else(|_| Text::plain(format!("Nested tick={tick}")));
            console.live_update(id, Box::new(nested));
        }

        if tick == 55 {
            if let Some(id) = nested_id.take() {
                // Remove nested entry. Print its final renderable as regular output to
                // match Rich's nested Live stop behavior when not transient.
                if let Some(renderable) = console.live_stop(id) {
                    console.print(renderable.as_ref(), None, None, None, false, "\n")?;
                }
            }
        }

        // Force a redraw.
        console.print(&Control::new(), None, None, None, false, "")?;
        sleep(Duration::from_millis(30));
    }

    // Final refresh with overflow visible, then clear the live stack.
    console.live_set_vertical_overflow(root_id, VerticalOverflowMethod::Visible);
    console.print(&Control::new(), None, None, None, false, "")?;
    console.live_clear();

    console.line(1)?;
    let _ = console.show_cursor(true)?;
    Ok(())
}
