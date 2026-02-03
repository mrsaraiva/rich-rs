//! Export format templates for SVG and HTML.
//!
//! This module contains the template strings used to generate SVG and HTML
//! output from recorded console content.

/// SVG format template for console export.
///
/// This template uses the following placeholder variables:
///
/// - `{unique_id}` - Unique identifier for CSS classes and element IDs
/// - `{width}` - Total SVG width including margins
/// - `{height}` - Total SVG height including margins
/// - `{char_width}` - Character width in pixels
/// - `{char_height}` - Character height in pixels
/// - `{line_height}` - Line height in pixels
/// - `{terminal_width}` - Terminal content width (excluding padding)
/// - `{terminal_height}` - Terminal content height (excluding padding)
/// - `{terminal_x}` - X offset for terminal content
/// - `{terminal_y}` - Y offset for terminal content
/// - `{styles}` - CSS style rules for text styling
/// - `{chrome}` - Terminal window chrome (background, title, buttons)
/// - `{backgrounds}` - Background rectangles for styled text
/// - `{matrix}` - Text content elements
/// - `{lines}` - Clip path definitions for each line
pub const CONSOLE_SVG_FORMAT: &str = r#"<svg class="rich-terminal" viewBox="0 0 {width} {height}" xmlns="http://www.w3.org/2000/svg">
    <!-- Generated with Rich-rs https://github.com/Textualize/rich -->
    <style>

    @font-face {
        font-family: "Fira Code";
        src: local("FiraCode-Regular"),
                url("https://cdnjs.cloudflare.com/ajax/libs/firacode/6.2.0/woff2/FiraCode-Regular.woff2") format("woff2"),
                url("https://cdnjs.cloudflare.com/ajax/libs/firacode/6.2.0/woff/FiraCode-Regular.woff") format("woff");
        font-style: normal;
        font-weight: 400;
    }
    @font-face {
        font-family: "Fira Code";
        src: local("FiraCode-Bold"),
                url("https://cdnjs.cloudflare.com/ajax/libs/firacode/6.2.0/woff2/FiraCode-Bold.woff2") format("woff2"),
                url("https://cdnjs.cloudflare.com/ajax/libs/firacode/6.2.0/woff/FiraCode-Bold.woff") format("woff");
        font-style: bold;
        font-weight: 700;
    }

    .{unique_id}-matrix {
        font-family: Fira Code, monospace;
        font-size: {char_height}px;
        line-height: {line_height}px;
        font-variant-east-asian: full-width;
    }

    .{unique_id}-title {
        font-size: 18px;
        font-weight: bold;
        font-family: arial;
    }

    {styles}
    </style>

    <defs>
    <clipPath id="{unique_id}-clip-terminal">
      <rect x="0" y="0" width="{terminal_width}" height="{terminal_height}" />
    </clipPath>
    {lines}
    </defs>

    {chrome}
    <g transform="translate({terminal_x}, {terminal_y})" clip-path="url(#{unique_id}-clip-terminal)">
    {backgrounds}
    <g class="{unique_id}-matrix">
    {matrix}
    </g>
    </g>
</svg>
"#;

/// HTML format template for console export.
///
/// This template uses the following placeholder variables:
///
/// - `{stylesheet}` - Additional CSS styles
/// - `{foreground}` - Foreground color (CSS color value)
/// - `{background}` - Background color (CSS color value)
/// - `{code}` - HTML-encoded console content
pub const CONSOLE_HTML_FORMAT: &str = r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<style>
{stylesheet}
body {
    color: {foreground};
    background-color: {background};
}
</style>
</head>
<body>
    <pre style="font-family:Menlo,'DejaVu Sans Mono',consolas,'Courier New',monospace"><code style="font-family:inherit">{code}</code></pre>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_format_contains_placeholders() {
        assert!(CONSOLE_SVG_FORMAT.contains("{unique_id}"));
        assert!(CONSOLE_SVG_FORMAT.contains("{width}"));
        assert!(CONSOLE_SVG_FORMAT.contains("{height}"));
        assert!(CONSOLE_SVG_FORMAT.contains("{styles}"));
        assert!(CONSOLE_SVG_FORMAT.contains("{chrome}"));
        assert!(CONSOLE_SVG_FORMAT.contains("{matrix}"));
    }

    #[test]
    fn test_html_format_contains_placeholders() {
        assert!(CONSOLE_HTML_FORMAT.contains("{stylesheet}"));
        assert!(CONSOLE_HTML_FORMAT.contains("{foreground}"));
        assert!(CONSOLE_HTML_FORMAT.contains("{background}"));
        assert!(CONSOLE_HTML_FORMAT.contains("{code}"));
    }
}
