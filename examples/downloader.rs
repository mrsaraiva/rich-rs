//! A rudimentary URL downloader (like wget or curl) to demonstrate Rich progress bars.
//!
//! This is a Rust port of Python Rich's `examples/downloader.py`.
//! Downloads multiple files concurrently with a progress bar for each.
//!
//! Run with:
//!   cargo run --example downloader -- URL1 URL2 URL3
//!
//! Example:
//!   cargo run --example downloader -- https://proof.ovh.net/files/10Mb.dat https://proof.ovh.net/files/1Mb.dat

use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use rich_rs::{
    BarColumn, DownloadColumn, JustifyMethod, LiveOptions, Progress, ProgressColumn, TaskID,
    TextColumn, TimeRemainingColumn, TransferSpeedColumn,
};

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        println!("A rudimentary URL downloader to demonstrate Rich progress bars.");
        println!("\nUsage:\n\tcargo run --example downloader -- URL1 URL2 URL3");
        println!("\nExample:");
        println!("\tcargo run --example downloader -- https://proof.ovh.net/files/10Mb.dat");
        return Ok(());
    }

    // Set up Ctrl+C handler
    let done = Arc::new(AtomicBool::new(false));
    let done_handler = done.clone();
    ctrlc::set_handler(move || {
        done_handler.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Create progress with custom columns matching Python Rich's downloader
    let columns: Vec<Box<dyn ProgressColumn>> = vec![
        Box::new(
            TextColumn::new("[bold blue]{task.description}").with_justify(JustifyMethod::Right),
        ),
        Box::new(BarColumn::new().with_bar_width(None)),
        Box::new(TextColumn::new(
            "[progress.percentage]{task.percentage:>3.1f}%",
        )),
        Box::new(TextColumn::new("•")),
        Box::new(DownloadColumn::new()),
        Box::new(TextColumn::new("•")),
        Box::new(TransferSpeedColumn::new()),
        Box::new(TextColumn::new("•")),
        Box::new(TimeRemainingColumn::new(false)),
    ];

    let live_options = LiveOptions {
        refresh_per_second: 12.5,
        ..Default::default()
    };

    // Create and start progress BEFORE wrapping in Arc
    let mut progress = Progress::new(columns, live_options, false, false);
    progress.start()?;

    // Now wrap in Arc for thread sharing
    let progress = Arc::new(progress);

    // Create all tasks upfront (before spawning threads)
    let tasks: Vec<(String, TaskID)> = args
        .iter()
        .map(|url| {
            let filename = url.split('/').last().unwrap_or("download").to_string();
            // Create task not started yet (we'll start it when we know the size)
            let task_id = progress.add_task(&filename, false, None, 0.0, true);
            (url.clone(), task_id)
        })
        .collect();

    // Spawn download threads
    let handles: Vec<_> = tasks
        .into_iter()
        .map(|(url, task_id)| {
            let progress = Arc::clone(&progress);
            let done = Arc::clone(&done);

            thread::spawn(move || {
                if let Err(e) = download_url(&progress, task_id, &url, &done) {
                    // Can't print during live display - just silently fail
                    // The task will show as incomplete
                    let _ = e;
                }
            })
        })
        .collect();

    // Wait for all downloads to complete
    for handle in handles {
        let _ = handle.join();
    }

    // Unwrap Arc and stop progress
    // All threads are done, so we're the only owner
    let mut progress =
        Arc::try_unwrap(progress).unwrap_or_else(|_| panic!("Threads should have finished"));
    progress.stop()?;

    if done.load(Ordering::SeqCst) {
        println!("Download interrupted by user.");
    } else {
        println!("All downloads complete!");
    }

    Ok(())
}

fn download_url(
    progress: &Progress,
    task_id: TaskID,
    url: &str,
    done: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Make HTTP request
    let response = ureq::get(url).call()?;

    // Get content length from headers
    let content_length: Option<u64> = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());

    // Update task with total size and start it
    progress.update(
        task_id,
        content_length.map(|c| Some(c as f64)), // total
        None,                                   // completed
        None,                                   // advance
        None,                                   // description
        None,                                   // visible
        false,                                  // refresh
        None,                                   // fields
    );
    progress.start_task(task_id);

    // Extract filename for the output file
    let filename = url.split('/').last().unwrap_or("download");
    let dest_path = Path::new("./").join(filename);
    let mut dest_file = File::create(&dest_path)?;

    // Read and write in chunks
    let mut reader = response.into_body().into_reader();
    let mut buffer = [0u8; 32768]; // 32KB chunks

    loop {
        if done.load(Ordering::SeqCst) {
            return Ok(());
        }

        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        dest_file.write_all(&buffer[..bytes_read])?;
        progress.advance(task_id, bytes_read as f64);
    }

    Ok(())
}
