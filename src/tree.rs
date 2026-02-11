//! Tree: hierarchical tree rendering.
//!
//! Tree renders tree structures with connecting lines (guides) to show
//! parent-child relationships. This is useful for displaying file trees,
//! hierarchical data structures, and nested content.
//!
//! # Example
//!
//! ```ignore
//! use rich_rs::{Tree, Text, Style, Console};
//!
//! let mut tree = Tree::new(Box::new(Text::plain("Root")));
//! tree.add(Box::new(Text::plain("Child 1")));
//! let child2 = tree.add(Box::new(Text::plain("Child 2")));
//! child2.add(Box::new(Text::plain("Grandchild")));
//!
//! let mut console = Console::new();
//! console.print(&tree, None, None, None, false, "\n").unwrap();
//! ```

use std::io::Stdout;

use crate::console::ConsoleOptions;
use crate::measure::Measurement;
use crate::segment::{Segment, Segments};
use crate::style::Style;
use crate::{Console, Renderable};

// ============================================================================
// Tree Guides
// ============================================================================

/// Guide characters for tree structure rendering.
///
/// Contains the four types of guide characters:
/// - `space`: Empty space for alignment (4 chars wide)
/// - `vertical`: Vertical continuation line
/// - `branch`: Branch connector (for non-last children)
/// - `end`: End connector (for last child)
#[derive(Debug, Clone, Copy)]
pub struct TreeGuides {
    /// Space for alignment (where no vertical line continues).
    pub space: &'static str,
    /// Vertical continuation line.
    pub vertical: &'static str,
    /// Branch connector for non-last children.
    pub branch: &'static str,
    /// End connector for last child.
    pub end: &'static str,
}

/// Unicode box-drawing tree guides.
pub const TREE_GUIDES: TreeGuides = TreeGuides {
    space: "    ",
    vertical: "\u{2502}   ",             // "│   "
    branch: "\u{251c}\u{2500}\u{2500} ", // "├── "
    end: "\u{2514}\u{2500}\u{2500} ",    // "└── "
};

/// Bold Unicode box-drawing tree guides.
///
/// Uses heavy-weight box characters (┃, ┣━━, ┗━━).
/// In Python Rich, these are selected when `guide_style` has `bold=True`.
pub const BOLD_TREE_GUIDES: TreeGuides = TreeGuides {
    space: "    ",
    vertical: "\u{2503}   ",             // "┃   "
    branch: "\u{2523}\u{2501}\u{2501} ", // "┣━━ "
    end: "\u{2517}\u{2501}\u{2501} ",    // "┗━━ "
};

/// Double-line Unicode tree guides.
///
/// Uses double-line box characters (║, ╠══, ╚══).
/// In Python Rich, these are selected when `guide_style` has `underline2=True`.
pub const DOUBLE_TREE_GUIDES: TreeGuides = TreeGuides {
    space: "    ",
    vertical: "\u{2551}   ",             // "║   "
    branch: "\u{2560}\u{2550}\u{2550} ", // "╠══ "
    end: "\u{255a}\u{2550}\u{2550} ",    // "╚══ "
};

/// ASCII tree guides for non-Unicode terminals.
pub const ASCII_GUIDES: TreeGuides = TreeGuides {
    space: "    ",
    vertical: "|   ",
    branch: "+-- ",
    end: "`-- ",
};

// ============================================================================
// Tree
// ============================================================================

