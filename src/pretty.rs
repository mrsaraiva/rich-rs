//! Pretty: pretty-print Rust data structures.
//!
//! This module provides pretty-printing functionality for Rust types that implement
//! the `Debug` trait. It formats debug output with proper indentation and line wrapping,
//! then applies syntax highlighting.
//!
//! # Example
//!
//! ```
//! use rich_rs::pretty::{Pretty, pretty_repr};
//!
//! let data = vec![1, 2, 3, 4, 5];
//! let pretty = Pretty::new(&data);
//!
//! // Or use the convenience function with a debug string:
//! let debug_str = format!("{:?}", data);
//! let formatted = pretty_repr(&debug_str, 80, 4, None, None, None, false);
//! ```
//!
//! # Differences from Python Rich
//!
//! - Python uses introspection (`__repr__`, attrs, dataclasses). Rust uses `Debug` trait.
//! - Python can traverse objects and build a tree. Rust parses the `Debug` output string.
//! - Configuration options remain similar for API compatibility.

use std::fmt::Debug;
use std::io::Stdout;

use crate::cells::cell_len;
use crate::console::{Console, ConsoleOptions, JustifyMethod, OverflowMethod};
use crate::highlighter::{repr_highlighter, repr_highlighter_with_theme, Highlighter};
use crate::measure::Measurement;
use crate::segment::Segments;
use crate::text::Text;
use crate::theme::Theme;
use crate::Renderable;

// ============================================================================
// Node - Tree structure for repr output
// ============================================================================

/// A node in a repr tree. May be atomic or a container.
#[derive(Debug, Clone)]
struct Node {
    /// Key representation (for key-value pairs like struct fields).
    key_repr: String,
    /// Value representation (for atomic values).
    value_repr: String,
    /// Opening brace/paren (e.g., "[", "{", "(").
    open_brace: String,
    /// Closing brace/paren (e.g., "]", "}", ")").
    close_brace: String,
    /// Empty representation (e.g., "[]", "{}").
    empty: String,
    /// Whether this is the last child in its parent container.
    last: bool,
    /// Whether this is a tuple (affects trailing comma for single-element tuples).
    is_tuple: bool,
    /// Child nodes (None for atomic values).
    children: Option<Vec<Node>>,
    /// Separator between key and value (": " for maps, "=" for struct fields).
    key_separator: String,
    /// Separator between children (", " typically).
    separator: String,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            key_repr: String::new(),
            value_repr: String::new(),
            open_brace: String::new(),
            close_brace: String::new(),
            empty: String::new(),
            last: false,
            is_tuple: false,
            children: None,
            key_separator: ": ".to_string(),
            separator: ", ".to_string(),
        }
    }
}

impl Node {
    /// Create a new atomic node with a value.
    fn atomic(value: impl Into<String>) -> Self {
        Node {
            value_repr: value.into(),
            ..Default::default()
        }
    }

    /// Create a new container node.
    fn container(open: impl Into<String>, close: impl Into<String>) -> Self {
        Node {
            open_brace: open.into(),
            close_brace: close.into(),
            children: Some(Vec::new()),
            ..Default::default()
        }
    }

    /// Generate tokens for this node.
    fn iter_tokens(&self) -> Vec<String> {
        let mut tokens = Vec::new();

        if !self.key_repr.is_empty() {
            tokens.push(self.key_repr.clone());
            tokens.push(self.key_separator.clone());
        }

        if !self.value_repr.is_empty() {
            tokens.push(self.value_repr.clone());
        } else if let Some(ref children) = self.children {
            if children.is_empty() {
                if !self.empty.is_empty() {
                    tokens.push(self.empty.clone());
                } else {
                    tokens.push(self.open_brace.clone());
                    tokens.push(self.close_brace.clone());
                }
            } else {
                tokens.push(self.open_brace.clone());
                // Handle single-element tuple with trailing comma
                if self.is_tuple && children.len() == 1 {
                    tokens.extend(children[0].iter_tokens());
                    tokens.push(",".to_string());
                } else {
                    for (i, child) in children.iter().enumerate() {
                        tokens.extend(child.iter_tokens());
                        if i < children.len() - 1 {
                            tokens.push(self.separator.clone());
                        }
                    }
                }
                tokens.push(self.close_brace.clone());
            }
        }

        tokens
    }

    /// Check if the node fits within a given length.
    fn check_length(&self, start_length: usize, max_length: usize) -> bool {
        let mut total = start_length;
        for token in self.iter_tokens() {
            total += cell_len(&token);
            if total > max_length {
                return false;
            }
        }
        true
    }

    /// Render to string (single line).
    fn to_string_inline(&self) -> String {
        self.iter_tokens().join("")
    }

    /// Render the node to a pretty repr string.
    fn render(&self, max_width: usize, indent_size: usize, expand_all: bool) -> String {
        let mut lines = vec![Line::new(self.clone(), true)];
        let mut line_no = 0;

        while line_no < lines.len() {
            let line = &lines[line_no];
            if line.expandable() && !line.expanded {
                if expand_all || !line.check_length(max_width) {
                    let expanded = lines[line_no].expand(indent_size);
                    lines.splice(line_no..line_no + 1, expanded);
                }
            }
            line_no += 1;
        }

        lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n")
    }
}

// ============================================================================
// Line - A line in repr output
// ============================================================================

/// A line in repr output.
#[derive(Debug, Clone)]
struct Line {
    /// The node for this line (if expandable).
    node: Option<Node>,
    /// Pre-rendered text (for non-expandable lines).
    text: String,
    /// Suffix after the node (e.g., ",").
    suffix: String,
    /// Leading whitespace.
    whitespace: String,
    /// Whether this line has been expanded.
    expanded: bool,
    /// Whether this is the last item in its parent.
    last: bool,
    /// Whether this is the root node.
    /// NOTE: Reserved for future use (e.g., special root formatting).
    #[allow(dead_code)]
    is_root: bool,
}

