// rich.hpp — header-only C++ RAII wrappers over the rich-rs C ABI (rich.h).
//
// Ownership model:
//   * Builder handles (Text, Table, Panel, Tree, Rule, Columns, AlignBox,
//     Padding, Syntax, Markdown, Json, Style, Progress, Status, Spinner) are
//     MOVE-ONLY RAII owners — they free their handle in the destructor.
//   * `Renderable` is the type-erased composition currency. `finish()` on a
//     builder consumes it and yields a `Renderable`.
//   * Container constructors/methods take `Renderable&&` and CONSUME it (they
//     call release(), so the moved-from wrapper will not double-free).
//   * `Console::render(const Renderable&)` borrows and returns a std::string.
//
// All wrappers are zero-overhead: each holds exactly one raw handle.
#ifndef RICH_FFI_HPP
#define RICH_FFI_HPP

#include <cstdint>
#include <initializer_list>
#include <string>
#include <vector>

#include "rich.h"

namespace rich {

namespace detail {
// Move-only owner of a C handle freed by `Free`.
template <typename H, void (*Free)(H*)>
class Owned {
public:
    Owned() = default;
    explicit Owned(H* h) noexcept : h_(h) {}
    Owned(Owned&& o) noexcept : h_(o.h_) { o.h_ = nullptr; }
    Owned& operator=(Owned&& o) noexcept {
        if (this != &o) {
            if (h_) Free(h_);
            h_ = o.h_;
            o.h_ = nullptr;
        }
        return *this;
    }
    Owned(const Owned&) = delete;
    Owned& operator=(const Owned&) = delete;
    ~Owned() { if (h_) Free(h_); }

    H* get() const noexcept { return h_; }
    H* release() noexcept { H* t = h_; h_ = nullptr; return t; }
    explicit operator bool() const noexcept { return h_ != nullptr; }

private:
    H* h_ = nullptr;
};

inline std::string take_string(char* out) {
    if (!out) return {};
    std::string s(out);
    rich_string_free(out);
    return s;
}
} // namespace detail

/// Color system selector mirroring the C ABI integer codes.
enum class ColorSystem : int { None = 0, Standard = 1, EightBit = 2, TrueColor = 3, Windows = 4 };
/// Horizontal alignment codes (Rule/Align): 0=Left, 1=Center, 2=Right.
enum class Align_ : int { Left = 0, Center = 1, Right = 2 };
/// Vertical alignment codes (Align): 0=Top, 1=Middle, 2=Bottom.
enum class VAlign : int { Top = 0, Middle = 1, Bottom = 2 };
/// Justify codes (Markdown): 0=Default, 1=Left, 2=Center, 3=Right, 4=Full.
enum class Justify : int { Default = 0, Left = 1, Center = 2, Right = 3, Full = 4 };
/// Box-drawing style codes (Table/Panel); see common::box_ids in the Rust source.
enum class Box : int {
    Rounded = 0, Heavy = 1, Double = 2, Ascii = 3, Minimal = 4, Square = 5,
    Simple = 6, HeavyHead = 7, HeavyEdge = 8, DoubleEdge = 9, Horizontals = 10,
    MinimalHeavyHead = 11, SimpleHeavy = 12, Markdown = 13,
};

/// Type-erased renderable handle — the composition currency. Move-only.
class Renderable {
public:
    explicit Renderable(RichRenderable* h) noexcept : h_(h) {}
    RichRenderable* get() const noexcept { return h_.get(); }
    RichRenderable* release() noexcept { return h_.release(); }
    explicit operator bool() const noexcept { return bool(h_); }
private:
    detail::Owned<RichRenderable, rich_renderable_free> h_;
};

/// A parsed style (e.g. "bold red on white"). `parse` returns an empty handle on
/// failure — check with `operator bool`.
class Style {
public:
    static Style parse(const char* s) { return Style(rich_style_parse(s)); }
    RichStyle* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }
private:
    explicit Style(RichStyle* h) noexcept : h_(h) {}
    detail::Owned<RichStyle, rich_style_free> h_;
};

