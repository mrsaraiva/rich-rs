#include "rich.hpp"
#include <cassert>

int main() {
    rich::Console console;
    console.set_force_terminal(false);
    rich::Tree tree("root");
    auto src = tree.add("src");
    tree.add("README.md");
    src.add("lib.rs").add("deep.rs");
    auto label = rich::Text("renderable.rs").finish();
    src.add(label);
    auto result = tree.finish();
    const auto output = console.render(result);
    assert(output.find("deep.rs") != std::string::npos);
    assert(console.render(label) == "renderable.rs");
}
