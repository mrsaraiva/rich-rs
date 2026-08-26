#include "rich.h"
#include <cassert>
#include <cstring>
#include <string>

static RichStr rs(const char* value) {
    return {reinterpret_cast<const uint8_t*>(value), static_cast<uint64_t>(std::strlen(value))};
}

int main() {
    RichConsole* console = nullptr;
    assert(rich_console_new(&console) == RICH_OK);
    RichText* text = nullptr;
    assert(rich_text_new(rs("plain"), &text) == RICH_OK);
    assert(rich_text_finish(&text, nullptr) == RICH_ERROR_NULL_ARGUMENT);
    assert(text != nullptr);
    RichRenderable* renderable = nullptr;
    assert(rich_text_finish(&text, &renderable) == RICH_OK && text == nullptr);
    RichBuffer out{nullptr, 0};
    assert(rich_console_render(console, renderable, &out) == RICH_OK);
    assert(std::string(reinterpret_cast<char*>(out.data), out.len) == "plain");
    rich_buffer_free(&out);
    rich_buffer_free(&out);
    rich_renderable_free(renderable);
    rich_console_free(console);
}