/// The console: configure it, then render markup or any Renderable to a string.
class Console {
public:
    Console() : h_(rich_console_new()) {}
    RichConsole* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }

    /// Pass isatty(): true emits ANSI styling, false emits plain text.
    Console& set_force_terminal(bool force) noexcept { rich_console_set_force_terminal(get(), force); return *this; }
    Console& set_size(std::uint32_t w, std::uint32_t h) noexcept { rich_console_set_size(get(), w, h); return *this; }
    Console& set_color_system(ColorSystem s) noexcept { rich_console_set_color_system(get(), static_cast<int>(s)); return *this; }
    Console& set_markup_enabled(bool e) noexcept { rich_console_set_markup_enabled(get(), e); return *this; }
    Console& set_emoji_enabled(bool e) noexcept { rich_console_set_emoji_enabled(get(), e); return *this; }
    std::uint32_t width() const noexcept { return rich_console_width(get()); }

    /// Render a markup string to a styled std::string (empty on failure).
    std::string render_markup(const char* markup) const { return detail::take_string(rich_console_render_markup(get(), markup)); }
    std::string render_markup(const std::string& m) const { return render_markup(m.c_str()); }
    /// Render any Renderable to a std::string (borrows; empty on failure).
    std::string render(const Renderable& r) const { return detail::take_string(rich_console_render(get(), r.get())); }

private:
    detail::Owned<RichConsole, rich_console_free> h_;
};

/// Styled-text builder.
class Text {
public:
    explicit Text(const char* s) : h_(rich_text_new(s)) {}
    /// Parse console markup (e.g. "[bold]hi[/]") into a Text.
    static Text markup(const Console& con, const char* m) { return Text(rich_text_new_markup(con.get(), m)); }
    Text& set_style(const char* style) { rich_text_set_style(get(), style); return *this; }
    RichText* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }
    Renderable finish() { return Renderable(rich_text_finish(h_.release())); }
private:
    explicit Text(RichText* h) noexcept : h_(h) {}
    detail::Owned<RichText, rich_text_free> h_;
};

/// Table builder. add_row consumes Renderable cells; finish() yields a Renderable.
class Table {
public:
    Table() : h_(rich_table_new()) {}
    RichTable* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }

    Table& set_title(const char* s) { rich_table_set_title(get(), s); return *this; }
    Table& set_title_markup(const Console& c, const char* m) { rich_table_set_title_markup(get(), c.get(), m); return *this; }
    Table& set_caption(const char* s) { rich_table_set_caption(get(), s); return *this; }
    Table& set_caption_markup(const Console& c, const char* m) { rich_table_set_caption_markup(get(), c.get(), m); return *this; }
    Table& set_box(Box b) { rich_table_set_box(get(), static_cast<int>(b)); return *this; }
    Table& set_show_header(bool v) { rich_table_set_show_header(get(), v); return *this; }
    Table& set_show_lines(bool v) { rich_table_set_show_lines(get(), v); return *this; }
    Table& set_show_edge(bool v) { rich_table_set_show_edge(get(), v); return *this; }
    Table& set_expand(bool v) { rich_table_set_expand(get(), v); return *this; }
    Table& set_padding(std::uint32_t l, std::uint32_t r) { rich_table_set_padding(get(), l, r); return *this; }
    Table& set_style(const char* s) { rich_table_set_style(get(), s); return *this; }

    Table& add_column(const char* header) { rich_table_add_column(get(), header); return *this; }
    Table& add_column(Renderable&& header) { rich_table_add_column_renderable(get(), header.release()); return *this; }
    Table& add_row(std::initializer_list<const char*> cells) {
        std::vector<const char*> v(cells);
        rich_table_add_row_strs(get(), v.data(), static_cast<unsigned int>(v.size()));
        return *this;
    }
    /// Consumes each Renderable cell.
    Table& add_row(std::vector<Renderable>&& cells) {
        std::vector<RichRenderable*> raw;
        raw.reserve(cells.size());
        for (auto& c : cells) raw.push_back(c.release());
        rich_table_add_row_renderables(get(), raw.data(), static_cast<unsigned int>(raw.size()));
        return *this;
    }
    Renderable finish() { return Renderable(rich_table_finish(h_.release())); }
private:
    detail::Owned<RichTable, rich_table_free> h_;
};

/// Panel builder. Constructed around a Renderable it CONSUMES.
class Panel {
public:
    explicit Panel(Renderable&& content) : h_(rich_panel_new(content.release())) {}
    RichPanel* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }
    Panel& set_title(const char* s) { rich_panel_set_title(get(), s); return *this; }
    Panel& set_subtitle(const char* s) { rich_panel_set_subtitle(get(), s); return *this; }
    Panel& set_box(Box b) { rich_panel_set_box(get(), static_cast<int>(b)); return *this; }
    Panel& set_expand(bool v) { rich_panel_set_expand(get(), v); return *this; }
    Panel& set_width(std::uint32_t w) { rich_panel_set_width(get(), w); return *this; }
    Panel& set_padding(std::uint32_t t, std::uint32_t r, std::uint32_t b, std::uint32_t l) { rich_panel_set_padding(get(), t, r, b, l); return *this; }
    Panel& set_style(const char* s) { rich_panel_set_style(get(), s); return *this; }
    Panel& set_border_style(const char* s) { rich_panel_set_border_style(get(), s); return *this; }
    Renderable finish() { return Renderable(rich_panel_finish(h_.release())); }