/// A tree node that can be rendered with guide lines.
///
/// Tree is a hierarchical data structure where each node has a label (content)
/// and zero or more children. When rendered, guide lines connect the nodes
/// to show the tree structure.
///
/// # Example
///
/// ```ignore
/// use rich_rs::{Tree, Text};
///
/// let mut root = Tree::new(Box::new(Text::plain("Documents")));
/// let mut projects = root.add(Box::new(Text::plain("Projects")));
/// projects.add(Box::new(Text::plain("project1")));
/// projects.add(Box::new(Text::plain("project2")));
/// root.add(Box::new(Text::plain("notes.txt")));
/// ```
/// Options for adding a child node to a tree.
///
/// Used with `Tree::add_with_options()` to specify per-node overrides.
#[derive(Debug, Clone, Default)]
pub struct TreeNodeOptions {
    /// Style for this node's label. If `None`, inherits from parent.
    pub style: Option<Style>,
    /// Style for guide lines from this node. If `None`, inherits from parent.
    pub guide_style: Option<Style>,
    /// Whether children are shown. Defaults to `true`.
    pub expanded: Option<bool>,
    /// Whether to apply highlighting. If `None`, inherits from parent.
    pub highlight: Option<bool>,
}

impl TreeNodeOptions {
    /// Create new default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the node style.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Set the guide style.
    pub fn with_guide_style(mut self, style: Style) -> Self {
        self.guide_style = Some(style);
        self
    }

    /// Set whether children are expanded.
    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }

    /// Set whether to highlight.
    pub fn with_highlight(mut self, highlight: bool) -> Self {
        self.highlight = Some(highlight);
        self
    }
}

pub struct Tree {
    /// The label/content of this node.
    label: Box<dyn Renderable + Send + Sync>,
    /// Child nodes.
    children: Vec<Tree>,
    /// Style for the label.
    style: Style,
    /// Style for the guide lines.
    guide_style: Style,
    /// Whether children are visible when rendered.
    expanded: bool,
    /// Whether to highlight labels (for future use with highlighters).
    highlight: bool,
    /// Whether to hide the root node when rendering.
    hide_root: bool,
}

impl std::fmt::Debug for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("children_count", &self.children.len())
            .field("style", &self.style)
            .field("guide_style", &self.guide_style)
            .field("expanded", &self.expanded)
            .field("highlight", &self.highlight)
            .field("hide_root", &self.hide_root)
            .finish_non_exhaustive()
    }
}

impl Tree {
    /// Create a new tree node with the given label.
    ///
    /// # Arguments
    ///
    /// * `label` - The content to display for this node.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rich_rs::{Tree, Text};
    ///
    /// let tree = Tree::new(Box::new(Text::plain("Root")));
    /// ```
    pub fn new(label: Box<dyn Renderable + Send + Sync>) -> Self {
        Tree {
            label,
            children: Vec::new(),
            style: Style::default(),
            guide_style: Style::default(),
            expanded: true,
            highlight: false,
            hide_root: false,
        }
    }

    /// Add a child node with the given label.
    ///
    /// Returns a mutable reference to the newly created child, allowing
    /// for chaining to build nested structures.
    ///
    /// # Arguments
    ///
    /// * `label` - The content to display for the child node.
    ///
    /// # Returns
    ///
    /// A mutable reference to the newly added child Tree.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rich_rs::{Tree, Text};
    ///
    /// let mut root = Tree::new(Box::new(Text::plain("Root")));
    /// let child = root.add(Box::new(Text::plain("Child")));
    /// child.add(Box::new(Text::plain("Grandchild")));
    /// ```
    pub fn add(&mut self, label: Box<dyn Renderable + Send + Sync>) -> &mut Tree {
        let child = Tree {
            label,
            children: Vec::new(),
            style: self.style,
            guide_style: self.guide_style,
            expanded: true,
            highlight: self.highlight,
            hide_root: false,
        };
        self.children.push(child);
        self.children.last_mut().unwrap()
    }

    /// Add a child node with the given label and options.
    ///
    /// Options allow overriding style, guide_style, expanded, and highlight
    /// on a per-node basis. Fields set to `None` inherit from the parent.
    ///
    /// # Arguments
    ///
    /// * `label` - The content to display for the child node.
    /// * `options` - Per-node overrides.
    ///
    /// # Returns
    ///
    /// A mutable reference to the newly added child Tree.
    pub fn add_with_options(
        &mut self,
        label: Box<dyn Renderable + Send + Sync>,
        options: TreeNodeOptions,
    ) -> &mut Tree {
        let child = Tree {
            label,
            children: Vec::new(),
            style: options.style.unwrap_or(self.style),
            guide_style: options.guide_style.unwrap_or(self.guide_style),
            expanded: options.expanded.unwrap_or(true),
            highlight: options.highlight.unwrap_or(self.highlight),
            hide_root: false,
        };
        self.children.push(child);
        self.children.last_mut().unwrap()
    }

