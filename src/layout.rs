//! Layout: split a terminal area into regions and render children.
//!
//! Port of Python Rich's `rich/layout.py` (subset).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use crate::region::Region;
use crate::screen_buffer::ScreenBuffer;
use crate::segment::{Segment, Segments};
use crate::{Console, ConsoleOptions, Renderable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitterKind {
    Row,
    Column,
}

#[derive(Debug, Clone)]
pub struct LayoutRender {
    pub region: Region,
    pub lines: Vec<Vec<Segment>>,
}

struct LayoutState {
    name: Option<String>,
    size: Option<usize>,
    minimum_size: usize,
    ratio: usize,
    visible: bool,
    splitter: SplitterKind,
    renderable: Arc<dyn Renderable>,
    children: Vec<Layout>,
    render_map: HashMap<usize, LayoutRender>,
}

/// A renderable that divides a fixed height in to rows or columns.
#[derive(Clone)]
pub struct Layout {
    id: usize,
    state: Arc<Mutex<LayoutState>>,
}

impl std::fmt::Debug for Layout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.state.lock().expect("layout mutex poisoned");
        f.debug_struct("Layout")
            .field("name", &state.name)
            .field("size", &state.size)
            .field("minimum_size", &state.minimum_size)
            .field("ratio", &state.ratio)
            .field("visible", &state.visible)
            .field("splitter", &state.splitter)
            .field("children", &state.children.len())
            .finish_non_exhaustive()
    }
}

fn next_layout_id() -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone)]
struct Placeholder {
    layout: Weak<Mutex<LayoutState>>,
}

impl Renderable for Placeholder {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        use crate::{Align, Panel, Pretty, Style, VerticalAlignMethod};

        let Some(layout) = self.layout.upgrade() else {
            return Segments::new();
        };
        let state = layout.lock().expect("layout mutex poisoned");
        let width = options.max_width;
        let height = options.height.unwrap_or(options.size.1);
        let title = if let Some(name) = &state.name {
            format!("{name:?} ({width} x {height})")
        } else {
            format!("({width} x {height})")
        };

        #[derive(Debug)]
        #[allow(dead_code)]
        struct LayoutInfo {
            name: Option<String>,
            size: Option<usize>,
            minimum_size: usize,
            ratio: usize,
            visible: bool,
            splitter: SplitterKind,
            children: usize,
        }

        let info = LayoutInfo {
            name: state.name.clone(),
            size: state.size,
            minimum_size: state.minimum_size,
            ratio: state.ratio,
            visible: state.visible,
            splitter: state.splitter,
            children: state.children.len(),
        };

        let content = Align::center(Box::new(Pretty::new(&info)))
            .with_vertical(VerticalAlignMethod::Middle);

        let panel = Panel::new(Box::new(content))
            .with_title(title)
            .with_border_style(Style::parse("blue").unwrap_or_else(Style::new))
            .with_height(height);

        panel.render(console, options)
    }
}

impl Layout {
    /// Create a new Layout with a placeholder renderable.
    pub fn new() -> Self {
        let id = next_layout_id();
        let state = Arc::new(Mutex::new(LayoutState {
            name: None,
            size: None,
            minimum_size: 1,
            ratio: 1,
            visible: true,
            splitter: SplitterKind::Column,
            renderable: Arc::new(String::new()),
            children: Vec::new(),
            render_map: HashMap::new(),
        }));
        let placeholder = Placeholder {
            layout: Arc::downgrade(&state),
        };
        {
            let mut st = state.lock().expect("layout mutex poisoned");
            st.renderable = Arc::new(placeholder);
        }

        Self { id, state }
    }

    /// Create a new leaf layout with a renderable.
    pub fn with_renderable(renderable: impl Renderable + 'static) -> Self {
        let layout = Self::new();
        layout.update(renderable);
        layout
    }

    pub fn with_name(self, name: impl Into<String>) -> Self {
        self.state.lock().expect("layout mutex poisoned").name = Some(name.into());
        self
    }

    pub fn with_size(self, size: usize) -> Self {
        self.state.lock().expect("layout mutex poisoned").size = Some(size);
        self
    }

    pub fn with_minimum_size(self, minimum_size: usize) -> Self {
        self.state.lock().expect("layout mutex poisoned").minimum_size = minimum_size.max(1);
        self
    }