impl Line {
    fn new(node: Node, is_root: bool) -> Self {
        Line {
            node: Some(node),
            text: String::new(),
            suffix: String::new(),
            whitespace: String::new(),
            expanded: false,
            last: false,
            is_root,
        }
    }

    fn text_only(text: impl Into<String>, whitespace: impl Into<String>) -> Self {
        Line {
            node: None,
            text: text.into(),
            suffix: String::new(),
            whitespace: whitespace.into(),
            expanded: false,
            last: false,
            is_root: false,
        }
    }

    fn expandable(&self) -> bool {
        if let Some(ref node) = self.node {
            node.children.as_ref().is_some_and(|c| !c.is_empty())
        } else {
            false
        }
    }

    fn check_length(&self, max_length: usize) -> bool {
        let start_length = self.whitespace.len() + cell_len(&self.text) + cell_len(&self.suffix);
        if let Some(ref node) = self.node {
            node.check_length(start_length, max_length)
        } else {
            start_length <= max_length
        }
    }

    fn expand(&self, indent_size: usize) -> Vec<Line> {
        let mut result = Vec::new();

        let node = match &self.node {
            Some(n) => n,
            None => return vec![self.clone()],
        };

        let children = match &node.children {
            Some(c) => c,
            None => return vec![self.clone()],
        };

        // Opening line
        let open_text = if !node.key_repr.is_empty() {
            format!("{}{}{}", node.key_repr, node.key_separator, node.open_brace)
        } else {
            node.open_brace.clone()
        };
        result.push(Line::text_only(open_text, &self.whitespace));

        // Child whitespace
        let child_whitespace = format!("{}{}", self.whitespace, " ".repeat(indent_size));

        // Children
        let tuple_of_one = node.is_tuple && children.len() == 1;
        for (i, child) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;
            let separator = if tuple_of_one {
                ","
            } else if !is_last {
                &node.separator
            } else {
                ""
            };

            let mut line = Line::new(child.clone(), false);
            line.whitespace = child_whitespace.clone();
            line.suffix = separator.to_string();
            line.last = is_last && !tuple_of_one;
            result.push(line);
        }

        // Closing line
        let mut close_line = Line::text_only(&node.close_brace, &self.whitespace);
        close_line.suffix = self.suffix.clone();
        close_line.last = self.last;
        result.push(close_line);

        result
    }
}

impl std::fmt::Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.last {
            write!(f, "{}{}{}", self.whitespace, self.text, self.node.as_ref().map_or(String::new(), |n| n.to_string_inline()))
        } else {
            write!(f, "{}{}{}{}", self.whitespace, self.text, self.node.as_ref().map_or(String::new(), |n| n.to_string_inline()), self.suffix.trim_end())
        }
    }
}

// ============================================================================
// Parser for Debug output
// ============================================================================

/// Parse a Debug output string into a Node tree.
fn parse_debug_output(s: &str, max_length: Option<usize>, max_string: Option<usize>, max_depth: Option<usize>) -> Node {
    let mut parser = DebugParser::new(s, max_length, max_string, max_depth);
    parser.parse()
}

struct DebugParser<'a> {
    input: &'a str,
    pos: usize,
    max_length: Option<usize>,
    max_string: Option<usize>,
    max_depth: Option<usize>,
}