    /// Add an existing tree as a child.
    ///
    /// This is useful for combining pre-built subtrees.
    ///
    /// # Arguments
    ///
    /// * `tree` - An existing Tree to add as a child.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rich_rs::{Tree, Text};
    ///
    /// let mut subtree = Tree::new(Box::new(Text::plain("Subtree")));
    /// subtree.add(Box::new(Text::plain("Leaf")));
    ///
    /// let mut root = Tree::new(Box::new(Text::plain("Root")));
    /// root.add_tree(subtree);
    /// ```
    pub fn add_tree(&mut self, tree: Tree) {
        self.children.push(tree);
    }

    /// Set the style for the label.
    ///
    /// # Arguments
    ///
    /// * `style` - Style to apply to the node's label.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the style for guide lines.
    ///
    /// # Arguments
    ///
    /// * `style` - Style to apply to guide characters.
    pub fn with_guide_style(mut self, style: Style) -> Self {
        self.guide_style = style;
        self
    }

    /// Set whether children are expanded (visible).
    ///
    /// # Arguments
    ///
    /// * `expanded` - If true, children are rendered; if false, only this node is shown.
    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Set whether to highlight labels.
    ///
    /// # Arguments
    ///
    /// * `highlight` - If true, labels may be highlighted (for future use).
    pub fn with_highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Set whether to hide the root node.
    ///
    /// When true, only children are rendered (no root label or root guide lines).
    ///
    /// # Arguments
    ///
    /// * `hide` - If true, the root node's label is hidden.
    pub fn with_hide_root(mut self, hide: bool) -> Self {
        self.hide_root = hide;
        self
    }

    /// Get the number of direct children.
    pub fn children_count(&self) -> usize {
        self.children.len()
    }

    /// Check if this node has any children.
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Check if children are expanded.
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Get the style for labels.
    pub fn style(&self) -> Style {
        self.style
    }

    /// Get the style for guide lines.
    pub fn guide_style(&self) -> Style {
        self.guide_style
    }

    /// Get a reference to the children.
    pub fn children(&self) -> &[Tree] {
        &self.children
    }

    /// Get a mutable reference to the children.
    pub fn children_mut(&mut self) -> &mut Vec<Tree> {
        &mut self.children
    }
}


/// State for each node during stack-based traversal.
struct TraversalState<'a> {
    /// Reference to the current tree node.
    node: &'a Tree,
    /// Index of the next child to process.
    child_index: usize,
    /// Whether this is the last sibling at its level.
    is_last: bool,
    /// Depth in the tree (0 = root).
    depth: usize,
}

