//! Markdown rendering with Rich formatting.
//!
//! This module provides a `Markdown` struct that parses and renders Markdown
//! text with beautiful terminal formatting, including:
//! - Headings (h1-h6) with different styles
//! - Code blocks with syntax highlighting (via Syntax)
//! - Block quotes with distinctive styling
//! - Ordered and unordered lists
//! - Tables
//! - Inline formatting (bold, italic, strikethrough, code)
//! - Links and images
//! - Horizontal rules
//!
//! # Example
//!
//! ```
//! use rich_rs::markdown::Markdown;
//! use rich_rs::{Console, ConsoleOptions, Renderable};
//!
//! let md = Markdown::new("# Hello\n\nThis is **bold** and *italic*.");
//! let console = Console::new();
//! let options = ConsoleOptions::default();
//! let segments = md.render(&console, &options);
//! // segments can now be printed or processed
//! ```

use std::collections::VecDeque;
use std::io::Stdout;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::Renderable;
use crate::align::Align;
use crate::cells::cell_len;
use crate::console::{Console, ConsoleOptions, JustifyMethod};
use crate::measure::Measurement;
use crate::padding::{Padding, PaddingDimensions};
use crate::rule::Rule;
use crate::segment::{Segment, Segments};
use crate::style::Style;
use crate::syntax::Syntax;
use crate::table::{Column, Table};
use crate::text::Text;

// ============================================================================
// Style definitions
// ============================================================================

/// Default styles for markdown elements.
mod styles {
    use crate::color::SimpleColor;
    use crate::style::Style;

    fn cyan() -> SimpleColor {
        SimpleColor::parse("cyan").unwrap_or(SimpleColor::Default)
    }

    fn magenta() -> SimpleColor {
        SimpleColor::parse("magenta").unwrap_or(SimpleColor::Default)
    }

    fn blue() -> SimpleColor {
        SimpleColor::parse("blue").unwrap_or(SimpleColor::Default)
    }

    fn black() -> SimpleColor {
        SimpleColor::parse("black").unwrap_or(SimpleColor::Default)
    }

    fn bright_blue() -> SimpleColor {
        SimpleColor::parse("bright_blue").unwrap_or(SimpleColor::Default)
    }

    pub fn heading1() -> Style {
        // Python Rich default: bold + underline
        Style::new().with_bold(true).with_underline(true)
    }

    pub fn heading2() -> Style {
        Style::new().with_color(magenta()).with_underline(true)
    }

    pub fn heading3() -> Style {
        Style::new().with_color(magenta()).with_bold(true)
    }

    pub fn heading4() -> Style {
        Style::new().with_color(magenta()).with_italic(true)
    }

    pub fn heading5() -> Style {
        Style::new().with_italic(true)
    }

    pub fn heading6() -> Style {
        Style::new().with_dim(true)
    }

    pub fn emphasis() -> Style {
        Style::new().with_italic(true)
    }

    pub fn strong() -> Style {
        Style::new().with_bold(true)
    }

    pub fn code() -> Style {
        // Python Rich uses "bold cyan on black" for inline code
        Style::new()
            .with_bold(true)
            .with_color(cyan())
            .with_bgcolor(black())
    }

    pub fn block_quote() -> Style {
        Style::new().with_color(magenta())
    }

    pub fn block_quote_border() -> Style {
        Style::new().with_color(magenta())
    }

    pub fn link() -> Style {
        Style::new().with_color(bright_blue())
    }

    pub fn link_url() -> Style {
        Style::new().with_color(blue()).with_dim(true)
    }

    pub fn strikethrough() -> Style {
        Style::new().with_strike(true).with_dim(true)
    }

    pub fn item_bullet() -> Style {
        Style::new().with_bold(true)
    }

    pub fn item_number() -> Style {
        Style::new().with_color(cyan())
    }

    pub fn horizontal_rule() -> Style {
        Style::new().with_dim(true)
    }
}

// ============================================================================
// Markdown Context
// ============================================================================

