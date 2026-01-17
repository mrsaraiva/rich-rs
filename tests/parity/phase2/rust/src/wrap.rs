use rich_rs::wrap::divide_line;

pub fn run() {
    println!("=== divide_line() ===");

    // Simple word wrap
    let offsets = divide_line("Hello World Test", 10, false);
    println!("divide_line('Hello World Test', 10) -> {:?}", offsets);

    // Word boundary
    let offsets = divide_line("Hello World", 5, false);
    println!("divide_line('Hello World', 5) -> {:?}", offsets);

    // Fold long word
    let offsets = divide_line("Supercalifragilistic", 8, true);
    println!("divide_line('Supercalifragilistic', 8, fold=True) -> {:?}", offsets);

    // No fold long word
    let offsets = divide_line("Supercalifragilistic", 8, false);
    println!("divide_line('Supercalifragilistic', 8, fold=False) -> {:?}", offsets);

    // Multiple words fit
    let offsets = divide_line("A B C D E F", 3, false);
    println!("divide_line('A B C D E F', 3) -> {:?}", offsets);

    // Empty string
    let offsets = divide_line("", 10, false);
    println!("divide_line('', 10) -> {:?}", offsets);

    // Single word fits
    let offsets = divide_line("Hello", 10, false);
    println!("divide_line('Hello', 10) -> {:?}", offsets);

    // Whitespace handling
    let offsets = divide_line("Hello   World", 8, false);
    println!("divide_line('Hello   World', 8) -> {:?}", offsets);
}
