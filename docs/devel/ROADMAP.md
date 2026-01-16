# Rich-rs Development Roadmap

A comprehensive task list for porting Python Rich to Rust. Reference: `/home/msaraiva/dev/mark/Proj/Libs/rich`

---

## Phase 1: Foundation

### 1.1 Color System

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `ColorTriplet` struct (r, g, b) with `hex`, `rgb`, `normalized` properties | `color_triplet.py` | No dependencies, start here |
| Todo | `CELL_WIDTHS` data table (~1000 entries) | `_cell_widths.py:CELL_WIDTHS` | Auto-generated Unicode width table |
| Todo | `Palette` struct with `match(triplet)` method | `palette.py:Palette` | Finds closest color via Euclidean distance |
| Todo | `STANDARD_PALETTE`, `EIGHT_BIT_PALETTE`, `WINDOWS_PALETTE` constants | `_palettes.py` | 16, 256, and Windows 10 palettes |
| Todo | `ColorSystem` enum (Standard, EightBit, TrueColor, Windows) | `color.py:ColorSystem` | IntEnum in Python |
| Todo | `ColorType` enum (Default, Standard, EightBit, TrueColor, Windows) | `color.py:ColorType` | Distinguishes color origin |
| Todo | `Color` struct with `name`, `type`, `number`, `triplet` | `color.py:Color` | NamedTuple in Python |
| Todo | `Color::parse()` - parse "red", "#ff0000", "rgb(255,0,0)", "color(196)" | `color.py:Color.parse` | Uses regex, ~250 named colors in `ANSI_COLOR_NAMES` |
| Todo | `Color::from_ansi()`, `from_triplet()`, `from_rgb()`, `default()` | `color.py:Color.*` | Factory methods |
| Todo | `Color::get_ansi_codes()` - generate SGR escape codes | `color.py:Color.get_ansi_codes` | Cached in Python (@lru_cache) |
| Todo | `Color::downgrade()` - convert to lower color system | `color.py:Color.downgrade` | TrueColor→EightBit→Standard conversion |
| Done | Basic `Color` enum with parse (partial) | `src/color.rs` | Needs full implementation |

### 1.2 Cell Width

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `cell_len()` using unicode-width | `cells.py:cell_len` | Wrapped in `src/cells.rs` |
| Todo | `get_character_cell_size()` - binary search in CELL_WIDTHS | `cells.py:get_character_cell_size` | Returns 0, 1, or 2 |
| Todo | `set_cell_size()` - truncate/pad to exact cell width | `cells.py:set_cell_size` | Handles double-width boundaries |
| Todo | `chop_cells()` - split text into width-limited lines | `cells.py:chop_cells` | For wrapping |
| Todo | Fast path for ASCII (all single-cell) | `cells.py:_is_single_cell_widths` | Optimization using frozenset |

### 1.3 Style

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Style` struct with `Option<bool>` attributes | `src/style.rs` | Basic structure done |
| Todo | Bitfield storage for attributes (_attributes, _set_attributes) | `style.py:Style` | 13 attributes as bits for memory efficiency |
| Todo | `Style::parse()` - parse "bold red on blue" | `style.py:Style.parse` | Cached @lru_cache(4096) in Python |
| Todo | `Style::render()` - generate ANSI escape sequence | `style.py:Style.render` | Core output method |
| Todo | `Style + Style` combination (impl Add) | `style.py:Style.__add__` | Combines styles, other takes precedence |
| Todo | `Style::get_html_style()` for HTML export | `style.py:Style.get_html_style` | CSS generation |
| Todo | Link support (`_link`, `_link_id` fields) | `style.py:Style` | For terminal hyperlinks |
| Todo | Metadata support (`_meta` field, pickled dict) | `style.py:Style` | For Textual events |
| Todo | `NULL_STYLE` singleton | `style.py:NULL_STYLE` | Optimization for empty style |

### 1.4 Segment

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Segment` struct (text, style, control) | `src/segment.rs` | Basic structure done |
| Done | `ControlType` enum | `src/segment.rs` | Basic controls done |
| Todo | Full `ControlType` (16 variants including cursor movement) | `segment.py:ControlType` | CURSOR_UP/DOWN/FORWARD/BACKWARD, ERASE_IN_LINE, etc. |
| Todo | `Segment::split_cells()` - split at cell boundary | `segment.py:Segment.split_cells` | Cached in Python, handles double-width |
| Todo | `Segment::apply_style()` for iterators | `segment.py:Segment.apply_style` | Composes: style + segment.style + post_style |
| Todo | `Segment::split_lines()` - split on newlines | `segment.py:Segment.split_lines` | Generator in Python |
| Todo | `Segment::split_and_crop_lines()` - layout core | `segment.py:Segment.split_and_crop_lines` | Critical for rendering |
| Todo | `Segment::adjust_line_length()` - crop or pad | `segment.py:Segment.adjust_line_length` | Width normalization |
| Todo | `Segment::simplify()` - merge adjacent same-style | `segment.py:Segment.simplify` | Output optimization |
| Todo | `Segment::divide()` - split at cell positions | `segment.py:Segment.divide` | For column layout |
| Todo | `Segments` and `SegmentLines` wrapper types | `segment.py:Segments/SegmentLines` | Simple renderables |

