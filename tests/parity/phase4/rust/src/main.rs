//! Phase 4 parity tests: Panel, Tree, Table

mod panel;
mod table;
mod tree;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "panel" => panel::run(),
            "tree" => tree::run(),
            "table" => table::run(),
            "all" | _ => {
                println!("========== PANEL ==========\n");
                panel::run();
                println!("\n========== TREE ==========\n");
                tree::run();
                println!("\n========== TABLE ==========\n");
                table::run();
            }
        }
    } else {
        println!("========== PANEL ==========\n");
        panel::run();
        println!("\n========== TREE ==========\n");
        tree::run();
        println!("\n========== TABLE ==========\n");
        table::run();
    }
}