/// Context for markdown rendering, managing style stack and state.
struct MarkdownContext {
    /// Stack of styles for nested formatting.
    style_stack: Vec<Style>,
    /// Whether we're inside a link.
    in_link: bool,
    /// Current link URL if in a link.
    link_url: Option<String>,
    /// Whether hyperlinks should use terminal hyperlinks.
    hyperlinks: bool,
    /// Lexer for inline code highlighting.
    inline_code_lexer: Option<String>,
    /// Code theme for syntax highlighting.
    code_theme: String,
    /// Optional padding to apply to code blocks (Rich CLI uses (0, 4)).
    code_block_padding: Option<PaddingDimensions>,
    /// Text justification.
    #[allow(dead_code)]
    justify: Option<JustifyMethod>,
}

impl MarkdownContext {
    fn new(
        hyperlinks: bool,
        inline_code_lexer: Option<String>,
        code_theme: String,
        code_block_padding: Option<PaddingDimensions>,
        justify: Option<JustifyMethod>,
    ) -> Self {
        Self {
            style_stack: vec![Style::default()],
            in_link: false,
            link_url: None,
            hyperlinks,
            inline_code_lexer,
            code_theme,
            code_block_padding,
            justify,
        }
    }

    fn push_style(&mut self, style: Style) {
        let current = self.current_style();
        self.style_stack.push(current + style);
    }

    fn pop_style(&mut self) {
        if self.style_stack.len() > 1 {
            self.style_stack.pop();
        }
    }

    fn current_style(&self) -> Style {
        self.style_stack.last().copied().unwrap_or_default()
    }

    fn enter_link(&mut self, url: String) {
        self.in_link = true;
        self.link_url = Some(url);
        self.push_style(styles::link());
    }

    fn leave_link(&mut self) -> Option<String> {
        self.in_link = false;
        self.pop_style();
        self.link_url.take()
    }
}

// ============================================================================
// Markdown Element Types
// ============================================================================

/// Represents a renderable block element.
enum BlockElement {
    Paragraph(Text),
    Heading {
        level: HeadingLevel,
        text: Text,
    },
    CodeBlock {
        code: String,
        language: Option<String>,
    },
    BlockQuote(Vec<BlockElement>),
    List {
        ordered: bool,
        start: u64,
        items: Vec<ListItem>,
    },
    Table {
        headers: Vec<Text>,
        rows: Vec<Vec<Text>>,
        alignments: Vec<Option<pulldown_cmark::Alignment>>,
    },
    HorizontalRule,
}

/// A list item containing block elements.
struct ListItem {
    blocks: Vec<BlockElement>,
}