### 1.5 Measurement

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Measurement` struct (minimum, maximum) | `src/measure.rs` | Basic structure done |
| Todo | `Measurement::get()` - delegate to renderable | `measure.py:Measurement.get` | Calls `__rich_measure__` |
| Todo | `measure_renderables()` - combine measurements | `measure.py:measure_renderables` | Takes max of mins/maxs |

---

## Phase 2: Text & Console

### 2.1 Text

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Text` struct with spans | `src/text.rs` | Basic structure done |
| Done | `Span` struct (start, end, style) | `src/text.rs` | Basic structure done |
| Todo | `Span::split()`, `move()`, `right_crop()`, `extend()` | `text.py:Span.*` | Span manipulation |
| Todo | `Text::from_markup()` - parse BBCode | `text.py:Text.from_markup` | Uses markup.py |
| Todo | `Text::from_ansi()` - parse ANSI codes | `text.py:Text.from_ansi` | Reverse rendering |
| Todo | `Text::assemble()` - build from (str, style) pairs | `text.py:Text.assemble` | Common construction pattern |
| Todo | `Text::stylize()`, `stylize_before()` | `text.py:Text.stylize*` | Apply style to range |
| Todo | `Text::highlight_regex()`, `highlight_words()` | `text.py:Text.highlight_*` | Pattern-based styling |
| Todo | `Text::wrap()` - word wrapping with justify | `text.py:Text.wrap` | Complex algorithm |
| Todo | `Text::divide()` - split at offsets | `text.py:Text.divide` | For column layout |
| Todo | `Text::render()` → `Vec<Segment>` | `text.py:Text.render` | Core rendering |
| Todo | `Text::__rich_console__()` trait impl | `text.py:Text.__rich_console__` | Renderable protocol |
| Todo | `Text::__rich_measure__()` trait impl | `text.py:Text.__rich_measure__` | Measurable protocol |

### 2.2 Markup Parser

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Basic `render()` function | `src/markup.rs` | Partial implementation |
| Todo | `Tag` struct (name, parameters) | `markup.py:Tag` | NamedTuple in Python |
| Todo | `_parse()` tokenizer yielding (pos, text, tag) | `markup.py:_parse` | Stack-based parser |
| Todo | `escape()` function | `markup.py:escape` | Escape brackets |
| Todo | Link syntax: `[link=url]text[/link]` | `markup.py` | URL handling |
| Todo | Metadata syntax: `[@name=value]` | `markup.py` | For Textual |
| Todo | Nested tag support with style stacking | `markup.py:render` | Combines styles |

### 2.3 Console

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Basic `Console` struct | `src/console.rs` | Minimal implementation |
| Done | `ConsoleOptions` struct | `src/console.rs` | Basic fields |
| Todo | Full `ConsoleOptions` fields (16+ fields) | `console.py:ConsoleOptions` | justify, overflow, no_wrap, highlight, markup, height |
| Todo | `ConsoleOptions::update()`, `copy()` | `console.py:ConsoleOptions.*` | Immutable updates |
| Todo | Color system detection (auto from TERM) | `console.py:Console._detect_color_system` | Environment-based |
| Todo | `Console::render()` - core render method | `console.py:Console.render` | Calls `__rich_console__` |
| Todo | `Console::render_lines()` - render to line grid | `console.py:Console.render_lines` | Uses split_and_crop_lines |
| Todo | `Console::render_str()` - string to Text | `console.py:Console.render_str` | With markup/emoji/highlight |
| Todo | `Console::print()` - main print method | `console.py:Console.print` | Many parameters |
| Todo | `Console::measure()` - measure renderable | `console.py:Console.measure` | Delegates to Measurement |
| Todo | Theme support (`Theme`, `ThemeStack`) | `console.py` + `theme.py` | Named style definitions |
| Todo | Capture context manager | `console.py:Console.capture` | For testing |
| Todo | Screen/alt screen support | `console.py:Console.screen` | Via crossterm |
| Todo | Pager support | `console.py:Console.pager` | Less-like output |

