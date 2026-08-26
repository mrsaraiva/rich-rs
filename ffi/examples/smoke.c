#include "rich.h"

#include <assert.h>
#include <stdint.h>
#include <string.h>

static RichStr rich_str(const char *value) {
    RichStr result = {(const uint8_t *)value, (uint64_t)strlen(value)};
    return result;
}

int main(void) {
    assert(rich_ffi_abi_version() == RICH_FFI_ABI_VERSION);
    RichConsole *console = NULL;
    assert(rich_console_new(&console) == RICH_OK);
    assert(rich_console_set_force_terminal(console, 0) == RICH_OK);
    assert(rich_console_set_force_terminal(console, 2) == RICH_ERROR_INVALID_ARGUMENT);

    RichText *text = NULL;
    assert(rich_text_new(rich_str("nested"), &text) == RICH_OK);
    RichRenderable *content = NULL;
    assert(rich_text_finish(&text, &content) == RICH_OK && text == NULL);

    RichPanel *panel = NULL;
    assert(rich_panel_new(content, &panel) == RICH_OK);
    assert(rich_panel_set_title(panel, rich_str("C11")) == RICH_OK);
    RichRenderable *nested = NULL;
    assert(rich_panel_finish(&panel, &nested) == RICH_OK && panel == NULL);

    RichBuffer output = {NULL, 0};
    assert(rich_console_render(console, nested, &output) == RICH_OK);
    assert(output.data != NULL && output.len != 0);
    rich_buffer_free(&output);
    assert(output.data == NULL && output.len == 0);

    assert(rich_console_render(console, content, &output) == RICH_OK);
    assert(output.len == 6 && memcmp(output.data, "nested", 6) == 0);
    rich_buffer_free(&output);
    rich_renderable_free(nested);
    rich_renderable_free(content);
    rich_console_free(console);
    return 0;
}
