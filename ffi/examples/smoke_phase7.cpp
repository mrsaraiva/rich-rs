#include "rich.hpp"
#include <cassert>

int main() {
    rich::Console console;
    console.set_force_terminal(false);
    rich::Progress progress;
    const auto id = progress.add_task("download", 100.0);
    assert(id == 0);
    progress.update(id, 50.0);
    assert(progress.render_frame(console).find("download") != std::string::npos);
    rich::Status status("working");
    assert(status.render_frame(console).find("working") != std::string::npos);
    rich::Spinner spinner("dots");
    assert(!spinner.render_frame(console, 0.5).empty());
}
