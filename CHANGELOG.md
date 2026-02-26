# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- Live display cursor repositioning now uses previous frame shape, preventing visual glitches when live content changes height.
- `Measurement::from_segments()` now correctly measures multi-line content by tracking per-line width instead of summing all lines.
- `Columns` now preserves original renderables instead of flattening to plain text, allowing proper per-cell measurement and rendering.

## [1.1.0] - 2026-02-12

### Added

#### Comprehensive Python Rich Parity Pass
- **Style**: 6 new attributes (`overline`, `blink2`, `conceal`, `underline2`, `frame`, `encircle`) with builders, parse support, and ANSI codes; `chain()`, `StyleStack`, `test()`, `background_style()`, `without_color()`, `to_markup_string()`.
- **Segment**: `align_top()`/`align_bottom()`/`align_middle()`, `split_lines_terminator()`, `strip_links()`, `remove_color()`, `SegmentLines` renderable.
- **Text**: `slice()`, `align()`, `with_indent_guides()` (full implementation), `contains()`, `to_markup()`, `get_style_at_offset()`, `set_length()`, `right_crop()`, `fit()`, `remove_suffix()`, `extend_style()`, `detect_indentation()`, `copy_styles()`, `Add` trait impl.
- **Console**: `status()`, `print_json()`, `export_text()`, `save_text()`, `out()`, `push_render_hook()`/`pop_render_hook()`.
- **Control**: convenience constructors (`bell`, `clear`, `show_cursor`, `alt_screen`, `title`), `strip_control_codes()`, `escape_control_codes()`.
- **Cells**: `split_graphemes()`.
- **JSON**: new `Json` renderable module (no serde dependency).
- **Table**: `title_style`/`caption_style`, 4-way cell padding.
- **Tree**: `hide_root`, `add_with_options(TreeNodeOptions)`, `BOLD_TREE_GUIDES`.
- **Group**: `Renderables` container (no newlines between children).
- **Layout**: `to_tree()` for hierarchy visualization.
- **Syntax**: `stylize_range()`, `with_highlight_range()`.
- **Constrain**: `with_default_width()` constructor.
- **Markup**: `render_with_emoji_variant()`.
- **Rule**: parse titles as markup.
- **Markdown**: `with_style()` builder.
- **Spinner**: `text` field, `Renderable` impl, `update()`.
- **Progress**: `RenderableColumn`, `tasks()`/`task_ids()`/`finished()` accessors.
- **Progress bar**: `percentage_completed()`.
- **Status**: `Renderable` impl, `status_text()`.
- **Prompt**: `with_stream()`, `pre_prompt` hook, numeric prompt choices.
- **Scope**: `overflow`, `max_depth` parameters.
- **Highlighter**: `highlight_text()` default trait method.
- **Filesize**: `decimal_with_params(size, precision, separator)`.
- **Lib**: `get_console()` singleton, `rich_print!` macro.

#### Live, Links, and Adapters
- **Live**: `redirect_stdout()`/`redirect_stderr()`, `get_renderable` callback, `console_mut()` accessor, `is_started()` query.
- **Console**: `begin_capture()`/`end_capture()` for output interception, `push_theme()`/`pop_theme()` for theme stacking, hyperlink rendering in `export_html()`.
- **Style**: `link` field on `Style` via `StyleMeta` integration, `meta()` accessor.
- **Text**: `highlight_words()`, `highlight_regex()` for text search highlighting.
- **Tree**: `ROUNDED_TREE_GUIDES` style constant.
- **Group**: `Lines` renderable (newline between each child).
- **ANSI decoder**: SGR 53/54/55 (overline/encircle/frame) support, OSC 8 hyperlink parsing.
- **rich-tracing** companion crate: `RichTracingLayer` for `tracing_subscriber`, `RichLogger` for `log` crate.

#### Animated Recordings and Screenshots
- `FrameRecorder` for generating animated SVG (CSS `@keyframes`) and asciicast v2 recordings of terminal output.
- `examples/recordings.rs` generating animated SVGs for README Progress Bars and Live Display sections.
- Backtrace screenshot in `examples/screenshots.rs` using real source files for syntax-highlighted code context.

### Changed
- Consolidated duplicated style serialization into `Style::to_markup_string()` (removes workarounds from `text.rs` and `theme.rs`).
- Added `markdown.table.border`, `markdown.table.header`, `blink2` to default theme styles.
- ANSI decoder now handles SGR 6/8/21/51-55 escape codes.

### Fixed
- Fixed `Padding` style application to content lines.
- Fixed README Markdown section broken by nested code fences (4-backtick outer fence).
- Renamed README "Tracebacks" section to "Backtraces" (Rust-idiomatic term).

