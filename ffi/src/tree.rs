//! Phase 4 — Tree C ABI.
//!
//! Implemented by the `ffi-phase4` lane. All functions here are
//! `#[unsafe(no_mangle)] pub extern "C"` and reuse the Phase-1 plumbing in
//! `crate::common` and the `crate::RichRenderable` composition currency.
//! cbindgen picks them up from this module automatically.
//!
//! # Handles
//!
//! * [`RichTree`] — an owning builder handle wrapping `Option<rich_rs::Tree>`.
//!   Build it, add nodes / set styles, then either `rich_tree_finish` it into a
//!   `RichRenderable` or `rich_tree_free` it.
//! * [`RichTreeNode`] — a **non-owning BORROW** into a child node living inside
//!   an owning `RichTree`. It is a raw pointer to the child `rich_rs::Tree`
//!   stored in its parent's `children` vector. See its doc for the lifetime
//!   contract. There is intentionally NO `rich_tree_node_free`.

use std::ffi::{CStr, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};

use rich_rs::{Renderable, Text, Tree};

use crate::RichRenderable;
use crate::common::parse_style;

/// Opaque owning `Tree` builder handle.
///
/// Wraps `Option<rich_rs::Tree>` so the consuming `rich_tree_finish` can move
/// the value out. Create with `rich_tree_new` / `rich_tree_new_renderable`,
/// then either `rich_tree_finish` (consumes into a `RichRenderable`) or
/// `rich_tree_free`.
///
/// While a `RichTree` is alive you may obtain `RichTreeNode*` borrows into it
/// via `rich_tree_add` / `rich_tree_add_renderable`; those borrows are only
/// valid until this handle is finished or freed.
pub struct RichTree(Option<Tree>);

/// Opaque, NON-OWNING handle to a child node inside an owning [`RichTree`].
///
/// A `RichTreeNode*` is, quite literally, a borrowed pointer to a child
/// `rich_rs::Tree` living inside the `children` vector of an owning `RichTree`.
/// The pointer value handed to C is that interior pointer reinterpreted as
/// `RichTreeNode*` — no separate allocation is made, so there is nothing to
/// free. It exists only so callers can attach grandchildren to a specific
/// sub-node (`rich_tree_node_add` / `rich_tree_node_add_renderable`).
///
/// # Lifetime contract (read carefully)
///
/// * A `RichTreeNode*` is valid ONLY while its owning `RichTree` is alive
///   (i.e. before `rich_tree_finish` or `rich_tree_free` is called on it).
///   After the owning `RichTree` is finished/freed, every node handle derived
///   from it is DANGLING — do not use it.
/// * There is NO `rich_tree_node_free`. Node handles are not owned by the
///   caller and must never be freed; their storage is owned by the `RichTree`.
/// * Adding further children to a *parent* node may reallocate that parent's
///   `children` vector, which can invalidate previously returned sibling
///   `RichTreeNode*` handles that point into the same vector. To stay safe,
///   finish building one sub-node's descendants (depth-first) before requesting
///   another sibling handle from the same parent, or re-fetch handles as needed.
pub struct RichTreeNode {
    _private: [u8; 0],
}

/// Reinterpret an interior `&mut Tree` as the opaque `RichTreeNode*` we hand to
/// C. No allocation: the handle IS the borrowed pointer, so it never needs a
/// matching free. See [`RichTreeNode`] for the lifetime contract.
#[inline]
fn node_handle(child: &mut Tree) -> *mut RichTreeNode {
    (child as *mut Tree).cast::<RichTreeNode>()
}

/// Reinterpret a `RichTreeNode*` back into the borrowed `*mut Tree`.
///
/// # Safety
/// `node` must be a live handle previously produced by `node_handle` whose
/// owning `RichTree` is still alive (see the [`RichTreeNode`] contract).
#[inline]
unsafe fn node_tree<'a>(node: *mut RichTreeNode) -> &'a mut Tree {
    // SAFETY: caller upholds the RichTreeNode lifetime contract.
    unsafe { &mut *node.cast::<Tree>() }
}

