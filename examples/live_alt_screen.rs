//! Live alt-screen example (Phase 5.2 prerequisite / Live parity).
//!
//! Run with:
//!   FORCE_COLOR=1 COLORTERM=truecolor cargo run --example live_alt_screen

use std::thread::sleep;
use std::time::Duration;

use rich_rs::{Live, LiveOptions, Text};

fn main() -> std::io::Result<()> {
    let options = LiveOptions {
        screen: true,
        refresh_per_second: 20.0,
        ..Default::default()
    };

    let mut live = Live::with_options(Box::new(Text::plain("starting...")), options);
    live.start(true)?;

    for i in 0..120u32 {
        let markup = format!("[bold magenta]Alt-screen[/] tick={i}");
        let t =
            Text::from_markup(&markup, false).unwrap_or_else(|_| Text::plain(format!("Alt-screen tick={i}")));
        live.update(Box::new(t), true)?;
        sleep(Duration::from_millis(25));
    }

    live.stop()?;
    Ok(())
}
