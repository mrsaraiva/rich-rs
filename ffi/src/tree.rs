use std::collections::HashMap;
use std::sync::Arc;

use rich_rs::{ConsoleOptions, Measurement, Renderable, Segments, Style, Text, Tree};

use crate::common::{
    SharedRenderable, destroy, ffi_call, finish, init_out, init_scalar_out, parse_style,
    required_mut, required_ref, rich_str, strict_bool,
};
use crate::{RICH_ERROR_INVALID_STATE, RICH_TREE_ROOT_ID, RichRenderable, RichResult, RichStr};

#[derive(Clone)]
struct Node {
    label: Arc<dyn Renderable + Send + Sync>,
    children: Vec<Node>,
}

struct TreeBuilder {
    root: Node,
    paths: HashMap<u64, Vec<usize>>,
    next_id: u64,
    style: Option<Style>,
    guide_style: Option<Style>,
    hide_root: bool,
}

impl TreeBuilder {
    fn new(label: Arc<dyn Renderable + Send + Sync>) -> Self {
        let mut paths = HashMap::new();
        paths.insert(RICH_TREE_ROOT_ID, Vec::new());
        Self {
            root: Node {
                label,
                children: Vec::new(),
            },
            paths,
            next_id: 1,
            style: None,
            guide_style: None,
            hide_root: false,
        }
    }

    fn node_mut(&mut self, path: &[usize]) -> Option<&mut Node> {
        let mut node = &mut self.root;
        for &index in path {
            node = node.children.get_mut(index)?;
        }
        Some(node)
    }

    #[cfg(test)]
    fn node(&self, path: &[usize]) -> Option<&Node> {
        let mut node = &self.root;
        for &index in path {
            node = node.children.get(index)?;
        }
        Some(node)
    }

    fn add(
        &mut self,
        parent_id: u64,
        label: Arc<dyn Renderable + Send + Sync>,
    ) -> Result<u64, RichResult> {
        let parent_path = self
            .paths
            .get(&parent_id)
            .cloned()
            .ok_or(RICH_ERROR_INVALID_STATE)?;
        let id = self.next_id;
        let next_id = id.checked_add(1).ok_or(RICH_ERROR_INVALID_STATE)?;
        self.paths
            .try_reserve(1)
            .map_err(|_| RICH_ERROR_INVALID_STATE)?;
        let child_index = {
            let parent = self
                .node_mut(&parent_path)
                .ok_or(RICH_ERROR_INVALID_STATE)?;
            parent
                .children
                .try_reserve(1)
                .map_err(|_| RICH_ERROR_INVALID_STATE)?;
            parent.children.len()
        };
        let mut child_path = parent_path.clone();
        child_path
            .try_reserve(1)
            .map_err(|_| RICH_ERROR_INVALID_STATE)?;
        child_path.push(child_index);
        let parent = self
            .node_mut(&parent_path)
            .ok_or(RICH_ERROR_INVALID_STATE)?;
        parent.children.push(Node {
            label,
            children: Vec::new(),
        });
        self.paths.insert(id, child_path);
        self.next_id = next_id;
        Ok(id)
    }

    fn materialize(&self) -> Tree {
        fn add_children(parent: &mut Tree, children: &[Node]) {
            for node in children {
                let child = parent.add(Box::new(SharedRenderable(Arc::clone(&node.label))));
                add_children(child, &node.children);
            }
        }

        let mut tree = Tree::new(Box::new(SharedRenderable(Arc::clone(&self.root.label))));
        if let Some(style) = self.style {
            tree = tree.with_style(style);
        }
        if let Some(style) = self.guide_style {
            tree = tree.with_guide_style(style);
        }
        tree = tree.with_hide_root(self.hide_root);
        add_children(&mut tree, &self.root.children);
        tree
    }
}

impl Renderable for TreeBuilder {
    fn render(&self, console: &rich_rs::Console, options: &ConsoleOptions) -> Segments {
        self.materialize().render(console, options)
    }

    fn measure(&self, console: &rich_rs::Console, options: &ConsoleOptions) -> Measurement {
        self.materialize().measure(console, options)
    }
}

/// Owning tree builder. Node IDs are stable values owned by this tree; ID zero
/// addresses the root. Renderable labels are borrowed and retained internally.
/// Successful finish consumes the tree and invalidates all of its node IDs.
pub struct RichTree(Option<TreeBuilder>);

fn builder_mut(tree: *mut RichTree) -> Result<&'static mut TreeBuilder, RichResult> {
    unsafe { required_mut(tree)? }
        .0
        .as_mut()
        .ok_or(RICH_ERROR_INVALID_STATE)
}

