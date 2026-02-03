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
| Done | `ColorTriplet` struct (r, g, b) with `hex`, `rgb`, `normalized` properties | `color_triplet.py` | No dependencies, start here |
| Done | `Palette` struct with `match(triplet)` method | `palette.py:Palette` | Finds closest color via Euclidean distance |
| Done | `STANDARD_PALETTE`, `EIGHT_BIT_PALETTE`, `WINDOWS_PALETTE` constants | `_palettes.py` | 16, 256, and Windows 10 palettes |
| Done | `ColorSystem` enum (Standard, EightBit, TrueColor, Windows) | `color.py:ColorSystem` | IntEnum in Python |
| Done | `ColorType` enum (Default, Standard, EightBit, TrueColor, Windows) | `color.py:ColorType` | Distinguishes color origin |
| Done | `Color` struct with `name`, `type`, `number`, `triplet` | `color.py:Color` | NamedTuple in Python |
| Done | `Color::parse()` - parse "red", "#ff0000", "rgb(255,0,0)", "color(196)" | `color.py:Color.parse` | Uses regex, ~250 named colors in `ANSI_COLOR_NAMES` |
| Done | `Color::from_ansi()`, `from_triplet()`, `from_rgb()`, `default()` | `color.py:Color.*` | Factory methods |
| Done | `Color::get_ansi_codes()` - generate SGR escape codes | `color.py:Color.get_ansi_codes` | Use once_cell::Lazy for caching |
| Done | `Color::downgrade()` - convert to lower color system | `color.py:Color.downgrade` | TrueColor→EightBit→Standard conversion |
| Done | `SimpleColor` enum for Copy-compatible colors in Style | N/A | Rust-specific optimization |

### 1.2 Cell Width

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `cell_len()` using unicode-width | `cells.py:cell_len` | Wrapped in `src/cells.rs` |
| Done | `char_width()` - single character width | N/A | Uses `unicode-width` crate |
| Done | `set_cell_size()` - truncate/pad to exact cell width | `cells.py:set_cell_size` | Handles double-width boundaries |
| Done | `chop_cells()` - split text into width-limited lines | `cells.py:chop_cells` | For wrapping |

**Note:** Using `unicode-width` crate instead of porting Python's `CELL_WIDTHS` table.

### 1.3 Style

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Style` struct with `Option<bool>` attributes | `src/style.rs` | Copy for efficiency |
| Done | `StyleMeta` struct for links/metadata | `style.py:Style` | Separate to keep Style Copy |
| Done | `Style + Style` combination (impl Add) | `style.py:Style.__add__` | Combines styles |
| Todo | Bitfield storage for attributes | `style.py:Style` | Optional optimization |
| Done | `Style::parse()` - basic implementation | `style.py:Style.parse` | Parses "bold red on blue" etc. |
| Done | `Style::render()` - generate ANSI escape sequence | `style.py:Style.render` | Core output method |
| Done | `Style::get_html_style()` for HTML export | `style.py:Style.get_html_style` | CSS generation |
| Done | `NULL_STYLE` constant and `is_null()` method | `style.py:NULL_STYLE` | Empty style constant |

### 1.4 Segment

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Segment` struct (text, style, control) | `src/segment.rs` | Uses `Cow<'static, str>` |
| Done | `Segments` collection (SmallVec backed) | N/A | Abstracts storage for future streaming |
| Done | `ControlType` enum (full 16 variants) | `segment.py:ControlType` | Cursor movement, erase, etc. |
| Done | `Segment::split_cells()` - split at cell boundary | `segment.py:Segment.split_cells` | Handles double-width |
| Done | `Segment::split_lines()` - split on newlines | `segment.py:Segment.split_lines` | Returns Vec<Vec<Segment>> |
| Done | `Segment::split_and_crop_lines()` - layout core | `segment.py:Segment.split_and_crop_lines` | Critical for rendering |
| Done | `Segment::adjust_line_length()` - crop or pad | `segment.py:Segment.adjust_line_length` | Width normalization |
| Done | `Segment::simplify()` - merge adjacent same-style | `segment.py:Segment.simplify` | Output optimization |
| Done | `Segment::divide()` - split at cell positions | `segment.py:Segment.divide` | For column layout |
| Done | `Segment::apply_style()` - apply style to segments | `segment.py:Segment.apply_style` | Pre/post style support |
| Done | `Segment::filter_control()` - filter by control | `segment.py:Segment.filter_control` | Filter control segments |
| Done | `Segment::strip_styles()` - remove all styles | `segment.py:Segment.strip_styles` | Style stripping |
| Done | `Segment::get_line_length()` - line cell width | `segment.py:Segment.get_line_length` | Sum of cell lengths |
| Done | `Segment::get_shape()` - get (width, height) | `segment.py:Segment.get_shape` | Enclosing rectangle |
| Done | `Segment::set_shape()` - set rectangle size | `segment.py:Segment.set_shape` | Pad/crop to shape |