## [1.0.5] - 2026-02-10

### Changed
- Removed redundant `unsafe impl Send/Sync` declarations from core renderables and examples where auto-traits are already satisfied by field types and trait bounds.
- Removed `WT_SESSION`-based Windows terminal heuristics; terminal behavior now relies on explicit overrides and capability-neutral defaults.
- Removed terminal-brand environment markers from color auto-detection (`KITTY_WINDOW_ID`, `WEZTERM_PANE`, `TERM_PROGRAM=*`) in favor of explicit overrides and standard `COLORTERM`/`TERM` signals.
- Switched Windows legacy-mode auto-detection to VT capability checks (`supports_ansi`) instead of terminal identity environment variables, matching Rich Python's capability-first approach from issue #140.

### Added
- Added `Style::with_reverse(bool)` builder for API consistency with other style attribute builders.

### Fixed
- Reduced `table_movie` flicker with screen-buffer diff cursor updates.
- Corrected live rendering test assertion to match screen-buffer diff path behavior.

## [1.0.3] - 2026-02-06

### Changed
- Updated terminal color-system auto-detection to prefer truecolor on modern interactive terminals.
- Added `WT_SESSION` as a truecolor marker (Windows Terminal) and prioritized modern terminal markers before `TERM=*256color`.
- Added `RICH_RS_COLOR_SYSTEM` override (`auto|none|16|256|truecolor`) for deterministic color behavior in apps and benchmarks.

## [1.0.2] - 2026-02-04

### Fixed
- Align ordered list markers with Python Rich spacing (no trailing dot).
- Clamp Text measurement to `max_width` to improve table column sizing parity.

## [1.0.1] - 2026-02-04

### Changed
- Aligned all markdown heading styles (H1–H6) with Python Rich 14.3.2 defaults.
- H1 now renders as bold + underline centered text instead of a double-bordered Panel.
- Block quote style simplified to magenta only (removed italic).
- Link style changed to bright blue without underline to match Python Rich.
- List bullet style simplified to bold only (removed yellow color).
- List number style simplified to cyan only (removed bold).
- Simplified demo example by removing manual OSC 8 hyperlink code.

### Added
- Automated crates.io publish job in release workflow with OIDC token auth and version-exists check.

## [1.0.0] - 2026-02-03

### Highlights

**rich-rs reaches feature parity with Python Rich's core rendering capabilities and is ready for crates.io.**

### Crates.io Release Preparation
- MIT LICENSE file
- Complete Cargo.toml metadata (repository, documentation, readme, rust-version)
- Package exclusions for development files (.idea/, *.dat, docs/devel/, tests/parity/, tools/)
- docs.rs configuration for documentation builds
- Demo attribution for Rust port author

### Added (Recent)
- `ScreenContext` for RAII alternate screen mode with automatic cleanup on drop
- `Console::screen()` method to enter alternate screen mode with context guard
- `ProgressReader<R: Read>` wrapper for file I/O progress tracking
- `Progress::open()` to open files with automatic progress tracking
- `Progress::wrap_file()` to wrap any reader with progress tracking
- `WrapFileBuilder` for flexible progress reader configuration
- `Console::print_traceback()` convenience method for rendering tracebacks
- `screen.rs` example demonstrating ScreenContext usage
- `cp_progress.rs` example - minimal file copy with progress bar
- `downloader.rs` example - concurrent HTTP downloads with progress bars
- `Bar` renderable for horizontal bars with smooth Unicode block characters
- `Status` wrapper for spinner + text on long-running operations
- `console.log()` method with timestamp and file/line support
- `log!` macro for ergonomic logging with automatic file/line capture
- Table/Column mutation API for runtime modification with Live displays
- `spinner_names()` now public for enumerating available spinners
- `escape_markup()` re-exported for escaping markup characters
- 25 ported examples from Python Rich (table_movie, spinners, layout, calendar, top_lite_simulator, etc.)
- Example porting plan document (`docs/devel/EXAMPLE_PORTING_PLAN.md`)

### Highlights (Features)

This release includes complete implementations of all major Rich features:
- Full color system (16/256/TrueColor)
- Text rendering with markup, wrapping, and justification
- Tables, Panels, Trees, and all standard renderables
- Syntax highlighting via syntect
- Markdown rendering via pulldown-cmark
- Progress bars with multi-task support
- Live display with real-time updates
- Beautiful panic tracebacks

### Added

#### Live Display & Progress (Phase 5.1-5.2)
- `Live` struct for real-time updating displays
  - Background refresh thread with configurable refresh rate
  - Transient mode (clear output on exit)
  - Alt-screen mode support
  - Vertical overflow handling (crop, ellipsis, visible)
  - Thread-safe updates via `update()` method
  - Nested Live display support
