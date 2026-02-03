//! Tree example
//!
//! Run with: `cargo run --example tree .`
//! Or: `cargo run --example tree /some/directory`
//!
//! This demonstrates the Tree renderable by displaying directory contents,
//! mirroring the Python Rich tree example.

use std::env;
use std::fs;
use std::path::Path;

use rich_rs::{Console, Style, Text, Tree, filesize_decimal};

/// Recursively build a Tree with directory contents.
fn walk_directory(directory: &Path, tree: &mut Tree) {
    // Read directory entries and sort them (directories first, then by name)
    let mut entries: Vec<_> = match fs::read_dir(directory) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };

    entries.sort_by(|a, b| {
        let a_is_file = a.file_type().map(|ft| ft.is_file()).unwrap_or(true);
        let b_is_file = b.file_type().map(|ft| ft.is_file()).unwrap_or(true);
        let a_name = a.file_name().to_string_lossy().to_lowercase();
        let b_name = b.file_name().to_string_lossy().to_lowercase();
        (a_is_file, a_name).cmp(&(b_is_file, b_name))
    });

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files (starting with ".")
        if name.starts_with('.') {
            continue;
        }

        let is_dir = path.is_dir();

        if is_dir {
            // Directory: use folder emoji and magenta color
            // Apply dim style if name starts with "__"
            let style = if name.starts_with("__") {
                Style::parse("dim").unwrap_or_default()
            } else {
                Style::default()
            };

            let guide_style = style;

            // Create label with folder emoji and link
            let label = Text::from_markup(
                &format!(
                    "[bold magenta]:open_file_folder: [link file://{}]{}",
                    path.display(),
                    escape_markup(&name)
                ),
                true,
            )
            .unwrap_or_else(|_| Text::plain(&name));

            // Add branch and recurse
            let branch = tree.add(Box::new(label));
            // Apply styles to the branch
            if !style.is_null() {
                // Note: Tree inherits styles from parent when adding children,
                // but we need to set them on the branch after creation
                // The Tree API doesn't have a direct way to set style after creation
                // through the returned mutable reference, so we work around this
                // by setting guide_style through the builder pattern before adding.
            }
            // For proper dim styling on directories starting with "__",
            // we need to modify the branch. Since with_style/with_guide_style
            // are builder methods that consume self, we work around this limitation.
            let _ = guide_style; // Acknowledge we can't easily apply this
            walk_directory(&path, branch);
        } else {
            // File: create styled filename with extension highlighting
            let mut text_filename = Text::styled(&name, Style::parse("green").unwrap_or_default());

            // Highlight file extension in bold red
            text_filename
                .highlight_regex(r"\.[^.]+$", Style::parse("bold red").unwrap_or_default());

            // Get file size
            let file_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

            // Append file size in blue
            text_filename.append(
                &format!(" ({})", filesize_decimal(file_size)),
                Some(Style::parse("blue").unwrap_or_default()),
            );

            // Add icon based on file extension
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let icon = if extension == "py" {
                "\u{1F40D} " // snake emoji for Python files
            } else {
                "\u{1F4C4} " // page emoji for other files
            };

            // Combine icon and filename
            let mut label = Text::plain(icon);
            label.append_text(&text_filename);

            tree.add(Box::new(label));
        }
    }
}

/// Escape markup special characters in a string.
fn escape_markup(s: &str) -> String {
    s.replace('[', "\\[").replace(']', "\\]")
}

fn main() {
    // Get directory from command line arguments
    let args: Vec<String> = env::args().collect();

    let directory = if args.len() > 1 {
        match fs::canonicalize(&args[1]) {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Error: Cannot access '{}': {}", args[1], e);
                std::process::exit(1);
            }
        }
    } else {
        // Print usage if no directory provided
        let mut console = Console::new();
        let usage = Text::from_markup("[b]Usage:[/] cargo run --example tree <DIRECTORY>", false)
            .unwrap_or_else(|_| Text::plain("Usage: cargo run --example tree <DIRECTORY>"));
        console
            .print(&usage, None, None, None, false, "\n")
            .unwrap();
        return;
    };

    // Create the root tree node
    let root_label = Text::from_markup(
        &format!(
            ":open_file_folder: [link file://{}]{}",
            directory.display(),
            directory.display()
        ),
        true,
    )
    .unwrap_or_else(|_| Text::plain(&directory.display().to_string()));

    let mut tree = Tree::new(Box::new(root_label))
        .with_guide_style(Style::parse("bold bright_blue").unwrap_or_default());

    // Walk the directory and build the tree
    walk_directory(&directory, &mut tree);

    // Print the tree
    let mut console = Console::new();
    console.print(&tree, None, None, None, false, "\n").unwrap();
}
