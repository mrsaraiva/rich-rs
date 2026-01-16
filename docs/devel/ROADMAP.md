# Rich-rs Development Roadmap

A comprehensive task list for porting Python Rich to Rust. Reference: `/home/msaraiva/dev/mark/Proj/Libs/rich`

**Note:** Design decisions finalized after Codex CLI review. See `CLAUDE.md` for rationale.

---

## Phase 1: Foundation

### 1.0 Error Handling

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `ParseError` enum in `src/error.rs` | `errors.py` | InvalidColor, InvalidStyle, InvalidMarkup |

### 1.1 Color System

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `ColorTriplet` struct (r, g, b) with `hex`, `rgb`, `normalized` properties | `color_triplet.py` | No dependencies, start here |
| Todo | `Palette` struct with `match(triplet)` method | `palette.py:Palette` | Finds closest color via Euclidean distance |
| Todo | `STANDARD_PALETTE`, `EIGHT_BIT_PALETTE`, `WINDOWS_PALETTE` constants | `_palettes.py` | 16, 256, and Windows 10 palettes |
| Todo | `ColorSystem` enum (Standard, EightBit, TrueColor, Windows) | `color.py:ColorSystem` | IntEnum in Python |
| Todo | `ColorType` enum (Default, Standard, EightBit, TrueColor, Windows) | `color.py:ColorType` | Distinguishes color origin |
| Todo | `Color` struct with `name`, `type`, `number`, `triplet` | `color.py:Color` | NamedTuple in Python |
| Todo | `Color::parse()` - parse "red", "#ff0000", "rgb(255,0,0)", "color(196)" | `color.py:Color.parse` | Uses regex, ~250 named colors in `ANSI_COLOR_NAMES` |
| Todo | `Color::from_ansi()`, `from_triplet()`, `from_rgb()`, `default()` | `color.py:Color.*` | Factory methods |
| Todo | `Color::get_ansi_codes()` - generate SGR escape codes | `color.py:Color.get_ansi_codes` | Use once_cell::Lazy for caching |
| Todo | `Color::downgrade()` - convert to lower color system | `color.py:Color.downgrade` | TrueColor→EightBit→Standard conversion |
| Done | Basic `Color` enum with parse (partial) | `src/color.rs` | Needs full implementation |

### 1.2 Cell Width

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `cell_len()` using unicode-width | `cells.py:cell_len` | Wrapped in `src/cells.rs` |
| Done | `char_width()` - single character width | N/A | Uses `unicode-width` crate |
| Todo | `set_cell_size()` - truncate/pad to exact cell width | `cells.py:set_cell_size` | Handles double-width boundaries |
| Todo | `chop_cells()` - split text into width-limited lines | `cells.py:chop_cells` | For wrapping |

**Note:** Using `unicode-width` crate instead of porting Python's `CELL_WIDTHS` table.

### 1.3 Style

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Style` struct with `Option<bool>` attributes | `src/style.rs` | Copy for efficiency |
| Done | `StyleMeta` struct for links/metadata | `style.py:Style` | Separate to keep Style Copy |
| Done | `Style + Style` combination (impl Add) | `style.py:Style.__add__` | Combines styles |
| Todo | Bitfield storage for attributes | `style.py:Style` | Optional optimization |
| Todo | `Style::parse()` - full implementation | `style.py:Style.parse` | Use once_cell::Lazy for caching |
| Todo | `Style::render()` - generate ANSI escape sequence | `style.py:Style.render` | Core output method |
| Todo | `Style::get_html_style()` for HTML export | `style.py:Style.get_html_style` | CSS generation |
| Todo | `NULL_STYLE` singleton | `style.py:NULL_STYLE` | Optimization for empty style |

### 1.4 Segment

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Segment` struct (text, style, control) | `src/segment.rs` | Uses `Cow<'static, str>` |
| Done | `Segments` collection (SmallVec backed) | N/A | Abstracts storage for future streaming |
| Done | `ControlType` enum (full 16 variants) | `segment.py:ControlType` | Cursor movement, erase, etc. |
| Todo | `Segment::split_cells()` - split at cell boundary | `segment.py:Segment.split_cells` | Handles double-width |
| Todo | `Segment::split_lines()` - split on newlines | `segment.py:Segment.split_lines` | Iterator |
| Todo | `Segment::split_and_crop_lines()` - layout core | `segment.py:Segment.split_and_crop_lines` | Critical for rendering |
| Todo | `Segment::adjust_line_length()` - crop or pad | `segment.py:Segment.adjust_line_length` | Width normalization |
| Todo | `Segment::simplify()` - merge adjacent same-style | `segment.py:Segment.simplify` | Output optimization |
| Todo | `Segment::divide()` - split at cell positions | `segment.py:Segment.divide` | For column layout |