### 1.5 Measurement

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Measurement` struct (minimum, maximum) | `src/measure.rs` | Basic structure |
| Done | `Measurement::from_segments()` | N/A | Default measurement strategy |
| Done | `Measurement::normalize()` | `measure.py:Measurement.normalize` | Ensure min <= max >= 0 |
| Done | `Measurement::with_maximum()` | `measure.py:Measurement.with_maximum` | Constrain to max width |
| Done | `Measurement::with_minimum()` | `measure.py:Measurement.with_minimum` | Constrain to min width |
| Done | `Measurement::clamp_bounds()` | `measure.py:Measurement.clamp` | Clamp with optional bounds |
| Done | `measure_renderables()` - combine measurements | `measure.py:measure_renderables` | Takes max of mins/maxs |

### 1.6 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Python test scripts for Phase 1 modules | `tests/parity/phase1/python/` | Recovered from JSONL transcripts |
| Done | Rust parity binary crate | `tests/parity/phase1/rust/` | Recovered from JSONL transcripts |
| Done | Parity test runner script | `tests/parity/run_parity.sh` | Recovered from JSONL transcripts |

**Reference:** See `tests/parity/phase1/` for the canonical parity test structure.

---

## Phase 2: Text & Console

### 2.0 Utilities (Needed by Text/Console)

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Emoji` lookup | `emoji.py` + `_emoji_codes.py` | :name: → character, needed by markup |
| Done | `Highlighter` base trait | `highlighter.py` | Regex-based highlighting |

### 2.1 Markup Parser

**Note:** Markup should be implemented before `Text::from_markup()`.

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Tag` struct (name, parameters) | `markup.py:Tag` | NamedTuple in Python |
| Done | `parse()` tokenizer yielding (pos, text, tag) | `markup.py:_parse` | Regex-based parser |
| Done | `escape()` function | `markup.py:escape` | Escape brackets |
| Done | Link syntax: `[link=url]text[/link]` | `markup.py` | Underlined cyan style |
| Done | Metadata syntax: `[@name=value]` | `markup.py` | Basic support |
| Done | Nested tag support with style stacking | `markup.py:render` | Combines styles |
| Done | Emoji code replacement | `_emoji_replace.py` | :warning: → ⚠️ |
| Done | `render()` returns `Result<Text>` | `markup.py:render` | Full implementation |
| Done | `render_with_style()` base style support | N/A | Rust convenience API |

### 2.2 Text

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Text` struct with spans | `src/text.rs` | Basic structure |
| Done | `Span` struct (start, end, style) | `src/text.rs` | Basic structure |
| Done | `Span::split()`, `move()`, `right_crop()`, `extend()` | `text.py:Span.*` | Span manipulation |
| Done | `Text::from_markup()` - parse BBCode | `text.py:Text.from_markup` | Depends on markup.rs |
| Done | `Text::assemble()` - build from (str, style) pairs | `text.py:Text.assemble` | Common construction |
| Done | `Text::stylize()`, `stylize_before()` | `text.py:Text.stylize*` | Apply style to range |
| Done | `Text::highlight_regex()`, `highlight_words()` | `text.py:Text.highlight_*` | Pattern-based styling |
| Done | `Text::divide()` - split at offsets | `text.py:Text.divide` | For column layout |
| Done | `impl Renderable for Text` | `text.py:Text.__rich_console__` | Returns Segments |