private:
    detail::Owned<RichPanel, rich_panel_free> h_;
};

/// Non-owning borrow into a child node of a Tree. Valid only while the owning
/// Tree is alive; never freed (see rich.h RichTreeNode contract).
class TreeNode {
public:
    explicit TreeNode(RichTreeNode* h) noexcept : h_(h) {}
    RichTreeNode* get() const noexcept { return h_; }
    explicit operator bool() const noexcept { return h_ != nullptr; }
    TreeNode add(const char* label) { return TreeNode(rich_tree_node_add(h_, label)); }
    TreeNode add(Renderable&& label) { return TreeNode(rich_tree_node_add_renderable(h_, label.release())); }
private:
    RichTreeNode* h_;
};

/// Tree builder.
class Tree {
public:
    explicit Tree(const char* label) : h_(rich_tree_new(label)) {}
    static Tree renderable(Renderable&& label) { return Tree(rich_tree_new_renderable(label.release())); }
    RichTree* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }
    TreeNode add(const char* label) { return TreeNode(rich_tree_add(get(), label)); }
    TreeNode add(Renderable&& label) { return TreeNode(rich_tree_add_renderable(get(), label.release())); }
    Tree& set_style(const char* s) { rich_tree_set_style(get(), s); return *this; }
    Tree& set_guide_style(const char* s) { rich_tree_set_guide_style(get(), s); return *this; }
    Tree& set_hide_root(bool v) { rich_tree_set_hide_root(get(), v); return *this; }
    Renderable finish() { return Renderable(rich_tree_finish(h_.release())); }
private:
    explicit Tree(RichTree* h) noexcept : h_(h) {}
    detail::Owned<RichTree, rich_tree_free> h_;
};

/// Horizontal rule (divider).
class Rule {
public:
    Rule() : h_(rich_rule_new()) {}
    RichRule* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }
    Rule& set_title(const char* s) { rich_rule_set_title(get(), s); return *this; }
    Rule& set_characters(const char* s) { rich_rule_set_characters(get(), s); return *this; }
    Rule& set_style(const char* s) { rich_rule_set_style(get(), s); return *this; }
    Rule& set_align(Align_ a) { rich_rule_set_align(get(), static_cast<int>(a)); return *this; }
    Renderable finish() { return Renderable(rich_rule_finish(h_.release())); }
private:
    detail::Owned<RichRule, rich_rule_free> h_;
};

/// Multi-column layout. add() consumes Renderables.
class Columns {
public:
    Columns() : h_(rich_columns_new()) {}
    RichColumns* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }
    Columns& add(Renderable&& r) { rich_columns_add(get(), r.release()); return *this; }
    Columns& add(const char* s) { rich_columns_add_str(get(), s); return *this; }
    Columns& set_equal(bool v) { rich_columns_set_equal(get(), v); return *this; }
    Columns& set_expand(bool v) { rich_columns_set_expand(get(), v); return *this; }
    Columns& set_padding(std::uint32_t vert, std::uint32_t horiz) { rich_columns_set_padding(get(), vert, horiz); return *this; }
    Renderable finish() { return Renderable(rich_columns_finish(h_.release())); }
private:
    detail::Owned<RichColumns, rich_columns_free> h_;
};

/// Alignment wrapper. Constructed around a Renderable it CONSUMES.
class AlignBox {
public:
    AlignBox(Renderable&& content, Align_ h) : h_(rich_align_new(content.release(), static_cast<int>(h))) {}
    static AlignBox center(Renderable&& content) { return AlignBox(rich_align_center(content.release())); }
    RichAlign* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }
    AlignBox& set_vertical(VAlign v) { rich_align_set_vertical(get(), static_cast<int>(v)); return *this; }
    AlignBox& set_width(std::uint32_t w) { rich_align_set_width(get(), w); return *this; }
    Renderable finish() { return Renderable(rich_align_finish(h_.release())); }
private:
    explicit AlignBox(RichAlign* h) noexcept : h_(h) {}
    detail::Owned<RichAlign, rich_align_free> h_;
};