impl<'a> DebugParser<'a> {
    fn new(input: &'a str, max_length: Option<usize>, max_string: Option<usize>, max_depth: Option<usize>) -> Self {
        DebugParser {
            input,
            pos: 0,
            max_length,
            max_string,
            max_depth,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn parse(&mut self) -> Node {
        self.skip_whitespace();
        self.parse_value(0)
    }

    fn parse_value(&mut self, depth: usize) -> Node {
        self.skip_whitespace();

        // Check max depth - must skip nested content to avoid infinite loop
        if let Some(max_d) = self.max_depth {
            if depth >= max_d {
                // Skip the nested structure we're not parsing
                match self.peek() {
                    Some('[') => {
                        self.advance();
                        self.skip_to_closing_bracket(']');
                    }
                    Some('{') => {
                        self.advance();
                        self.skip_to_closing_bracket('}');
                    }
                    Some('(') => {
                        self.advance();
                        self.skip_to_closing_bracket(')');
                    }
                    Some('"') => {
                        self.parse_string();
                        return Node::atomic("...");
                    }
                    Some('\'') => {
                        self.parse_char();
                        return Node::atomic("...");
                    }
                    Some(c) if c.is_alphabetic() || c == '_' => {
                        self.skip_identifier_or_struct();
                    }
                    _ => {
                        self.parse_literal();
                        return Node::atomic("...");
                    }
                }
                return Node::atomic("...");
            }
        }

        match self.peek() {
            None => Node::atomic(""),
            Some('[') => self.parse_list(depth),
            Some('{') => self.parse_brace_container(depth),
            Some('(') => self.parse_tuple(depth),
            Some('"') => self.parse_string(),
            Some('\'') => self.parse_char(),
            Some(c) if c.is_alphabetic() || c == '_' => self.parse_identifier_or_struct(depth),
            _ => self.parse_literal(),
        }
    }

    fn parse_list(&mut self, depth: usize) -> Node {
        self.advance(); // consume '['
        let mut node = Node::container("[", "]");
        node.empty = "[]".to_string();

        self.skip_whitespace();

        let mut count = 0;
        while let Some(c) = self.peek() {
            if c == ']' {
                self.advance();
                break;
            }
            if c == ',' {
                self.advance();
                self.skip_whitespace();
                continue;
            }

            // Check max_length
            if let Some(max_len) = self.max_length {
                if count >= max_len {
                    // Skip to closing bracket and add ellipsis
                    let remaining = self.skip_to_closing_bracket(']');
                    if remaining > 0 {
                        let mut ellipsis = Node::atomic(format!("... +{}", remaining));
                        ellipsis.last = true;
                        if let Some(ref mut children) = node.children {
                            children.push(ellipsis);
                        }
                    }
                    break;
                }
            }

            let child = self.parse_value(depth + 1);
            if let Some(ref mut children) = node.children {
                children.push(child);
            }
            count += 1;
            self.skip_whitespace();
        }

        // Mark last child
        if let Some(ref mut children) = node.children {
            if let Some(last) = children.last_mut() {
                last.last = true;
            }
        }

        node
    }

    fn parse_brace_container(&mut self, depth: usize) -> Node {
        self.advance(); // consume '{'
        let mut node = Node::container("{", "}");
        node.empty = "{}".to_string();

        self.skip_whitespace();

        let mut count = 0;
        while let Some(c) = self.peek() {
            if c == '}' {
                self.advance();
                break;
            }
            if c == ',' {
                self.advance();
                self.skip_whitespace();
                continue;
            }

            // Check max_length
            if let Some(max_len) = self.max_length {
                if count >= max_len {
                    let remaining = self.skip_to_closing_bracket('}');
                    if remaining > 0 {
                        let mut ellipsis = Node::atomic(format!("... +{}", remaining));
                        ellipsis.last = true;
                        if let Some(ref mut children) = node.children {
                            children.push(ellipsis);
                        }
                    }
                    break;
                }
            }

            let child = self.parse_key_value_or_value(depth + 1);
            if let Some(ref mut children) = node.children {
                children.push(child);
            }
            count += 1;
            self.skip_whitespace();
        }

        if let Some(ref mut children) = node.children {
            if let Some(last) = children.last_mut() {
                last.last = true;
            }
        }

        node
    }

    fn parse_tuple(&mut self, depth: usize) -> Node {
        self.advance(); // consume '('
        let mut node = Node::container("(", ")");
        node.empty = "()".to_string();
        node.is_tuple = true;

        self.skip_whitespace();

        let mut count = 0;
        while let Some(c) = self.peek() {
            if c == ')' {
                self.advance();
                break;
            }
            if c == ',' {
                self.advance();
                self.skip_whitespace();
                continue;
            }

            if let Some(max_len) = self.max_length {
                if count >= max_len {
                    let remaining = self.skip_to_closing_bracket(')');
                    if remaining > 0 {
                        let mut ellipsis = Node::atomic(format!("... +{}", remaining));
                        ellipsis.last = true;
                        if let Some(ref mut children) = node.children {
                            children.push(ellipsis);
                        }
                    }
                    break;
                }
            }

            let child = self.parse_value(depth + 1);
            if let Some(ref mut children) = node.children {
                children.push(child);
            }
            count += 1;
            self.skip_whitespace();
        }

        if let Some(ref mut children) = node.children {
            if let Some(last) = children.last_mut() {
                last.last = true;
            }
        }

        node
    }

    fn parse_string(&mut self) -> Node {
        let start = self.pos;
        self.advance(); // consume opening quote

        let mut escaped = false;
        while let Some(c) = self.peek() {
            if escaped {
                escaped = false;
                self.advance();
            } else if c == '\\' {
                escaped = true;
                self.advance();
            } else if c == '"' {
                self.advance();
                break;
            } else {
                self.advance();
            }
        }

        let mut s = self.input[start..self.pos].to_string();

        // Apply max_string truncation (using char indices for UTF-8 safety)
        if let Some(max_str) = self.max_string {
            // Extract the content inside quotes
            let chars: Vec<char> = s.chars().collect();
            if chars.len() > 2 {
                // Get content between quotes (skip first and last char)
                let content_chars: Vec<char> = chars[1..chars.len()-1].to_vec();
                if content_chars.len() > max_str {
                    let truncated: String = content_chars[..max_str].iter().collect();
                    let truncated_len = content_chars.len() - max_str;
                    s = format!("\"{}\"...+{}", truncated, truncated_len);
                }
            }
        }

        Node::atomic(s)
    }

    fn parse_char(&mut self) -> Node {
        let start = self.pos;
        self.advance(); // consume opening quote

        let mut escaped = false;
        while let Some(c) = self.peek() {
            if escaped {
                escaped = false;
                self.advance();
            } else if c == '\\' {
                escaped = true;
                self.advance();
            } else if c == '\'' {
                self.advance();
                break;
            } else {
                self.advance();
            }
        }

        Node::atomic(&self.input[start..self.pos])
    }

    fn parse_identifier_or_struct(&mut self, depth: usize) -> Node {
        let ident = self.parse_identifier();
        self.skip_whitespace();

        match self.peek() {
            Some('(') => {
                // Could be struct with tuple fields or named fields
                self.advance(); // consume '('
                self.skip_whitespace();

                // Check if it's a unit struct (empty parens)
                if self.peek() == Some(')') {
                    self.advance();
                    return Node::atomic(format!("{}()", ident));
                }

                // Parse struct fields
                let mut node = Node::container(format!("{}(", ident), ")");
                node.empty = format!("{}()", ident);

                let mut count = 0;
                while let Some(c) = self.peek() {
                    if c == ')' {
                        self.advance();
                        break;
                    }
                    if c == ',' {
                        self.advance();
                        self.skip_whitespace();
                        continue;
                    }

                    if let Some(max_len) = self.max_length {
                        if count >= max_len {
                            let remaining = self.skip_to_closing_bracket(')');
                            if remaining > 0 {
                                let mut ellipsis = Node::atomic(format!("... +{}", remaining));
                                ellipsis.last = true;
                                if let Some(ref mut children) = node.children {
                                    children.push(ellipsis);
                                }
                            }
                            break;
                        }
                    }

                    let child = self.parse_struct_field(depth + 1);
                    if let Some(ref mut children) = node.children {
                        children.push(child);
                    }
                    count += 1;
                    self.skip_whitespace();
                }

                if let Some(ref mut children) = node.children {
                    if let Some(last) = children.last_mut() {
                        last.last = true;
                    }
                }

                node
            }
            Some('{') => {
                // Struct with named fields using braces
                self.advance(); // consume '{'
                self.skip_whitespace();

                if self.peek() == Some('}') {
                    self.advance();
                    return Node::atomic(format!("{} {{}}", ident));
                }

                let mut node = Node::container(format!("{} {{ ", ident), " }");
                node.empty = format!("{} {{}}", ident);

                let mut count = 0;
                while let Some(c) = self.peek() {
                    if c == '}' {
                        self.advance();
                        break;
                    }
                    if c == ',' {
                        self.advance();
                        self.skip_whitespace();
                        continue;
                    }

                    if let Some(max_len) = self.max_length {
                        if count >= max_len {
                            let remaining = self.skip_to_closing_bracket('}');
                            if remaining > 0 {
                                let mut ellipsis = Node::atomic(format!("... +{}", remaining));
                                ellipsis.last = true;
                                if let Some(ref mut children) = node.children {
                                    children.push(ellipsis);
                                }
                            }
                            break;
                        }
                    }

                    let child = self.parse_struct_field(depth + 1);
                    if let Some(ref mut children) = node.children {
                        children.push(child);
                    }
                    count += 1;
                    self.skip_whitespace();
                }

                if let Some(ref mut children) = node.children {
                    if let Some(last) = children.last_mut() {
                        last.last = true;
                    }
                }

                node
            }
            _ => Node::atomic(ident),
        }
    }

    fn parse_struct_field(&mut self, depth: usize) -> Node {
        self.skip_whitespace();

        // Check if this looks like "field_name: value" where field_name is a simple identifier
        let start = self.pos;

        // First, check if the field starts with a valid identifier character
        // If it starts with a quote or bracket, it's definitely a value, not a field name
        match self.peek() {
            Some(c) if c.is_alphabetic() || c == '_' => {}
            _ => return self.parse_value(depth),
        }

        // Scan the potential field name (must be valid Rust identifier)
        let mut field_end = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
                field_end = self.pos;
            } else {
                break;
            }
        }

        self.skip_whitespace();

        // Check for single colon (not ::) followed by the value
        // The colon must come immediately after the identifier (with optional whitespace)
        if self.peek() == Some(':') && !self.input[self.pos..].starts_with("::") {
            let field_name = self.input[start..field_end].to_string();
            self.advance(); // consume ':'
            self.skip_whitespace();

            let mut value_node = self.parse_value(depth);
            value_node.key_repr = field_name;
            value_node.key_separator = ": ".to_string();
            value_node
        } else {
            // Reset and parse as regular value
            self.pos = start;
            self.parse_value(depth)
        }
    }