impl BlockElement {
    /// Render this block element to segments.
    fn render(
        &self,
        console: &Console<Stdout>,
        options: &ConsoleOptions,
        context: &MarkdownContext,
    ) -> Segments {
        match self {
            BlockElement::Paragraph(text) => {
                let mut result = text.render(console, options);
                result.push(Segment::line());
                result
            }

            BlockElement::Heading { level, text } => {
                let mut result = Segments::new();

                match level {
                    HeadingLevel::H1 => {
                        // Python Rich: centered h1 with bold + underline (no panel).
                        let mut styled = text.clone();
                        styled.stylize(0, styled.len(), styles::heading1());
                        let centered = Align::center(Box::new(styled));
                        result = centered.render(console, options);
                    }
                    HeadingLevel::H2 => {
                        let mut styled = text.clone();
                        styled.stylize(0, styled.len(), styles::heading2());
                        let segs = styled.render(console, options);
                        for seg in segs {
                            result.push(seg);
                        }
                        result.push(Segment::line());
                    }
                    HeadingLevel::H3 => {
                        let mut styled = text.clone();
                        styled.stylize(0, styled.len(), styles::heading3());
                        let segs = styled.render(console, options);
                        for seg in segs {
                            result.push(seg);
                        }
                        result.push(Segment::line());
                    }
                    HeadingLevel::H4 => {
                        let mut styled = text.clone();
                        styled.stylize(0, styled.len(), styles::heading4());
                        let segs = styled.render(console, options);
                        for seg in segs {
                            result.push(seg);
                        }
                        result.push(Segment::line());
                    }
                    HeadingLevel::H5 => {
                        let mut styled = text.clone();
                        styled.stylize(0, styled.len(), styles::heading5());
                        let segs = styled.render(console, options);
                        for seg in segs {
                            result.push(seg);
                        }
                        result.push(Segment::line());
                    }
                    HeadingLevel::H6 => {
                        let mut styled = text.clone();
                        styled.stylize(0, styled.len(), styles::heading6());
                        let segs = styled.render(console, options);
                        for seg in segs {
                            result.push(seg);
                        }
                        result.push(Segment::line());
                    }
                }

                result
            }

            BlockElement::CodeBlock { code, language } => {
                let lexer = language.as_deref().unwrap_or("text");
                let syntax = Syntax::new(code, lexer)
                    .with_theme(&context.code_theme)
                    .with_line_numbers(false);
                let mut result = if let Some(pad) = context.code_block_padding {
                    Padding::new(Box::new(syntax), pad)
                        .with_expand(false)
                        .render(console, options)
                } else {
                    syntax.render(console, options)
                };
                result.push(Segment::line());
                result
            }

            BlockElement::BlockQuote(blocks) => {
                let mut result = Segments::new();

                // Render child blocks with reduced width
                let mut child_options = options.clone();
                child_options.max_width = options.max_width.saturating_sub(4);

                for block in blocks {
                    let block_segs = block.render(console, &child_options, context);

                    // Add block quote border to each line
                    let segs_vec: Vec<Segment> = block_segs.iter().cloned().collect();
                    let lines = Segment::split_lines(segs_vec);
                    for line in lines {
                        // Add the "▌ " prefix
                        result.push(Segment::styled("▌ ", styles::block_quote_border()));
                        for seg in line {
                            // Apply block quote style to content
                            let styled_seg = Segment::styled(
                                seg.text.clone(),
                                seg.style
                                    .map(|s| s + styles::block_quote())
                                    .unwrap_or_else(styles::block_quote),
                            );
                            result.push(styled_seg);
                        }
                        result.push(Segment::line());
                    }
                }

                result
            }

            BlockElement::List {
                ordered,
                start,
                items,
            } => {
                let mut result = Segments::new();

                // Calculate max number width for padding (using saturating add to prevent overflow)
                let start_usize = *start as usize;
                let max_num = start_usize.saturating_add(items.len());
                let num_width = max_num.to_string().len();

                for (i, item) in items.iter().enumerate() {
                    let bullet = if *ordered {
                        let num = start_usize.saturating_add(i);
                        format!(" {:>width$} ", num, width = num_width)
                    } else {
                        // Python Rich uses " • " (space-bullet-space) for unordered lists
                        " • ".to_string()
                    };

                    let bullet_style = if *ordered {
                        styles::item_number()
                    } else {
                        styles::item_bullet()
                    };

                    let bullet_width = cell_len(&bullet);

                    // Render first block with bullet
                    let mut first = true;
                    for block in &item.blocks {
                        let mut child_options = options.clone();
                        child_options.max_width = options.max_width.saturating_sub(bullet_width);

                        let block_segs = block.render(console, &child_options, context);
                        let segs_vec: Vec<Segment> = block_segs.iter().cloned().collect();
                        let lines = Segment::split_lines(segs_vec);

                        for (line_idx, line) in lines.into_iter().enumerate() {
                            if first && line_idx == 0 {
                                result.push(Segment::styled(bullet.clone(), bullet_style));
                                first = false;
                            } else {
                                // Indent continuation lines
                                result.push(Segment::new(" ".repeat(bullet_width)));
                            }
                            for seg in line {
                                result.push(seg);
                            }
                            result.push(Segment::line());
                        }
                    }
                }

                result
            }

            BlockElement::Table {
                headers,
                rows,
                alignments,
            } => {
                let mut table = Table::new();

                // Add header columns
                for (i, header) in headers.iter().enumerate() {
                    let justify = alignments.get(i).and_then(|a| *a).map(|a| match a {
                        pulldown_cmark::Alignment::Left => JustifyMethod::Left,
                        pulldown_cmark::Alignment::Center => JustifyMethod::Center,
                        pulldown_cmark::Alignment::Right => JustifyMethod::Right,
                        pulldown_cmark::Alignment::None => JustifyMethod::Left,
                    });

                    let mut col = Column::with_header(Box::new(header.clone()));
                    if let Some(j) = justify {
                        col = col.justify(j);
                    }
                    table.add_column(col);
                }

                // Add rows
                for row in rows {
                    let cells: Vec<Box<dyn Renderable + Send + Sync>> = row
                        .iter()
                        .map(|t| Box::new(t.clone()) as Box<dyn Renderable + Send + Sync>)
                        .collect();
                    table.add_row_renderables(cells);
                }

                let mut result = table.render(console, options);
                result.push(Segment::line());
                result
            }

            BlockElement::HorizontalRule => {
                let rule = Rule::new().with_style(styles::horizontal_rule());
                let mut result = rule.render(console, options);
                result.push(Segment::line());
                result
            }
        }
    }
}