### 2.3 Console & Text Wrapping

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Basic `Console` struct | `src/console.rs` | Minimal implementation |
| Done | `ConsoleOptions` struct | `src/console.rs` | Basic fields |
| Done | Full `ConsoleOptions` fields (16+ fields) | `console.py:ConsoleOptions` | justify, overflow, etc. |
| Done | `Console<W: Write>` generic over writer | N/A | Enables testing |
| Done | Color system detection (auto from TERM) | `console.py:Console._detect_color_system` | Environment-based |
| Done | `Console::render()` - core render method | `console.py:Console.render` | Calls Renderable::render |
| Done | `Console::render_lines()` - render to line grid | `console.py:Console.render_lines` | Uses split_and_crop_lines |
| Done | `Console::render_str()` - string to Text | `console.py:Console.render_str` | With markup/emoji/highlight |
| Done | `Console::print()` - main print method | `console.py:Console.print` | Many parameters |
| Done | Theme support (`Theme`, `ThemeStack`) | `console.py` + `theme.py` | Named style definitions |
| Done | Capture for testing | `console.py:Console.capture` | Returns string |
| Done | Screen/alt screen support | `console.py:Console.screen` | Via crossterm |
| Done | `divide_line()` - word wrap helper | `_wrap.py:divide_line` | Find wrap offsets for Text::wrap() |
| Done | `Text::wrap()` - word wrapping with justify | `text.py:Text.wrap` | Uses divide_line + Console |

### 2.4 Traits

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Renderable` trait with `render()` + default `measure()` | `protocol.py` | Send + Sync required |
| Done | `RichCast` trait with associated type | `protocol.py` | Avoids Box allocation |
| Done | `impl Renderable for str` | N/A | Basic string rendering |
| Done | `impl Renderable for String` | N/A | Basic string rendering |

**Note:** No separate `Measurable` trait. Measurement is a default method on `Renderable`.

### 2.5 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Phase 2 parity tests | `tests/parity/phase2/` | Recovered from JSONL transcripts |

**Reference:** Follow the structure in `tests/parity/phase1/` for test organization.

---

## Phase 3: Box Drawing & Simple Renderables

### 3.1 Box Characters

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Box` struct with 28 character fields | `src/box.rs` | Full 8-row structure for tables |
| Done | All 19 box constants | `src/box.rs` | ASCII, ASCII2, ASCII_DOUBLE_HEAD, SQUARE, SQUARE_DOUBLE_HEAD, MINIMAL, MINIMAL_HEAVY_HEAD, MINIMAL_DOUBLE_HEAD, SIMPLE, SIMPLE_HEAD, SIMPLE_HEAVY, HORIZONTALS, ROUNDED, HEAVY, HEAVY_EDGE, HEAVY_HEAD, DOUBLE, DOUBLE_EDGE, MARKDOWN |
| Done | `Box::substitute()` - platform compatibility | `box.py:Box.substitute` | Windows legacy + ASCII fallback |
| Done | `Box::get_plain_headed_box()` | `box.py:Box.get_plain_headed_box` | Header character substitution |
| Done | `Box::get_top()`, `get_row()`, `get_bottom()` | `box.py:Box.get_*` | Column-aware table borders |
| Done | `RowLevel` enum | N/A | Head, Row, Foot, Mid variants |
| Done | Backward compat `BoxChars` type alias | N/A | Deprecated alias to `Box` |

### 3.2 Rule (Horizontal Line)

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Rule` struct | `rule.py:Rule` | title, characters, style, end, align |
| Done | `AlignMethod` enum | N/A | Left, Center, Right |
| Done | `impl Renderable for Rule` | `rule.py:Rule.__rich_console__` | Horizontal line with optional title |
| Done | Builder pattern | N/A | with_title(), with_characters(), with_style(), with_align() |
| Done | ASCII-only fallback | `rule.py` | Substitutes "-" for non-ASCII characters |

### 3.3 Padding

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Padding` struct | `padding.py:Padding` | Wraps Box<dyn Renderable> |
| Done | `PaddingDimensions` enum | `padding.py:PaddingDimensions` | CSS-style 1/2/4 values |
| Done | `Padding::unpack()` | `padding.py:Padding.unpack` | CSS-style parsing |
| Done | `Padding::indent()` | `padding.py:Padding.indent` | Left-indent convenience |
| Done | `impl Renderable for Padding` | `padding.py:Padding.__rich_console__` | Wrap with space |
| Done | `impl measure()` | `padding.py:Padding.__rich_measure__` | Add padding to measurement |

