# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `Emoji` struct with 3608 emoji entries and `:name:` replacement
- `Highlighter` trait for regex-based text highlighting
- `RegexHighlighter`, `NullHighlighter` implementations
- Factory functions: `repr_highlighter()`, `json_highlighter()`, `iso8601_highlighter()`
- `NoEmoji` error variant for unknown emoji names
- Full Console module (Phase 2.3):
  - `Theme` struct with style registry and INI config parsing
  - `ThemeStack` for nested theme contexts
  - 100+ default styles matching Python Rich (info, warning, error, repr.*, etc.)
  - `JustifyMethod` enum (Left, Center, Right, Full)
  - `OverflowMethod` enum (Fold, Crop, Ellipsis, Ignore)
  - Expanded `ConsoleOptions` with all Python Rich fields (16+ fields)
  - `Console<W: Write>` generic over writer for testability
  - Color system detection from TERM/COLORTERM/NO_COLOR environment
  - `Console::render()` - core render method returning Segments
  - `Console::render_lines()` - render to cropped line grid
  - `Console::render_str()` - convert string to Text with markup/emoji/highlight
  - `Console::print()` - full-featured print with style, justify, highlight options
  - `Console::capture()` - capture output for testing
  - Alt screen support via crossterm (`Console::set_alt_screen()`)
- Wrap module (Phase 2.3):
  - `divide_line()` - find word wrap offsets for text wrapping
  - `Words` iterator for tokenizing text into words/whitespace
- Text wrapping methods (Phase 2.3):
  - `Text::pad_left()`, `pad_right()`, `center()` - alignment padding
  - `Text::expand_tabs()` - expand tabs with configurable width
  - `Text::rstrip()`, `rstrip_end()` - trailing whitespace removal
  - `Text::truncate()` - truncate to width with optional ellipsis/pad
  - `Text::split()` - split on separator string
  - `Text::wrap()` - full word wrapping with justify and overflow support
- Full Text module (Phase 2.2):
  - `Span::split()` - split span at offset into two parts
  - `Span::move_by()` - shift span by offset (supports negative)
  - `Span::right_crop()` - crop span at offset
  - `Span::extend()` - extend span end by cells
  - `Text::from_markup()` - parse BBCode markup into styled Text
  - `Text::assemble()` - build Text from mixed parts (str, Text, (str, Style))
  - `TextPart` enum for flexible assembly
  - `Text::stylize_range()` - apply style with negative index support
  - `Text::stylize_before()` - apply style at lower priority
  - `Text::highlight_regex()` - highlight regex matches with style
  - `Text::highlight_words()` - highlight word occurrences (case-insensitive option)
  - `Text::divide()` - split text at offsets preserving spans
  - `Text::append_text()` - append Text preserving base style and spans
  - `Text::join()` - join multiple Text objects with separator
  - `Text::copy()`, `blank_copy()` - copying utilities
  - `impl Renderable for Text` - render to Segments with style combination
  - `impl Measurable for Text` - calculate min/max width
- Full markup parser (Phase 2.1):
  - `Tag` struct with name and optional parameters
  - `parse()` function - regex-based tokenizer yielding `(position, text, tag)` tuples
  - `escape()` function - escape text for safe inclusion in markup
  - `render()` function - render markup to `Text` with styled spans
  - `render_with_style()` function - render with base style applied
  - Link syntax: `[link=url]text[/link]` (styled as underlined cyan)
  - Metadata syntax: `[@handler=params]text[/@handler]`
  - Nested tag support with style stacking
  - Emoji code replacement integration (`:smile:` → emoji)
  - Escaped bracket support (`\[` → `[`)
  - Implicit close tags (`[/]` closes most recent)

### Changed
- `markup::render()` now returns `Result<Text>` with proper error handling
- Markup module completely rewritten to match Python Rich behavior

### Dependencies
- `phf` 0.11 - Compile-time perfect hash map for emoji lookup
- `regex` 1.x - Regular expression support for highlighters and markup parsing

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
- `cell_len()` function wrapping unicode-width
- `char_width()` function for single character width
- `set_cell_size()` function for padding/truncating text to exact cell width
- `chop_cells()` function for splitting text into lines by cell width
- `NULL_STYLE` constant for empty style
- `Style::is_null()` method to check for empty styles
- `Style::render()` method for ANSI escape code generation
- `Style::get_html_style()` method for CSS style generation
- `SimpleColor::downgrade()` method for color system downgrading
- `SimpleColor::get_hex()` method for hex color strings
- `Segment::split_cells()` method for splitting at cell boundaries
- `Segment::split_lines()` for splitting segments on newlines
- `Segment::split_and_crop_lines()` for layout rendering (split + crop to width)
- `Segment::adjust_line_length()` for cropping/padding lines to exact width
- `Segment::simplify()` for merging adjacent same-style segments
- `Segment::divide()` for splitting at multiple cell positions
- `Segment::apply_style()`, `filter_control()`, `strip_styles()` utilities
- `Segment::get_line_length()`, `get_shape()`, `set_shape()` for layout
- `Measurement::normalize()`, `with_maximum()`, `with_minimum()`, `clamp_bounds()`
- `measure_renderables()` function for combining measurements
- BBCode-like markup parser (basic implementation)
- Box drawing character sets (ASCII, ROUNDED, HEAVY, DOUBLE, SQUARE)
- `Renderable` trait with `Send + Sync` requirement and default `measure()` method
- `RichCast` trait with associated type (avoids Box allocation)
- Development roadmap at `docs/devel/ROADMAP.md`

### Changed
- `Renderable::render()` now returns `Segments` instead of `Vec<Segment>`
- Merged `Measurable` trait into `Renderable` as default method
- `Segment.text` now uses `Cow<'static, str>` for efficiency
- `Measurement::clamp()` renamed to `clamp_width()` to avoid confusion with new `clamp_bounds()`
- `ParseError` now implements `Clone`, `PartialEq`, `Eq` and is `#[non_exhaustive]`
- `Segment::divide()` now always yields a trailing partition (matches Python Rich behavior)

### Fixed
- Color downgrade to Windows palette now works correctly (was incorrectly skipped)
- `chop_cells()` no longer creates leading empty lines when first char exceeds width
- Style parsing now supports negation ("not bold", "not italic", etc.)
- Style ANSI codes now emit proper SGR reset codes (22-29) for `Some(false)` attributes
- `Style::get_html_style()` combines underline and strike into single `text-decoration` property
- Removed incorrect `unsafe impl Send/Sync` on Segment/Segments (now derived automatically)

### Dependencies
- `crossterm` 0.28 - Terminal abstraction
- `unicode-width` 0.2 - Cell width calculation
- `atty` 0.2 - Terminal detection
- `smallvec` 1.13 - Stack-allocated vectors
- `thiserror` 2.0 - Error type derivation
- `once_cell` 1.19 - Lazy static initialization