// ============================================================================
// Markdown Parser State Machine
// ============================================================================

/// State for parsing markdown into block elements.
struct MarkdownParser<'a> {
    events: VecDeque<Event<'a>>,
    context: &'a mut MarkdownContext,
}

impl<'a> MarkdownParser<'a> {
    fn new(events: impl Iterator<Item = Event<'a>>, context: &'a mut MarkdownContext) -> Self {
        Self {
            events: events.collect(),
            context,
        }
    }

    fn next_event(&mut self) -> Option<Event<'a>> {
        self.events.pop_front()
    }

    fn parse_blocks(&mut self) -> Vec<BlockElement> {
        let mut blocks = Vec::new();

        while let Some(event) = self.next_event() {
            if let Some(block) = self.parse_block(event) {
                blocks.push(block);
            }
        }

        blocks
    }

    fn parse_block(&mut self, event: Event<'a>) -> Option<BlockElement> {
        match event {
            Event::Start(Tag::Paragraph) => {
                let text = self.parse_inline_content(TagEnd::Paragraph);
                Some(BlockElement::Paragraph(text))
            }

            Event::Start(Tag::Heading { level, .. }) => {
                let text = self.parse_inline_content(TagEnd::Heading(level));
                Some(BlockElement::Heading { level, text })
            }

            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(lang) => {
                        let lang_str = lang.to_string();
                        if lang_str.is_empty() {
                            None
                        } else {
                            Some(lang_str)
                        }
                    }
                    CodeBlockKind::Indented => None,
                };

                let mut code = String::new();
                while let Some(event) = self.next_event() {
                    match event {
                        Event::Text(t) | Event::Code(t) => code.push_str(&t),
                        Event::End(TagEnd::CodeBlock) => break,
                        _ => {}
                    }
                }

                // Remove trailing newline
                if code.ends_with('\n') {
                    code.pop();
                }

                Some(BlockElement::CodeBlock { code, language })
            }

            Event::Start(Tag::BlockQuote(_)) => {
                let inner_blocks = self.parse_until_end(TagEnd::BlockQuote(None));
                Some(BlockElement::BlockQuote(inner_blocks))
            }

            Event::Start(Tag::List(start_num)) => {
                let ordered = start_num.is_some();
                let start = start_num.unwrap_or(1);
                let mut items = Vec::new();

                while let Some(event) = self.next_event() {
                    match event {
                        Event::Start(Tag::Item) => {
                            let item_blocks = self.parse_until_end(TagEnd::Item);
                            items.push(ListItem {
                                blocks: item_blocks,
                            });
                        }
                        Event::End(TagEnd::List(_)) => break,
                        _ => {}
                    }
                }

                Some(BlockElement::List {
                    ordered,
                    start,
                    items,
                })
            }

            Event::Start(Tag::Table(alignments)) => {
                let mut headers = Vec::new();
                let mut rows = Vec::new();
                let alignments_vec: Vec<_> = alignments.iter().map(|a| Some(*a)).collect();

                while let Some(event) = self.next_event() {
                    match event {
                        Event::Start(Tag::TableHead) => {
                            // Parse header row
                            while let Some(ev) = self.next_event() {
                                match ev {
                                    Event::Start(Tag::TableRow) => {}
                                    Event::Start(Tag::TableCell) => {
                                        let text = self.parse_inline_content(TagEnd::TableCell);
                                        headers.push(text);
                                    }
                                    Event::End(TagEnd::TableHead) => break,
                                    _ => {}
                                }
                            }
                        }
                        Event::Start(Tag::TableRow) => {
                            let mut row = Vec::new();
                            while let Some(ev) = self.next_event() {
                                match ev {
                                    Event::Start(Tag::TableCell) => {
                                        let text = self.parse_inline_content(TagEnd::TableCell);
                                        row.push(text);
                                    }
                                    Event::End(TagEnd::TableRow) => break,
                                    _ => {}
                                }
                            }
                            if !row.is_empty() {
                                rows.push(row);
                            }
                        }
                        Event::End(TagEnd::Table) => break,
                        _ => {}
                    }
                }

                Some(BlockElement::Table {
                    headers,
                    rows,
                    alignments: alignments_vec,
                })
            }

            Event::Rule => Some(BlockElement::HorizontalRule),

            _ => None,
        }
    }

    fn parse_until_end(&mut self, end_tag: TagEnd) -> Vec<BlockElement> {
        let mut blocks = Vec::new();
        // For collecting inline text that isn't wrapped in a paragraph (tight lists)
        let mut inline_text = Text::default();

        while let Some(event) = self.next_event() {
            if matches!(&event, Event::End(tag) if std::mem::discriminant(tag) == std::mem::discriminant(&end_tag))
            {
                break;
            }

            match &event {
                // Handle bare text events (tight lists)
                Event::Text(t) => {
                    inline_text.append(&**t, Some(self.context.current_style()));
                }
                Event::Code(t) => {
                    inline_text.append(&**t, Some(self.context.current_style() + styles::code()));
                }
                Event::SoftBreak => {
                    inline_text.append(" ", None);
                }
                Event::HardBreak => {
                    inline_text.append("\n", None);
                }
                // Handle inline formatting within list items
                Event::Start(Tag::Emphasis) => {
                    self.context.push_style(styles::emphasis());
                }
                Event::End(TagEnd::Emphasis) => {
                    self.context.pop_style();
                }
                Event::Start(Tag::Strong) => {
                    self.context.push_style(styles::strong());
                }
                Event::End(TagEnd::Strong) => {
                    self.context.pop_style();
                }
                Event::Start(Tag::Strikethrough) => {
                    self.context.push_style(styles::strikethrough());
                }
                Event::End(TagEnd::Strikethrough) => {
                    self.context.pop_style();
                }
                _ => {
                    // First, flush any accumulated inline text as a paragraph
                    if inline_text.len() > 0 {
                        blocks.push(BlockElement::Paragraph(std::mem::take(&mut inline_text)));
                    }
                    // Then try to parse as a block element
                    if let Some(block) = self.parse_block(event) {
                        blocks.push(block);
                    }
                }
            }
        }

        // Flush any remaining inline text
        if inline_text.len() > 0 {
            blocks.push(BlockElement::Paragraph(inline_text));
        }

        blocks
    }

    fn parse_inline_content(&mut self, end_tag: TagEnd) -> Text {
        let mut text = Text::default();

        while let Some(event) = self.next_event() {
            match event {
                Event::End(tag)
                    if std::mem::discriminant(&tag) == std::mem::discriminant(&end_tag) =>
                {
                    break;
                }

                Event::Text(t) => {
                    let style = self.context.current_style();
                    text.append(&*t, Some(style));
                }

                Event::Code(t) => {
                    // Inline code
                    if let Some(ref lexer) = self.context.inline_code_lexer {
                        // Use syntax highlighting for inline code
                        let syntax = Syntax::new(&*t, lexer);
                        let highlighted = syntax.highlight();
                        text.append_text(&highlighted);
                    } else {
                        text.append(&*t, Some(self.context.current_style() + styles::code()));
                    }
                }

                Event::SoftBreak => {
                    text.append(" ", None);
                }

                Event::HardBreak => {
                    text.append("\n", None);
                }

                Event::Start(Tag::Emphasis) => {
                    self.context.push_style(styles::emphasis());
                }

                Event::End(TagEnd::Emphasis) => {
                    self.context.pop_style();
                }

                Event::Start(Tag::Strong) => {
                    self.context.push_style(styles::strong());
                }

                Event::End(TagEnd::Strong) => {
                    self.context.pop_style();
                }

                Event::Start(Tag::Strikethrough) => {
                    self.context.push_style(styles::strikethrough());
                }

                Event::End(TagEnd::Strikethrough) => {
                    self.context.pop_style();
                }

                Event::Start(Tag::Link { dest_url, .. }) => {
                    self.context.enter_link(dest_url.to_string());
                }

                Event::End(TagEnd::Link) => {
                    if !self.context.hyperlinks {
                        // Show URL in parentheses for non-hyperlink mode
                        if let Some(url) = self.context.leave_link() {
                            text.append(" (", Some(styles::link_url()));
                            text.append(&url, Some(styles::link_url()));
                            text.append(")", Some(styles::link_url()));
                        }
                    } else {
                        self.context.leave_link();
                    }
                }

                Event::Start(Tag::Image { dest_url, .. }) => {
                    // Consume all events until End(Image) to get alt text
                    let mut alt_text = String::new();
                    while let Some(ev) = self.next_event() {
                        match ev {
                            Event::Text(t) => alt_text.push_str(&*t),
                            Event::End(TagEnd::Image) => break,
                            _ => {}
                        }
                    }

                    // Render as placeholder with emoji
                    text.append("🌆 ", None);
                    if !alt_text.is_empty() {
                        text.append(&alt_text, None);
                    } else {
                        text.append(&dest_url.to_string(), Some(styles::link_url()));
                    }
                }

                _ => {}
            }
        }

        text
    }
}

