#ifndef RICH_FFI_HPP
#define RICH_FFI_HPP

#include "rich.h"

#include <cstdint>
#include <functional>
#include <initializer_list>
#include <memory>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace rich {

struct Error {
    RichResult code;
};

class Exception : public std::runtime_error {
public:
    explicit Exception(RichResult code)
        : std::runtime_error("rich-ffi error " + std::to_string(code)), code_(code) {}
    RichResult code() const noexcept { return code_; }
    Error error() const noexcept { return Error{code_}; }
private:
    RichResult code_;
};

namespace detail {

inline void check(RichResult result) {
    if (result != RICH_OK) throw Exception(result);
}

inline void ensure_abi() {
    if (rich_ffi_abi_version() != RICH_FFI_ABI_VERSION) {
        throw std::runtime_error("rich-ffi ABI version mismatch");
    }
}

inline RichStr str(std::string_view value) noexcept {
    return RichStr{reinterpret_cast<const std::uint8_t*>(value.data()),
                   static_cast<std::uint64_t>(value.size())};
}

inline std::uint32_t count(std::size_t value) {
    if (value > UINT32_MAX) throw Exception(RICH_ERROR_INVALID_ARGUMENT);
    return static_cast<std::uint32_t>(value);
}

template <typename H, void (*Free)(H*)>
class Owned {
public:
    Owned() noexcept = default;
    explicit Owned(H* handle) noexcept : handle_(handle) {}
    Owned(Owned&& other) noexcept : handle_(other.handle_) { other.handle_ = nullptr; }
    Owned& operator=(Owned&& other) noexcept {
        if (this != &other) {
            reset();
            handle_ = other.handle_;
            other.handle_ = nullptr;
        }
        return *this;
    }
    Owned(const Owned&) = delete;
    Owned& operator=(const Owned&) = delete;
    ~Owned() { reset(); }

    H* get() const noexcept { return handle_; }
    H** address() noexcept { return &handle_; }
    H* release() noexcept {
        H* value = handle_;
        handle_ = nullptr;
        return value;
    }
    void reset(H* handle = nullptr) noexcept {
        if (handle_) Free(handle_);
        handle_ = handle;
    }
    explicit operator bool() const noexcept { return handle_ != nullptr; }
private:
    H* handle_ = nullptr;
};

struct TreeState {
    explicit TreeState(RichTree* value) noexcept : handle(value) {}
    TreeState(const TreeState&) = delete;
    TreeState& operator=(const TreeState&) = delete;
    ~TreeState() { rich_tree_free(handle); }
    RichTree* handle;
};

inline std::shared_ptr<TreeState> make_tree_state(RichTree* handle) {
    Owned<RichTree, rich_tree_free> guard(handle);
    auto state = std::make_shared<TreeState>(handle);
    guard.release();
    return state;
}

class Buffer {
public:
    Buffer() noexcept = default;
    Buffer(const Buffer&) = delete;
    Buffer& operator=(const Buffer&) = delete;
    ~Buffer() { rich_buffer_free(&value_); }
    RichBuffer* out() noexcept { return &value_; }
    std::string copy() const {
        return value_.len == 0
            ? std::string()
            : std::string(reinterpret_cast<const char*>(value_.data),
                          static_cast<std::size_t>(value_.len));
    }
private:
    RichBuffer value_{nullptr, 0};
};

template <typename F>
inline std::string buffered(F&& call) {
    Buffer buffer;
    check(call(buffer.out()));
    return buffer.copy();
}

} // namespace detail