### 3.4 Align

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Align` struct | `align.py:Align` | horizontal + vertical alignment |
| Done | `Align::left()`, `center()`, `right()` constructors | `align.py:Align.*` | Convenience methods |
| Done | `impl Renderable for Align` | `align.py:Align.__rich_console__` | Pad to width |
| Done | `VerticalAlignMethod` enum | `align.py:VerticalAlignMethod` | Top, Middle, Bottom with parse() |
| Done | Builder methods | N/A | with_style(), with_vertical(), with_pad(), with_width(), with_height() |

### 3.5 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Phase 3 parity tests | `tests/parity/phase3/` | Recovered from JSONL transcripts |

**Reference:** Follow the structure in `tests/parity/phase1/` for test organization.

---

## Phase 4 Blocker: Console State in Renderable (RESOLVED)

| Status | Task | Notes |
|--------|------|-------|
| Done | Fix Console state not passed to renderables | Solution: pass state through `ConsoleOptions` |

### Problem (Historical)

The `Renderable::render()` trait method takes `&Console<Stdout>`, but our generic `Console<W>` methods (like `render_lines()`, `print()`, `measure_renderable()`) create a **temporary** `Console<Stdout>` to call renderables. This meant renderables couldn't access `theme_stack`, `markup_enabled`, etc.

### Solution Implemented

**Option 3: Pass state through ConsoleOptions** was implemented:

1. **Enriched `ConsoleOptions`** with console state fields:
   - `theme_stack: ThemeStack`
   - `markup_enabled`, `emoji_enabled`, `highlight_enabled`: `bool`
   - `tab_size: usize`
   - `color_system: Option<ColorSystem>`

2. **Console setters sync to options** - Calling `set_markup_enabled()` etc. updates both `self.markup_enabled` AND `self.options.markup_enabled`, keeping them in sync.

3. **`Console::with_options()` initializes from options** - When a temp Console is created, it reads state from the provided options, ensuring nested renderables see correct state.

4. **Renderables access state via options** - `options.get_style("name")`, `options.theme_stack`, `options.markup_enabled`, etc.

### Benefits

- No trait signature changes required
- No lifetime complexity
- State flows naturally through the existing `ConsoleOptions` parameter
- Nested renderables work correctly (verified by tests)

---

## Phase 4: Complex Renderables

### 4.1 Panel

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Panel` struct | `panel.py:Panel` | box, title, subtitle, padding |
| Done | `Panel::fit()` - non-expanding variant | `panel.py:Panel.fit` | Constructor |
| Done | `impl Renderable for Panel` | `panel.py:Panel.__rich_console__` | ~100 lines in Python |

### 4.2 Tree

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Tree` struct | `tree.py:Tree` | label, children, guide_style |
| Done | `Tree::add()` - add child node | `tree.py:Tree.add` | Returns child for chaining |
| Done | Guide constants (ASCII_GUIDES, TREE_GUIDES) | `tree.py` | 4 guide character sets |
| Done | `impl Renderable for Tree` | `tree.py:Tree.__rich_console__` | Stack-based traversal |

### 4.3 Table

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Column` struct | `table.py:Column` | header, footer, width, ratio |
| Done | `Row` struct | `table.py:Row` | style, end_section |
| Done | `Table` struct | `table.py:Table` | columns, rows, box, title |
| Done | `Table::grid()` - headerless table | `table.py:Table.grid` | Common pattern |
| Done | `Table::add_column()`, `add_row()` | `table.py:Table.*` | Builder methods |
| Done | `_calculate_column_widths()` | `table.py` | Ratio distribution |
| Done | `impl Renderable for Table` | `table.py:Table.__rich_console__` | ~300 lines |
| Done | Ratio distribution utilities | `_ratio.py` | ratio_distribute, ratio_reduce |