#[unsafe(no_mangle)]
/// Create a tree with a plain UTF-8 root label.
pub extern "C" fn rich_tree_new(label: RichStr, out_tree: *mut *mut RichTree) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_tree)? };
        let label = unsafe { rich_str(label)? };
        let builder = TreeBuilder::new(Arc::new(Text::plain(label)));
        unsafe { *out_tree = Box::into_raw(Box::new(RichTree(Some(builder)))) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_new_renderable(
    label: *const RichRenderable,
    out_tree: *mut *mut RichTree,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_tree)? };
        let label = unsafe { required_ref(label)? };
        let builder = TreeBuilder::new(Arc::clone(&label.0));
        unsafe { *out_tree = Box::into_raw(Box::new(RichTree(Some(builder)))) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Add a child under `parent_id` and write its stable ID. Unknown parents return
/// `RICH_ERROR_INVALID_STATE` without mutating the tree.
pub extern "C" fn rich_tree_add(
    tree: *mut RichTree,
    parent_id: u64,
    label: RichStr,
    out_node_id: *mut u64,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_scalar_out(out_node_id)? };
        let label = unsafe { rich_str(label)? };
        let id = builder_mut(tree)?.add(parent_id, Arc::new(Text::plain(label)))?;
        unsafe { *out_node_id = id };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_add_renderable(
    tree: *mut RichTree,
    parent_id: u64,
    label: *const RichRenderable,
    out_node_id: *mut u64,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_scalar_out(out_node_id)? };
        let label = unsafe { required_ref(label)? };
        let id = builder_mut(tree)?.add(parent_id, Arc::clone(&label.0))?;
        unsafe { *out_node_id = id };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_set_style(tree: *mut RichTree, style: RichStr) -> RichResult {
    ffi_call(|| {
        let style = parse_style(unsafe { rich_str(style)? })?;
        builder_mut(tree)?.style = Some(style);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_set_guide_style(tree: *mut RichTree, style: RichStr) -> RichResult {
    ffi_call(|| {
        let style = parse_style(unsafe { rich_str(style)? })?;
        builder_mut(tree)?.guide_style = Some(style);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_set_hide_root(tree: *mut RichTree, hide: u8) -> RichResult {
    ffi_call(|| {
        let hide = strict_bool(hide)?;
        builder_mut(tree)?.hide_root = hide;
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Consume a live tree on success and write an owned renderable.
pub extern "C" fn rich_tree_finish(
    tree: *mut *mut RichTree,
    out_renderable: *mut *mut RichRenderable,
) -> RichResult {
    unsafe { finish(tree, out_renderable, |v| v.0.is_some(), |v| v.0) }
}

#[unsafe(no_mangle)]
/// Destroy an unfinished tree. NULL is accepted.
pub extern "C" fn rich_tree_free(tree: *mut RichTree) {
    unsafe { destroy(tree) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    fn s(value: &str) -> RichStr {
        RichStr {
            data: value.as_ptr(),
            len: value.len() as u64,
        }
    }

    #[test]
    fn ids_survive_breadth_and_depth_and_unknown_is_atomic() {
        let mut tree = ptr::null_mut();
        assert_eq!(rich_tree_new(s("root"), &mut tree), crate::RICH_OK);
        let mut first = 0;
        let mut sibling = 0;
        let mut grandchild = 0;
        assert_eq!(
            rich_tree_add(tree, 0, s("first"), &mut first),
            crate::RICH_OK
        );
        assert_eq!(
            rich_tree_add(tree, 0, s("sibling"), &mut sibling),
            crate::RICH_OK
        );
        assert_eq!(
            rich_tree_add(tree, first, s("deep"), &mut grandchild),
            crate::RICH_OK
        );
        assert_eq!((first, sibling, grandchild), (1, 2, 3));
        let mut parent = grandchild;
        for depth in 0..32 {
            let mut child = 0;
            assert_eq!(
                rich_tree_add(tree, parent, s(&format!("depth-{depth}")), &mut child),
                crate::RICH_OK
            );
            parent = child;
        }
        for breadth in 0..32 {
            let mut child = 0;
            assert_eq!(
                rich_tree_add(
                    tree,
                    RICH_TREE_ROOT_ID,
                    s(&format!("breadth-{breadth}")),
                    &mut child
                ),
                crate::RICH_OK
            );
        }
        let before = unsafe { (*tree).0.as_ref().unwrap().paths.len() };
        let mut output = 99;
        assert_eq!(
            rich_tree_add(tree, 999, s("bad"), &mut output),
            RICH_ERROR_INVALID_STATE
        );
        assert_eq!(output, 0);
        assert_eq!(unsafe { (*tree).0.as_ref().unwrap().paths.len() }, before);
        let builder = unsafe { (*tree).0.as_ref().unwrap() };
        assert!(
            builder
                .paths
                .values()
                .all(|path| builder.node(path).is_some())
        );
        let mut renderable = ptr::null_mut();
        assert_eq!(rich_tree_finish(&mut tree, &mut renderable), crate::RICH_OK);
        assert!(tree.is_null());
        crate::rich_renderable_free(renderable);
        rich_tree_free(tree);
    }
}
