//! Dynamic Progress example - Dynamic group of progress bars for multi-level progress.
//!
//! This is a Rust port of Python Rich's `examples/dynamic_progress.py`.
//!
//! Demonstrates how to create a dynamic group of progress bars showing multi-level
//! progress for multiple tasks (installing apps), each consisting of multiple steps.
//!
//! Run with: `cargo run --example dynamic_progress`

use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use rich_rs::{
    BarColumn, Console, ConsoleOptions, Group, Live, LiveOptions, Panel, Progress, ProgressColumn,
    Renderable, Segments, SpinnerColumn, TextColumn, TimeElapsedColumn,
};

// Step actions
const STEP_ACTIONS: [&str; 4] = ["downloading", "configuring", "building", "installing"];

// Apps to install: (name, step_times)
// step_times is how long each step takes (in 0.5s units)
const APPS: [(&str, [u64; 4]); 3] = [
    ("one", [2, 1, 4, 2]),
    ("two", [1, 3, 8, 4]),
    ("three", [2, 1, 3, 2]),
];

fn main() -> std::io::Result<()> {
    // Create progress bars
    // 1. Current app progress - shows elapsed time, will stay visible when app is installed
    let current_app_columns: Vec<Box<dyn ProgressColumn>> = vec![
        Box::new(TimeElapsedColumn::new()),
        Box::new(TextColumn::new("{task.description}")),
    ];
    let current_app_progress = Arc::new(Progress::new(
        current_app_columns,
        LiveOptions {
            auto_refresh: false,
            ..Default::default()
        },
        false,
        false,
    ));

    // 2. Step progress - shows elapsed time + action + spinner (will be hidden when step is done)
    let step_columns: Vec<Box<dyn ProgressColumn>> = vec![
        Box::new(TextColumn::new("  ")),
        Box::new(TimeElapsedColumn::new()),
        Box::new(TextColumn::new("[bold purple]{task.description}")),
        Box::new(SpinnerColumn::with_spinner("simpleDots")),
    ];
    let step_progress = Arc::new(Progress::new(
        step_columns,
        LiveOptions {
            auto_refresh: false,
            ..Default::default()
        },
        false,
        false,
    ));

    // 3. App steps progress - shows progress through steps of current app
    let app_steps_columns: Vec<Box<dyn ProgressColumn>> = vec![
        Box::new(TextColumn::new(
            "[bold blue]Progress for app {task.description}: {task.percentage:.0f}%",
        )),
        Box::new(BarColumn::new()),
        Box::new(TextColumn::new(
            "({task.completed:.0f} of {task.total:.0f} steps done)",
        )),
    ];
    let app_steps_progress = Arc::new(Progress::new(
        app_steps_columns,
        LiveOptions {
            auto_refresh: false,
            ..Default::default()
        },
        false,
        false,
    ));

    // 4. Overall progress bar
    let overall_columns: Vec<Box<dyn ProgressColumn>> = vec![
        Box::new(TimeElapsedColumn::new()),
        Box::new(BarColumn::new()),
        Box::new(TextColumn::new("{task.description}")),
    ];
    let overall_progress = Arc::new(Progress::new(
        overall_columns,
        LiveOptions {
            auto_refresh: false,
            ..Default::default()
        },
        false,
        false,
    ));

    // Create overall progress task
    let overall_task_id = overall_progress.add_task("", true, Some(APPS.len() as f64), 0.0, true);

    // Create the dynamic progress group renderable
    let progress_group = DynamicProgressGroup {
        current_app_progress: current_app_progress.clone(),
        step_progress: step_progress.clone(),
        app_steps_progress: app_steps_progress.clone(),
        overall_progress: overall_progress.clone(),
    };

    // Create live display
    let live_options = LiveOptions {
        auto_refresh: true,
        refresh_per_second: 10.0,
        ..Default::default()
    };

    let mut live = Live::with_options(Box::new(progress_group), live_options);
    live.start(true)?;

    // Process each app
    for (idx, (name, step_times)) in APPS.iter().enumerate() {
        // Update message on overall progress bar
        let top_descr = format!(
            "[bold #AAAAAA]({} out of {} apps installed)",
            idx,
            APPS.len()
        );
        overall_progress.update(
            overall_task_id,
            None,
            None,
            None,
            Some(top_descr),
            None,
            false,
            None,
        );

        // Add progress bar for current app
        let current_task_id = current_app_progress.add_task(
            &format!("Installing app {}", name),
            true,
            None,
            0.0,
            true,
        );

        // Add progress bar for steps of this app
        let app_steps_task_id = app_steps_progress.add_task(
            name, // Use name as description for the template
            true,
            Some(step_times.len() as f64),
            0.0,
            true,
        );

        // Run steps for this app
        run_steps(
            name,
            step_times,
            app_steps_task_id,
            &step_progress,
            &app_steps_progress,
        );

        // Stop and hide steps progress bar for this specific app
        app_steps_progress.update(
            app_steps_task_id,
            None,
            None,
            None,
            None,
            Some(false),
            false,
            None,
        );

        // Update current app progress to show completion
        current_app_progress.stop_task(current_task_id);
        current_app_progress.update(
            current_task_id,
            None,
            None,
            None,
            Some(format!("[bold green]App {} installed!", name)),
            None,
            false,
            None,
        );

        // Increase overall progress
        overall_progress.advance(overall_task_id, 1.0);
    }

    // Final update for message on overall progress bar
    let final_descr = format!("[bold green]{} apps installed, done!", APPS.len());
    overall_progress.update(
        overall_task_id,
        None,
        None,
        None,
        Some(final_descr),
        None,
        false,
        None,
    );

    // Give a moment to see the final state
    sleep(Duration::from_millis(500));

    live.stop()?;

    Ok(())
}