// ============================================================================
// Markdown Struct
// ============================================================================

/// A renderable that parses and displays Markdown text.
///
/// # Example
///
/// ```
/// use rich_rs::markdown::Markdown;
/// use rich_rs::{Console, ConsoleOptions, Renderable};
///
/// let md = Markdown::new("# Hello World\n\nThis is **bold** text.");
/// let console = Console::new();
/// let options = ConsoleOptions::default();
/// let segments = md.render(&console, &options);
/// ```
pub struct Markdown {
    /// The raw markdown text.
    markup: String,
    /// Whether to use terminal hyperlinks for links.
    hyperlinks: bool,
    /// Lexer for inline code highlighting (e.g., "python").
    inline_code_lexer: Option<String>,
    /// Code theme for syntax highlighting.
    code_theme: String,
    /// Optional padding to apply around fenced code blocks.
    code_block_padding: Option<PaddingDimensions>,
    /// Text justification for paragraphs.
    justify: Option<JustifyMethod>,
    /// Optional base style applied to all rendered output.
    style: Option<Style>,
}

impl Markdown {
    /// Create a new Markdown renderable from markup text.
    pub fn new(markup: impl Into<String>) -> Self {
        Self {
            markup: markup.into(),
            hyperlinks: false,
            inline_code_lexer: None,
            code_theme: crate::syntax::DEFAULT_THEME.to_string(),
            code_block_padding: None,
            justify: None,
            style: None,
        }
    }