    fn parse_key_value_or_value(&mut self, depth: usize) -> Node {
        // Similar to parse_struct_field but for map-like containers
        self.skip_whitespace();

        let start = self.pos;

        // Parse the key (could be a complex value)
        let key_node = self.parse_value(depth);

        self.skip_whitespace();

        // Check for colon or arrow separator
        if self.peek() == Some(':') {
            self.advance();
            self.skip_whitespace();

            let mut value_node = self.parse_value(depth);
            value_node.key_repr = key_node.to_string_inline();
            value_node.key_separator = ": ".to_string();
            value_node
        } else {
            // Reset and just return the key as a value
            self.pos = start;
            self.parse_value(depth)
        }
    }

    fn parse_identifier(&mut self) -> String {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == ':' {
                self.advance();
            } else {
                break;
            }
        }
        self.input[start..self.pos].to_string()
    }

    fn parse_literal(&mut self) -> Node {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == ',' || c == ']' || c == '}' || c == ')' || c.is_whitespace() {
                break;
            }
            self.advance();
        }
        Node::atomic(&self.input[start..self.pos])
    }

    fn skip_to_closing_bracket(&mut self, closing: char) -> usize {
        let mut depth = 1;
        let mut count = 0;
        let mut in_string = false;
        let mut escaped = false;

        while let Some(c) = self.peek() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    in_string = false;
                }
                self.advance();
                continue;
            }

            if c == '"' {
                in_string = true;
                self.advance();
                continue;
            }

            match c {
                '[' | '{' | '(' => depth += 1,
                ']' | '}' | ')' => {
                    depth -= 1;
                    if depth == 0 && c == closing {
                        self.advance();
                        return count;
                    }
                }
                ',' if depth == 1 => count += 1,
                _ => {}
            }
            self.advance();
        }
        count
    }

    /// Skip over an identifier (including :: paths) and any following struct body
    fn skip_identifier_or_struct(&mut self) {
        loop {
            // Skip the identifier segment
            while let Some(c) = self.peek() {
                if c.is_alphanumeric() || c == '_' {
                    self.advance();
                } else {
                    break;
                }
            }

            // Check for :: path separator
            if self.input[self.pos..].starts_with("::") {
                self.advance(); // consume first :
                self.advance(); // consume second :
                continue; // continue to next path segment
            }
            break;
        }

        self.skip_whitespace();

        // Skip any following container
        match self.peek() {
            Some('(') => {
                self.advance();
                self.skip_to_closing_bracket(')');
            }
            Some('{') => {
                self.advance();
                self.skip_to_closing_bracket('}');
            }
            _ => {}
        }
    }
}

