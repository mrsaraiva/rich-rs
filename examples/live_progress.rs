//! Live Progress example - Multiple Progress bars in a single Live display.
//!
//! This is a Rust port of Python Rich's `examples/live_progress.py`.
//!
//! Demonstrates the use of multiple Progress instances in a single Live display
//! by wrapping them in Panels inside a Table grid.
//!
//! Run with: `cargo run --example live_progress`

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::sleep;
use std::time::Duration;

use rich_rs::{
    BarColumn, Console, ConsoleOptions, Live, LiveOptions, Panel, Progress, ProgressColumn,
    Renderable, Segments, SpinnerColumn, Style, Table, TextColumn,
};

fn main() -> std::io::Result<()> {
    // Create shared state for tracking job completion (since Progress state is internal)
    let job1_completed = Arc::new(AtomicU64::new(0));
    let job2_completed = Arc::new(AtomicU64::new(0));
    let job3_completed = Arc::new(AtomicU64::new(0));

    // Job totals
    let job1_total: f64 = 100.0;
    let job2_total: f64 = 200.0;
    let job3_total: f64 = 400.0;
    let overall_total: f64 = job1_total + job2_total + job3_total;

    // Create the job progress with custom columns:
    // {task.description}, Spinner, Bar, Percentage
    let job_columns: Vec<Box<dyn ProgressColumn>> = vec![
        Box::new(TextColumn::new("{task.description}")),
        Box::new(SpinnerColumn::new()),
        Box::new(BarColumn::new()),
        Box::new(TextColumn::new(
            "[progress.percentage]{task.percentage:>3.0f}%",
        )),
    ];

    let job_live_options = LiveOptions {
        auto_refresh: false,
        ..Default::default()
    };

    let job_progress = Progress::new(job_columns, job_live_options, false, false);

    // Add job tasks
    let job1 = job_progress.add_task("[green]Cooking", true, Some(job1_total), 0.0, true);
    let job2 = job_progress.add_task("[magenta]Baking", true, Some(job2_total), 0.0, true);
    let job3 = job_progress.add_task("[cyan]Mixing", true, Some(job3_total), 0.0, true);

    // Create overall progress with default columns
    let overall_live_options = LiveOptions {
        auto_refresh: false,
        ..Default::default()
    };

    let overall_progress = Progress::new_default(overall_live_options, false, false, false);
    let overall_task = overall_progress.add_task("All Jobs", true, Some(overall_total), 0.0, true);

    // Create the combined renderable that displays both progress bars in panels
    let combined = CombinedProgress {
        job_progress: Arc::new(job_progress),
        overall_progress: Arc::new(overall_progress),
    };

    // Create the live display
    let live_options = LiveOptions {
        auto_refresh: true,
        refresh_per_second: 10.0,
        ..Default::default()
    };

    let mut live = Live::with_options(Box::new(combined.clone()), live_options);
    live.start(true)?;

    // Run until all jobs are finished
    loop {
        sleep(Duration::from_millis(100));

        // Advance each unfinished job
        let c1 = job1_completed.load(Ordering::SeqCst) as f64;
        let c2 = job2_completed.load(Ordering::SeqCst) as f64;
        let c3 = job3_completed.load(Ordering::SeqCst) as f64;

        if c1 < job1_total {
            job1_completed.fetch_add(1, Ordering::SeqCst);
            combined.job_progress.advance(job1, 1.0);
        }
        if c2 < job2_total {
            job2_completed.fetch_add(1, Ordering::SeqCst);
            combined.job_progress.advance(job2, 1.0);
        }
        if c3 < job3_total {
            job3_completed.fetch_add(1, Ordering::SeqCst);
            combined.job_progress.advance(job3, 1.0);
        }

        // Calculate overall completed
        let total_completed = job1_completed.load(Ordering::SeqCst) as f64
            + job2_completed.load(Ordering::SeqCst) as f64
            + job3_completed.load(Ordering::SeqCst) as f64;

        // Update overall progress
        combined.overall_progress.update(
            overall_task,
            None,
            Some(total_completed),
            None,
            None,
            None,
            false,
            None,
        );

        // Check if overall is finished
        if total_completed >= overall_total {
            break;
        }
    }

    live.stop()?;

    println!("All jobs completed!");
    Ok(())
}

/// A renderable that combines two progress displays in a table grid (side by side)
#[derive(Clone)]
struct CombinedProgress {
    job_progress: Arc<Progress>,
    overall_progress: Arc<Progress>,
}

impl Renderable for CombinedProgress {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        // Create a grid table with two columns
        let mut table = Table::grid().with_expand(true);
        table.add_column(rich_rs::Column::new());
        table.add_column(rich_rs::Column::new());

        // Create panels for overall and job progress
        // Note: We wrap Arc<Progress> in a newtype to implement Renderable
        let overall_panel = Panel::fit(Box::new(ProgressRef(self.overall_progress.clone())))
            .with_title("[bold]Overall Progress")
            .with_border_style(Style::parse("green").unwrap_or_default())
            .with_padding((2, 2));

        let jobs_panel = Panel::fit(Box::new(ProgressRef(self.job_progress.clone())))
            .with_title("[bold]Jobs")
            .with_border_style(Style::parse("red").unwrap_or_default())
            .with_padding((1, 2));

        // Add both panels as a row
        let cells: Vec<Box<dyn Renderable + Send + Sync>> =
            vec![Box::new(overall_panel), Box::new(jobs_panel)];
        table.add_row(rich_rs::Row::new(cells));

        table.render(console, options)
    }
}

/// Wrapper to make Arc<Progress> implement Renderable
#[derive(Clone)]
struct ProgressRef(Arc<Progress>);

impl Renderable for ProgressRef {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        self.0.render(console, options)
    }

    fn measure(&self, console: &Console, options: &ConsoleOptions) -> rich_rs::Measurement {
        self.0.measure(console, options)
    }
}