    /// Enable terminal hyperlinks for links.
    pub fn with_hyperlinks(mut self, hyperlinks: bool) -> Self {
        self.hyperlinks = hyperlinks;
        self
    }

    /// Set a lexer for inline code highlighting.
    pub fn with_inline_code_lexer(mut self, lexer: impl Into<String>) -> Self {
        self.inline_code_lexer = Some(lexer.into());
        self
    }

    /// Set the code theme for syntax highlighting.
    pub fn with_code_theme(mut self, theme: impl Into<String>) -> Self {
        self.code_theme = theme.into();
        self
    }

    /// Set padding applied around code blocks.
    ///
    /// This is useful for matching Rich CLI's Markdown rendering, which indents fenced code blocks.
    pub fn with_code_block_padding(mut self, padding: impl Into<PaddingDimensions>) -> Self {
        self.code_block_padding = Some(padding.into());
        self
    }

    /// Set text justification for paragraphs.
    pub fn with_justify(mut self, justify: JustifyMethod) -> Self {
        self.justify = Some(justify);
        self
    }

    /// Set a base style applied to all rendered output.
    ///
    /// Matches Python Rich's `Markdown(style=...)` parameter.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Parse the markdown and return block elements.
    fn parse(&self) -> (Vec<BlockElement>, MarkdownContext) {
        let mut context = MarkdownContext::new(
            self.hyperlinks,
            self.inline_code_lexer.clone(),
            self.code_theme.clone(),
            self.code_block_padding,
            self.justify,
        );

        // Enable tables and strikethrough
        let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
        let parser = Parser::new_ext(&self.markup, options);

        let mut md_parser = MarkdownParser::new(parser, &mut context);
        let blocks = md_parser.parse_blocks();

        (blocks, context)
    }
}