### 4.4 Columns (Multi-column Layout)

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Columns` struct | `columns.py:Columns` | Uses Table.grid() internally |
| Done | `impl Renderable for Columns` | `columns.py:Columns.__rich_console__` | Delegates to Table |

### 4.5 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Phase 4 parity tests | `tests/parity/phase4/` | Recovered from JSONL transcripts |

**Reference:** Follow the structure in `tests/parity/phase1/` for test organization.

---

## Phase 5: Advanced Features (Optional)

## Final Parity Tasks (Remaining)

The demo and core features are now visually matched to Python Rich. To reach **1:1 terminal parity** (behavior + API),
the remaining work is focused on regression-proofing (parity tests) and completing key utility modules Rich relies on.

### Task 1 — Phase 5 Parity Test Suite

**Goal:** Prevent regressions and lock in behavioral parity for Live + Progress.

- Add `tests/parity/phase5/` with deterministic snapshots for Progress rendering (fixed width, fixed task state).
- Add targeted Live parity tests for cursor / erase controls and nesting correctness.
- Add a documented “parity matrix” checklist for Live/Progress behaviors covered by tests (TTY, dumb terminals, nested Live, transient mode, alt-screen).

### Task 2 — ANSI Decoder + `Text::from_ansi()`

**Goal:** Parse ANSI output back into structured `Text`/`Segments` for robust interop and better parity testing.

- Implement `AnsiDecoder` (`ansi.py:AnsiDecoder` parity) to parse:
  - SGR attribute toggles + resets (including tri-state semantics like dim on/off)
  - 8/16 colors, 256 colors, TrueColor
  - cursor / erase controls as `ControlType` where applicable
- Implement `Text::from_ansi()` using `AnsiDecoder` and existing style/segment models.

### Task 3 — Spinner Catalog Parity

**Goal:** Ensure spinner names/frames/intervals match Python Rich.

- Port Rich’s `_spinners.py` definitions (frames + interval).
- Ensure `Spinner::new(name)` recognizes all Rich spinner names.
- Add tests for a small sample of spinner names (including multi-cell frames).

### Task 4 — `Styled` Wrapper

**Goal:** Rich-compatible “apply style to any renderable” adapter.

- Implement `Styled` renderable (`styled.py` parity): wraps a renderable and combines a style over all segments.
- Ensure style composition matches Rich semantics (outer style combines with inner).
- Add tests to verify style layering and reset behavior.

### Task 5 — `Constrain` Wrapper

**Goal:** Rich-compatible width constraints to match layout edge cases.

- Implement `Constrain` renderable (`constrain.py` parity): clamps rendered output to a maximum width.
- Use existing measurement + crop/pad helpers to match Rich behavior.
- Add tests that assert exact cell widths and cropping.

### Task 6 — Loop Helpers

**Goal:** Utility iterator helpers used across Rich internals.

- Implement `loop_first`, `loop_last`, `loop_first_last` (`_loop.py` parity).
- Add unit tests for edge cases (empty iterator, singleton, multi-item).

### 5.1 Progress System

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `TaskID` newtype | `progress.py:TaskID` | Task identifier |
| Done | `ProgressTask` struct | `progress.py:ProgressTask` | Task state |
| Done | `ProgressColumn` trait | `progress.py:ProgressColumn` | Abstract column |
| Done | `BarColumn`, `TextColumn`, `SpinnerColumn` | `progress.py` | Column types |
| Done | `Progress` struct | `progress.py:Progress` | Task management |
| Done | `Progress::track()` - iterate with progress | `progress.py:Progress.track` | Common pattern |
| Done | Live updating (requires Live) | `progress.py` + `live.py` | Threading/async |

### 5.2 Live Display

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Live` struct | `live.py:Live` | Real-time updates |
| Done | Refresh loop (async or threaded) | `live.py:Live._refresh_thread` | Background updates |
| Done | Transient mode | `live.py:Live` | Clear on exit |

### 5.3 Syntax Highlighting

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Syntax` struct | `syntax.py:Syntax` | Code highlighting |
| Done | Integration with syntect | N/A | Uses syntect crate for highlighting |
| Done | `SyntaxTheme` trait + `AnsiTheme`, `SyntectTheme` | `syntax.py` | Configurable themes |
| Done | Line numbers, line range, dedent, tab expansion | `syntax.py` | Full feature set |
| Done | `highlight()` for standalone highlighting | `syntax.py:Syntax.highlight` | Returns styled Text |

### 5.4 Pretty Printing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Pretty` struct | `pretty.py:Pretty` | Value formatting via Debug trait |
| Done | `pprint()` function | `pretty.py:pprint` | Simple API |
| Done | `pretty_repr()` function | `pretty.py:pretty_repr` | Debug string formatting |
| Done | Debug output parser | N/A | Parses Rust {:?} format |
| Done | Syntax highlighting of output | `pretty.py` | Uses repr_highlighter |

