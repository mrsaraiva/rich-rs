# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- BBCode-like markup parser (basic implementation)
- Box drawing character sets (ASCII, ROUNDED, HEAVY, DOUBLE, SQUARE)
- `Renderable` trait with `Send + Sync` requirement and default `measure()` method
- `RichCast` trait with associated type (avoids Box allocation)
- Development roadmap at `docs/devel/ROADMAP.md`

### Changed
- `Renderable::render()` now returns `Segments` instead of `Vec<Segment>`
- Merged `Measurable` trait into `Renderable` as default method
- `Segment.text` now uses `Cow<'static, str>` for efficiency

### Dependencies
- `crossterm` 0.28 - Terminal abstraction
- `unicode-width` 0.2 - Cell width calculation
- `atty` 0.2 - Terminal detection
- `smallvec` 1.13 - Stack-allocated vectors
- `thiserror` 2.0 - Error type derivation
- `once_cell` 1.19 - Lazy static initialization