    pub fn with_ratio(self, ratio: usize) -> Self {
        self.state.lock().expect("layout mutex poisoned").ratio = ratio.max(1);
        self
    }

    pub fn with_visible(self, visible: bool) -> Self {
        self.state.lock().expect("layout mutex poisoned").visible = visible;
        self
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn name(&self) -> Option<String> {
        self.state.lock().expect("layout mutex poisoned").name.clone()
    }

    pub fn children(&self) -> Vec<Layout> {
        let state = self.state.lock().expect("layout mutex poisoned");
        state
            .children
            .iter()
            .cloned()
            .filter(|c| c.state.lock().expect("layout mutex poisoned").visible)
            .collect()
    }

    pub fn get(&self, name: &str) -> Option<Layout> {
        let state = self.state.lock().expect("layout mutex poisoned");
        if state.name.as_deref() == Some(name) {
            return Some(self.clone());
        }
        for child in &state.children {
            if let Some(found) = child.get(name) {
                return Some(found);
            }
        }
        None
    }

    pub fn update(&self, renderable: impl Renderable + 'static) {
        self.state.lock().expect("layout mutex poisoned").renderable = Arc::new(renderable);
    }

    pub fn split(&self, splitter: SplitterKind, layouts: Vec<Layout>) {
        let mut state = self.state.lock().expect("layout mutex poisoned");
        state.splitter = splitter;
        state.children = layouts;
    }

    pub fn split_row(&self, layouts: Vec<Layout>) {
        self.split(SplitterKind::Row, layouts)
    }

    pub fn split_column(&self, layouts: Vec<Layout>) {
        self.split(SplitterKind::Column, layouts)
    }

    pub fn add_split(&self, layouts: Vec<Layout>) {
        self.state.lock().expect("layout mutex poisoned").children.extend(layouts);
    }

    pub fn unsplit(&self) {
        self.state.lock().expect("layout mutex poisoned").children.clear();
    }

    fn visible_children(state: &LayoutState) -> Vec<Layout> {
        state
            .children
            .iter()
            .cloned()
            .filter(|c| c.state.lock().expect("layout mutex poisoned").visible)
            .collect()
    }

    fn divide_region(children: &[Layout], region: Region, splitter: SplitterKind) -> Vec<(Layout, Region)> {
        let x = region.x;
        let y = region.y;
        let width = region.width as usize;
        let height = region.height as usize;
        match splitter {
            SplitterKind::Row => {
                let widths = ratio_resolve(width as i64, children);
                let mut offset: i32 = 0;
                children
                    .iter()
                    .cloned()
                    .zip(widths.into_iter())
                    .map(|(child, w)| {
                        let r = Region::new(x + offset, y, w as u32, region.height);
                        offset += w as i32;
                        (child, r)
                    })
                    .collect()
            }
            SplitterKind::Column => {
                let heights = ratio_resolve(height as i64, children);
                let mut offset: i32 = 0;
                children
                    .iter()
                    .cloned()
                    .zip(heights.into_iter())
                    .map(|(child, h)| {
                        let r = Region::new(x, y + offset, region.width, h as u32);
                        offset += h as i32;
                        (child, r)
                    })
                    .collect()
            }
        }
    }

    fn make_region_map(&self, width: usize, height: usize) -> Vec<(Layout, Region)> {
        let mut stack: Vec<(Layout, Region)> = vec![(self.clone(), Region::new(0, 0, width as u32, height as u32))];
        let mut layout_regions: Vec<(Layout, Region)> = Vec::new();
        while let Some((layout, region)) = stack.pop() {
            layout_regions.push((layout.clone(), region));

            let state = layout.state.lock().expect("layout mutex poisoned");
            let children = Self::visible_children(&state);
            if !children.is_empty() {
                let divided = Self::divide_region(&children, region, state.splitter);
                for item in divided {
                    stack.push(item);
                }
            }
        }

        layout_regions.sort_by(|a, b| {
            // Python sorts by Region tuple (x, y, width, height)
            let ra = a.1;
            let rb = b.1;
            (ra.x, ra.y, ra.width, ra.height).cmp(&(rb.x, rb.y, rb.width, rb.height))
        });

        layout_regions
    }

    fn render_map(&self, console: &Console, options: &ConsoleOptions) -> HashMap<usize, LayoutRender> {
        let width = options.max_width.max(1);
        let height = options.height.unwrap_or_else(|| console.height()).max(1);
        let regions = self.make_region_map(width, height);
        let leaves: Vec<(Layout, Region)> = regions
            .into_iter()
            .filter(|(layout, _)| layout.children().is_empty())
            .collect();

        let mut render_map: HashMap<usize, LayoutRender> = HashMap::new();

        for (layout, region) in leaves {
            let mut child_opts = options.clone();
            let w = region.width as usize;
            let h = region.height as usize;
            child_opts.size = (w.max(1), h.max(1));
            child_opts.min_width = w.max(1);
            child_opts.max_width = w.max(1);
            child_opts.max_height = h.max(1);
            child_opts.height = Some(h.max(1));

            let renderable = {
                let state = layout.state.lock().expect("layout mutex poisoned");
                state.renderable.clone()
            };

            let lines = console.render_lines(renderable.as_ref(), Some(&child_opts), None, true, false);
            render_map.insert(
                layout.id,
                LayoutRender { region, lines },
            );
        }

        render_map
    }

    /// Refresh a sub-layout in an alternate screen.
    ///
    /// This matches Rich's `Layout.refresh_screen` behavior and requires alt-screen mode.
    pub fn refresh_screen(&self, console: &mut crate::Console<std::io::Stdout>, layout_name: &str) -> std::io::Result<()> {
        let Some(layout) = self.get(layout_name) else {
            return Ok(());
        };

        let region = {
            let state = self.state.lock().expect("layout mutex poisoned");
            let Some(render) = state.render_map.get(&layout.id) else {
                return Ok(());
            };
            render.region
        };

        let mut child_opts = console.options().clone();
        let w = region.width as usize;
        let h = region.height as usize;
        child_opts.size = (w.max(1), h.max(1));
        child_opts.min_width = w.max(1);
        child_opts.max_width = w.max(1);
        child_opts.max_height = h.max(1);
        child_opts.height = Some(h.max(1));

        let lines = console.render_lines(&layout, Some(&child_opts), None, true, false);

        // Store updated lines in the root render_map.
        self.state.lock().expect("layout mutex poisoned").render_map.insert(
            layout.id,
            LayoutRender { region, lines: lines.clone() },
        );

        console.update_screen_lines(&lines, region.x.max(0) as u16, region.y.max(0) as u16)?;
        Ok(())
    }
}

impl Renderable for Layout {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        // Leaf layouts render their stored renderable.
        if self.children().is_empty() {
            let renderable = self.state.lock().expect("layout mutex poisoned").renderable.clone();
            return renderable.render(console, options);
        }