- `Progress` struct for multi-task progress tracking
  - `TaskID` newtype for task identification
  - `ProgressTask` with timing, speed calculation, and ETA
  - `ProgressColumn` trait for custom columns
  - Built-in columns: `TextColumn`, `BarColumn`, `SpinnerColumn`, `TimeElapsedColumn`, `TimeRemainingColumn`, `MofNCompleteColumn`, `DownloadColumn`, `TransferSpeedColumn`
  - `Progress::track()` iterator for easy progress tracking
  - `ProgressIteratorExt` trait for `.progress()` on any iterator
- `ProgressBar` renderable for standalone progress bars
  - Configurable width, completed/total ratio
  - Pulse animation for indeterminate progress
  - Style customization (background, complete, finished, pulse)
- `Spinner` renderable with 80+ animation styles
  - All spinner definitions from cli-spinners
  - Configurable speed and style
  - Text label support
- `Control` renderable for terminal escape sequences
  - Cursor positioning, show/hide
  - Screen clear, line erase
  - Alt screen enter/leave
  - Bell, carriage return, home

#### Utilities
- `filesize` module for human-readable file sizes
  - `decimal()` - SI units (kB, MB, GB)
  - `binary()` - Binary units (KiB, MiB, GiB)
  - `pick_unit_and_suffix()` for custom formatting
- `loop_helpers` module with iterator utilities
  - `loop_first()` - yields `(is_first, item)` tuples
  - `loop_last()` - yields `(is_last, item)` tuples
  - `loop_first_last()` - yields `(is_first, is_last, item)` tuples
- `Styled` wrapper renderable for applying styles to any renderable
- `Constrain` wrapper renderable for width constraints
- `AnsiDecoder` for parsing ANSI escape sequences back to styled text
- `Text::from_ansi()` for converting ANSI-styled strings to Text

#### Console Export
- `Console::export_svg()` for programmatic SVG screenshot generation
  - Record mode captures segments via `Console::new_with_record()`
  - Customizable terminal chrome with title
  - `TerminalTheme` struct for export color schemes
  - Built-in themes: `SVG_EXPORT_THEME`, `MONOKAI`, `DIMMED_MONOKAI`, `NIGHT_OWLISH`
  - `save_svg()` convenience method for file output
  - XSS-safe HTML escaping for text content

#### Demo & Examples
- `cargo run --example demo` - Full feature showcase matching `python -m rich`
- `cargo run --example progress` - Progress bar demonstrations
- `cargo run --example live_stress` - Live display stress test
- `cargo run --example live_alt_screen` - Alt-screen mode example

#### Previous Releases (included in 1.0.0)
- Demo example (Phase 6.1):
  - ColorBox renderable with HLS→RGB TrueColor gradient
  - Colors, Styles, Text, Asian language support sections
  - Markup with BBCode and emoji display
  - Tables section with styled movie data
  - Syntax highlighting + Pretty printing side-by-side
  - Markdown raw vs rendered comparison
  - Panel with sponsor message and timing output
- Traceback rendering (Phase 5.5):
  - `impl Renderable for Traceback` for complete exception display
  - Renders stack frames with syntax-highlighted source code
  - Panel-style output with styled borders and title
  - Exception chaining support with cause messages
  - Syntax error display with offset indicator (▲)
  - Frame suppression and max_frames limiting
  - Local variables display via scope module
  - `install()` and `install_with_options()` for panic hook registration
- Syntax highlighting module (Phase 5.3):
  - `Syntax` struct for code highlighting with syntect integration
  - `SyntaxTheme` trait with `AnsiTheme` and `SyntectTheme` implementations
  - Line numbers, line range, dedent, tab expansion features
  - `highlight()` method for standalone text highlighting
  - 7 built-in themes (base16-ocean.dark, Solarized, InspiredGitHub, etc.)
- Pretty printing module (Phase 5.4):
  - `Pretty` struct for Debug trait formatting
  - `pprint()` and `pretty_repr()` convenience functions
  - Debug output parser with syntax highlighting
  - Configurable indentation, max depth, max length, max string
- Markdown module (Phase 5.6):
  - `Markdown` struct for CommonMark + GFM rendering via pulldown-cmark
  - Headings (H1-H6) with Panel wrapping for H1
  - Fenced code blocks with syntax highlighting
  - Block quotes with border styling
  - Ordered and unordered lists (including tight lists)
  - Tables using Table module
  - Inline formatting (bold, italic, strikethrough, code)
  - Links, images (placeholder with emoji), horizontal rules
