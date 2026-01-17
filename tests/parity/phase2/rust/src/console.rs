use rich_rs::{Console, Style, Theme};
use std::collections::HashMap;

pub fn run() {
    println!("=== Console.render_str() ===");

    let console = Console::new();

    // Basic markup
    let text = console.render_str("[bold]Hello[/bold] World", Some(true), Some(true), None, None);
    println!(
        r#"render_str("[bold]Hello[/] World") -> plain="{}", spans={}"#,
        text.plain_text(),
        text.spans().len()
    );

    // Nested markup
    let text = console.render_str("[bold][red]Nested[/red][/bold]", Some(true), Some(true), None, None);
    println!(
        r#"render_str("[bold][red]Nested[/]") -> plain="{}", spans={}"#,
        text.plain_text(),
        text.spans().len()
    );

    // Emoji replacement
    let text = console.render_str(":smile: emoji", Some(true), Some(true), None, None);
    let has_emoji = text.plain_text().contains('\u{1f604}');
    println!(
        r#"render_str(":smile: emoji") -> has_emoji={}"#,
        has_emoji
    );

    // Markup disabled
    let text = console.render_str("[bold]literal[/bold]", Some(false), Some(true), None, None);
    println!(
        r#"render_str(markup=False) -> plain="{}""#,
        text.plain_text()
    );

    // Emoji disabled
    let text = console.render_str(":smile: literal", Some(true), Some(false), None, None);
    let has_colon = text.plain_text().contains(":smile:");
    println!(
        r#"render_str(emoji=False) -> has_colon={}"#,
        has_colon
    );

    // Both disabled
    let text = console.render_str("[bold]:smile:[/bold]", Some(false), Some(false), None, None);
    println!(
        r#"render_str(both=False) -> plain="{}""#,
        text.plain_text()
    );

    println!("\n=== Theme.styles ===");

    // Default theme has standard styles
    let theme = Theme::default();

    // Check some default style names exist
    let has_bold = theme.has_style("bold");
    let has_red = theme.has_style("red");
    let has_italic = theme.has_style("italic");
    println!("default theme has 'bold': {}", has_bold);
    println!("default theme has 'red': {}", has_red);
    println!("default theme has 'italic': {}", has_italic);

    // Custom theme
    let mut styles: HashMap<String, Style> = HashMap::new();
    styles.insert("custom.test".to_string(), Style::parse("bold magenta").unwrap());
    let custom_theme = Theme::with_styles(styles, false);
    let has_custom = custom_theme.has_style("custom.test");
    println!("custom theme has 'custom.test': {}", has_custom);

    // Style inheritance
    let mut styles: HashMap<String, Style> = HashMap::new();
    styles.insert("myerror".to_string(), Style::parse("bold red").unwrap());
    let inherited_theme = Theme::with_styles(styles, true);
    let has_bold_inherited = inherited_theme.has_style("bold");
    let has_myerror = inherited_theme.has_style("myerror");
    println!("inherited theme has 'bold': {}", has_bold_inherited);
    println!("inherited theme has 'myerror': {}", has_myerror);

    println!("\n=== Console.get_style() ===");

    let console = Console::new();

    // Get standard styles from theme stack
    let style = console.theme_stack().get_style("bold");
    let is_bold = style.map(|s| s.bold == Some(true)).unwrap_or(false);
    println!(r#"get_style("bold") -> bold={}"#, is_bold);

    let style = console.theme_stack().get_style("italic");
    let is_italic = style.map(|s| s.italic == Some(true)).unwrap_or(false);
    println!(r#"get_style("italic") -> italic={}"#, is_italic);

    let style = console.theme_stack().get_style("red");
    let is_red = style.is_some(); // Has the style
    println!(r#"get_style("red") -> is_red={}"#, is_red);

    // Parse style string (using Style::parse directly since Console doesn't have get_style for arbitrary strings)
    let style = Style::parse("bold red on blue").unwrap();
    let has_bold = style.bold == Some(true);
    let has_color = style.color.is_some();
    let has_bgcolor = style.bgcolor.is_some();
    println!(
        r#"get_style("bold red on blue") -> bold={}, has_color={}, has_bgcolor={}"#,
        has_bold, has_color, has_bgcolor
    );

    println!("\n=== Console with custom theme ===");

    let mut styles: HashMap<String, Style> = HashMap::new();
    styles.insert("highlight".to_string(), Style::parse("bold yellow").unwrap());
    let custom = Theme::with_styles(styles, true);
    let mut console = Console::new();
    console.push_theme(custom);

    let style = console.theme_stack().get_style("highlight");
    let is_bold = style.map(|s| s.bold == Some(true)).unwrap_or(false);
    let is_yellow = style.is_some(); // Has some color
    println!(
        r#"get_style("highlight") -> bold={}, yellow={}"#,
        is_bold, is_yellow
    );
}