impl Renderable for Markdown {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let (blocks, context) = self.parse();

        let mut result = Segments::new();

        for (idx, block) in blocks.iter().enumerate() {
            let block_segs = block.render(console, options, &context);
            for mut seg in block_segs {
                // Apply base style to all segments (matches Python Rich Markdown(style=...))
                if let Some(base) = self.style {
                    seg.style = Some(match seg.style {
                        Some(existing) => base.combine(&existing),
                        None => base,
                    });
                }
                result.push(seg);
            }

            // Preserve the visual blank line separation between markdown blocks (as in Python Rich).
            // Most block renderers already end with a newline; we add one more newline between blocks
            // unless there is already a trailing blank line.
            if idx + 1 < blocks.len() {
                let has_trailing_blank = (&result)
                    .into_iter()
                    .rev()
                    .take(2)
                    .all(|s| s.control.is_none() && s.text.as_ref() == "\n");
                if !has_trailing_blank {
                    result.push(Segment::line());
                }
            }
        }

        result
    }

    fn measure(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        // Markdown renders multiple lines. `Measurement::from_segments` assumes single-line
        // content, so split into lines and take the union.
        let segments = self.render(console, options);
        let lines = Segment::split_lines(segments);

        let mut measurement = Measurement::new(0, 0);
        for line in lines {
            let line_segments: Segments = line.into();
            measurement = measurement.union(&Measurement::from_segments(&line_segments));
        }
        measurement
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn render_to_string(md: &Markdown) -> String {
        let console = Console::new();
        let options = ConsoleOptions::default();
        let segments = md.render(&console, &options);
        segments.iter().map(|s| s.text.to_string()).collect()
    }

    #[test]
    fn test_markdown_new() {
        let md = Markdown::new("Hello");
        assert_eq!(md.markup, "Hello");
    }

    #[test]
    fn test_markdown_paragraph() {
        let md = Markdown::new("Hello world");
        let output = render_to_string(&md);
        assert!(output.contains("Hello world"));
    }

    #[test]
    fn test_markdown_heading() {
        let md = Markdown::new("# Heading 1\n\n## Heading 2");
        let output = render_to_string(&md);
        assert!(output.contains("Heading 1"));
        assert!(output.contains("Heading 2"));
    }

    #[test]
    fn test_markdown_measure_is_multiline_safe() {
        let md = Markdown::new("# Title\n\n- Item 1\n- Item 2");
        let console = Console::new();
        let options = ConsoleOptions::default();
        let measurement = md.measure(&console, &options);

        // Must be within the console width, and not the (much larger) total width of all lines.
        assert!(measurement.maximum <= options.max_width);
        assert!(measurement.minimum <= measurement.maximum);
    }

    #[test]
    fn test_markdown_bold() {
        let md = Markdown::new("This is **bold** text");
        let output = render_to_string(&md);
        assert!(output.contains("bold"));
    }

    #[test]
    fn test_markdown_italic() {
        let md = Markdown::new("This is *italic* text");
        let output = render_to_string(&md);
        assert!(output.contains("italic"));
    }

    #[test]
    fn test_markdown_code_inline() {
        let md = Markdown::new("Use `println!` macro");
        let output = render_to_string(&md);
        assert!(output.contains("println!"));
    }

    #[test]
    fn test_markdown_code_inline_style() {
        // Verify that inline code gets bold + cyan + black background styling
        let console = Console::new();
        let options = ConsoleOptions::default();
        let md = Markdown::new("Use `code` here");
        let segments = md.render(&console, &options);

        let code_seg = segments.iter().find(|s| s.text.contains("code"));
        assert!(code_seg.is_some(), "Should find 'code' segment");
        let seg = code_seg.unwrap();
        let style = seg.style.unwrap_or_default();
        assert_eq!(style.bold, Some(true), "Code should be bold");
        assert!(style.color.is_some(), "Code should have a color (cyan)");
        assert!(style.bgcolor.is_some(), "Code should have bgcolor (black)");
    }

    #[test]
    fn test_markdown_code_block() {
        let md = Markdown::new("```rust\nfn main() {}\n```");
        let output = render_to_string(&md);
        assert!(output.contains("fn"));
        assert!(output.contains("main"));
    }

    #[test]
    fn test_markdown_list_unordered() {
        let md = Markdown::new("- Item 1\n- Item 2\n- Item 3");
        let output = render_to_string(&md);
        assert!(output.contains("Item 1"));
        assert!(output.contains("Item 2"));
        assert!(output.contains("Item 3"));
    }

    #[test]
    fn test_markdown_list_ordered() {
        let md = Markdown::new("1. First\n2. Second\n3. Third");
        let output = render_to_string(&md);
        assert!(output.contains("First"));
        assert!(output.contains("Second"));
        assert!(output.contains("Third"));
    }

    #[test]
    fn test_markdown_blockquote() {
        let md = Markdown::new("> This is a quote");
        let output = render_to_string(&md);
        assert!(output.contains("This is a quote"));
    }

    #[test]
    fn test_markdown_link() {
        let md = Markdown::new("[Rust](https://rust-lang.org)");
        let output = render_to_string(&md);
        assert!(output.contains("Rust"));
        assert!(output.contains("rust-lang.org"));
    }

    #[test]
    fn test_markdown_horizontal_rule() {
        let md = Markdown::new("Above\n\n---\n\nBelow");
        let output = render_to_string(&md);
        assert!(output.contains("Above"));
        assert!(output.contains("Below"));
    }

    #[test]
    fn test_markdown_table() {
        let md = Markdown::new("| A | B |\n|---|---|\n| 1 | 2 |");
        let output = render_to_string(&md);
        assert!(output.contains("A"));
        assert!(output.contains("B"));
        assert!(output.contains("1"));
        assert!(output.contains("2"));
    }

    #[test]
    fn test_markdown_strikethrough() {
        let md = Markdown::new("~~deleted~~");
        let output = render_to_string(&md);
        assert!(output.contains("deleted"));
    }

    #[test]
    fn test_markdown_nested_formatting() {
        let md = Markdown::new("This is ***bold and italic***");
        let output = render_to_string(&md);
        assert!(output.contains("bold and italic"));
    }

    #[test]
    fn test_markdown_with_style() {
        let style = Style::new().with_bold(true);
        let md = Markdown::new("Hello world").with_style(style);
        assert_eq!(md.style, Some(style));

        // Verify style is applied to rendered segments
        let console = Console::new();
        let options = ConsoleOptions::default();
        let segments = md.render(&console, &options);
        let text_seg = segments.iter().find(|s| s.text.contains("Hello"));
        assert!(text_seg.is_some());
        let seg_style = text_seg.unwrap().style.unwrap_or_default();
        assert_eq!(seg_style.bold, Some(true));
    }

    #[test]
    fn test_markdown_builder_methods() {
        let md = Markdown::new("# Test")
            .with_hyperlinks(true)
            .with_inline_code_lexer("python")
            .with_code_theme("base16-ocean.dark")
            .with_justify(JustifyMethod::Left);

        assert!(md.hyperlinks);
        assert_eq!(md.inline_code_lexer, Some("python".to_string()));
        assert_eq!(md.code_theme, "base16-ocean.dark");
        assert_eq!(md.justify, Some(JustifyMethod::Left));
    }
}
