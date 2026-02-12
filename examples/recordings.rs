//! Animated Recording Generator for README
//!
//! Run with: `cargo run --example recordings`
//!
//! Generates animated SVGs (and asciicast files for GIF conversion) for README
//! sections that need motion capture. This is the animated counterpart to
//! `screenshots.rs`.
//!
//! Each recording reproduces the exact code shown in the README so the visual
//! output matches what the user sees when running the example.

use std::time::{Duration, Instant};

use rich_rs::{FrameRecorder, LiveOptions, MONOKAI, Progress, Text};

const IMG_DIR: &str = "imgs";

fn main() -> std::io::Result<()> {
    std::fs::create_dir_all(IMG_DIR)?;

    println!("Generating recordings...");

    record_progress()?;
    record_live_display()?;

    println!("Generated all recordings in {}/", IMG_DIR);
    Ok(())
}

// ============================================================================
// Progress Bars (README section)
// ============================================================================

/// Records the Progress Bars example from the README.
///
/// README code:
/// ```ignore
/// let mut progress = Progress::new_default(LiveOptions::default(), false, false, false);
/// progress.start().unwrap();
/// let config = TrackConfig {
///     total: None, completed: 0.0, task_id: None,
///     description: "Working...".to_string(),
///     update_period: Duration::from_millis(100),
/// };
/// for _ in progress.track_sequence(0..100, config) {
///     sleep(Duration::from_millis(25));
/// }
/// progress.stop().unwrap();
/// ```
///
/// We reproduce the same visual output by using `Progress::new_default()` for
/// identical columns, advancing the task at the same rate (25ms per step) with
/// real sleeps so `TimeRemainingColumn` shows accurate estimates.
fn record_progress() -> std::io::Result<()> {
    let width = 80;
    let height = 2;
    let mut recorder = FrameRecorder::new(width, height);

    // Same as README: Progress::new_default(LiveOptions::default(), false, false, false)
    // Columns: TextColumn(description), BarColumn, TaskProgressColumn, TimeRemainingColumn
    let progress = Progress::new_default(LiveOptions::default(), false, false, false);

    // track_sequence(0..100) infers total=100; description from TrackConfig
    let task = progress.add_task("Working...", true, Some(100.0), 0.0, true);

    let start = Instant::now();

    // 100 iterations × 25ms sleep = ~2.5s, matching the README code.
    // Capture every 4 iterations (~100ms) to match the README's update_period.
    for i in 0..=100 {
        progress.update(task, None, Some(i as f64), None, None, None, false, None);
        std::thread::sleep(Duration::from_millis(25));

        if i % 4 == 0 || i == 100 {
            let t = start.elapsed().as_secs_f64();
            recorder.capture(t, &progress);
        }
    }

    let duration = start.elapsed().as_secs_f64();
    println!(
        "  progress: {} frames, {:.1}s duration",
        recorder.frame_count(),
        duration,
    );

    save_recording(&recorder, "progress", "Progress Bars")?;
    Ok(())
}

// ============================================================================
// Live Display (README section)
// ============================================================================

/// Records the Live Display example from the README.
///
/// README code:
/// ```ignore
/// let mut live = Live::new(Box::new(Text::plain("Count: 0")));
/// live.start(true).unwrap();
/// for i in 0..10 {
///     live.update(Box::new(Text::plain(&format!("Count: {}", i))), true).unwrap();
///     std::thread::sleep(Duration::from_millis(500));
/// }
/// live.stop().unwrap();
/// ```
///
/// Each frame is `Text::plain("Count: X")` at 500ms intervals for X in 0..10.
fn record_live_display() -> std::io::Result<()> {
    let width = 40;
    let height = 2;
    let mut recorder = FrameRecorder::new(width, height);

    // The live display shows "Count: X" for X in 0..10, each lasting 500ms
    for i in 0..10 {
        let t = i as f64 * 0.5;
        let text = Text::plain(&format!("Count: {}", i));
        recorder.capture(t, &text);
    }

    let duration = 10.0 * 0.5;
    println!(
        "  live_display: {} frames, {:.1}s duration",
        recorder.frame_count(),
        duration,
    );

    save_recording(&recorder, "live_display", "Live Display")?;
    Ok(())
}

// ============================================================================
// Helpers
// ============================================================================

/// Save both SVG and asciicast for a recording.
fn save_recording(recorder: &FrameRecorder, name: &str, title: &str) -> std::io::Result<()> {
    recorder.save_animated_svg(
        &format!("{}/{}.svg", IMG_DIR, name),
        title,
        Some(&MONOKAI),
        0.61,
    )?;
    recorder.save_asciicast(&format!("{}/{}.cast", IMG_DIR, name), Some(&MONOKAI))?;
    Ok(())
}