impl Renderable for Tree {
    fn render(&self, _console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let mut result = Segments::new();

        // Select guides based on encoding and guide_style.
        // Python Rich selects bold guides when guide_style has bold=True,
        // and double guides when guide_style has underline2=True.
        let guides = if options.ascii_only() {
            &ASCII_GUIDES
        } else if self.guide_style.bold == Some(true) {
            &BOLD_TREE_GUIDES
        } else if self.guide_style.underline == Some(true) {
            // Use double guides for underline (Rust doesn't have underline2,
            // so we use underline as the trigger, matching the spirit of Python Rich).
            &DOUBLE_TREE_GUIDES
        } else {
            &TREE_GUIDES
        };

        // When hide_root is true, render children as if they are root-level.
        // We push each child at depth 0 instead of the root.
        let mut stack: Vec<TraversalState> = if self.hide_root {
            // Push children in reverse so they render in order
            self.children
                .iter()
                .enumerate()
                .rev()
                .map(|(i, child)| TraversalState {
                    node: child,
                    child_index: 0,
                    is_last: i == self.children.len() - 1,
                    depth: 0,
                })
                .collect()
        } else {
            vec![TraversalState {
                node: self,
                child_index: 0,
                is_last: true,
                depth: 0,
            }]
        };

        // Track "is_last" for each depth level for guide prefix calculation
        // levels[i] = true means the node at depth i was the last child
        let mut levels: Vec<bool> = Vec::new();

        // Create temp console for nested rendering
        let temp_console = Console::<Stdout>::with_options(options.clone());

        while let Some(state) = stack.last_mut() {
            let TraversalState {
                node,
                child_index,
                is_last,
                depth,
            } = state;

            if *child_index == 0 {
                // First time visiting this node - render its label

                // Ensure levels vec is the right size
                while levels.len() < *depth {
                    levels.push(false);
                }
                if *depth > 0 {
                    if *depth <= levels.len() {
                        levels[*depth - 1] = *is_last;
                    } else {
                        levels.push(*is_last);
                    }
                }

                // Build guide prefix for this node
                // Don't add guides for root-level nodes (depth 0)
                if *depth > 0 {
                    let mut prefix = String::new();

                    // Add continuation guides for all ancestor levels (except current)
                    for i in 0..(*depth - 1) {
                        let ancestor_is_last = levels.get(i).copied().unwrap_or(false);
                        if ancestor_is_last {
                            prefix.push_str(guides.space);
                        } else {
                            prefix.push_str(guides.vertical);
                        }
                    }

                    // Add the connector for this node
                    if *is_last {
                        prefix.push_str(guides.end);
                    } else {
                        prefix.push_str(guides.branch);
                    }

                    // Add styled guide prefix
                    if !prefix.is_empty() {
                        result.push(Segment::styled(prefix, node.guide_style));
                    }
                }

                // Calculate available width for label (subtract guide prefix width)
                let guide_width = if *depth > 0 { *depth * 4 } else { 0 };
                let label_width = options.max_width.saturating_sub(guide_width);
                let label_options = options.update_width(label_width);

                // Render the label - may produce multiple lines
                let label_segments = node.label.render(&temp_console, &label_options);

                // Split into lines and handle multi-line labels
                let lines = Segment::split_lines(label_segments);

                for (line_idx, line) in lines.iter().enumerate() {
                    if line_idx > 0 {
                        // Continuation lines need guide prefix too
                        if *depth > 0 {
                            let mut prefix = String::new();
                            for i in 0..*depth {
                                let ancestor_is_last = levels.get(i).copied().unwrap_or(false);
                                if ancestor_is_last {
                                    prefix.push_str(guides.space);
                                } else {
                                    prefix.push_str(guides.vertical);
                                }
                            }
                            result.push(Segment::styled(prefix, node.guide_style));
                        }
                    }

                    // Add line segments
                    for seg in line {
                        // Apply node style to label if not already styled
                        if !node.style.is_null() && seg.style.is_none() {
                            result.push(Segment::styled(seg.text.clone(), node.style));
                        } else if !node.style.is_null() {
                            // Combine styles
                            let combined = node.style.combine(&seg.style.unwrap_or_default());
                            result.push(Segment::styled(seg.text.clone(), combined));
                        } else {
                            result.push(seg.clone());
                        }
                    }

                    // Add newline
                    result.push(Segment::line());
                }

                // If the label had no content, still add a newline
                if lines.is_empty() {
                    result.push(Segment::line());
                }
            }

            // Process children
            if node.expanded && *child_index < node.children.len() {
                let child_node = &node.children[*child_index];
                let child_is_last = *child_index == node.children.len() - 1;
                let child_depth = *depth + 1;
                *child_index += 1;

                stack.push(TraversalState {
                    node: child_node,
                    child_index: 0,
                    is_last: child_is_last,
                    depth: child_depth,
                });
            } else {
                // Done with this node
                stack.pop();
            }
        }

        result
    }