enum class ColorSystem : std::uint32_t {
    None = RICH_COLOR_NONE, Standard = RICH_COLOR_STANDARD,
    EightBit = RICH_COLOR_EIGHT_BIT, TrueColor = RICH_COLOR_TRUECOLOR,
    Windows = RICH_COLOR_WINDOWS,
};
enum class Align : std::uint32_t {
    Left = RICH_ALIGN_LEFT, Center = RICH_ALIGN_CENTER, Right = RICH_ALIGN_RIGHT,
};
enum class VAlign : std::uint32_t {
    Top = RICH_VERTICAL_TOP, Middle = RICH_VERTICAL_MIDDLE, Bottom = RICH_VERTICAL_BOTTOM,
};
enum class Justify : std::uint32_t {
    Default = RICH_JUSTIFY_DEFAULT, Left = RICH_JUSTIFY_LEFT,
    Center = RICH_JUSTIFY_CENTER, Right = RICH_JUSTIFY_RIGHT, Full = RICH_JUSTIFY_FULL,
};
enum class Box : std::uint32_t {
    Rounded = RICH_BOX_ROUNDED, Heavy = RICH_BOX_HEAVY, Double = RICH_BOX_DOUBLE,
    Ascii = RICH_BOX_ASCII, Minimal = RICH_BOX_MINIMAL, Square = RICH_BOX_SQUARE,
    Simple = RICH_BOX_SIMPLE, HeavyHead = RICH_BOX_HEAVY_HEAD,
    HeavyEdge = RICH_BOX_HEAVY_EDGE, DoubleEdge = RICH_BOX_DOUBLE_EDGE,
    Horizontals = RICH_BOX_HORIZONTALS, MinimalHeavyHead = RICH_BOX_MINIMAL_HEAVY_HEAD,
    SimpleHeavy = RICH_BOX_SIMPLE_HEAVY, Markdown = RICH_BOX_MARKDOWN,
};

class Renderable {
public:
    explicit Renderable(RichRenderable* handle) noexcept : handle_(handle) {}
    RichRenderable* get() const noexcept { return handle_.get(); }
    explicit operator bool() const noexcept { return bool(handle_); }
private:
    detail::Owned<RichRenderable, rich_renderable_free> handle_;
};

class Console {
public:
    Console() {
        detail::ensure_abi();
        detail::check(rich_console_new(handle_.address()));
    }
    RichConsole* get() const noexcept { return handle_.get(); }
    Console& set_force_terminal(bool value) {
        detail::check(rich_console_set_force_terminal(get(), value ? 1 : 0)); return *this;
    }
    Console& set_size(std::uint32_t width, std::uint32_t height) {
        detail::check(rich_console_set_size(get(), width, height)); return *this;
    }
    Console& set_color_system(ColorSystem value) {
        detail::check(rich_console_set_color_system(get(), static_cast<std::uint32_t>(value))); return *this;
    }
    Console& set_markup_enabled(bool value) {
        detail::check(rich_console_set_markup_enabled(get(), value ? 1 : 0)); return *this;
    }
    Console& set_emoji_enabled(bool value) {
        detail::check(rich_console_set_emoji_enabled(get(), value ? 1 : 0)); return *this;
    }
    std::uint32_t width() const {
        std::uint32_t value = 0; detail::check(rich_console_width(get(), &value)); return value;
    }
    std::string render_markup(std::string_view markup) {
        return detail::buffered([&](RichBuffer* out) { return rich_console_render_markup(get(), detail::str(markup), out); });
    }
    std::string render(const Renderable& renderable) {
        return detail::buffered([&](RichBuffer* out) { return rich_console_render(get(), renderable.get(), out); });
    }
private:
    detail::Owned<RichConsole, rich_console_free> handle_;
};

class Style {
public:
    explicit Style(std::string_view value) {
        detail::ensure_abi();
        detail::check(rich_style_parse(detail::str(value), handle_.address()));
    }
    RichStyle* get() const noexcept { return handle_.get(); }
private:
    detail::Owned<RichStyle, rich_style_free> handle_;
};

#define RICH_FINISH_METHOD(Class, Handle, Native) \
    Renderable finish() { \
        RichRenderable* out = nullptr; \
        detail::check(Native(handle_.address(), &out)); \
        return Renderable(out); \
    }