/// Run steps for a single app and update corresponding progress bars
fn run_steps(
    name: &str,
    step_times: &[u64; 4],
    app_steps_task_id: rich_rs::TaskID,
    step_progress: &Arc<Progress>,
    app_steps_progress: &Arc<Progress>,
) {
    for (idx, &step_time) in step_times.iter().enumerate() {
        let action = STEP_ACTIONS[idx];

        // Add progress bar for this step (time elapsed + action description)
        let step_task_id = step_progress.add_task(
            &format!("{} {}...", action, name),
            true,
            Some(step_time as f64),
            0.0,
            true,
        );

        // Run the step, updating progress
        for _ in 0..step_time {
            sleep(Duration::from_millis(500));
            step_progress.advance(step_task_id, 1.0);
        }

        // Stop and hide progress bar for this step when done
        step_progress.stop_task(step_task_id);
        step_progress.update(
            step_task_id,
            None,
            None,
            None,
            None,
            Some(false),
            false,
            None,
        );

        // Update progress bar for current app when step is done
        app_steps_progress.advance(app_steps_task_id, 1.0);
    }
}

/// A renderable that combines multiple progress displays in a group
#[derive(Clone)]
struct DynamicProgressGroup {
    current_app_progress: Arc<Progress>,
    step_progress: Arc<Progress>,
    app_steps_progress: Arc<Progress>,
    overall_progress: Arc<Progress>,
}

impl Renderable for DynamicProgressGroup {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        // Create a group with:
        // 1. Panel containing current_app_progress, step_progress, and app_steps_progress
        // 2. Overall progress bar outside the panel

        // Create inner group for the panel content
        let inner_group = Group::from_arcs([
            Arc::new(ProgressRef(self.current_app_progress.clone())) as Arc<dyn Renderable>,
            Arc::new(ProgressRef(self.step_progress.clone())) as Arc<dyn Renderable>,
            Arc::new(ProgressRef(self.app_steps_progress.clone())) as Arc<dyn Renderable>,
        ]);

        let panel = Panel::new(Box::new(inner_group));

        // Create outer group with panel and overall progress
        let outer_group = Group::from_arcs([
            Arc::new(panel) as Arc<dyn Renderable>,
            Arc::new(ProgressRef(self.overall_progress.clone())) as Arc<dyn Renderable>,
        ]);

        outer_group.render(console, options)
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