### 1.5 Measurement

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Measurement` struct (minimum, maximum) | `src/measure.rs` | Basic structure |
| Done | `Measurement::from_segments()` | N/A | Default measurement strategy |
| Todo | `measure_renderables()` - combine measurements | `measure.py:measure_renderables` | Takes max of mins/maxs |

---

## Phase 2: Text & Console

### 2.0 Utilities (Needed by Text/Console)

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Emoji` lookup | `emoji.py` + `_emoji_codes.py` | :name: → character, needed by markup |
| Todo | `Highlighter` base trait | `highlighter.py` | Regex-based highlighting |

### 2.1 Markup Parser

**Note:** Markup should be implemented before `Text::from_markup()`.

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Basic `render()` function | `src/markup.rs` | Partial implementation |
| Todo | `Tag` struct (name, parameters) | `markup.py:Tag` | NamedTuple in Python |
| Todo | `_parse()` tokenizer yielding (pos, text, tag) | `markup.py:_parse` | Stack-based parser |
| Todo | `escape()` function | `markup.py:escape` | Escape brackets |
| Todo | Link syntax: `[link=url]text[/link]` | `markup.py` | URL handling |
| Todo | Metadata syntax: `[@name=value]` | `markup.py` | For Textual |
| Todo | Nested tag support with style stacking | `markup.py:render` | Combines styles |
| Todo | Emoji code replacement | `_emoji_replace.py` | :warning: → ⚠️ |

### 2.2 Text

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Text` struct with spans | `src/text.rs` | Basic structure |
| Done | `Span` struct (start, end, style) | `src/text.rs` | Basic structure |
| Todo | `Span::split()`, `move()`, `right_crop()`, `extend()` | `text.py:Span.*` | Span manipulation |
| Todo | `Text::from_markup()` - parse BBCode | `text.py:Text.from_markup` | Depends on markup.rs |
| Todo | `Text::from_ansi()` - parse ANSI codes | `text.py:Text.from_ansi` | Reverse rendering |
| Todo | `Text::assemble()` - build from (str, style) pairs | `text.py:Text.assemble` | Common construction |
| Todo | `Text::stylize()`, `stylize_before()` | `text.py:Text.stylize*` | Apply style to range |
| Todo | `Text::highlight_regex()`, `highlight_words()` | `text.py:Text.highlight_*` | Pattern-based styling |
| Todo | `Text::wrap()` - word wrapping with justify | `text.py:Text.wrap` | Complex algorithm |
| Todo | `Text::divide()` - split at offsets | `text.py:Text.divide` | For column layout |
| Todo | `impl Renderable for Text` | `text.py:Text.__rich_console__` | Returns Segments |

### 2.3 Console

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Basic `Console` struct | `src/console.rs` | Minimal implementation |
| Done | `ConsoleOptions` struct | `src/console.rs` | Basic fields |
| Todo | Full `ConsoleOptions` fields (16+ fields) | `console.py:ConsoleOptions` | justify, overflow, etc. |
| Todo | `Console<W: Write>` generic over writer | N/A | Enables testing |
| Todo | Color system detection (auto from TERM) | `console.py:Console._detect_color_system` | Environment-based |
| Todo | `Console::render()` - core render method | `console.py:Console.render` | Calls Renderable::render |
| Todo | `Console::render_lines()` - render to line grid | `console.py:Console.render_lines` | Uses split_and_crop_lines |
| Todo | `Console::render_str()` - string to Text | `console.py:Console.render_str` | With markup/emoji/highlight |
| Todo | `Console::print()` - main print method | `console.py:Console.print` | Many parameters |
| Todo | Theme support (`Theme`, `ThemeStack`) | `console.py` + `theme.py` | Named style definitions |
| Todo | Capture for testing | `console.py:Console.capture` | Returns string |
| Todo | Screen/alt screen support | `console.py:Console.screen` | Via crossterm |

### 2.4 Traits

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Renderable` trait with `render()` + default `measure()` | `protocol.py` | Send + Sync required |
| Done | `RichCast` trait with associated type | `protocol.py` | Avoids Box allocation |
| Done | `impl Renderable for str` | N/A | Basic string rendering |
| Done | `impl Renderable for String` | N/A | Basic string rendering |

**Note:** No separate `Measurable` trait. Measurement is a default method on `Renderable`.

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

### 3.2 Rule (Horizontal Line)

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Rule` struct | `rule.py:Rule` | title, characters, style, align |
| Todo | `impl Renderable for Rule` | `rule.py:Rule.__rich_console__` | Render horizontal line |

### 3.3 Padding

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Padding` struct | `padding.py:Padding` | (top, right, bottom, left) |
| Todo | `Padding::unpack()` - CSS-style parsing | `padding.py:Padding.unpack` | 1, 2, or 4 values |
| Todo | `impl Renderable for Padding` | `padding.py:Padding.__rich_console__` | Wrap with space |