// ============================================================================
// Pretty struct
// ============================================================================

/// A renderable that pretty-prints Rust data structures.
///
/// Pretty takes any type that implements `Debug` and renders it with
/// proper indentation, line wrapping, and syntax highlighting.
///
/// # Example
///
/// ```
/// use rich_rs::pretty::Pretty;
/// use rich_rs::{Console, ConsoleOptions};
/// use rich_rs::Renderable;
///
/// let data = vec![1, 2, 3];
/// let pretty = Pretty::new(&data);
/// ```
pub struct Pretty {
    /// The debug representation of the object.
    debug_str: String,
    /// Highlighter to apply to the output.
    highlighter: Box<dyn Highlighter>,
    /// Number of spaces per indent level.
    indent_size: usize,
    /// Text justification method.
    /// NOTE: Not yet implemented - stored for future use.
    #[allow(dead_code)]
    justify: Option<JustifyMethod>,
    /// Overflow handling method.
    /// NOTE: Not yet implemented - stored for future use.
    #[allow(dead_code)]
    overflow: Option<OverflowMethod>,
    /// Disable word wrapping.
    /// NOTE: Not yet implemented - stored for future use.
    #[allow(dead_code)]
    no_wrap: bool,
    /// Enable indentation guides.
    indent_guides: bool,
    /// Maximum number of items in containers before abbreviating.
    max_length: Option<usize>,
    /// Maximum length of strings before truncating.
    max_string: Option<usize>,
    /// Maximum depth of nested structures.
    max_depth: Option<usize>,
    /// Expand all containers regardless of width.
    expand_all: bool,
    /// Margin to subtract from width.
    margin: usize,
    /// Insert a new line if output has multiple lines.
    insert_line: bool,
    /// Whether an explicit theme was set (if false, use Console's theme).
    explicit_theme: bool,
}

impl std::fmt::Debug for Pretty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pretty")
            .field("debug_str", &self.debug_str)
            .field("indent_size", &self.indent_size)
            .field("justify", &self.justify)
            .field("overflow", &self.overflow)
            .field("no_wrap", &self.no_wrap)
            .field("indent_guides", &self.indent_guides)
            .field("max_length", &self.max_length)
            .field("max_string", &self.max_string)
            .field("max_depth", &self.max_depth)
            .field("expand_all", &self.expand_all)
            .field("margin", &self.margin)
            .field("insert_line", &self.insert_line)
            .field("explicit_theme", &self.explicit_theme)
            .finish_non_exhaustive()
    }
}

impl Pretty {
    /// Create a new Pretty renderable from a Debug value.
    ///
    /// # Arguments
    ///
    /// * `value` - Any value that implements `Debug`
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::pretty::Pretty;
    ///
    /// let data = vec!["hello", "world"];
    /// let pretty = Pretty::new(&data);
    /// ```
    pub fn new<T: Debug>(value: &T) -> Self {
        Pretty {
            debug_str: format!("{:?}", value),
            highlighter: Box::new(repr_highlighter()),
            indent_size: 4,
            justify: None,
            overflow: None,
            no_wrap: false,
            indent_guides: false,
            max_length: None,
            max_string: None,
            max_depth: None,
            expand_all: false,
            margin: 0,
            insert_line: false,
            explicit_theme: false,
        }
    }

    /// Create a Pretty renderable from a pre-formatted debug string.
    ///
    /// This is useful when you have already formatted the debug output.
    pub fn from_str(debug_str: impl Into<String>) -> Self {
        Pretty {
            debug_str: debug_str.into(),
            highlighter: Box::new(repr_highlighter()),
            indent_size: 4,
            justify: None,
            overflow: None,
            no_wrap: false,
            indent_guides: false,
            max_length: None,
            max_string: None,
            max_depth: None,
            expand_all: false,
            margin: 0,
            insert_line: false,
            explicit_theme: false,
        }
    }

    /// Set a custom highlighter.
    pub fn with_highlighter(mut self, highlighter: impl Highlighter + 'static) -> Self {
        self.highlighter = Box::new(highlighter);
        self
    }

    /// Set the number of spaces per indent level.
    pub fn with_indent_size(mut self, size: usize) -> Self {
        self.indent_size = size;
        self
    }

    /// Set the text justification method.
    ///
    /// NOTE: Not yet implemented - option is stored for future use.
    pub fn with_justify(mut self, justify: JustifyMethod) -> Self {
        self.justify = Some(justify);
        self
    }

    /// Set the overflow handling method.
    ///
    /// NOTE: Not yet implemented - option is stored for future use.
    pub fn with_overflow(mut self, overflow: OverflowMethod) -> Self {
        self.overflow = Some(overflow);
        self
    }

    /// Set whether to disable word wrapping.
    ///
    /// NOTE: Not yet implemented - option is stored for future use.
    pub fn with_no_wrap(mut self, no_wrap: bool) -> Self {
        self.no_wrap = no_wrap;
        self
    }

    /// Enable or disable indentation guides.
    pub fn with_indent_guides(mut self, guides: bool) -> Self {
        self.indent_guides = guides;
        self
    }

