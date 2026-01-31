//! Padding module parity tests.

use rich_rs::padding::Padding;
use rich_rs::text::Text;

pub fn run() {
    println!("=== Padding.unpack ===");

    // Single value
    let result = Padding::unpack(2);
    println!("Padding.unpack(2) -> {:?}", result);

    // Single value tuple
    let result = Padding::unpack((3,));
    println!("Padding.unpack((3,)) -> {:?}", result);

    // Two values (vertical, horizontal)
    let result = Padding::unpack((1, 4));
    println!("Padding.unpack((1, 4)) -> {:?}", result);

    // Four values (top, right, bottom, left)
    let result = Padding::unpack((1, 2, 3, 4));
    println!("Padding.unpack((1, 2, 3, 4)) -> {:?}", result);

    // Zero padding
    let result = Padding::unpack(0);
    println!("Padding.unpack(0) -> {:?}", result);

    println!("\n=== Padding properties ===");

    let text = Text::plain("Test");
    let padding = Padding::new(Box::new(text), (1, 2, 3, 4));
    println!("Padding((1, 2, 3, 4)).top -> {}", padding.top());
    println!("Padding((1, 2, 3, 4)).right -> {}", padding.right());
    println!("Padding((1, 2, 3, 4)).bottom -> {}", padding.bottom());
    println!("Padding((1, 2, 3, 4)).left -> {}", padding.left());

    println!("\n=== Padding.indent ===");

    let text = Text::plain("Indented");
    let padding = Padding::indent(Box::new(text), 4);
    println!("Padding.indent(4).left -> {}", padding.left());
    println!("Padding.indent(4).expand -> {}", padding.expand());
}
