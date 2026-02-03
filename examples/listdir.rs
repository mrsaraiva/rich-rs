//! Listdir example - A simple `ls` clone
//!
//! Run with: `cargo run --example listdir <DIRECTORY>`
//!
//! If your terminal supports hyperlinks, you should be able to launch files
//! by clicking the filename (usually with cmd / ctrl).
//!
//! This is a port of Python Rich's `examples/listdir.py`.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use rich_rs::{Columns, Console, Renderable, Span, Style, StyleMeta, Text};

/// Create a styled filename Text with hyperlink and extension highlighting.
fn make_filename_text(root_path: &str, filename: &str) -> Text {
    // Get the absolute path for the hyperlink
    let path = PathBuf::from(root_path).join(filename);
    let abs_path = fs::canonicalize(&path).unwrap_or(path.clone());
    let file_url = format!("file://{}", abs_path.display());

    // Determine if it's a directory
    let is_dir = abs_path.is_dir();

    // Choose style based on type: bold blue for directories, default for files
    let base_style = if is_dir {
        Style::parse("bold blue").unwrap_or_default()
    } else {
        Style::default()
    };

    // Create the text
    let mut text = Text::plain(filename);

    // Apply the base style with hyperlink metadata
    let meta = StyleMeta::with_link(Arc::<str>::from(file_url));
    let len = text.len();
    text.spans_mut()
        .push(Span::new_with_meta(0, len, base_style, Some(meta)));

    // Highlight file extension (the part after the last dot) with bold
    if !is_dir {
        if let Some(dot_pos) = filename.rfind('.') {
            if dot_pos > 0 && dot_pos < filename.len() - 1 {
                // Extension exists and is not at start or end
                let ext_style = Style::new().with_bold(true);
                text.stylize(dot_pos, len, ext_style);
            }
        }
    }

    text
}

fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    let root_path = match args.get(1) {
        Some(path) => path.clone(),
        None => {
            println!("Usage: listdir <DIRECTORY>");
            println!("\nExample: cargo run --example listdir /tmp");
            return;
        }
    };

    // Read the directory
    let entries = match fs::read_dir(&root_path) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Error reading directory '{}': {}", root_path, e);
            return;
        }
    };

    // Collect and filter filenames (skip hidden files starting with '.')
    let mut filenames: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| !name.starts_with('.'))
        .collect();

    // Sort case-insensitively
    filenames.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

    if filenames.is_empty() {
        println!(
            "Directory '{}' is empty or contains only hidden files.",
            root_path
        );
        return;
    }

    // Create Text objects for each filename
    let filename_texts: Vec<Box<dyn Renderable + Send + Sync>> = filenames
        .iter()
        .map(|name| {
            let text = make_filename_text(&root_path, name);
            Box::new(text) as Box<dyn Renderable + Send + Sync>
        })
        .collect();

    // Create columns display with equal width and column-first ordering
    let columns = Columns::new(filename_texts)
        .with_equal(true)
        .with_column_first(true);

    // Print the columns
    let mut console = Console::new();
    let _ = console.print(&columns, None, None, None, false, "\n");
}
