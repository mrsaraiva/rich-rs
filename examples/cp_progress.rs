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
use std::fs::File;
use std::io;
use std::path::Path;

use rich_rs::{LiveOptions, Progress};

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

        // Open source file with progress tracking
        let mut src = progress.open(&src_path, &description)?;

        // Open destination file for writing
        let mut dst = File::create(&dst_path)?;

        // Copy with progress
        io::copy(&mut src, &mut dst)?;

        progress.stop()?;
    } else {
        println!("Copy a file with a progress bar.");
        println!("Usage:\n\tcargo run --example cp_progress -- SRC DST");
    }

    Ok(())
}
