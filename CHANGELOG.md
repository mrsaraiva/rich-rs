# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project scaffolding with core module stubs
- `Segment` struct - atomic unit of terminal output
- `Style` struct with builder pattern and parsing
- `Color` enum with named color and hex parsing
- `Text` struct with styled spans
- `Measurement` struct for width requirements
- `Console` struct with basic terminal detection
- `cell_len()` function wrapping unicode-width
- BBCode-like markup parser (basic implementation)
- Box drawing character sets (ASCII, ROUNDED, HEAVY, DOUBLE, SQUARE)
- `Renderable`, `Measurable`, and `RichCast` traits
- Development roadmap at `docs/devel/ROADMAP.md`

### Dependencies
- `crossterm` 0.28 - Terminal abstraction
- `unicode-width` 0.2 - Cell width calculation
- `atty` 0.2 - Terminal detection