class Text {
public:
    explicit Text(std::string_view value) {
        detail::ensure_abi(); detail::check(rich_text_new(detail::str(value), handle_.address()));
    }
    static Text markup(Console& console, std::string_view value) {
        detail::ensure_abi();
        RichText* out = nullptr;
        detail::check(rich_text_new_markup(console.get(), detail::str(value), &out));
        return Text(out);
    }
    Text& set_style(std::string_view value) {
        detail::check(rich_text_set_style(get(), detail::str(value))); return *this;
    }
    RichText* get() const noexcept { return handle_.get(); }
    RICH_FINISH_METHOD(Text, RichText, rich_text_finish)
private:
    explicit Text(RichText* handle) noexcept : handle_(handle) {}
    detail::Owned<RichText, rich_text_free> handle_;
};

class Table {
public:
    Table() { detail::ensure_abi(); detail::check(rich_table_new(handle_.address())); }
    RichTable* get() const noexcept { return handle_.get(); }
    Table& set_title(std::string_view v) { detail::check(rich_table_set_title(get(), detail::str(v))); return *this; }
    Table& set_caption(std::string_view v) { detail::check(rich_table_set_caption(get(), detail::str(v))); return *this; }
    Table& set_title_markup(Console& c, std::string_view v) { detail::check(rich_table_set_title_markup(get(), c.get(), detail::str(v))); return *this; }
    Table& set_caption_markup(Console& c, std::string_view v) { detail::check(rich_table_set_caption_markup(get(), c.get(), detail::str(v))); return *this; }
    Table& set_box(Box v) { detail::check(rich_table_set_box(get(), static_cast<std::uint32_t>(v))); return *this; }
    Table& set_show_header(bool v) { detail::check(rich_table_set_show_header(get(), v ? 1 : 0)); return *this; }
    Table& set_show_lines(bool v) { detail::check(rich_table_set_show_lines(get(), v ? 1 : 0)); return *this; }
    Table& set_show_edge(bool v) { detail::check(rich_table_set_show_edge(get(), v ? 1 : 0)); return *this; }
    Table& set_expand(bool v) { detail::check(rich_table_set_expand(get(), v ? 1 : 0)); return *this; }
    Table& set_padding(std::uint32_t left, std::uint32_t right) { detail::check(rich_table_set_padding(get(), left, right)); return *this; }
    Table& set_style(std::string_view v) { detail::check(rich_table_set_style(get(), detail::str(v))); return *this; }
    Table& add_column(std::string_view v) { detail::check(rich_table_add_column(get(), detail::str(v))); return *this; }
    Table& add_column(const Renderable& v) { detail::check(rich_table_add_column_renderable(get(), v.get())); return *this; }
    Table& add_row(const std::vector<std::string>& cells) {
        std::vector<RichStr> raw; raw.reserve(cells.size());
        for (const auto& cell : cells) raw.push_back(detail::str(cell));
        detail::check(rich_table_add_row_strs(get(), raw.data(), detail::count(raw.size()))); return *this;
    }
    Table& add_row(std::initializer_list<const char*> cells) {
        std::vector<std::string> values; values.reserve(cells.size());
        for (const char* cell : cells) {
            values.emplace_back(cell);
        }
        return add_row(values);
    }
    Table& add_row(const std::vector<const Renderable*>& cells) {
        std::vector<const RichRenderable*> raw; raw.reserve(cells.size());
        for (const auto* cell : cells) raw.push_back(cell ? cell->get() : nullptr);
        detail::check(rich_table_add_row_renderables(get(), raw.data(), detail::count(raw.size()))); return *this;
    }
    RICH_FINISH_METHOD(Table, RichTable, rich_table_finish)
private:
    detail::Owned<RichTable, rich_table_free> handle_;
};

