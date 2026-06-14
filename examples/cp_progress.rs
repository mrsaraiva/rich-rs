//! A minimal `cp` clone that displays a progress bar.
//!
//! This is a Rust port of Python Rich's `examples/cp_progress.py`.
//!
//! Run with:
//!   cargo run --example cp_progress -- SRC DST
//!
//! Example:
//!   cargo run --example cp_progress -- /path/to/source.bin /path/to/dest.bin

use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use rich_rs::{LiveOptions, Progress};

fn copy_with_progress(
    progress: &Progress,
    src_path: &Path,
    dst_path: &Path,
    description: &str,
) -> io::Result<()> {
    let total = fs::metadata(src_path)?.len();

    let task_id = progress.add_task(description, true, Some(total as f64), 0.0, true);

    let src_owned = src_path.to_path_buf();
    let dst_owned = dst_path.to_path_buf();

    // Use fs::copy in a worker thread so the kernel can offload the transfer
    // (copy_file_range/sendfile fast paths). Wrapping reads via progress.open()
    // forces a userspace read/write loop, which significantly increases syscall
    // count and sys time for large files.
    let (tx, rx) = mpsc::channel::<io::Result<u64>>();
    thread::spawn(move || {
        let result = fs::copy(src_owned, dst_owned);
        let _ = tx.send(result);
    });

    let tick = Duration::from_millis(50);
    loop {
        match rx.try_recv() {
            Ok(result) => {
                let copied = result?;
                progress.update(
                    task_id,
                    None,
                    Some(copied as f64),
                    None,
                    None,
                    None,
                    true,
                    None,
                );
                break;
            }
            Err(mpsc::TryRecvError::Empty) => {
                // Poll destination size as an approximation of completed bytes.
                if let Ok(meta) = fs::metadata(dst_path) {
                    let pos = meta.len().min(total);
                    progress.update(
                        task_id,
                        None,
                        Some(pos as f64),
                        None,
                        None,
                        None,
                        false,
                        None,
                    );
                }
                thread::sleep(tick);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(io::Error::other("copy thread disconnected"));
            }
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() == 3 {
        let src_path = Path::new(&args[1]);
        let mut dst_path = Path::new(&args[2]).to_path_buf();

        // If destination is a directory, append the source filename (like cp)
        if dst_path.is_dir() {
            if let Some(filename) = src_path.file_name() {
                dst_path = dst_path.join(filename);
            }
        }

        // Get the filename for the description (like Python's os.path.basename)
        let description = src_path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| args[1].clone());

        let live_options = LiveOptions {
            refresh_per_second: 10.0,
            ..Default::default()
        };

        // Create progress with default columns (description, bar, percentage, time remaining)
        let mut progress = Progress::new_default(live_options, false, false, false);
        progress.start()?;

        copy_with_progress(&progress, &src_path, &dst_path, &description)?;

        progress.stop()?;
    } else {
        println!("Copy a file with a progress bar.");
        println!("Usage:\n\tcargo run --example cp_progress -- SRC DST");
    }

    Ok(())
}