/// Create a `RichTree` from a plain-text label (no markup parsing).
///
/// Equivalent to `Tree::new(Box::new(Text::plain(label)))`. `label` must be
/// valid NUL-terminated UTF-8. Returns NULL if `label` is NULL or not valid
/// UTF-8. Free with `rich_tree_finish` or `rich_tree_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_new(label: *const c_char) -> *mut RichTree {
    if label.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees a valid NUL-terminated string.
        let s = unsafe { CStr::from_ptr(label) }.to_str().ok()?;
        let tree = Tree::new(Box::new(Text::plain(s)));
        Some(Box::into_raw(Box::new(RichTree(Some(tree)))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Create a `RichTree` whose root label is an arbitrary renderable.
///
/// CONSUMES `label`: the `RichRenderable*` is invalid afterward (do not free or
/// reuse it). Returns NULL if `label` is NULL. Free the returned handle with
/// `rich_tree_finish` or `rich_tree_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_new_renderable(label: *mut RichRenderable) -> *mut RichTree {
    if label.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `label` came from a `*_finish` fn (`Box::into_raw`).
        let inner = unsafe { Box::from_raw(label) }.0;
        let tree = Tree::new(inner);
        Box::into_raw(Box::new(RichTree(Some(tree))))
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Add a child node with a plain-text label to a `RichTree`.
///
/// Returns a `RichTreeNode*` BORROW into the new child (for attaching
/// grandchildren via `rich_tree_node_add*`). The returned handle is valid only
/// until `tree` is finished/freed and must NOT be freed (see [`RichTreeNode`]).
///
/// `label` must be valid NUL-terminated UTF-8. Returns NULL if `tree`/`label`
/// is NULL, `label` is not valid UTF-8, or the tree was already finished.
#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_add(tree: *mut RichTree, label: *const c_char) -> *mut RichTreeNode {
    if tree.is_null() || label.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from a `rich_tree_new*` fn.
        let t = unsafe { &mut *tree };
        let s = unsafe { CStr::from_ptr(label) }.to_str().ok()?;
        let inner = t.0.as_mut()?;
        let child: &mut Tree = inner.add(Box::new(Text::plain(s)));
        Some(node_handle(child))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Add a child node whose label is an arbitrary renderable to a `RichTree`.
///
/// CONSUMES `label`: the `RichRenderable*` is invalid afterward. Returns a
/// `RichTreeNode*` BORROW into the new child (valid only until `tree` is
/// finished/freed; must NOT be freed — see [`RichTreeNode`]).
///
/// Returns NULL if `tree`/`label` is NULL or the tree was already finished. If
/// it returns NULL the `label` renderable has still been consumed only when a
/// non-NULL `label` was passed and the tree was valid; on a NULL-pointer guard
/// nothing is consumed.
#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_add_renderable(
    tree: *mut RichTree,
    label: *mut RichRenderable,
) -> *mut RichTreeNode {
    if tree.is_null() || label.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from a `rich_tree_new*` fn.
        let t = unsafe { &mut *tree };
        // SAFETY: `label` came from a `*_finish` fn (`Box::into_raw`).
        let inner_label = unsafe { Box::from_raw(label) }.0;
        let inner = t.0.as_mut()?;
        let child: &mut Tree = inner.add(inner_label);
        Some(node_handle(child))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Add a plain-text child to a sub-node, returning a handle to the grandchild.
///
/// `node` must be a live `RichTreeNode*` whose owning `RichTree` has not been
/// finished/freed. Returns a `RichTreeNode*` BORROW into the new grandchild
/// (same lifetime/never-free contract — see [`RichTreeNode`]).
///
/// `label` must be valid NUL-terminated UTF-8. Returns NULL if `node`/`label`
/// is NULL or `label` is not valid UTF-8.
#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_node_add(
    node: *mut RichTreeNode,
    label: *const c_char,
) -> *mut RichTreeNode {
    if node.is_null() || label.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        let s = unsafe { CStr::from_ptr(label) }.to_str().ok()?;
        // SAFETY: per the RichTreeNode contract the handle points to a live
        // child Tree while the owning RichTree is alive; the caller upholds it.
        let inner = unsafe { node_tree(node) };
        let child: &mut Tree = inner.add(Box::new(Text::plain(s)));
        Some(node_handle(child))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Add a renderable-labelled child to a sub-node, returning a grandchild handle.
///
/// CONSUMES `label`: the `RichRenderable*` is invalid afterward. `node` must be
/// a live `RichTreeNode*` whose owning `RichTree` is still alive. Returns a
/// `RichTreeNode*` BORROW into the new grandchild (never free it — see
/// [`RichTreeNode`]).
///
/// Returns NULL if `node`/`label` is NULL.
#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_node_add_renderable(
    node: *mut RichTreeNode,
    label: *mut RichRenderable,
) -> *mut RichTreeNode {
    if node.is_null() || label.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `label` came from a `*_finish` fn (`Box::into_raw`).
        let inner_label = unsafe { Box::from_raw(label) }.0;
        // SAFETY: per the RichTreeNode contract the handle points to a live
        // child Tree while the owning RichTree is alive; the caller upholds it.
        let inner = unsafe { node_tree(node) };
        let child: &mut Tree = inner.add(inner_label);
        node_handle(child)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Set the label style (e.g. `"bold cyan"`) on a `RichTree`.
///
/// Parses `style` with `Style::parse`; a NULL/invalid/unparseable style is a
/// no-op (the tree is left unchanged). Maps to `Tree::with_style`. Does not
/// consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_set_style(tree: *mut RichTree, style: *const c_char) {
    if tree.is_null() || style.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from a `rich_tree_new*` fn.
        let t = unsafe { &mut *tree };
        let Some(s) = (unsafe { CStr::from_ptr(style) }).to_str().ok() else {
            return;
        };
        if let Some(parsed) = parse_style(s)
            && let Some(inner) = t.0.take()
        {
            t.0 = Some(inner.with_style(parsed));
        }
    }));
}

