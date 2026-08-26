#include "rich.hpp"
#include <cassert>

int main() {
    rich::Console console;
    console.set_force_terminal(false).set_size(80, 24);
    rich::Syntax syntax("fn main() {}", "rust");
    syntax.set_line_numbers(true).set_line_range(false, 0, false, 0);
    assert(console.render(syntax.finish()).find("main") != std::string::npos);
    rich::Markdown markdown("# Heading\n\n**bold**");
    assert(console.render(markdown.finish()).find("Heading") != std::string::npos);
    rich::Json json("{\"b\":2,\"a\":1}", 2, false, true);
    assert(console.render(json.finish()).find("\"a\"") != std::string::npos);
}
