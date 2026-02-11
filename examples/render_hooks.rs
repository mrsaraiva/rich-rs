//! Render hook example
//!
//! Run with: `cargo run --example render_hooks`
//!
//! Demonstrates `push_render_hook()` by transforming all printable segments.

use rich_rs::{Console, Segments, Text};

fn main() -> std::io::Result<()> {
    let mut console = Console::capture();

    console.print(&Text::plain("before hook"), None, None, None, false, "\n")?;
    let baseline = console.get_captured();
    console.clear_captured();

    // Hook runs in the print pipeline and can rewrite rendered segments.
    console.push_render_hook(Box::new(|segments: &Segments| {
        Segments::from_iter(segments.iter().map(|seg| {
            if seg.control.is_some() {
                seg.clone()
            } else {
                let mut updated = seg.clone();
                updated.text = format!("[HOOKED] {}", seg.text.to_uppercase()).into();
                updated
            }
        }))
    }));

    console.print(&Text::plain("after hook"), None, None, None, false, "\n")?;
    let hooked = console.get_captured();

    // Keep the output easy to compare while preserving exact text.
    println!("baseline: {:?}", baseline.trim_end());
    println!("hooked  : {:?}", hooked.trim_end());

    // Optional cleanup for parity with push/pop API.
    console.pop_render_hook();

    Ok(())
}