/// Padding wrapper. Constructed around a Renderable it CONSUMES.
class Padding {
public:
    Padding(Renderable&& content, std::uint32_t t, std::uint32_t r, std::uint32_t b, std::uint32_t l)
        : h_(rich_padding_new(content.release(), t, r, b, l)) {}
    RichPadding* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }
    Padding& set_style(const char* s) { rich_padding_set_style(get(), s); return *this; }
    Padding& set_expand(bool v) { rich_padding_set_expand(get(), v); return *this; }
    Renderable finish() { return Renderable(rich_padding_finish(h_.release())); }
private:
    detail::Owned<RichPadding, rich_padding_free> h_;
};

/// Syntax-highlighted source code.
class Syntax {
public:
    Syntax(const char* code, const char* lexer) : h_(rich_syntax_new(code, lexer)) {}
    /// Read from a file (auto-detect lexer). Empty handle on IO error.
    static Syntax from_path(const char* path) { return Syntax(rich_syntax_from_path(path)); }
    RichSyntax* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }
    Syntax& set_theme(const char* t) { rich_syntax_set_theme(get(), t); return *this; }
    Syntax& set_line_numbers(bool v) { rich_syntax_set_line_numbers(get(), v); return *this; }
    Syntax& set_word_wrap(bool v) { rich_syntax_set_word_wrap(get(), v); return *this; }
    /// Pass -1 for either bound to leave it unset.
    Syntax& set_line_range(int start, int end) { rich_syntax_set_line_range(get(), start, end); return *this; }
    Renderable finish() { return Renderable(rich_syntax_finish(h_.release())); }
private:
    explicit Syntax(RichSyntax* h) noexcept : h_(h) {}
    detail::Owned<RichSyntax, rich_syntax_free> h_;
};

/// Markdown document.
class Markdown {
public:
    explicit Markdown(const char* source) : h_(rich_markdown_new(source)) {}
    RichMarkdown* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }
    Markdown& set_code_theme(const char* t) { rich_markdown_set_code_theme(get(), t); return *this; }
    Markdown& set_hyperlinks(bool v) { rich_markdown_set_hyperlinks(get(), v); return *this; }
    Markdown& set_justify(Justify j) { rich_markdown_set_justify(get(), static_cast<int>(j)); return *this; }
    Renderable finish() { return Renderable(rich_markdown_finish(h_.release())); }
private:
    detail::Owned<RichMarkdown, rich_markdown_free> h_;
};

/// Pretty-printed, highlighted JSON. Empty handle if `data` is not valid JSON.
class Json {
public:
    Json(const char* data, std::uint32_t indent, bool highlight, bool sort_keys)
        : h_(rich_json_new(data, indent, highlight, sort_keys)) {}
    RichJson* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }
    Renderable finish() { return Renderable(rich_json_finish(h_.release())); }
private:
    detail::Owned<RichJson, rich_json_free> h_;
};

// ── Live widgets (frame-based: the caller owns the loop and cursor) ──────────

/// Multi-task progress display. Drive the loop yourself: update(), then
/// render_frame() per tick.
class Progress {
public:
    Progress() : h_(rich_progress_new()) {}
    RichProgress* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }
    std::uint64_t add_task(const char* description, double total) { return rich_progress_add_task(get(), description, total); }
    void update(std::uint64_t task_id, double completed) { rich_progress_update(get(), task_id, completed); }
    std::string render_frame(const Console& con) const { return detail::take_string(rich_progress_render_frame(get(), con.get())); }
private:
    detail::Owned<RichProgress, rich_progress_free> h_;
};

/// A spinner + status message.
class Status {
public:
    explicit Status(const char* message) : h_(rich_status_new(message)) {}
    RichStatus* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }
    std::string render_frame(const Console& con) const { return detail::take_string(rich_status_render_frame(get(), con.get())); }
private:
    detail::Owned<RichStatus, rich_status_free> h_;
};

/// A standalone spinner. Empty handle if the spinner name is unknown.
class Spinner {
public:
    explicit Spinner(const char* name) : h_(rich_spinner_new(name)) {}
    RichSpinner* get() const noexcept { return h_.get(); }
    explicit operator bool() const noexcept { return bool(h_); }
    std::string render_frame(const Console& con, double time_seconds) const { return detail::take_string(rich_spinner_render_frame(get(), con.get(), time_seconds)); }
private:
    detail::Owned<RichSpinner, rich_spinner_free> h_;
};

} // namespace rich

#endif // RICH_FFI_HPP