    /// Set the theme by name.
    ///
    /// Available themes: "default", "dracula", "gruvbox-dark", "nord"
    ///
    /// This overrides the Console's theme for this renderable.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::pretty::Pretty;
    ///
    /// let data = vec![1, 2, 3];
    /// let pretty = Pretty::new(&data).with_theme("dracula");
    /// ```
    pub fn with_theme(mut self, name: &str) -> Self {
        if let Some(theme) = Theme::from_name(name) {
            self.highlighter = Box::new(repr_highlighter_with_theme(theme));
            self.explicit_theme = true;
        }
        self
    }

    /// Set a custom theme.
    ///
    /// This overrides the Console's theme for this renderable.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::pretty::Pretty;
    /// use rich_rs::Theme;
    ///
    /// let theme = Theme::from_name("dracula").unwrap();
    /// let data = vec![1, 2, 3];
    /// let pretty = Pretty::new(&data).with_custom_theme(theme);
    /// ```
    pub fn with_custom_theme(mut self, theme: Theme) -> Self {
        self.highlighter = Box::new(repr_highlighter_with_theme(theme));
        self.explicit_theme = true;
        self
    }

    /// Set the maximum number of items in containers before abbreviating.
    pub fn with_max_length(mut self, max_length: Option<usize>) -> Self {
        self.max_length = max_length;
        self
    }

    /// Set the maximum length of strings before truncating.
    pub fn with_max_string(mut self, max_string: Option<usize>) -> Self {
        self.max_string = max_string;
        self
    }

    /// Set the maximum depth of nested structures.
    pub fn with_max_depth(mut self, max_depth: Option<usize>) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Set whether to expand all containers regardless of width.
    pub fn with_expand_all(mut self, expand_all: bool) -> Self {
        self.expand_all = expand_all;
        self
    }

    /// Set the margin to subtract from available width.
    pub fn with_margin(mut self, margin: usize) -> Self {
        self.margin = margin;
        self
    }

    /// Set whether to insert a new line if output has multiple lines.
    pub fn with_insert_line(mut self, insert_line: bool) -> Self {
        self.insert_line = insert_line;
        self
    }

    /// Get the raw debug string.
    pub fn debug_str(&self) -> &str {
        &self.debug_str
    }
}

// SAFETY: Pretty is Send + Sync because:
// - debug_str is String (Send + Sync)
// - highlighter is Box<dyn Highlighter> where Highlighter: Send + Sync
// - All other fields are primitive types that are Send + Sync
unsafe impl Send for Pretty {}
unsafe impl Sync for Pretty {}

impl Renderable for Pretty {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let max_width = options.max_width.saturating_sub(self.margin);

        let pretty_str = pretty_repr(
            &self.debug_str,
            max_width,
            self.indent_size,
            self.max_length,
            self.max_string,
            self.max_depth,
            self.expand_all,
        );

        if pretty_str.is_empty() {
            let dim_text = Text::styled("<empty repr>", crate::style::Style::new().with_dim(true));
            return dim_text.render(console, options);
        }

        // Apply indent guides if enabled
        let processed_str: String = if self.indent_guides {
            let mut result_lines = Vec::new();
            for line in pretty_str.lines() {
                let leading_spaces: usize = line.chars().take_while(|c| *c == ' ').count();
                let num_guides = leading_spaces / self.indent_size;

                if num_guides > 0 {
                    // Build guide prefix: "│   │   " etc.
                    let mut guide_prefix = String::new();
                    for _ in 0..num_guides {
                        guide_prefix.push('│');
                        for _ in 0..(self.indent_size - 1) {
                            guide_prefix.push(' ');
                        }
                    }
                    // Append the rest of the line (after leading spaces)
                    let remaining = &line[leading_spaces..];
                    result_lines.push(format!("{}{}", guide_prefix, remaining));
                } else {
                    result_lines.push(line.to_string());
                }
            }
            result_lines.join("\n")
        } else {
            pretty_str.clone()
        };
        let has_newlines = processed_str.contains('\n');

        let mut text = Text::plain(&processed_str);

        // Apply highlighting
        // If no explicit theme was set, use the Console's theme
        if !self.explicit_theme && options.theme_name != "default" {
            // Create a highlighter with the Console's theme
            if let Some(theme) = Theme::from_name(&options.theme_name) {
                let console_highlighter = repr_highlighter_with_theme(theme);
                console_highlighter.highlight(&mut text);
            } else {
                self.highlighter.highlight(&mut text);
            }
        } else {
            self.highlighter.highlight(&mut text);
        }

        // Apply dim + green style to indent guide characters
        if self.indent_guides {
            use crate::color::SimpleColor;
            let guide_style = crate::style::Style::new()
                .with_color(SimpleColor::Standard(2)) // green
                .with_dim(true);

            // Find all │ characters and style them
            let plain = text.plain_text().to_string();
            for (idx, ch) in plain.char_indices() {
                if ch == '│' {
                    // Style the guide character
                    let char_idx = plain[..idx].chars().count();
                    text.stylize(char_idx, char_idx + 1, guide_style);
                }
            }
        }

        let mut result = Segments::new();

        // Insert line if requested and output has multiple lines
        if self.insert_line && has_newlines {
            result.push(crate::segment::Segment::line());
        }

        // Render the text
        let text_segments = text.render(console, options);
        for seg in text_segments {
            result.push(seg);
        }

        result
    }

    fn measure(&self, _console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        // Subtract margin from available width, matching render behavior
        let max_width = options.max_width.saturating_sub(self.margin);

        let pretty_str = pretty_repr(
            &self.debug_str,
            max_width,
            self.indent_size,
            self.max_length,
            self.max_string,
            self.max_depth,
            self.expand_all,
        );

        let text_width = if pretty_str.is_empty() {
            0
        } else {
            pretty_str.lines().map(cell_len).max().unwrap_or(0)
        };

        Measurement::new(text_width, text_width)
    }
}

