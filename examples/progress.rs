//! Progress example (Phase 5.1)
//!
//! Run with:
//!   cargo run --example progress

use std::thread::sleep;
use std::time::Duration;

use rich_rs::{LiveOptions, Progress, TrackConfig};

fn main() -> std::io::Result<()> {
    let live_options = LiveOptions {
        refresh_per_second: 10.0,
        ..Default::default()
    };

    // Mirrors Python Rich's default `Progress` settings (expand = false).
    let mut progress = Progress::new_default(live_options, false, false, false);
    progress.start()?;

    let config = TrackConfig {
        total: None,
        completed: 0.0,
        task_id: None,
        description: "Working...".to_string(),
        update_period: Duration::from_millis(100),
    };

    for _ in progress.track_sequence(0..100, config) {
        sleep(Duration::from_millis(25));
    }

    progress.stop()?;
    Ok(())
}