### 5.5 Traceback

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Frame` struct | `traceback.py:Frame` | Stack frame data |
| Done | `Stack` struct | `traceback.py:Stack` | Exception with frames |
| Done | `Trace` struct | `traceback.py:Trace` | Collection of stacks |
| Done | `Traceback` struct + TracebackBuilder | `traceback.py:Traceback` | Configuration/display |
| Done | `SyntaxErrorInfo` struct | `traceback.py` | Syntax error details |
| Done | `ScopeRenderable` + `render_scope()` | `scope.py` | Local variables table |
| Done | `impl Renderable for Traceback` | `traceback.py:Traceback.__rich_console__` | Rendering logic |
| Done | `install()` for panic hook | N/A | Rust-specific |

### 5.6 Markdown

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Markdown` struct | `markdown.py:Markdown` | Markdown rendering |
| Done | Heading rendering (H1-H6) | `markdown.py` | H1 in Panel, H2-H6 styled |
| Done | Code blocks (inline and fenced) | `markdown.py` | Uses Syntax for fenced blocks |
| Done | Lists (ordered, unordered) | `markdown.py` | Bullet/numbered lists with nested support |
| Done | Block quotes | `markdown.py` | Bordered indented quotes |
| Done | Links and emphasis | `markdown.py` | Bold, italic, strikethrough, links |
| Done | Tables | `markdown.py` | Uses Table for markdown tables |
| Done | Images (placeholder) | `markdown.py` | Shows 🌆 emoji with alt text |
| Done | Horizontal rules | `markdown.py` | Simple divider lines |
| Done | Integration with pulldown-cmark | N/A | Full CommonMark + GFM support |

**Note:** Uses `pulldown-cmark` crate for parsing. Syntax highlighting via the Syntax module.

### 5.7 Parity Testing

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Unit tests for Syntax, Pretty, Markdown | `src/syntax.rs`, `src/pretty.rs`, `src/markdown.rs` | Extensive test coverage |
| Done | Parity test framework | `tests/parity/` | Recovered from JSONL transcripts |
| Todo | Phase 5 parity tests | `tests/parity/phase5/` | Add parity tests for advanced features |

**Reference:** Follow the structure in `tests/parity/phase1/` for test organization.

---

## Phase 6: Demo & Polish

### 6.1 Demo Example

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Create `cargo run --example demo` | `python -m rich` | Showcase all features |
| Done | Colors section (4-bit, 8-bit, Truecolor gradients) | `__main__.py` | ColorBox with HLS→RGB gradient |
| Done | Styles section (bold, italic, underline, etc.) | `__main__.py` | All ANSI styles |
| Done | Text section (wrapping, justification) | `__main__.py` | Left/center/right/full |
| Done | Asian language support section | `__main__.py` | CJK text with flag emoji |
| Done | Markup section (BBCode + emoji) | `__main__.py` | Styled text with emoji |
| Done | Tables section | `__main__.py` | Movie data table with styling |
| Done | Syntax + Pretty side-by-side | `__main__.py` | Python code + DemoData struct |
| Done | Markdown section (raw + rendered) | `__main__.py` | Side-by-side comparison |
| Done | Panel with sponsor message | `__main__.py` | rich-rs title with border |
| Done | Timing output (cold/warm cache) | `__main__.py` | Cold ~50ms, warm ~8ms |

**Prerequisites:** Phase 5.3 (Syntax) ✓, Phase 5.4 (Pretty) ✓, Phase 5.6 (Markdown) ✓ - All complete!

**Reference:** See `rich/__main__.py` for the exact demo structure and content.

---

## Utilities & Helpers

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `loop_first`, `loop_last`, `loop_first_last` | `_loop.py` | Iterator helpers |
| Done | `ratio_distribute`, `ratio_reduce` | `_ratio.py` | Width distribution |
| Done | `Constrain` wrapper | `constrain.py` | Width constraint |
| Done | `Styled` wrapper | `styled.py` | Apply style to renderable |
| Done | `Control` for escape sequences | `control.py` | Terminal control codes |
| Done | `Spinner` animations | `spinner.py` + `_spinners.py` | Animation frames |
| Done | `ProgressBar` visual bar | `progress_bar.py` | Bar rendering |
| Done | `filesize` formatting | `filesize.py` | Human-readable sizes |
| Todo | `AnsiDecoder` struct | `ansi.py:AnsiDecoder` | Parse ANSI escape sequences |
| Todo | `Text::from_ansi()` | `text.py:Text.from_ansi` | Uses AnsiDecoder (lower priority) |

