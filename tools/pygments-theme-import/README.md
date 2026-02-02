# Pygments Theme Importer

A tool to convert [Pygments](https://pygments.org/) syntax highlighting themes to:

1. **TextMate `.tmTheme` format** - For syntect-based syntax highlighting (`Syntax`, `Markdown` code blocks)
2. **rich-rs `.theme` INI format** - For `Pretty` and `RegexHighlighter` styling

## Overview

This tool consists of two parts:

1. **Python exporter** (`export_pygments_theme.py`) - Extracts theme data from Pygments into a JSON intermediate format
2. **Rust converter** (`src/main.rs`) - Converts the JSON to both `.tmTheme` and `.theme` formats

## Requirements

- Python 3 with Pygments installed (`pip install pygments`)
- Rust toolchain for building the converter

## Usage

### Step 1: Export from Pygments

```bash
python export_pygments_theme.py <theme_name> > <theme_name>.json
```

Available Pygments themes can be listed with:
```bash
python -c "from pygments.styles import get_all_styles; print(list(get_all_styles()))"
```

### Step 2: Convert to both formats

```bash
cargo run -- <theme_name>.json -o <output_path>.tmTheme
```

This generates two files:
- `<output_path>.tmTheme` - For syntax highlighting
- `<output_path>.theme` - For Pretty/RegexHighlighter

### Example: Import the "one-dark" theme

```bash
python export_pygments_theme.py one-dark > one-dark.json
cargo run -- one-dark.json -o ../../src/themes/one-dark.tmTheme
```

### Step 3: Integrate into rich-rs

After creating the theme files, add them to rich-rs:

#### For Syntax highlighting (`src/syntax.rs`):

```rust
// Add the embedded theme data
const ONE_DARK_THEME_DATA: &[u8] = include_bytes!("themes/one-dark.tmTheme");

// In the lazy_static block, add theme loading
lazy_static! {
    static ref ONE_DARK_THEME: Option<SyntectThemeData> = {
        ThemeSet::load_from_reader(&mut std::io::Cursor::new(ONE_DARK_THEME_DATA)).ok()
    };
}

// In get_theme(), add the theme lookup
"one-dark" => {
    if let Some(ref theme) = *ONE_DARK_THEME {
        return Box::new(SyntectTheme::new(theme.clone()));
    }
}

// In available_themes(), add to the list
themes.extend([
    // ... existing themes ...
    "one-dark",
]);
```

#### For Pretty/RegexHighlighter (`src/theme.rs`):

```rust
// Add the embedded theme data
const ONE_DARK_THEME_DATA: &str = include_str!("themes/one-dark.theme");

// In from_name(), add the theme lookup
"one-dark" => {
    let reader = Cursor::new(ONE_DARK_THEME_DATA);
    Self::from_reader(reader, true).ok()
}

// In available_themes(), add to the list
vec!["default", "dracula", "gruvbox-dark", "nord", "one-dark"]
```

## Output Formats

### `.tmTheme` (TextMate Format)

Used by syntect for syntax highlighting. Maps Pygments tokens to TextMate scopes.

Key mappings include:

| Pygments Token | TextMate Scope(s) |
|----------------|-------------------|
| `Comment` | `comment` |
| `Comment.Single` | `comment.line` |
| `Comment.Multiline` | `comment.block` |
| `String` | `string` |
| `String.Doc` | `comment.block.documentation`, `string.quoted.docstring` |
| `Number` | `constant.numeric` |
| `Keyword` | `keyword` |
| `Keyword.Type` | `storage.type`, `support.type` |
| `Name.Function` | `entity.name.function` |
| `Name.Class` | `entity.name.class`, `entity.name.type.class` |
| `Name.Exception` | `support.type.exception`, `entity.name.type.class.exception` |
| `Operator` | `keyword.operator` |

### `.theme` (INI Format)

Used by `Theme` for `Pretty` and `RegexHighlighter`. Maps Pygments tokens to repr/json style names.

Example output:
```ini
[styles]
repr.number = rgb(255,184,108)
repr.str = rgb(189,147,249)
repr.bool_true = italic rgb(255,121,198)
repr.bool_false = italic rgb(255,121,198)
repr.none = italic rgb(255,121,198)
json.number = rgb(255,184,108)
json.str = rgb(189,147,249)
json.key = rgb(255,121,198)
```

## JSON Intermediate Format

The JSON format captures all Pygments style information:

```json
{
  "name": "theme_name",
  "background_color": "#282828",
  "highlight_color": "#3c3836",
  "line_number_color": "#928374",
  "line_number_background_color": "#1d2021",
  "styles": {
    "Token.Name": {
      "color": "#ebdbb2",
      "bold": false,
      "italic": false,
      "underline": false
    }
  }
}
```

## Unified Theming

With both formats generated, you can use the same theme name across components:

```rust
use rich_rs::{Syntax, Pretty};

// Syntax highlighting with Dracula
let syntax = Syntax::new(code, "python3").with_theme("dracula");

// Pretty printing with Dracula
let pretty = Pretty::new(&data).with_theme("dracula");
```

## Included Themes

The following themes have been imported and embedded in rich-rs:

- `monokai` - Classic dark theme (default for Syntax)
- `dracula` - Popular dark theme with purple accents
- `gruvbox-dark` - Retro groove dark theme
- `nord` - Arctic, north-bluish color palette

## Troubleshooting

### Theme colors look wrong

Some Pygments tokens don't have direct TextMate equivalents. Check the mapping tables in `src/main.rs` and adjust scopes if needed.

### Missing styles in Pretty output

The `.theme` file may not have all repr/json styles mapped. Check `get_repr_mapping()` in `src/main.rs` to add missing Pygments-to-repr mappings.

### Theme not loading

For `.tmTheme`: Ensure valid XML (proper hex colors, XML escaping).
For `.theme`: Ensure valid INI format with `[styles]` section header.