class Panel {
public:
    explicit Panel(const Renderable& content) {
        detail::ensure_abi(); detail::check(rich_panel_new(content.get(), handle_.address()));
    }
    Panel& set_title(std::string_view v) { detail::check(rich_panel_set_title(handle_.get(), detail::str(v))); return *this; }
    Panel& set_subtitle(std::string_view v) { detail::check(rich_panel_set_subtitle(handle_.get(), detail::str(v))); return *this; }
    Panel& set_box(Box v) { detail::check(rich_panel_set_box(handle_.get(), static_cast<std::uint32_t>(v))); return *this; }
    Panel& set_expand(bool v) { detail::check(rich_panel_set_expand(handle_.get(), v ? 1 : 0)); return *this; }
    Panel& set_width(std::uint32_t v) { detail::check(rich_panel_set_width(handle_.get(), v)); return *this; }
    Panel& set_padding(std::uint32_t t, std::uint32_t r, std::uint32_t b, std::uint32_t l) { detail::check(rich_panel_set_padding(handle_.get(), t, r, b, l)); return *this; }
    Panel& set_style(std::string_view v) { detail::check(rich_panel_set_style(handle_.get(), detail::str(v))); return *this; }
    Panel& set_border_style(std::string_view v) { detail::check(rich_panel_set_border_style(handle_.get(), detail::str(v))); return *this; }
    RICH_FINISH_METHOD(Panel, RichPanel, rich_panel_finish)
private:
    detail::Owned<RichPanel, rich_panel_free> handle_;
};

class Tree;
class TreeNode {
public:
    std::uint64_t id() const noexcept { return id_; }
    TreeNode add(std::string_view label) const;
    TreeNode add(const Renderable& label) const;
private:
    friend class Tree;
    TreeNode(const std::shared_ptr<detail::TreeState>& owner, std::uint64_t id) noexcept
        : owner_(owner), id_(id) {}
    std::weak_ptr<detail::TreeState> owner_;
    std::uint64_t id_;
};

class Tree {
public:
    explicit Tree(std::string_view label) {
        detail::ensure_abi();
        RichTree* out = nullptr;
        detail::check(rich_tree_new(detail::str(label), &out));
        state_ = detail::make_tree_state(out);
    }
    static Tree renderable(const Renderable& label) {
        detail::ensure_abi();
        RichTree* out = nullptr; detail::check(rich_tree_new_renderable(label.get(), &out)); return Tree(out);
    }
    Tree(Tree&&) noexcept = default;
    Tree& operator=(Tree&&) noexcept = default;
    Tree(const Tree&) = delete;
    Tree& operator=(const Tree&) = delete;
    TreeNode add(std::string_view label) { return add_at(RICH_TREE_ROOT_ID, label); }
    TreeNode add(const Renderable& label) { return add_at(RICH_TREE_ROOT_ID, label); }
    TreeNode add_at(std::uint64_t parent, std::string_view label) {
        auto state = require_state();
        std::uint64_t id = 0;
        detail::check(rich_tree_add(state->handle, parent, detail::str(label), &id));
        return TreeNode(state, id);
    }
    TreeNode add_at(std::uint64_t parent, const Renderable& label) {
        auto state = require_state();
        std::uint64_t id = 0;
        detail::check(rich_tree_add_renderable(state->handle, parent, label.get(), &id));
        return TreeNode(state, id);
    }
    Tree& set_style(std::string_view v) { detail::check(rich_tree_set_style(require_state()->handle, detail::str(v))); return *this; }
    Tree& set_guide_style(std::string_view v) { detail::check(rich_tree_set_guide_style(require_state()->handle, detail::str(v))); return *this; }
    Tree& set_hide_root(bool v) { detail::check(rich_tree_set_hide_root(require_state()->handle, v ? 1 : 0)); return *this; }
    Renderable finish() {
        auto state = require_state();
        RichRenderable* out = nullptr;
        detail::check(rich_tree_finish(&state->handle, &out));
        return Renderable(out);
    }
private:
    explicit Tree(RichTree* handle) : state_(detail::make_tree_state(handle)) {}
    std::shared_ptr<detail::TreeState> require_state() const {
        if (!state_ || !state_->handle) throw Exception(RICH_ERROR_INVALID_STATE);
        return state_;
    }
    std::shared_ptr<detail::TreeState> state_;
};

inline TreeNode TreeNode::add(std::string_view label) const {
    auto owner = owner_.lock();
    if (!owner || !owner->handle) throw Exception(RICH_ERROR_INVALID_STATE);
    std::uint64_t id = 0;
    detail::check(rich_tree_add(owner->handle, id_, detail::str(label), &id));
    return TreeNode(owner, id);
}
inline TreeNode TreeNode::add(const Renderable& label) const {
    auto owner = owner_.lock();
    if (!owner || !owner->handle) throw Exception(RICH_ERROR_INVALID_STATE);
    std::uint64_t id = 0;
    detail::check(rich_tree_add_renderable(owner->handle, id_, label.get(), &id));
    return TreeNode(owner, id);
}

