//! Box module parity tests.

use rich_rs::r#box::{RowLevel, ASCII, DOUBLE, HEAVY, ROUNDED, SQUARE};

pub fn run() {
    println!("=== Box Constants ===");

    println!("ASCII.ascii -> {}", ASCII.ascii);
    println!("ROUNDED.ascii -> {}", ROUNDED.ascii);
    println!("HEAVY.ascii -> {}", HEAVY.ascii);
    println!("DOUBLE.ascii -> {}", DOUBLE.ascii);
    println!("SQUARE.ascii -> {}", SQUARE.ascii);

    println!("\n=== Box Characters ===");

    // ASCII characters
    println!("ASCII.top_left -> '{}'", ASCII.top_left);
    println!("ASCII.top -> '{}'", ASCII.top);
    println!("ASCII.top_right -> '{}'", ASCII.top_right);

    // ROUNDED characters
    println!("ROUNDED.top_left -> '{}'", ROUNDED.top_left);
    println!("ROUNDED.top -> '{}'", ROUNDED.top);
    println!("ROUNDED.top_right -> '{}'", ROUNDED.top_right);

    // HEAVY characters
    println!("HEAVY.top_left -> '{}'", HEAVY.top_left);
    println!("HEAVY.top -> '{}'", HEAVY.top);
    println!("HEAVY.top_right -> '{}'", HEAVY.top_right);

    println!("\n=== get_top ===");

    let widths = [10, 10, 10];
    println!(
        "SQUARE.get_top([10, 10, 10]) -> \"{}\"",
        SQUARE.get_top(&widths)
    );
    println!(
        "ASCII.get_top([10, 10, 10]) -> \"{}\"",
        ASCII.get_top(&widths)
    );
    println!(
        "ROUNDED.get_top([10, 10, 10]) -> \"{}\"",
        ROUNDED.get_top(&widths)
    );
    println!(
        "HEAVY.get_top([10, 10, 10]) -> \"{}\"",
        HEAVY.get_top(&widths)
    );
    println!(
        "DOUBLE.get_top([10, 10, 10]) -> \"{}\"",
        DOUBLE.get_top(&widths)
    );

    println!("\n=== get_row ===");

    println!(
        "SQUARE.get_row([10, 10, 10], Head) -> \"{}\"",
        SQUARE.get_row(&widths, RowLevel::Head, true)
    );
    println!(
        "ASCII.get_row([10, 10, 10], Head) -> \"{}\"",
        ASCII.get_row(&widths, RowLevel::Head, true)
    );
    println!(
        "SQUARE.get_row([10, 10, 10], Row) -> \"{}\"",
        SQUARE.get_row(&widths, RowLevel::Row, true)
    );
    println!(
        "SQUARE.get_row([10, 10, 10], Mid) -> \"{}\"",
        SQUARE.get_row(&widths, RowLevel::Mid, true)
    );
    println!(
        "SQUARE.get_row([10, 10, 10], Foot) -> \"{}\"",
        SQUARE.get_row(&widths, RowLevel::Foot, true)
    );
    println!(
        "SQUARE.get_row([10, 10, 10], edge=false) -> \"{}\"",
        SQUARE.get_row(&widths, RowLevel::Row, false)
    );

    println!("\n=== get_bottom ===");

    println!(
        "SQUARE.get_bottom([10, 10, 10]) -> \"{}\"",
        SQUARE.get_bottom(&widths)
    );
    println!(
        "ASCII.get_bottom([10, 10, 10]) -> \"{}\"",
        ASCII.get_bottom(&widths)
    );
    println!(
        "ROUNDED.get_bottom([10, 10, 10]) -> \"{}\"",
        ROUNDED.get_bottom(&widths)
    );
    println!(
        "HEAVY.get_bottom([10, 10, 10]) -> \"{}\"",
        HEAVY.get_bottom(&widths)
    );
    println!(
        "DOUBLE.get_bottom([10, 10, 10]) -> \"{}\"",
        DOUBLE.get_bottom(&widths)
    );

    println!("\n=== substitute ===");

    // ROUNDED with legacy_windows -> SQUARE
    let result = ROUNDED.substitute(true, false);
    println!(
        "ROUNDED.substitute(legacy_windows=true) -> is_square={}",
        result == SQUARE
    );

    // SQUARE with ascii_only -> ASCII
    let result = SQUARE.substitute(false, true);
    println!(
        "SQUARE.substitute(ascii_only=true) -> is_ascii={}",
        result == ASCII
    );

    // ASCII stays ASCII
    let result = ASCII.substitute(false, true);
    println!(
        "ASCII.substitute(ascii_only=true) -> is_ascii={}",
        result == ASCII
    );

    println!("\n=== Single column ===");

    let single = [10];
    println!("SQUARE.get_top([10]) -> \"{}\"", SQUARE.get_top(&single));
    println!(
        "SQUARE.get_bottom([10]) -> \"{}\"",
        SQUARE.get_bottom(&single)
    );
}