/// Set the guide-line style (e.g. `"dim"`) on a `RichTree`.
///
/// Parses `style` with `Style::parse`; a NULL/invalid/unparseable style is a
/// no-op. Maps to `Tree::with_guide_style`. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_set_guide_style(tree: *mut RichTree, style: *const c_char) {
    if tree.is_null() || style.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from a `rich_tree_new*` fn.
        let t = unsafe { &mut *tree };
        let Some(s) = (unsafe { CStr::from_ptr(style) }).to_str().ok() else {
            return;
        };
        if let Some(parsed) = parse_style(s)
            && let Some(inner) = t.0.take()
        {
            t.0 = Some(inner.with_guide_style(parsed));
        }
    }));
}

/// Set whether the root node is hidden when rendering.
///
/// When `true`, only children render (no root label/guides). Maps to
/// `Tree::with_hide_root`. NULL `tree` is a no-op. Does not consume the handle.
#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_set_hide_root(tree: *mut RichTree, hide: bool) {
    if tree.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: non-NULL handle from a `rich_tree_new*` fn.
        let t = unsafe { &mut *tree };
        if let Some(inner) = t.0.take() {
            t.0 = Some(inner.with_hide_root(hide));
        }
    }));
}

/// CONSUME a `RichTree`, erasing it into the type-erased `RichRenderable`
/// composition currency. The `RichTree*` is invalid afterward, and so is every
/// `RichTreeNode*` previously derived from it.
///
/// Returns NULL if `tree` is NULL or was already finished. The returned
/// `RichRenderable*` must be freed with `rich_renderable_free` (unless it is
/// passed into a consuming container).
#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_finish(tree: *mut RichTree) -> *mut RichRenderable {
    if tree.is_null() {
        return std::ptr::null_mut();
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in a `rich_tree_new*` function.
        let boxed = unsafe { Box::from_raw(tree) };
        let inner = boxed.0?;
        let erased: Box<dyn Renderable + Send + Sync> = Box::new(inner);
        Some(Box::into_raw(Box::new(RichRenderable(erased))))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Free a `RichTree` that was created but never `rich_tree_finish`ed. NULL is a
/// no-op. This also invalidates every `RichTreeNode*` derived from this tree.
#[unsafe(no_mangle)]
pub extern "C" fn rich_tree_free(tree: *mut RichTree) {
    if tree.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: came from `Box::into_raw` in a `rich_tree_new*` function.
        drop(unsafe { Box::from_raw(tree) });
    }));
}