class Rule {
public:
    Rule() { detail::ensure_abi(); detail::check(rich_rule_new(handle_.address())); }
    Rule& set_title(std::string_view v) { detail::check(rich_rule_set_title(handle_.get(), detail::str(v))); return *this; }
    Rule& set_characters(std::string_view v) { detail::check(rich_rule_set_characters(handle_.get(), detail::str(v))); return *this; }
    Rule& set_style(std::string_view v) { detail::check(rich_rule_set_style(handle_.get(), detail::str(v))); return *this; }
    Rule& set_align(Align v) { detail::check(rich_rule_set_align(handle_.get(), static_cast<std::uint32_t>(v))); return *this; }
    RICH_FINISH_METHOD(Rule, RichRule, rich_rule_finish)
private:
    detail::Owned<RichRule, rich_rule_free> handle_;
};

class Columns {
public:
    Columns() { detail::ensure_abi(); detail::check(rich_columns_new(handle_.address())); }
    Columns& add(const Renderable& v) { detail::check(rich_columns_add(handle_.get(), v.get())); return *this; }
    Columns& add(std::string_view v) { detail::check(rich_columns_add_str(handle_.get(), detail::str(v))); return *this; }
    Columns& set_equal(bool v) { detail::check(rich_columns_set_equal(handle_.get(), v ? 1 : 0)); return *this; }
    Columns& set_expand(bool v) { detail::check(rich_columns_set_expand(handle_.get(), v ? 1 : 0)); return *this; }
    Columns& set_padding(std::uint32_t v, std::uint32_t h) { detail::check(rich_columns_set_padding(handle_.get(), v, h)); return *this; }
    RICH_FINISH_METHOD(Columns, RichColumns, rich_columns_finish)
private:
    detail::Owned<RichColumns, rich_columns_free> handle_;
};

class AlignBox {
public:
    AlignBox(const Renderable& content, Align align) {
        detail::ensure_abi(); detail::check(rich_align_new(content.get(), static_cast<std::uint32_t>(align), handle_.address()));
    }
    static AlignBox center(const Renderable& content) {
        detail::ensure_abi();
        RichAlign* out = nullptr; detail::check(rich_align_center(content.get(), &out)); return AlignBox(out);
    }
    AlignBox& set_vertical(VAlign v) { detail::check(rich_align_set_vertical(handle_.get(), static_cast<std::uint32_t>(v))); return *this; }
    AlignBox& set_width(std::uint32_t v) { detail::check(rich_align_set_width(handle_.get(), v)); return *this; }
    RICH_FINISH_METHOD(AlignBox, RichAlign, rich_align_finish)
private:
    explicit AlignBox(RichAlign* handle) noexcept : handle_(handle) {}
    detail::Owned<RichAlign, rich_align_free> handle_;
};

class Padding {
public:
    Padding(const Renderable& content, std::uint32_t t, std::uint32_t r, std::uint32_t b, std::uint32_t l) {
        detail::ensure_abi(); detail::check(rich_padding_new(content.get(), t, r, b, l, handle_.address()));
    }
    Padding& set_style(std::string_view v) { detail::check(rich_padding_set_style(handle_.get(), detail::str(v))); return *this; }
    Padding& set_expand(bool v) { detail::check(rich_padding_set_expand(handle_.get(), v ? 1 : 0)); return *this; }
    RICH_FINISH_METHOD(Padding, RichPadding, rich_padding_finish)
private:
    detail::Owned<RichPadding, rich_padding_free> handle_;
};

