//! Tree module parity tests.

use rich_rs::text::Text;
use rich_rs::tree::Tree;
use rich_rs::{Console, ConsoleOptions};

/// Helper to render tree to plain text
fn render_tree(tree: &Tree, width: usize) -> String {
    let console = Console::with_options(ConsoleOptions {
        max_width: width,
        is_terminal: true,
        color_system: None,
        ..Default::default()
    });
    let options = console.options().clone();
    let lines = console.render_lines(tree, Some(&options), None, false, false);
    let mut out = String::new();
    for line in lines {
        for segment in line {
            out.push_str(&segment.text);
        }
        out.push('\n');
    }
    out
}

fn bool_py(value: bool) -> &'static str {
    if value {
        "True"
    } else {
        "False"
    }
}

pub fn run() {
    println!("=== Single node tree ===");

    let tree = Tree::new(Box::new(Text::plain("Root")));
    let output = render_tree(&tree, 40);
    let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
    println!("Tree('Root') lines={}", lines.len());
    println!(
        "  first line: '{}'",
        lines.first().copied().unwrap_or("")
    );

    println!("\n=== Tree with children ===");

    let mut tree = Tree::new(Box::new(Text::plain("Parent")));
    tree.add(Box::new(Text::plain("Child 1")));
    tree.add(Box::new(Text::plain("Child 2")));
    let output = render_tree(&tree, 40);
    let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
    println!("Tree with 2 children lines={}", lines.len());
    for (i, line) in lines.iter().enumerate() {
        let has_branch = line.contains("├") || line.contains("└");
        println!("  line[{}]: has_branch={}", i, bool_py(has_branch));
    }

    println!("\n=== Nested tree ===");

    let mut tree = Tree::new(Box::new(Text::plain("Root")));
    {
        let branch1 = tree.add(Box::new(Text::plain("Branch 1")));
        branch1.add(Box::new(Text::plain("Leaf 1.1")));
        branch1.add(Box::new(Text::plain("Leaf 1.2")));
    }
    tree.add(Box::new(Text::plain("Branch 2")));
    let output = render_tree(&tree, 40);
    let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
    println!("Nested tree lines={}", lines.len());
    for (i, line) in lines.iter().enumerate() {
        let indent = line.len() - line.trim_start().len();
        println!("  line[{}]: indent={}", i, indent);
    }

    println!("\n=== Tree guide characters ===");

    let mut tree = Tree::new(Box::new(Text::plain("Root")));
    tree.add(Box::new(Text::plain("Child 1")));
    tree.add(Box::new(Text::plain("Child 2")));
    tree.add(Box::new(Text::plain("Child 3")));
    let output = render_tree(&tree, 40);
    let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("├") {
            println!("  line[{}]: uses ├── (branch)", i);
        } else if line.contains("└") {
            println!("  line[{}]: uses └── (end)", i);
        }
    }
}