    fn measure(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        // Stack-based measurement traversal
        let mut stack: Vec<(&Tree, usize)> = vec![(self, 0)];
        let mut minimum: usize = 0;
        let mut maximum: usize = 0;

        while let Some((node, depth)) = stack.pop() {
            // Measure the label
            let label_measurement = node.label.measure(console, options);
            let indent = depth * 4;

            minimum = minimum.max(label_measurement.minimum + indent);
            maximum = maximum.max(label_measurement.maximum + indent);

            // Add children to stack if expanded
            if node.expanded {
                for child in node.children.iter().rev() {
                    stack.push((child, depth + 1));
                }
            }
        }

        Measurement::new(minimum, maximum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Text;

    // ==================== TreeGuides tests ====================

    #[test]
    fn test_tree_guides_unicode() {
        assert_eq!(TREE_GUIDES.space, "    ");
        assert_eq!(TREE_GUIDES.vertical, "│   ");
        assert_eq!(TREE_GUIDES.branch, "├── ");
        assert_eq!(TREE_GUIDES.end, "└── ");
    }

    #[test]
    fn test_tree_guides_ascii() {
        assert_eq!(ASCII_GUIDES.space, "    ");
        assert_eq!(ASCII_GUIDES.vertical, "|   ");
        assert_eq!(ASCII_GUIDES.branch, "+-- ");
        assert_eq!(ASCII_GUIDES.end, "`-- ");
    }

    // ==================== Tree creation tests ====================

    #[test]
    fn test_tree_new() {
        let tree = Tree::new(Box::new(Text::plain("Root")));
        assert_eq!(tree.children_count(), 0);
        assert!(!tree.has_children());
        assert!(tree.is_expanded());
    }

    #[test]
    fn test_tree_add_child() {
        let mut tree = Tree::new(Box::new(Text::plain("Root")));
        tree.add(Box::new(Text::plain("Child")));
        assert_eq!(tree.children_count(), 1);
        assert!(tree.has_children());
    }

    #[test]
    fn test_tree_add_returns_child() {
        let mut tree = Tree::new(Box::new(Text::plain("Root")));
        let child = tree.add(Box::new(Text::plain("Child")));
        child.add(Box::new(Text::plain("Grandchild")));

        assert_eq!(tree.children_count(), 1);
        assert_eq!(tree.children()[0].children_count(), 1);
    }

    #[test]
    fn test_tree_add_tree() {
        let mut subtree = Tree::new(Box::new(Text::plain("Subtree")));
        subtree.add(Box::new(Text::plain("Leaf")));

        let mut root = Tree::new(Box::new(Text::plain("Root")));
        root.add_tree(subtree);

        assert_eq!(root.children_count(), 1);
        assert_eq!(root.children()[0].children_count(), 1);
    }

    #[test]
    fn test_tree_chained_add() {
        let mut root = Tree::new(Box::new(Text::plain("Root")));
        root.add(Box::new(Text::plain("A")))
            .add(Box::new(Text::plain("A1")))
            .add(Box::new(Text::plain("A1a")));

        // Root -> A -> A1 -> A1a
        assert_eq!(root.children_count(), 1);
        assert_eq!(root.children()[0].children_count(), 1);
        assert_eq!(root.children()[0].children()[0].children_count(), 1);
    }

    // ==================== Tree builder tests ====================

    #[test]
    fn test_tree_with_style() {
        let style = Style::new().with_bold(true);
        let tree = Tree::new(Box::new(Text::plain("Root"))).with_style(style);
        assert_eq!(tree.style().bold, Some(true));
    }

    #[test]
    fn test_tree_with_guide_style() {
        let style = Style::new().with_dim(true);
        let tree = Tree::new(Box::new(Text::plain("Root"))).with_guide_style(style);
        assert_eq!(tree.guide_style().dim, Some(true));
    }

    #[test]
    fn test_tree_with_expanded() {
        let tree = Tree::new(Box::new(Text::plain("Root"))).with_expanded(false);
        assert!(!tree.is_expanded());
    }

    #[test]
    fn test_tree_with_highlight() {
        let tree = Tree::new(Box::new(Text::plain("Root"))).with_highlight(true);
        assert!(tree.highlight);
    }

    // ==================== hide_root tests ====================

    #[test]
    fn test_tree_hide_root() {
        let mut tree = Tree::new(Box::new(Text::plain("Root"))).with_hide_root(true);
        tree.add(Box::new(Text::plain("Child 1")));
        tree.add(Box::new(Text::plain("Child 2")));

        let console = Console::with_options(ConsoleOptions::default());
        let options = console.options().clone();
        let segments = tree.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Root should not appear
        assert!(!output.contains("Root"));
        // Children should appear
        assert!(output.contains("Child 1"));
        assert!(output.contains("Child 2"));
    }

    #[test]
    fn test_tree_hide_root_no_children() {
        let tree = Tree::new(Box::new(Text::plain("Root"))).with_hide_root(true);

        let console = Console::with_options(ConsoleOptions::default());
        let options = console.options().clone();
        let segments = tree.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Nothing should appear
        assert!(!output.contains("Root"));
    }

    // ==================== add_with_options tests ====================

    #[test]
    fn test_tree_add_with_options() {
        let mut tree = Tree::new(Box::new(Text::plain("Root")));
        let opts = TreeNodeOptions::new()
            .with_style(Style::new().with_bold(true))
            .with_expanded(false);
        let child = tree.add_with_options(Box::new(Text::plain("Child")), opts);
        assert!(!child.is_expanded());
        assert_eq!(child.style().bold, Some(true));
    }

    #[test]
    fn test_tree_add_with_options_inherits() {
        let style = Style::new().with_dim(true);
        let mut tree = Tree::new(Box::new(Text::plain("Root"))).with_style(style);
        let child = tree.add_with_options(Box::new(Text::plain("Child")), TreeNodeOptions::new());
        // Should inherit parent style when options don't override
        assert_eq!(child.style().dim, Some(true));
    }

    // ==================== Guide variant tests ====================

    #[test]
    fn test_bold_tree_guides() {
        assert_eq!(BOLD_TREE_GUIDES.vertical, "┃   ");
        assert_eq!(BOLD_TREE_GUIDES.branch, "┣━━ ");
        assert_eq!(BOLD_TREE_GUIDES.end, "┗━━ ");
    }

    #[test]
    fn test_double_tree_guides() {
        assert_eq!(DOUBLE_TREE_GUIDES.vertical, "║   ");
        assert_eq!(DOUBLE_TREE_GUIDES.branch, "╠══ ");
        assert_eq!(DOUBLE_TREE_GUIDES.end, "╚══ ");
    }

    #[test]
    fn test_tree_renders_bold_guides() {
        let mut tree =
            Tree::new(Box::new(Text::plain("Root"))).with_guide_style(Style::new().with_bold(true));
        tree.add(Box::new(Text::plain("Child")));

        let console = Console::with_options(ConsoleOptions::default());
        let options = console.options().clone();
        let segments = tree.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains("┗━━ "), "Should use bold guides");
    }

    // ==================== Tree render tests ====================

    #[test]
    fn test_tree_render_single_node() {
        let tree = Tree::new(Box::new(Text::plain("Root")));
        let console = Console::with_options(ConsoleOptions::default());
        let options = console.options().clone();

        let segments = tree.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains("Root"));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn test_tree_render_with_children() {
        let mut tree = Tree::new(Box::new(Text::plain("Root")));
        tree.add(Box::new(Text::plain("Child 1")));
        tree.add(Box::new(Text::plain("Child 2")));

        let console = Console::with_options(ConsoleOptions::default());
        let options = console.options().clone();

        let segments = tree.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains("Root"));
        assert!(output.contains("Child 1"));
        assert!(output.contains("Child 2"));
        // Should contain branch and end guides
        assert!(output.contains("├── ") || output.contains("+-- ")); // branch
        assert!(output.contains("└── ") || output.contains("`-- ")); // end
    }