- Rule module (Phase 3.2):
  - `Rule` struct for horizontal line renderables
  - `AlignMethod` enum (Left, Center, Right) for title alignment
  - Builder pattern with `with_title()`, `with_characters()`, `with_style()`, `with_align()`
  - ASCII-only fallback (substitutes "-" for non-ASCII characters)
  - Title truncation with ellipsis for narrow widths
- Padding module (Phase 3.3):
  - `Padding` struct wrapping `Box<dyn Renderable + Send + Sync>`
  - `PaddingDimensions` enum for CSS-style 1/2/4 value padding
  - `Padding::unpack()` for CSS-style padding parsing
  - `Padding::indent()` convenience constructor for left-indent
  - Proportional padding collapse when padding exceeds available width
- Full Box module (Phase 3.1):
  - `Box` struct with 28 character fields for table borders (8-row structure)
  - All 19 box constants matching Python Rich
  - `RowLevel` enum (Head, Row, Foot, Mid) for row separator types
  - `Box::substitute()` - platform-safe substitution (legacy Windows, ASCII-only)
  - `Box::get_plain_headed_box()` - header character substitution
  - `Box::get_top()`, `get_row()`, `get_bottom()` - table border generation
- `Emoji` struct with 3608 emoji entries and `:name:` replacement
- `Highlighter` trait for regex-based text highlighting
- `RegexHighlighter`, `NullHighlighter` implementations
- Factory functions: `repr_highlighter()`, `json_highlighter()`, `iso8601_highlighter()`
- Full Console module (Phase 2.3):
  - `Theme` struct with style registry and INI config parsing
  - `ThemeStack` for nested theme contexts
  - 100+ default styles matching Python Rich
  - `JustifyMethod` enum (Left, Center, Right, Full)
  - `OverflowMethod` enum (Fold, Crop, Ellipsis, Ignore)
  - `Console<W: Write>` generic over writer for testability
  - Color system detection from TERM/COLORTERM/NO_COLOR environment
  - Alt screen support via crossterm
- Full Text module with markup, wrapping, and span manipulation
- Full markup parser with BBCode-like syntax

### Dependencies
- `crossterm` 0.28 - Terminal abstraction
- `unicode-width` 0.2 - Cell width calculation
- `atty` 0.2 - Terminal detection
- `smallvec` 1.13 - Stack-allocated vectors
- `thiserror` 2.0 - Error type derivation
- `once_cell` 1.19 - Lazy static initialization
- `phf` 0.11 - Compile-time perfect hash map for emoji lookup
- `regex` 1.x - Regular expression support
- `syntect` 5.x - Syntax highlighting
- `pulldown-cmark` 0.12 - Markdown parsing

## [0.1.0] - 2026-01-16

### Added
- Initial project scaffolding with core module stubs
- `Segment` struct - atomic unit of terminal output
- `Segments` collection - SmallVec-backed collection with future streaming support
- `Style` struct with builder pattern and parsing (Copy for efficiency)
- `StyleMeta` struct for hyperlinks and metadata (separate to keep Style Copy)
- Full color system with ~250 named colors, 256-color palette, truecolor support
- `ColorTriplet` struct for RGB colors
- `Color` struct with parsing, ANSI code generation, and downgrading
- `SimpleColor` enum for Copy-compatible colors in Style
- `Palette` struct with Euclidean distance color matching
- Static palettes: `STANDARD_PALETTE`, `EIGHT_BIT_PALETTE`, `WINDOWS_PALETTE`
- `ParseError` enum for unified error handling
- `Text` struct with styled spans
- `Measurement` struct with `from_segments()` default measurement
- `Console` struct with basic terminal detection
- Cell width utilities: `cell_len()`, `char_width()`, `set_cell_size()`, `chop_cells()`
- `NULL_STYLE` constant and `Style::is_null()` method
- `Style::render()` method for ANSI escape code generation
- `Style::get_html_style()` method for CSS style generation
- Segment utilities for splitting, cropping, measuring, and layout
- `Measurement` methods for normalization and clamping
- `Renderable` trait with `Send + Sync` requirement and default `measure()` method
- `RichCast` trait with associated type (avoids Box allocation)
- Development roadmap at `docs/devel/ROADMAP.md`

### Changed
- `Renderable::render()` returns `Segments` instead of `Vec<Segment>`
- Merged `Measurable` trait into `Renderable` as default method
- `Segment.text` uses `Cow<'static, str>` for efficiency

### Fixed
- Color downgrade to Windows palette
- `chop_cells()` edge cases
- Style parsing negation support
- Style ANSI codes emit proper SGR reset codes
