//! Pygments to TextMate theme converter.
//!
//! This tool converts Pygments theme JSON exports to:
//! 1. TextMate .tmTheme format for syntect-based syntax highlighters
//! 2. INI config format for rich-rs Theme (Pretty/RegexHighlighter)

use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Convert Pygments themes to TextMate .tmTheme and rich-rs Theme formats.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input JSON file (exported from Pygments)
    input: String,

    /// Output .tmTheme file (for Syntax highlighting)
    #[arg(short, long)]
    output: Option<String>,

    /// Also generate .theme INI file (for Pretty/RegexHighlighter)
    #[arg(long, default_value = "true")]
    generate_theme: bool,
}

#[derive(Debug, Deserialize)]
struct PygmentsTheme {
    name: String,
    background_color: String,
    #[serde(default)]
    highlight_color: Option<String>,
    #[serde(default)]
    line_number_color: Option<String>,
    #[serde(default)]
    line_number_background_color: Option<String>,
    styles: HashMap<String, TokenStyle>,
}

#[derive(Debug, Deserialize, Clone)]
struct TokenStyle {
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    bgcolor: Option<String>,
    #[serde(default)]
    bold: bool,
    #[serde(default)]
    italic: bool,
    #[serde(default)]
    underline: bool,
}

/// Mapping from Pygments token names to TextMate scopes.
/// Each Pygments token maps to one or more TextMate scopes.
fn get_token_mapping() -> HashMap<&'static str, Vec<&'static str>> {
    let mut m = HashMap::new();

    // Comments
    m.insert("Comment", vec!["comment"]);
    m.insert("Comment.Hashbang", vec!["comment.line.shebang"]);
    m.insert("Comment.Multiline", vec!["comment.block"]);
    m.insert("Comment.Preproc", vec!["comment.block.preprocessor", "meta.preprocessor"]);
    m.insert("Comment.PreprocFile", vec!["comment.block.preprocessor"]);
    m.insert("Comment.Single", vec!["comment.line"]);
    m.insert("Comment.Special", vec!["comment.block.documentation"]);

    // Errors
    m.insert("Error", vec!["invalid.illegal"]);

    // Escapes
    m.insert("Escape", vec!["constant.character.escape"]);

    // Generic tokens (for diffs, etc.)
    m.insert("Generic", vec!["markup"]);
    m.insert("Generic.Deleted", vec!["markup.deleted"]);
    m.insert("Generic.Emph", vec!["markup.italic"]);
    m.insert("Generic.Error", vec!["invalid.illegal"]);
    m.insert("Generic.Heading", vec!["markup.heading"]);
    m.insert("Generic.Inserted", vec!["markup.inserted"]);
    m.insert("Generic.Output", vec!["markup.raw"]);
    m.insert("Generic.Prompt", vec!["markup.raw"]);
    m.insert("Generic.Strong", vec!["markup.bold"]);
    m.insert("Generic.Subheading", vec!["markup.heading"]);
    m.insert("Generic.Traceback", vec!["invalid.deprecated"]);

    // Keywords
    m.insert("Keyword", vec!["keyword"]);
    m.insert("Keyword.Constant", vec!["constant.language"]);
    m.insert("Keyword.Declaration", vec!["storage.type", "keyword.declaration"]);
    m.insert("Keyword.Namespace", vec!["keyword.control.import", "keyword.other.import"]);
    m.insert("Keyword.Pseudo", vec!["keyword.other"]);
    m.insert("Keyword.Reserved", vec!["keyword.reserved"]);
    m.insert("Keyword.Type", vec!["storage.type", "support.type"]);

    // Literals
    m.insert("Literal", vec!["constant"]);
    m.insert("Literal.Date", vec!["constant.other.date"]);

    // Names
    m.insert("Name", vec!["variable"]);
    m.insert("Name.Attribute", vec!["entity.other.attribute-name"]);
    m.insert("Name.Builtin", vec!["support.function", "variable.language"]);
    m.insert("Name.Builtin.Pseudo", vec!["variable.language"]);
    m.insert("Name.Class", vec!["entity.name.class", "entity.name.type.class"]);
    m.insert("Name.Constant", vec!["constant.other", "variable.other.constant"]);
    m.insert("Name.Decorator", vec!["entity.name.function.decorator", "meta.decorator"]);
    m.insert("Name.Entity", vec!["constant.character.entity"]);
    m.insert("Name.Exception", vec!["support.type.exception", "entity.name.type.class.exception"]);
    m.insert("Name.Function", vec!["entity.name.function"]);
    m.insert("Name.Function.Magic", vec!["support.function.magic"]);
    m.insert("Name.Label", vec!["entity.name.label"]);
    m.insert("Name.Namespace", vec!["entity.name.namespace", "entity.name.module"]);
    m.insert("Name.Other", vec!["variable.other"]);
    m.insert("Name.Property", vec!["variable.other.property"]);
    m.insert("Name.Tag", vec!["entity.name.tag"]);
    m.insert("Name.Variable", vec!["variable"]);
    m.insert("Name.Variable.Class", vec!["variable.other.class"]);
    m.insert("Name.Variable.Global", vec!["variable.other.global"]);
    m.insert("Name.Variable.Instance", vec!["variable.other.instance"]);
    m.insert("Name.Variable.Magic", vec!["variable.language"]);

    // Numbers
    m.insert("Number", vec!["constant.numeric"]);
    m.insert("Number.Bin", vec!["constant.numeric.binary"]);
    m.insert("Number.Float", vec!["constant.numeric.float"]);
    m.insert("Number.Hex", vec!["constant.numeric.hex"]);
    m.insert("Number.Integer", vec!["constant.numeric.integer"]);
    m.insert("Number.Integer.Long", vec!["constant.numeric.integer.long"]);
    m.insert("Number.Oct", vec!["constant.numeric.octal"]);

    // Operators
    m.insert("Operator", vec!["keyword.operator"]);
    m.insert("Operator.Word", vec!["keyword.operator.word"]);

    // Punctuation
    m.insert("Punctuation", vec!["punctuation"]);
    m.insert("Punctuation.Marker", vec!["punctuation.separator"]);

    // Strings
    m.insert("String", vec!["string"]);
    m.insert("String.Affix", vec!["storage.type.string"]);
    m.insert("String.Backtick", vec!["string.quoted.other"]);
    m.insert("String.Char", vec!["string.quoted.single", "constant.character"]);
    m.insert("String.Delimiter", vec!["punctuation.definition.string"]);
    m.insert("String.Doc", vec!["comment.block.documentation", "string.quoted.docstring"]);
    m.insert("String.Double", vec!["string.quoted.double"]);
    m.insert("String.Escape", vec!["constant.character.escape"]);
    m.insert("String.Heredoc", vec!["string.unquoted.heredoc"]);
    m.insert("String.Interpol", vec!["meta.embedded", "string.interpolated"]);
    m.insert("String.Other", vec!["string.other"]);
    m.insert("String.Regex", vec!["string.regexp"]);
    m.insert("String.Single", vec!["string.quoted.single"]);
    m.insert("String.Symbol", vec!["constant.other.symbol"]);

    // Text
    m.insert("Text", vec!["text"]);
    m.insert("Text.Whitespace", vec!["text"]);

    // Other common patterns
    m.insert("Keyword.Control", vec!["keyword.control"]);

    m
}

