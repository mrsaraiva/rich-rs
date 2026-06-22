// rich.hpp — thin C++ RAII wrapper over the rich-rs C ABI (rich.h).
//
// Header-only. Owns the console handle via unique_ptr; returns std::string from
// render. Matches the ergonomics SafeTunnel already gets from Slint::Slint.
#ifndef RICH_FFI_HPP
#define RICH_FFI_HPP

#include <cstdint>
#include <memory>
#include <string>

#include "rich.h"

namespace rich {

namespace detail {
struct ConsoleDeleter {
    void operator()(RichConsole* c) const noexcept { rich_console_free(c); }
};
} // namespace detail

/// Color system selector mirroring the C ABI integer codes.
enum class ColorSystem : int {
    None = 0,
    Standard = 1,
    EightBit = 2,
    TrueColor = 3,
    Windows = 4,
};

class Console {
public:
    Console() : handle_(rich_console_new()) {}

    /// True if the underlying handle was created successfully.
    explicit operator bool() const noexcept { return static_cast<bool>(handle_); }
    RichConsole* get() const noexcept { return handle_.get(); }

    /// Pass isatty() here: true emits ANSI styling, false emits plain text.
    Console& set_force_terminal(bool force) noexcept {
        rich_console_set_force_terminal(handle_.get(), force);
        return *this;
    }
    Console& set_size(std::uint32_t width, std::uint32_t height) noexcept {
        rich_console_set_size(handle_.get(), width, height);
        return *this;
    }
    Console& set_color_system(ColorSystem sys) noexcept {
        rich_console_set_color_system(handle_.get(), static_cast<int>(sys));
        return *this;
    }
    Console& set_markup_enabled(bool enabled) noexcept {
        rich_console_set_markup_enabled(handle_.get(), enabled);
        return *this;
    }
    Console& set_emoji_enabled(bool enabled) noexcept {
        rich_console_set_emoji_enabled(handle_.get(), enabled);
        return *this;
    }
    std::uint32_t width() const noexcept { return rich_console_width(handle_.get()); }

    /// Render markup to a std::string. Returns empty string on failure.
    /// No trailing newline is appended.
    std::string render_markup(const char* markup) const {
        char* out = rich_console_render_markup(handle_.get(), markup);
        if (!out) return {};
        std::string s(out);
        rich_string_free(out);
        return s;
    }
    std::string render_markup(const std::string& markup) const {
        return render_markup(markup.c_str());
    }

private:
    std::unique_ptr<RichConsole, detail::ConsoleDeleter> handle_;
};

} // namespace rich

#endif // RICH_FFI_HPP