### 2.4 Protocol

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Renderable` trait | `src/lib.rs` | Basic definition |
| Done | `Measurable` trait | `src/lib.rs` | Basic definition |
| Done | `RichCast` trait | `src/lib.rs` | Basic definition |
| Todo | `is_renderable()` function | `protocol.py:is_renderable` | Check for trait impl |
| Todo | `rich_cast()` function | `protocol.py:rich_cast` | Recursive conversion |

---

## Phase 3: Box Drawing & Simple Renderables

### 3.1 Box Characters

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `BoxChars` struct | `src/box_chars.rs` | Basic structure |
| Done | `ASCII`, `SQUARE`, `ROUNDED`, `HEAVY`, `DOUBLE` constants | `src/box_chars.rs` | 5 of ~20 boxes |
| Todo | Remaining box types (MINIMAL, SIMPLE, MARKDOWN, etc.) | `box.py` | ~15 more variants |
| Todo | `Box::substitute()` - platform compatibility | `box.py:Box.substitute` | Windows legacy fallback |
| Todo | `Box::get_top()`, `get_row()`, `get_bottom()` for tables | `box.py:Box.get_*` | Column-aware borders |
| Todo | `LEGACY_WINDOWS_SUBSTITUTIONS` map | `box.py` | Box fallback mapping |

### 3.2 Rule (Horizontal Line)

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Rule` struct | `rule.py:Rule` | title, characters, style, align |
| Todo | `Rule::__rich_console__()` | `rule.py:Rule.__rich_console__` | Render horizontal line |
| Todo | Title centering with character fill | `rule.py:Rule._rule_line` | CJK-aware width |

### 3.3 Padding

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Padding` struct | `padding.py:Padding` | (top, right, bottom, left) |
| Todo | `Padding::unpack()` - CSS-style parsing | `padding.py:Padding.unpack` | 1, 2, or 4 values |
| Todo | `Padding::indent()` - left padding helper | `padding.py:Padding.indent` | Common use case |
| Todo | `Padding::__rich_console__()` | `padding.py:Padding.__rich_console__` | Wrap with space |

### 3.4 Align

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Align` struct | `align.py:Align` | horizontal + vertical alignment |
| Todo | `Align::left()`, `center()`, `right()` constructors | `align.py:Align.*` | Convenience methods |
| Todo | `Align::__rich_console__()` | `align.py:Align.__rich_console__` | Pad to width |
| Todo | Vertical alignment (top/middle/bottom) | `align.py:Align` | Height-based |

---

## Phase 4: Complex Renderables

### 4.1 Panel

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Panel` struct | `panel.py:Panel` | box, title, subtitle, padding |
| Todo | `Panel::fit()` - non-expanding variant | `panel.py:Panel.fit` | Constructor |
| Todo | Title/subtitle alignment within border | `panel.py:align_text` | Internal helper |
| Todo | `Panel::__rich_console__()` | `panel.py:Panel.__rich_console__` | ~100 lines in Python |
| Todo | `Panel::__rich_measure__()` | `panel.py:Panel.__rich_measure__` | Account for border |

### 4.2 Tree

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Tree` struct | `tree.py:Tree` | label, children, guide_style |
| Todo | `Tree::add()` - add child node | `tree.py:Tree.add` | Returns child for chaining |
| Todo | Guide constants (ASCII_GUIDES, TREE_GUIDES) | `tree.py` | 4 guide character sets |
| Todo | `Tree::__rich_console__()` | `tree.py:Tree.__rich_console__` | Stack-based traversal |
| Todo | `Tree::__rich_measure__()` | `tree.py:Tree.__rich_measure__` | Recursive measurement |

### 4.3 Table

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Column` struct | `table.py:Column` | header, footer, width, ratio, style, justify |
| Todo | `Row` struct | `table.py:Row` | style, end_section |
| Todo | `Table` struct | `table.py:Table` | columns, rows, box, title, caption |
| Todo | `Table::grid()` - headerless table | `table.py:Table.grid` | Common pattern |
| Todo | `Table::add_column()` | `table.py:Table.add_column` | Column configuration |
| Todo | `Table::add_row()` | `table.py:Table.add_row` | Row with renderables |
| Todo | `_calculate_column_widths()` | `table.py:Table._calculate_column_widths` | Ratio distribution algorithm |
| Todo | `_measure_column()` | `table.py:Table._measure_column` | Per-column measurement |
| Todo | `_render_cell()` | `table.py:Table._render_cell` | Cell with alignment |
| Todo | `Table::__rich_console__()` | `table.py:Table.__rich_console__` | ~300 lines in Python |
| Todo | `Table::__rich_measure__()` | `table.py:Table.__rich_measure__` | Sum column widths |
| Todo | Ratio distribution utilities | `_ratio.py` | ratio_distribute, ratio_reduce |

### 4.4 Columns (Multi-column Layout)

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Columns` struct | `columns.py:Columns` | Uses Table.grid() internally |
| Todo | `Columns::add_renderable()` | `columns.py:Columns.add_renderable` | Add item |
| Todo | Column-first vs row-first iteration | `columns.py:iter_renderables` | Layout strategy |
| Todo | `Columns::__rich_console__()` | `columns.py:Columns.__rich_console__` | Delegates to Table |