/// Generate a tmTheme XML file from a Pygments theme.
fn generate_tmtheme(theme: &PygmentsTheme) -> String {
    let mut xml = String::new();

    // XML header
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>name</key>
	<string>"#);
    xml.push_str(&capitalize(&theme.name));
    xml.push_str(r#"</string>
	<key>settings</key>
	<array>
		<!-- Global settings -->
		<dict>
			<key>settings</key>
			<dict>
				<key>background</key>
				<string>"#);
    xml.push_str(&theme.background_color);
    xml.push_str(r#"</string>
				<key>foreground</key>
				<string>"#);

    // Find default foreground from Text or Name token
    let default_fg = theme
        .styles
        .get("Text")
        .or_else(|| theme.styles.get("Name"))
        .and_then(|s| s.color.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("#f8f8f2");
    xml.push_str(default_fg);

    xml.push_str(r#"</string>
				<key>caret</key>
				<string>"#);
    xml.push_str(default_fg);
    xml.push_str(r#"</string>
				<key>selection</key>
				<string>"#);
    // Use highlight color or derive from background
    let selection = theme
        .highlight_color
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("#49483e");
    xml.push_str(selection);
    xml.push_str(r#"</string>
				<key>lineHighlight</key>
				<string>"#);
    xml.push_str(selection);
    xml.push_str(r#"</string>
			</dict>
		</dict>

"#);

    // Get the mapping
    let mapping = get_token_mapping();

    // Generate style rules
    for (pygments_token, style) in &theme.styles {
        if let Some(scopes) = mapping.get(pygments_token.as_str()) {
            let scope_string = scopes.join(", ");
            xml.push_str(&generate_style_rule(pygments_token, &scope_string, style));
        }
    }

    // Close the XML
    xml.push_str(r#"	</array>
</dict>
</plist>
"#);

    xml
}

fn generate_style_rule(name: &str, scope: &str, style: &TokenStyle) -> String {
    let mut rule = String::new();

    rule.push_str("\t\t<!-- ");
    rule.push_str(name);
    rule.push_str(" -->\n");
    rule.push_str("\t\t<dict>\n");
    rule.push_str("\t\t\t<key>name</key>\n");
    rule.push_str("\t\t\t<string>");
    rule.push_str(name);
    rule.push_str("</string>\n");
    rule.push_str("\t\t\t<key>scope</key>\n");
    rule.push_str("\t\t\t<string>");
    rule.push_str(scope);
    rule.push_str("</string>\n");
    rule.push_str("\t\t\t<key>settings</key>\n");
    rule.push_str("\t\t\t<dict>\n");

    if let Some(ref color) = style.color {
        rule.push_str("\t\t\t\t<key>foreground</key>\n");
        rule.push_str("\t\t\t\t<string>");
        rule.push_str(color);
        rule.push_str("</string>\n");
    }

    if let Some(ref bgcolor) = style.bgcolor {
        rule.push_str("\t\t\t\t<key>background</key>\n");
        rule.push_str("\t\t\t\t<string>");
        rule.push_str(bgcolor);
        rule.push_str("</string>\n");
    }

    // Build font style
    let mut font_styles = Vec::new();
    if style.bold {
        font_styles.push("bold");
    }
    if style.italic {
        font_styles.push("italic");
    }
    if style.underline {
        font_styles.push("underline");
    }

    if !font_styles.is_empty() {
        rule.push_str("\t\t\t\t<key>fontStyle</key>\n");
        rule.push_str("\t\t\t\t<string>");
        rule.push_str(&font_styles.join(" "));
        rule.push_str("</string>\n");
    }

    rule.push_str("\t\t\t</dict>\n");
    rule.push_str("\t\t</dict>\n\n");

    rule
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

/// Mapping from Pygments tokens to rich-rs Theme style names.
/// This enables Pretty and RegexHighlighter to use Pygments theme colors.
fn get_repr_mapping() -> HashMap<&'static str, Vec<&'static str>> {
    let mut m = HashMap::new();

    // Numbers
    m.insert("Number", vec!["repr.number", "repr.number_complex", "json.number"]);
    m.insert("Literal.Number", vec!["repr.number", "repr.number_complex", "json.number"]);

    // Strings
    m.insert("String", vec!["repr.str", "json.str"]);
    m.insert("Literal.String", vec!["repr.str", "json.str"]);

    // Booleans - True uses green-ish, False uses red-ish from Keyword.Constant
    // We'll handle these specially
    m.insert("Keyword.Constant", vec!["repr.bool_true", "repr.bool_false", "repr.none", "json.bool_true", "json.bool_false", "json.null"]);

    // Names/identifiers
    m.insert("Name.Function", vec!["repr.call"]);
    m.insert("Name.Class", vec!["repr.tag_name"]);
    m.insert("Name.Attribute", vec!["repr.attrib_name"]);

    // URLs and special types
    m.insert("Name.Label", vec!["repr.url", "repr.path"]);
    m.insert("String.Other", vec!["repr.uuid", "repr.ipv4", "repr.ipv6"]);

    // Operators and punctuation
    m.insert("Punctuation", vec!["repr.brace", "repr.comma", "repr.tag_start", "repr.tag_end", "json.brace"]);
    m.insert("Operator", vec!["repr.attrib_equal"]);

    // Comments/docs for indent guides
    m.insert("Comment", vec!["repr.indent"]);

    // Errors
    m.insert("Error", vec!["repr.error"]);
    m.insert("Generic.Error", vec!["repr.error"]);

    // Keys (using Name.Attribute or similar)
    m.insert("Name.Tag", vec!["json.key"]);

    m
}

/// Generate a rich-rs Theme INI config file from a Pygments theme.
fn generate_theme_ini(theme: &PygmentsTheme) -> String {
    let mut ini = String::new();
    ini.push_str(&format!("# Rich-rs theme generated from Pygments '{}' theme\n", theme.name));
    ini.push_str("# Use with Theme::from_file() or Theme::from_reader()\n\n");
    ini.push_str("[styles]\n");

    let mapping = get_repr_mapping();

    // Collect all style assignments
    let mut style_lines: Vec<(String, String)> = Vec::new();

    for (pygments_token, style) in &theme.styles {
        if let Some(repr_names) = mapping.get(pygments_token.as_str()) {
            let style_str = token_style_to_ini(style);
            if !style_str.is_empty() {
                for repr_name in repr_names {
                    // Handle special cases for bool_true/bool_false/none
                    if *repr_name == "repr.bool_true" || *repr_name == "json.bool_true" {
                        // Use the color but with italic
                        let mut modified = style.clone();
                        modified.italic = true;
                        style_lines.push((repr_name.to_string(), token_style_to_ini(&modified)));
                    } else if *repr_name == "repr.bool_false" || *repr_name == "json.bool_false" {
                        // Use a slightly different style (could be same color, italic)
                        let mut modified = style.clone();
                        modified.italic = true;
                        style_lines.push((repr_name.to_string(), token_style_to_ini(&modified)));
                    } else if *repr_name == "repr.none" || *repr_name == "json.null" {
                        let mut modified = style.clone();
                        modified.italic = true;
                        style_lines.push((repr_name.to_string(), token_style_to_ini(&modified)));
                    } else {
                        style_lines.push((repr_name.to_string(), style_str.clone()));
                    }
                }
            }
        }
    }

    // Also try to map some styles that might come from different tokens
    // Number styles from Literal.Number.* variants
    for (pygments_token, style) in &theme.styles {
        let style_str = token_style_to_ini(style);
        if style_str.is_empty() {
            continue;
        }

        // Map specific Number subtypes
        if pygments_token.starts_with("Number.") || pygments_token.starts_with("Literal.Number") {
            if !style_lines.iter().any(|(n, _)| n == "repr.number") {
                style_lines.push(("repr.number".to_string(), style_str.clone()));
                style_lines.push(("json.number".to_string(), style_str.clone()));
            }
        }

        // Map specific String subtypes
        if pygments_token.starts_with("String.") || pygments_token.starts_with("Literal.String") {
            if !style_lines.iter().any(|(n, _)| n == "repr.str") {
                style_lines.push(("repr.str".to_string(), style_str.clone()));
                style_lines.push(("json.str".to_string(), style_str.clone()));
            }
        }
    }

    // Sort and deduplicate
    style_lines.sort_by(|a, b| a.0.cmp(&b.0));
    style_lines.dedup_by(|a, b| a.0 == b.0);

    for (name, style) in style_lines {
        ini.push_str(&format!("{} = {}\n", name, style));
    }

    ini
}

/// Convert a TokenStyle to INI style string format.
fn token_style_to_ini(style: &TokenStyle) -> String {
    let mut parts = Vec::new();

    if style.bold {
        parts.push("bold".to_string());
    }
    if style.italic {
        parts.push("italic".to_string());
    }
    if style.underline {
        parts.push("underline".to_string());
    }

    if let Some(ref color) = style.color {
        // Convert #RRGGBB to rgb(r,g,b) format
        if color.starts_with('#') && color.len() == 7 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&color[1..3], 16),
                u8::from_str_radix(&color[3..5], 16),
                u8::from_str_radix(&color[5..7], 16),
            ) {
                parts.push(format!("rgb({},{},{})", r, g, b));
            }
        }
    }

    if let Some(ref bgcolor) = style.bgcolor {
        if bgcolor.starts_with('#') && bgcolor.len() == 7 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&bgcolor[1..3], 16),
                u8::from_str_radix(&bgcolor[3..5], 16),
                u8::from_str_radix(&bgcolor[5..7], 16),
            ) {
                parts.push(format!("on rgb({},{},{})", r, g, b));
            }
        }
    }

    parts.join(" ")
}

fn main() {
    let args = Args::parse();

    // Read the input JSON
    let json_content = fs::read_to_string(&args.input).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", args.input, e);
        std::process::exit(1);
    });

    // Parse the theme
    let theme: PygmentsTheme = serde_json::from_str(&json_content).unwrap_or_else(|e| {
        eprintln!("Error parsing JSON: {}", e);
        std::process::exit(1);
    });

    // Generate tmTheme
    let tmtheme = generate_tmtheme(&theme);

    // Determine output path
    let output_path = args
        .output
        .unwrap_or_else(|| format!("{}.tmTheme", theme.name));

    // Write tmTheme output
    let mut file = fs::File::create(&output_path).unwrap_or_else(|e| {
        eprintln!("Error creating {}: {}", output_path, e);
        std::process::exit(1);
    });

    file.write_all(tmtheme.as_bytes()).unwrap_or_else(|e| {
        eprintln!("Error writing {}: {}", output_path, e);
        std::process::exit(1);
    });

    println!("Generated: {}", output_path);
    println!("  Theme: {}", theme.name);
    println!("  Background: {}", theme.background_color);
    println!("  Styles: {} token types mapped", theme.styles.len());

    // Generate Theme INI file if requested
    if args.generate_theme {
        let theme_ini = generate_theme_ini(&theme);

        // Derive theme path from output path
        let theme_path = Path::new(&output_path)
            .with_extension("theme")
            .to_string_lossy()
            .to_string();

        let mut theme_file = fs::File::create(&theme_path).unwrap_or_else(|e| {
            eprintln!("Error creating {}: {}", theme_path, e);
            std::process::exit(1);
        });

        theme_file
            .write_all(theme_ini.as_bytes())
            .unwrap_or_else(|e| {
                eprintln!("Error writing {}: {}", theme_path, e);
                std::process::exit(1);
            });

        println!("Generated: {}", theme_path);
        println!("  For Pretty/RegexHighlighter theming");
    }
}