class Syntax {
public:
    Syntax(std::string_view code, std::string_view lexer) {
        detail::ensure_abi(); detail::check(rich_syntax_new(detail::str(code), detail::str(lexer), handle_.address()));
    }
    static Syntax from_path(std::string_view path) {
        detail::ensure_abi();
        RichSyntax* out = nullptr; detail::check(rich_syntax_from_path(detail::str(path), &out)); return Syntax(out);
    }
    Syntax& set_theme(std::string_view v) { detail::check(rich_syntax_set_theme(handle_.get(), detail::str(v))); return *this; }
    Syntax& set_line_numbers(bool v) { detail::check(rich_syntax_set_line_numbers(handle_.get(), v ? 1 : 0)); return *this; }
    Syntax& set_word_wrap(bool v) { detail::check(rich_syntax_set_word_wrap(handle_.get(), v ? 1 : 0)); return *this; }
    Syntax& set_line_range(bool hs, std::uint32_t s, bool he, std::uint32_t e) { detail::check(rich_syntax_set_line_range(handle_.get(), hs ? 1 : 0, s, he ? 1 : 0, e)); return *this; }
    RICH_FINISH_METHOD(Syntax, RichSyntax, rich_syntax_finish)
private:
    explicit Syntax(RichSyntax* handle) noexcept : handle_(handle) {}
    detail::Owned<RichSyntax, rich_syntax_free> handle_;
};

class Markdown {
public:
    explicit Markdown(std::string_view source) { detail::ensure_abi(); detail::check(rich_markdown_new(detail::str(source), handle_.address())); }
    Markdown& set_code_theme(std::string_view v) { detail::check(rich_markdown_set_code_theme(handle_.get(), detail::str(v))); return *this; }
    Markdown& set_hyperlinks(bool v) { detail::check(rich_markdown_set_hyperlinks(handle_.get(), v ? 1 : 0)); return *this; }
    Markdown& set_justify(Justify v) { detail::check(rich_markdown_set_justify(handle_.get(), static_cast<std::uint32_t>(v))); return *this; }
    RICH_FINISH_METHOD(Markdown, RichMarkdown, rich_markdown_finish)
private:
    detail::Owned<RichMarkdown, rich_markdown_free> handle_;
};

class Json {
public:
    Json(std::string_view data, std::uint32_t indent = 2, bool highlight = true, bool sort = false) {
        detail::ensure_abi(); detail::check(rich_json_new(detail::str(data), indent, highlight ? 1 : 0, sort ? 1 : 0, handle_.address()));
    }
    RICH_FINISH_METHOD(Json, RichJson, rich_json_finish)
private:
    detail::Owned<RichJson, rich_json_free> handle_;
};

class Progress {
public:
    Progress() { detail::ensure_abi(); detail::check(rich_progress_new(handle_.address())); }
    std::uint64_t add_task(std::string_view description, double total) {
        std::uint64_t id = 0; detail::check(rich_progress_add_task(handle_.get(), detail::str(description), total, &id)); return id;
    }
    Progress& update(std::uint64_t id, double completed) { detail::check(rich_progress_update(handle_.get(), id, completed)); return *this; }
    std::string render_frame(Console& console) {
        return detail::buffered([&](RichBuffer* out) { return rich_progress_render_frame(handle_.get(), console.get(), out); });
    }
private:
    detail::Owned<RichProgress, rich_progress_free> handle_;
};

class Status {
public:
    explicit Status(std::string_view message) { detail::ensure_abi(); detail::check(rich_status_new(detail::str(message), handle_.address())); }
    std::string render_frame(Console& console) {
        return detail::buffered([&](RichBuffer* out) { return rich_status_render_frame(handle_.get(), console.get(), out); });
    }
private:
    detail::Owned<RichStatus, rich_status_free> handle_;
};

class Spinner {
public:
    explicit Spinner(std::string_view name) { detail::ensure_abi(); detail::check(rich_spinner_new(detail::str(name), handle_.address())); }
    std::string render_frame(Console& console, double seconds) {
        return detail::buffered([&](RichBuffer* out) { return rich_spinner_render_frame(handle_.get(), console.get(), seconds, out); });
    }
private:
    detail::Owned<RichSpinner, rich_spinner_free> handle_;
};

#undef RICH_FINISH_METHOD

inline std::string version() {
    detail::ensure_abi();
    return detail::buffered([](RichBuffer* out) { return rich_ffi_version(out); });
}

} // namespace rich

#endif