### 3.4 Align

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Align` struct | `align.py:Align` | horizontal + vertical alignment |
| Todo | `Align::left()`, `center()`, `right()` constructors | `align.py:Align.*` | Convenience methods |
| Todo | `impl Renderable for Align` | `align.py:Align.__rich_console__` | Pad to width |

---

## Phase 4: Complex Renderables

### 4.1 Panel

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Panel` struct | `panel.py:Panel` | box, title, subtitle, padding |
| Todo | `Panel::fit()` - non-expanding variant | `panel.py:Panel.fit` | Constructor |
| Todo | `impl Renderable for Panel` | `panel.py:Panel.__rich_console__` | ~100 lines in Python |

### 4.2 Tree

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Tree` struct | `tree.py:Tree` | label, children, guide_style |
| Todo | `Tree::add()` - add child node | `tree.py:Tree.add` | Returns child for chaining |
| Todo | Guide constants (ASCII_GUIDES, TREE_GUIDES) | `tree.py` | 4 guide character sets |
| Todo | `impl Renderable for Tree` | `tree.py:Tree.__rich_console__` | Stack-based traversal |

### 4.3 Table

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Column` struct | `table.py:Column` | header, footer, width, ratio |
| Todo | `Row` struct | `table.py:Row` | style, end_section |
| Todo | `Table` struct | `table.py:Table` | columns, rows, box, title |
| Todo | `Table::grid()` - headerless table | `table.py:Table.grid` | Common pattern |
| Todo | `Table::add_column()`, `add_row()` | `table.py:Table.*` | Builder methods |
| Todo | `_calculate_column_widths()` | `table.py` | Ratio distribution |
| Todo | `impl Renderable for Table` | `table.py:Table.__rich_console__` | ~300 lines |
| Todo | Ratio distribution utilities | `_ratio.py` | ratio_distribute, ratio_reduce |

### 4.4 Columns (Multi-column Layout)

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Columns` struct | `columns.py:Columns` | Uses Table.grid() internally |
| Todo | `impl Renderable for Columns` | `columns.py:Columns.__rich_console__` | Delegates to Table |

---

## Phase 5: Advanced Features (Optional)

### 5.1 Progress System

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `TaskID` newtype | `progress.py:TaskID` | Task identifier |
| Todo | `ProgressTask` struct | `progress.py:ProgressTask` | Task state |
| Todo | `ProgressColumn` trait | `progress.py:ProgressColumn` | Abstract column |
| Todo | `BarColumn`, `TextColumn`, `SpinnerColumn` | `progress.py` | Column types |
| Todo | `Progress` struct | `progress.py:Progress` | Task management |
| Todo | `Progress::track()` - iterate with progress | `progress.py:Progress.track` | Common pattern |
| Todo | Live updating (requires Live) | `progress.py` + `live.py` | Threading/async |

### 5.2 Live Display

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Live` struct | `live.py:Live` | Real-time updates |
| Todo | Refresh loop (async or threaded) | `live.py:Live._refresh_thread` | Background updates |
| Todo | Transient mode | `live.py:Live` | Clear on exit |

### 5.3 Syntax Highlighting

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Syntax` struct | `syntax.py:Syntax` | Code highlighting |
| Todo | Integration with syntect or tree-sitter | N/A | Rust-native highlighting |

### 5.4 Pretty Printing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Pretty` struct | `pretty.py:Pretty` | Value formatting |
| Todo | `pprint()` function | `pretty.py:pprint` | Simple API |

### 5.5 Traceback

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Traceback` struct | `traceback.py:Traceback` | Error formatting |
| Todo | `install()` for panic hook | N/A | Rust-specific |

---

## Utilities & Helpers

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `loop_first`, `loop_last`, `loop_first_last` | `_loop.py` | Iterator helpers |
| Todo | `ratio_distribute`, `ratio_reduce` | `_ratio.py` | Width distribution |
| Todo | `Constrain` wrapper | `constrain.py` | Width constraint |
| Todo | `Styled` wrapper | `styled.py` | Apply style to renderable |
| Todo | `Control` for escape sequences | `control.py` | Terminal control codes |
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
| `Protocol` with `__rich_console__` | `trait Renderable: Send + Sync` |
| `Protocol` with `__rich_measure__` | `Renderable::measure()` (default impl) |
| `__rich__` method | `trait RichCast { type Output: Renderable; }` |
| `@lru_cache(N)` | `once_cell::sync::Lazy` |
| `Optional[bool]` tri-state | `Option<bool>` |
| `Iterable[Segment]` | `Segments` newtype (SmallVec backed) |
| `Union[str, Style]` | `impl Into<Style>` or enum |
| Thread-local storage | `thread_local!` macro |
| Generator | Iterator with state struct |