    #[test]
    fn test_tree_render_nested() {
        let mut tree = Tree::new(Box::new(Text::plain("Root")));
        let child = tree.add(Box::new(Text::plain("Child")));
        child.add(Box::new(Text::plain("Grandchild")));

        let console = Console::with_options(ConsoleOptions::default());
        let options = console.options().clone();

        let segments = tree.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains("Root"));
        assert!(output.contains("Child"));
        assert!(output.contains("Grandchild"));
    }

    #[test]
    fn test_tree_render_ascii_guides() {
        let mut tree = Tree::new(Box::new(Text::plain("Root")));
        tree.add(Box::new(Text::plain("Child")));

        let console = Console::with_options(ConsoleOptions {
            encoding: "ascii".to_string(),
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = tree.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should use ASCII guides
        assert!(output.contains("`-- ")); // ASCII end guide
        assert!(!output.contains("└")); // No Unicode
    }

    #[test]
    fn test_tree_render_unicode_guides() {
        let mut tree = Tree::new(Box::new(Text::plain("Root")));
        tree.add(Box::new(Text::plain("Child")));

        let console = Console::with_options(ConsoleOptions {
            encoding: "utf-8".to_string(),
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = tree.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should use Unicode guides
        assert!(output.contains("└── ")); // Unicode end guide
    }

    #[test]
    fn test_tree_render_collapsed() {
        let mut tree = Tree::new(Box::new(Text::plain("Root"))).with_expanded(false);
        tree.add(Box::new(Text::plain("Child")));

        let console = Console::with_options(ConsoleOptions::default());
        let options = console.options().clone();

        let segments = tree.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains("Root"));
        assert!(!output.contains("Child")); // Children hidden
    }

    #[test]
    fn test_tree_render_complex_structure() {
        // Build a more complex tree
        let mut root = Tree::new(Box::new(Text::plain("Documents")));

        let projects = root.add(Box::new(Text::plain("Projects")));
        projects.add(Box::new(Text::plain("project1")));
        projects.add(Box::new(Text::plain("project2")));

        root.add(Box::new(Text::plain("notes.txt")));

        let console = Console::with_options(ConsoleOptions::default());
        let options = console.options().clone();

        let segments = root.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains("Documents"));
        assert!(output.contains("Projects"));
        assert!(output.contains("project1"));
        assert!(output.contains("project2"));
        assert!(output.contains("notes.txt"));
    }

    // ==================== Tree measure tests ====================

    #[test]
    fn test_tree_measure_single_node() {
        let tree = Tree::new(Box::new(Text::plain("Root")));
        let console = Console::with_options(ConsoleOptions::default());
        let options = console.options().clone();

        let measurement = tree.measure(&console, &options);
        // "Root" is 4 characters
        assert!(measurement.minimum >= 4);
        assert!(measurement.maximum >= measurement.minimum);
    }

    #[test]
    fn test_tree_measure_with_children() {
        let mut tree = Tree::new(Box::new(Text::plain("R"))); // 1 char
        tree.add(Box::new(Text::plain("Child"))); // 5 chars + 4 indent = 9

        let console = Console::with_options(ConsoleOptions::default());
        let options = console.options().clone();

        let measurement = tree.measure(&console, &options);
        // Maximum should be at least 9 (longest line with indent)
        assert!(measurement.maximum >= 9);
    }

    // ==================== Send + Sync tests ====================

    #[test]
    fn test_tree_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Tree>();
        assert_sync::<Tree>();
    }

    #[test]
    fn test_tree_guides_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<TreeGuides>();
        assert_sync::<TreeGuides>();
    }

    // ==================== Debug tests ====================

    #[test]
    fn test_tree_debug() {
        let tree = Tree::new(Box::new(Text::plain("Root")));
        let debug_str = format!("{:?}", tree);
        assert!(debug_str.contains("Tree"));
        assert!(debug_str.contains("children_count"));
    }
}
