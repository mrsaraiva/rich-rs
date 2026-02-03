//! Jobs example - Manual refresh progress with multiple tasks.
//!
//! This is a Rust port of Python Rich's `examples/jobs.py`.
//!
//! Demonstrates a progress display with:
//! - A master task tracking overall progress across all jobs
//! - A jobs task that is reset for each individual job
//! - Manual refresh (auto_refresh=false) for precise control
//!
//! Run with: `cargo run --example jobs`

use std::thread::sleep;
use std::time::Duration;

use rich_rs::{Console, LiveOptions, Panel, Progress, Text, TrackConfig};

fn main() -> std::io::Result<()> {
    // Job sizes (amount of work in each job)
    let jobs: Vec<u64> = vec![100, 150, 25, 70, 110, 90];
    let total_work: u64 = jobs.iter().sum();

    // Create progress with manual refresh
    let live_options = LiveOptions {
        auto_refresh: false,
        refresh_per_second: 10.0,
        ..Default::default()
    };

    let mut progress = Progress::new_default(live_options, false, false, false);
    progress.start()?;

    // Add master task to track overall progress
    let master_task = progress.add_task("overall", true, Some(total_work as f64), 0.0, true);

    // Add jobs task (will be reset for each job)
    let jobs_task = progress.add_task("jobs", false, None, 0.0, true);

    // Print a header panel
    let mut console = Console::new();
    console.print(
        &Panel::new(Box::new(
            Text::from_markup(
                "[bold blue]A demonstration of progress with a current task and overall progress.",
                false,
            )
            .unwrap_or_else(|_| {
                Text::plain("A demonstration of progress with a current task and overall progress.")
            }),
        ))
        .with_padding(1),
        None,
        None,
        None,
        false,
        "\n",
    )?;

    // Process each job
    for (job_no, &job_size) in jobs.iter().enumerate() {
        // Log job start
        let _ = rich_rs::log!(
            console,
            &Text::from_markup(&format!("Starting job [bold]#{}[/]", job_no), false,)
                .unwrap_or_else(|_| Text::plain(format!("Starting job #{}", job_no)))
        );

        sleep(Duration::from_millis(200));

        // Reset the jobs task for this specific job
        progress.reset(
            jobs_task,
            true,
            Some(Some(job_size as f64)),
            0.0,
            None,
            Some(format!("job [bold yellow]#{}", job_no)),
            None,
        );
        progress.start_task(jobs_task);

        // Process this job using track
        let config = TrackConfig::new(format!("job #{}", job_no))
            .with_task_id(jobs_task)
            .with_total(Some(job_size as f64))
            .with_update_period(Duration::from_millis(50));

        for _ in progress.track_sequence(0..job_size, config) {
            sleep(Duration::from_millis(10));
        }

        // Advance master task by job size
        progress.advance(master_task, job_size as f64);

        // Log job completion
        let _ = rich_rs::log!(
            console,
            &Text::from_markup(&format!("Job [bold]#{}[/] is complete", job_no), false,)
                .unwrap_or_else(|_| Text::plain(format!("Job #{} is complete", job_no)))
        );
    }

    // Final message
    let _ = rich_rs::log!(
        console,
        &Panel::new(Box::new(
            Text::from_markup("[bold green]All done![/]", false,)
                .unwrap_or_else(|_| Text::plain("All done!"))
        ))
        .with_border_style(rich_rs::Style::parse("green").unwrap_or_default())
        .with_padding(1)
    );

    progress.stop()?;

    Ok(())
}