---

## Phase 7: Screen & Region (Done)

Textual-style TUIs require robust region math, a way to render to a bounded screen, and cursor-safe live updates.
This phase implements the direct Rich equivalents used by Live / Progress (`Region`, `Screen`, `LiveRender`).

### 7.1 Region

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Region` struct (x, y, width, height) | `rich/region.py` | Core rectangle math |
| Done | Intersection / union / contains / crop | `rich/region.py` | Required for clipping and layout |
| Done | Region tests | `rich/region.py` | Deterministic unit tests |

### 7.2 Screen

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Screen` renderable (fill + crop) | `rich/screen.py` | Pads/crops render output to terminal size |

### 7.3 Live render

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `LiveRender` equivalent | `rich/live_render.py` | Tracks shape; provides cursor controls |

### Future: Screen buffer + diffing (Textual foundation)

The following items are required for a full Textual-style render engine (cell grid + diff updates), but are not
required for Rich's Live/Progress parity as implemented in `rich-rs` today.

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | Cell grid buffer (`ScreenBuffer`) | N/A (Textual foundation) | Stores (cell text, style) per cell |
| Done | Render → ScreenBuffer (clipping + padding) | `rich/console.py` | Converts rendered lines to a buffer |
| Done | Diff ScreenBuffer → terminal updates | N/A (Textual foundation) | Cursor-safe control + styled segments (no `\n`) |
| Done | ScreenBuffer tests (apply diff) | N/A | Validates diff output reproduces target buffer |

---

## Phase 8: Layout System (TUI Composition)

Rich’s `Layout` is a general-purpose way to split the terminal into regions and render different content
into each region. Textual has its own layout engine, but Rich Layout is still valuable as a stepping stone
and compatibility layer for Rich-based TUIs.

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Layout` tree (name, children, ratio, minimum_size) | `rich/layout.py` | Node-based composition |
| Todo | `split_row` / `split_column` | `rich/layout.py` | Primary API for region partitioning |
| Todo | Region assignment / refresh | `rich/layout.py` | Recompute regions on size changes |
| Todo | Render Layout → Screen | `rich/layout.py` + `rich/screen.py` | Requires Phase 7 |
| Todo | Layout parity tests | `rich/layout.py` | Snapshot tests + dimension cases |

---

## Phase 9: Interactive Utilities (Done)

These are user-facing terminal UX features. They're useful for end-user CLI apps, but not strictly required
for the rendering engine; decide based on product goals.

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Done | `Prompt` / `Confirm` equivalents | `rich/prompt.py` | Interactive stdin; validation |
| Done | `Pager` equivalent | `rich/pager.py` | Less-style paging; often platform-specific |
| Done | Stdout proxy during Live | `rich/file_proxy.py` | Prevent external prints from corrupting live region |

---

## Phase 10: Nice-to-haves (Post Parity)

These items are not required for demo parity, but would improve compatibility with real-world Rich apps.

| Status | Task | Python Reference | Notes |
|--------|------|------------------|-------|
| Todo | `Style.link` + `Style.meta` | `rich/style.py` | Carry hyperlink + metadata at the style level |
| Todo | OSC8 hyperlinks via style pipeline | `rich/style.py:Style.render()` | Emit `\x1b]8;id=...;url\x1b\\` around styled text (not demo-only controls) |
| Todo | `link_id` generation + combine semantics | `rich/style.py` | Preserve link ids across style composition; needed for robust Live/Progress redraws |
| Todo | Markup link tags | `rich/markup.py` | Parse `[link=...]...[/]` into `Style.link` |
| Todo | Markup metadata handlers | `rich/markup.py` | Support `[@handler=params]` (currently parsed but ignored) |
| Todo | Mouse / hyperlink tests in TTY captures | `rich/console.py` | Add parity tests that assert OSC8 sequences (where supported) |

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
