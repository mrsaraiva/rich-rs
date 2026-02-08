//! Lite simulation of the top linux command.
//!
//! Run with:
//!   FORCE_COLOR=1 COLORTERM=truecolor cargo run --example top_lite_simulator
//!
//! This is a Rust port of Python Rich's `top_lite_simulator.py` example.
//! Unlike the Python version which runs indefinitely, this runs for ~10 seconds.

use std::thread::sleep;
use std::time::{Duration, Instant};

use rand::Rng;

use rich_rs::r#box::SIMPLE;
use rich_rs::{
    Column, Console, ConsoleOptions, JustifyMethod, Live, LiveOptions, Measurement, Renderable,
    Row, Segments, Style, Table, Text,
};

/// Process state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProcessState {
    Running,
    Sleeping,
}

impl ProcessState {
    fn as_str(&self) -> &'static str {
        match self {
            ProcessState::Running => "running",
            ProcessState::Sleeping => "sleeping",
        }
    }
}

/// A simulated process with fields matching the Python example.
#[derive(Clone)]
struct Process {
    pid: u32,
    command: String,
    cpu_percent: f64,
    memory: u64,
    start_time: Instant,
    thread_count: u32,
    state: ProcessState,
}

impl Process {
    /// Generate a random process with the given PID.
    fn random(pid: u32) -> Self {
        let mut rng = rand::rng();
        let cpu_percent = rng.random::<f64>() * 20.0;
        let memory = {
            let base: u64 = rng.random_range(10..=200);
            base * base * base // cubed, like Python
        };
        // Simulate start time in the past (0 to 250000 seconds ago)
        let seconds_ago = {
            let base: u64 = rng.random_range(0..=500);
            base * base
        };
        let start_time = Instant::now() - Duration::from_secs(seconds_ago);
        let thread_count = rng.random_range(1..=32);
        let state = if rng.random_range(0..10) < 8 {
            ProcessState::Running
        } else {
            ProcessState::Sleeping
        };

        Process {
            pid,
            command: format!("Process {pid}"),
            cpu_percent,
            memory,
            start_time,
            thread_count,
            state,
        }
    }

    /// Format memory as a human-readable string.
    fn memory_str(&self) -> String {
        if self.memory > 1_000_000 {
            format!("{}M", self.memory / 1_000_000)
        } else if self.memory > 1_000 {
            format!("{}K", self.memory / 1_000)
        } else {
            self.memory.to_string()
        }
    }

    /// Format elapsed time since process start.
    fn time_str(&self) -> String {
        let elapsed = self.start_time.elapsed();
        let total_secs = elapsed.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        let micros = elapsed.subsec_micros();
        format!("{hours}:{minutes:02}:{seconds:02}.{micros:06}")
    }
}

/// Generate a table showing the top processes.
fn create_process_table(num_processes: usize) -> Table {
    // Generate and sort processes by CPU % descending
    let mut processes: Vec<Process> = (0..num_processes as u32).map(Process::random).collect();
    processes.sort_by(|a, b| {
        b.cpu_percent
            .partial_cmp(&a.cpu_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Create table with SIMPLE box style (like Python)
    let mut table = Table::new().with_box(Some(SIMPLE)).with_show_header(true);

    // Add columns: PID, Command, CPU %, Memory, Time, Thread #, State
    table.add_column(
        Column::with_header_str("PID")
            .justify(JustifyMethod::Right)
            .style(Style::parse("cyan").unwrap_or_default()),
    );
    table.add_column(
        Column::with_header_str("Command").style(Style::parse("green").unwrap_or_default()),
    );
    table.add_column(
        Column::with_header_str("CPU %")
            .justify(JustifyMethod::Right)
            .style(Style::parse("red").unwrap_or_default()),
    );
    table.add_column(
        Column::with_header_str("Memory")
            .justify(JustifyMethod::Right)
            .style(Style::parse("magenta").unwrap_or_default()),
    );
    table.add_column(
        Column::with_header_str("Time")
            .justify(JustifyMethod::Right)
            .style(Style::parse("blue").unwrap_or_default()),
    );
    table.add_column(
        Column::with_header_str("Thread #")
            .justify(JustifyMethod::Right)
            .style(Style::parse("yellow").unwrap_or_default()),
    );
    table.add_column(
        Column::with_header_str("State").style(Style::parse("white").unwrap_or_default()),
    );

    // Add rows for each process
    for proc in &processes {
        table.add_row(Row::new(vec![
            Box::new(Text::plain(proc.pid.to_string())),
            Box::new(Text::plain(&proc.command)),
            Box::new(Text::plain(format!("{:.1}", proc.cpu_percent))),
            Box::new(Text::plain(proc.memory_str())),
            Box::new(Text::plain(proc.time_str())),
            Box::new(Text::plain(proc.thread_count.to_string())),
            Box::new(Text::plain(proc.state.as_str())),
        ]));
    }

    table
}

/// Wrapper to make Table work with Live display.
struct ProcessTableRenderer {
    num_processes: usize,
}

impl ProcessTableRenderer {
    fn new(num_processes: usize) -> Self {
        Self { num_processes }
    }
}

impl Renderable for ProcessTableRenderer {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        let table = create_process_table(self.num_processes);
        table.render(console, options)
    }

    fn measure(&self, console: &Console, options: &ConsoleOptions) -> Measurement {
        let table = create_process_table(self.num_processes);
        table.measure(console, options)
    }
}


fn main() -> std::io::Result<()> {
    // Get console height to determine number of processes to display
    let console = Console::new();
    let height = console.options().height.unwrap_or(25);
    let num_processes = height.saturating_sub(4).max(10);

    // Create Live display with screen mode (alternate screen buffer)
    let options = LiveOptions {
        screen: true,
        refresh_per_second: 4.0, // Python uses refresh_per_second=4
        auto_refresh: false,     // We'll manually refresh
        ..Default::default()
    };

    let renderer = ProcessTableRenderer::new(num_processes);
    let mut live = Live::with_options(Box::new(renderer), options);

    // Start the live display
    live.start(true)?;

    // Run for ~10 seconds (unlike Python which runs indefinitely)
    let duration = Duration::from_secs(10);
    let start = Instant::now();

    while start.elapsed() < duration {
        // Sleep for 1 second between updates (like Python)
        sleep(Duration::from_secs(1));

        // Update with a new table (processes are regenerated each time, like Python)
        let new_renderer = ProcessTableRenderer::new(num_processes);
        live.update(Box::new(new_renderer), true)?;
    }

    // Stop the live display
    live.stop()?;

    println!("Top lite simulator finished after ~10 seconds.");
    Ok(())
}