---

## Phase 5: Advanced Features (Optional)

### 5.1 Progress System

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `TaskID` newtype | `progress.py:TaskID` | Task identifier |
| Todo | `ProgressTask` struct | `progress.py:ProgressTask` | Task state tracking |
| Todo | `ProgressColumn` trait | `progress.py:ProgressColumn` | Abstract column |
| Todo | `BarColumn` - visual bar | `progress.py:BarColumn` | ASCII/Unicode bars |
| Todo | `TextColumn` - text with markup | `progress.py:TextColumn` | Description |
| Todo | `SpinnerColumn` - animated spinner | `progress.py:SpinnerColumn` | Uses spinner.py |
| Todo | `TimeRemainingColumn` - ETA | `progress.py:TimeRemainingColumn` | Speed calculation |
| Todo | `Progress` struct | `progress.py:Progress` | Task management |
| Todo | `Progress::add_task()`, `update()`, `advance()` | `progress.py:Progress.*` | Task lifecycle |
| Todo | `Progress::track()` - iterate with progress | `progress.py:Progress.track` | Common pattern |
| Todo | Live updating (requires Live) | `progress.py` + `live.py` | Threading |
| Todo | `track()` module function | `progress.py:track` | Simple API |

### 5.2 Live Display

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Live` struct | `live.py:Live` | Real-time updates |
| Todo | Refresh loop (async or threaded) | `live.py:Live._refresh_thread` | Background updates |
| Todo | `Live::update()` | `live.py:Live.update` | Change renderable |
| Todo | Transient mode | `live.py:Live` | Clear on exit |

### 5.3 Syntax Highlighting

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Syntax` struct | `syntax.py:Syntax` | Code highlighting |
| Todo | Integration with syntect or tree-sitter | N/A | Rust-native highlighting |
| Todo | Line numbers | `syntax.py:Syntax` | Gutter rendering |
| Todo | Theme support | `syntax.py` | Color schemes |

### 5.4 Pretty Printing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Pretty` struct | `pretty.py:Pretty` | Value formatting |
| Todo | Recursive object rendering | `pretty.py` | Dict, list, struct |
| Todo | Depth limiting | `pretty.py:Pretty` | max_depth parameter |
| Todo | `pprint()` function | `pretty.py:pprint` | Simple API |

### 5.5 Traceback

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Traceback` struct | `traceback.py:Traceback` | Error formatting |
| Todo | Source code context | `traceback.py` | Show surrounding lines |
| Todo | Syntax highlighting of code | `traceback.py` | Uses Syntax |
| Todo | `install()` for panic hook | N/A | Rust-specific |

---

## Utilities & Helpers

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `loop_first`, `loop_last`, `loop_first_last` | `_loop.py` | Iterator helpers |
| Todo | `pick_bool` | `_pick.py` | Boolean selection |
| Todo | `ratio_distribute`, `ratio_reduce` | `_ratio.py` | Width distribution |
| Todo | `Constrain` wrapper | `constrain.py` | Width constraint |
| Todo | `Styled` wrapper | `styled.py` | Apply style to any renderable |
| Todo | `Control` for escape sequences | `control.py` | Terminal control codes |
| Todo | `Emoji` lookup | `emoji.py` + `_emoji_codes.py` | :name: → character |
| Todo | `Highlighter` base class | `highlighter.py` | Regex-based highlighting |
| Todo | `Spinner` animations | `spinner.py` + `_spinners.py` | Animation frames |
| Todo | `ProgressBar` visual bar | `progress_bar.py` | Bar rendering |
| Todo | `filesize` formatting | `filesize.py` | Human-readable sizes |

---

## Legend

| Status | Meaning |
|--------|---------|
| Todo | Not started |
| In Progress | Currently being implemented |
| Done | Implemented and tested |

---

## Quick Reference: Python → Rust Patterns

| Python | Rust |
|--------|------|
| `NamedTuple` | `#[derive(Clone, Copy)] struct` |
| `@dataclass` | `#[derive(Clone)] struct` |
| `Protocol` with `__rich_console__` | `trait Renderable` |
| `Protocol` with `__rich_measure__` | `trait Measurable` |
| `@lru_cache(N)` | `once_cell::Lazy` or memoization |
| `Optional[bool]` tri-state | `Option<bool>` |
| `Iterable[Segment]` | `impl Iterator<Item=Segment>` or `Vec<Segment>` |
| `Union[str, Style]` | `impl Into<Style>` or enum |
| Thread-local storage | `thread_local!` macro |
| Generator | Iterator with state struct |