        let width = options.max_width.max(1);
        let height = options.height.unwrap_or_else(|| console.height()).max(1);

        let render_map = self.render_map(console, options);

        // Store last render_map on root.
        self.state.lock().expect("layout mutex poisoned").render_map = render_map.clone();

        let mut buffer = ScreenBuffer::new(width, height, None);
        // Insert in region order: sort by region tuple.
        let mut items: Vec<_> = render_map.into_values().collect();
        items.sort_by(|a, b| {
            let ra = a.region;
            let rb = b.region;
            (ra.x, ra.y, ra.width, ra.height).cmp(&(rb.x, rb.y, rb.width, rb.height))
        });

        for item in items {
            let x = item.region.x.max(0) as usize;
            let y = item.region.y.max(0) as usize;
            let w = item.region.width as usize;
            buffer.blit_lines(x, y, w, &item.lines);
        }

        let lines = buffer.to_styled_lines();
        let mut out = Segments::new();
        let new_line = Segment::line();
        for line in lines {
            for seg in line {
                out.push(seg);
            }
            out.push(new_line.clone());
        }
        out
    }
}

// =============================================================================
// Ratio resolve (copied from Rich _ratio.py)
// =============================================================================

fn ratio_resolve(total: i64, layouts: &[Layout]) -> Vec<i64> {
    let mut sizes: Vec<Option<i64>> = layouts
        .iter()
        .map(|layout| layout.state.lock().unwrap().size.map(|s| s as i64))
        .collect();

    while sizes.iter().any(|s| s.is_none()) {
        let flexible: Vec<(usize, &Layout)> = sizes
            .iter()
            .zip(layouts.iter())
            .enumerate()
            .filter_map(|(i, (size, edge))| size.is_none().then_some((i, edge)))
            .collect();

        let used: i64 = sizes.iter().map(|s| s.unwrap_or(0)).sum();
        let remaining = total - used;
        if remaining <= 0 {
            return sizes
                .into_iter()
                .zip(layouts.iter())
                .map(|(size, edge)| match size {
                    Some(v) => v,
                    None => edge.state.lock().unwrap().minimum_size.max(1) as i64,
                })
                .collect();
        }

        let total_ratio: i64 = flexible
            .iter()
            .map(|(_, edge)| edge.state.lock().unwrap().ratio.max(1) as i64)
            .sum();
        let portion_num = remaining;
        let portion_den = total_ratio.max(1);

        // Ensure minimum sizes first.
        let mut fixed_any = false;
        for (index, edge) in &flexible {
            let st = edge.state.lock().unwrap();
            let min = st.minimum_size.max(1) as i64;
            let ratio = st.ratio.max(1) as i64;
            // portion * ratio <= minimum_size
            if portion_num * ratio <= min * portion_den {
                sizes[*index] = Some(min);
                fixed_any = true;
                break;
            }
        }
        if fixed_any {
            continue;
        }

        // Distribute with remainder.
        let mut remainder_num: i64 = 0;
        for (index, edge) in flexible {
            let ratio = edge.state.lock().unwrap().ratio.max(1) as i64;
            let num = portion_num * ratio + remainder_num;
            let size = num / portion_den;
            remainder_num = num % portion_den;
            sizes[index] = Some(size);
        }
        break;
    }

    sizes.into_iter().map(|s| s.unwrap_or(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Text;

    #[test]
    fn test_ratio_resolve_respects_fixed_size() {
        let a = Layout::new().with_size(3);
        let b = Layout::new().with_ratio(1);
        let widths = ratio_resolve(10, &[a, b]);
        assert_eq!(widths, vec![3, 7]);
    }

    #[test]
    fn test_layout_split_row_renders_side_by_side() {
        let console = Console::new();
        let mut options = console.options().clone();
        options.max_width = 6;
        options.size = (6, 2);
        options.height = Some(2);

        let left = Layout::with_renderable(Text::plain("L")).with_name("left");
        let right = Layout::with_renderable(Text::plain("R")).with_name("right");
        let root = Layout::new();
        root.split_row(vec![left, right]);

        let output: String = root.render(&console, &options).iter().map(|s| s.text.to_string()).collect();
        let lines: Vec<&str> = output.split('\n').collect();
        assert!(lines[0].contains('L'));
        assert!(lines[0].contains('R'));
    }

    #[test]
    fn test_layout_get_by_name() {
        let child = Layout::new().with_name("child");
        let root = Layout::new();
        root.split_column(vec![child.clone()]);
        assert!(root.get("child").is_some());
    }

    #[test]
    fn test_layout_split_column_stacks() {
        let console = Console::new();
        let mut options = console.options().clone();
        options.max_width = 4;
        options.size = (4, 2);
        options.height = Some(2);

        let top = Layout::with_renderable(Text::plain("A")).with_size(1);
        let bottom = Layout::with_renderable(Text::plain("B"));
        let root = Layout::new();
        root.split_column(vec![top, bottom]);

        let output: String = root.render(&console, &options).iter().map(|s| s.text.to_string()).collect();
        let lines: Vec<&str> = output.split('\n').collect();
        assert!(lines[0].contains('A'));
        assert!(lines[1].contains('B'));
    }

    #[test]
    fn test_layout_nested_regions() {
        let console = Console::new();
        let mut options = console.options().clone();
        options.max_width = 6;
        options.size = (6, 3);
        options.height = Some(3);

        let header = Layout::with_renderable(Text::plain("H")).with_size(1).with_name("header");
        let body = Layout::new().with_name("body");
        let root = Layout::new();
        root.split_column(vec![header, body.clone()]);

        let left = Layout::with_renderable(Text::plain("L")).with_size(2);
        let right = Layout::with_renderable(Text::plain("R"));
        body.split_row(vec![left, right]);

        let output: String = root.render(&console, &options).iter().map(|s| s.text.to_string()).collect();
        let lines: Vec<&str> = output.split('\n').collect();
        // Header on first row.
        assert!(lines[0].contains('H'));
        // Body on second row: L should appear before R.
        let lpos = lines[1].find('L').unwrap();
        let rpos = lines[1].find('R').unwrap();
        assert!(lpos < rpos);
    }
}