// ============================================================================
// Public API functions
// ============================================================================

/// Prettify a debug representation string by expanding onto new lines.
///
/// # Arguments
///
/// * `debug_str` - The debug string to format
/// * `max_width` - Maximum width of output lines
/// * `indent_size` - Number of spaces per indent level
/// * `max_length` - Maximum number of items in containers before abbreviating
/// * `max_string` - Maximum length of strings before truncating
/// * `max_depth` - Maximum depth of nested structures
/// * `expand_all` - Expand all containers regardless of width
///
/// # Returns
///
/// A formatted string with proper indentation.
///
/// # Example
///
/// ```
/// use rich_rs::pretty::pretty_repr;
///
/// let data = vec![1, 2, 3, 4, 5];
/// let debug_str = format!("{:?}", data);
/// let formatted = pretty_repr(&debug_str, 40, 4, None, None, None, false);
/// ```
pub fn pretty_repr(
    debug_str: &str,
    max_width: usize,
    indent_size: usize,
    max_length: Option<usize>,
    max_string: Option<usize>,
    max_depth: Option<usize>,
    expand_all: bool,
) -> String {
    let node = parse_debug_output(debug_str, max_length, max_string, max_depth);
    node.render(max_width, indent_size, expand_all)
}

/// Pretty-print a value to the console.
///
/// # Arguments
///
/// * `value` - Any value that implements `Debug`
/// * `console` - Optional console instance (uses default if None)
/// * `indent_guides` - Enable indentation guides
/// * `max_length` - Maximum number of items in containers
/// * `max_string` - Maximum length of strings
/// * `max_depth` - Maximum depth of nested structures
/// * `expand_all` - Expand all containers
///
/// # Example
///
/// ```no_run
/// use rich_rs::pretty::pprint;
/// use rich_rs::Console;
///
/// let data = vec![1, 2, 3];
/// let mut console = Console::new();
/// pprint(&data, Some(&mut console), true, None, None, None, false);
/// ```
pub fn pprint<T: Debug>(
    value: &T,
    console: Option<&mut Console>,
    indent_guides: bool,
    max_length: Option<usize>,
    max_string: Option<usize>,
    max_depth: Option<usize>,
    expand_all: bool,
) {
    let pretty = Pretty::new(value)
        .with_indent_guides(indent_guides)
        .with_max_length(max_length)
        .with_max_string(max_string)
        .with_max_depth(max_depth)
        .with_expand_all(expand_all)
        .with_overflow(OverflowMethod::Ignore);

    if let Some(console) = console {
        let _ = console.print(&pretty, None, None, None, false, "\n");
    } else {
        let mut default_console = Console::new();
        let _ = default_console.print(&pretty, None, None, None, false, "\n");
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Node tests ====================

    #[test]
    fn test_node_atomic() {
        let node = Node::atomic("42");
        assert_eq!(node.to_string_inline(), "42");
    }

    #[test]
    fn test_node_container_empty() {
        let mut node = Node::container("[", "]");
        node.empty = "[]".to_string();
        assert_eq!(node.to_string_inline(), "[]");
    }

    #[test]
    fn test_node_container_with_children() {
        let mut node = Node::container("[", "]");
        if let Some(ref mut children) = node.children {
            children.push(Node::atomic("1"));
            children.push(Node::atomic("2"));
            children.push(Node::atomic("3"));
        }
        assert_eq!(node.to_string_inline(), "[1, 2, 3]");
    }

    #[test]
    fn test_node_tuple_single_element() {
        let mut node = Node::container("(", ")");
        node.is_tuple = true;
        if let Some(ref mut children) = node.children {
            children.push(Node::atomic("1"));
        }
        assert_eq!(node.to_string_inline(), "(1,)");
    }

    #[test]
    fn test_node_with_key() {
        let mut node = Node::atomic("42");
        node.key_repr = "answer".to_string();
        assert_eq!(node.to_string_inline(), "answer: 42");
    }

    // ==================== Parser tests ====================

    #[test]
    fn test_parse_simple_list() {
        let node = parse_debug_output("[1, 2, 3]", None, None, None);
        assert_eq!(node.to_string_inline(), "[1, 2, 3]");
    }

    #[test]
    fn test_parse_empty_list() {
        let node = parse_debug_output("[]", None, None, None);
        assert_eq!(node.to_string_inline(), "[]");
    }

    #[test]
    fn test_parse_nested_list() {
        let node = parse_debug_output("[[1, 2], [3, 4]]", None, None, None);
        assert_eq!(node.to_string_inline(), "[[1, 2], [3, 4]]");
    }

    #[test]
    fn test_parse_tuple() {
        let node = parse_debug_output("(1, 2, 3)", None, None, None);
        assert_eq!(node.to_string_inline(), "(1, 2, 3)");
    }

    #[test]
    fn test_parse_single_element_tuple() {
        let node = parse_debug_output("(1,)", None, None, None);
        assert_eq!(node.to_string_inline(), "(1,)");
    }

    #[test]
    fn test_parse_struct() {
        let node = parse_debug_output("Point { x: 1, y: 2 }", None, None, None);
        let rendered = node.to_string_inline();
        assert!(rendered.contains("Point"));
        assert!(rendered.contains("x"));
        assert!(rendered.contains("y"));
    }

    #[test]
    fn test_parse_string() {
        let node = parse_debug_output("\"hello\"", None, None, None);
        assert_eq!(node.to_string_inline(), "\"hello\"");
    }

    #[test]
    fn test_parse_escaped_string() {
        let node = parse_debug_output("\"hello\\nworld\"", None, None, None);
        assert_eq!(node.to_string_inline(), "\"hello\\nworld\"");
    }

    #[test]
    fn test_parse_max_length() {
        let node = parse_debug_output("[1, 2, 3, 4, 5]", Some(3), None, None);
        let rendered = node.to_string_inline();
        assert!(rendered.contains("..."));
    }

    #[test]
    fn test_parse_max_depth() {
        let node = parse_debug_output("[[1, 2], [3, 4]]", None, None, Some(1));
        let rendered = node.to_string_inline();
        assert!(rendered.contains("..."));
    }

    // ==================== Pretty struct tests ====================

    #[test]
    fn test_pretty_new() {
        let data = vec![1, 2, 3];
        let pretty = Pretty::new(&data);
        assert!(pretty.debug_str().contains("[1, 2, 3]"));
    }

    #[test]
    fn test_pretty_with_indent_size() {
        let data = vec![1, 2, 3];
        let pretty = Pretty::new(&data).with_indent_size(2);
        assert_eq!(pretty.indent_size, 2);
    }

    #[test]
    fn test_pretty_with_max_length() {
        let data = vec![1, 2, 3, 4, 5];
        let pretty = Pretty::new(&data).with_max_length(Some(3));
        assert_eq!(pretty.max_length, Some(3));
    }

    #[test]
    fn test_pretty_with_expand_all() {
        let data = vec![1, 2, 3];
        let pretty = Pretty::new(&data).with_expand_all(true);
        assert!(pretty.expand_all);
    }

    // ==================== pretty_repr tests ====================

    #[test]
    fn test_pretty_repr_fits_single_line() {
        let result = pretty_repr("[1, 2, 3]", 80, 4, None, None, None, false);
        assert_eq!(result, "[1, 2, 3]");
    }

    #[test]
    fn test_pretty_repr_expands_when_too_wide() {
        let result = pretty_repr("[1, 2, 3, 4, 5]", 10, 4, None, None, None, false);
        assert!(result.contains('\n'));
    }

    #[test]
    fn test_pretty_repr_expand_all() {
        let result = pretty_repr("[1, 2, 3]", 80, 4, None, None, None, true);
        assert!(result.contains('\n'));
    }

    #[test]
    fn test_pretty_repr_nested() {
        let result = pretty_repr("[[1, 2], [3, 4]]", 15, 4, None, None, None, false);
        assert!(result.contains('\n'));
    }

    // ==================== Renderable tests ====================

    #[test]
    fn test_pretty_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Pretty>();
        assert_sync::<Pretty>();
    }

    #[test]
    fn test_pretty_render() {
        let data = vec![1, 2, 3];
        let pretty = Pretty::new(&data);
        let console = Console::with_options(ConsoleOptions {
            max_width: 80,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = pretty.render(&console, &options);
        assert!(!segments.is_empty());
    }

    #[test]
    fn test_pretty_measure() {
        let data = vec![1, 2, 3];
        let pretty = Pretty::new(&data);
        let console = Console::new();
        let options = ConsoleOptions::default();

        let measurement = pretty.measure(&console, &options);
        assert!(measurement.minimum > 0);
        assert!(measurement.maximum >= measurement.minimum);
    }

    // ==================== Debug trait tests ====================

    #[test]
    fn test_pretty_debug() {
        let data = vec![1, 2, 3];
        let pretty = Pretty::new(&data);
        let debug_str = format!("{:?}", pretty);
        assert!(debug_str.contains("Pretty"));
    }

    // ==================== Complex data structure tests ====================

    #[test]
    fn test_pretty_hashmap() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert("a", 1);
        map.insert("b", 2);
        let pretty = Pretty::new(&map);
        assert!(pretty.debug_str().contains("a"));
        assert!(pretty.debug_str().contains("b"));
    }

    #[test]
    fn test_pretty_option() {
        let some_value: Option<i32> = Some(42);
        let none_value: Option<i32> = None;

        let pretty_some = Pretty::new(&some_value);
        let pretty_none = Pretty::new(&none_value);

        assert!(pretty_some.debug_str().contains("Some"));
        assert!(pretty_some.debug_str().contains("42"));
        assert!(pretty_none.debug_str().contains("None"));
    }

    #[test]
    fn test_pretty_result() {
        let ok_value: Result<i32, &str> = Ok(42);
        let err_value: Result<i32, &str> = Err("error");

        let pretty_ok = Pretty::new(&ok_value);
        let pretty_err = Pretty::new(&err_value);

        assert!(pretty_ok.debug_str().contains("Ok"));
        assert!(pretty_err.debug_str().contains("Err"));
    }

    #[derive(Debug)]
    struct TestStruct {
        name: String,
        value: i32,
        items: Vec<i32>,
    }

    #[test]
    fn test_pretty_custom_struct() {
        let s = TestStruct {
            name: "test".to_string(),
            value: 42,
            items: vec![1, 2, 3],
        };
        let pretty = Pretty::new(&s);
        assert!(pretty.debug_str().contains("TestStruct"));
        assert!(pretty.debug_str().contains("name"));
        assert!(pretty.debug_str().contains("value"));
        assert!(pretty.debug_str().contains("items"));
    }

    #[test]
    fn test_pretty_deeply_nested() {
        let data = vec![vec![vec![1, 2], vec![3, 4]], vec![vec![5, 6], vec![7, 8]]];
        let result = pretty_repr(&format!("{:?}", data), 20, 4, None, None, None, false);
        // Should expand due to width constraint
        assert!(result.contains('\n'));
    }
}
