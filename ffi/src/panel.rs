use rich_rs::Panel;

use crate::common::{
    box_from_id, destroy, ffi_call, finish, init_out, parse_style, required_mut, required_ref,
    rich_str, shared_box, strict_bool,
};
use crate::{
    RICH_ERROR_INVALID_ARGUMENT, RICH_ERROR_INVALID_STATE, RichRenderable, RichResult, RichStr,
};

/// Owning panel builder. Content is borrowed at construction and retained
/// internally; successful finish consumes and NULLs only the panel builder.
pub struct RichPanel(pub(crate) Option<Panel>);

fn map_panel(panel: *mut RichPanel, f: impl FnOnce(Panel) -> Panel) -> Result<(), RichResult> {
    let panel = unsafe { required_mut(panel)? };
    let value = panel.0.take().ok_or(RICH_ERROR_INVALID_STATE)?;
    panel.0 = Some(f(value));
    Ok(())
}

#[unsafe(no_mangle)]
/// Create a panel that borrows reusable `content` and writes an owning builder.
pub extern "C" fn rich_panel_new(
    content: *const RichRenderable,
    out_panel: *mut *mut RichPanel,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_panel)? };
        let content = unsafe { required_ref(content)? };
        unsafe {
            *out_panel = Box::into_raw(Box::new(RichPanel(Some(Panel::new(shared_box(content))))));
        }
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_title(panel: *mut RichPanel, title: RichStr) -> RichResult {
    ffi_call(|| {
        let title = unsafe { rich_str(title)? }.to_owned();
        map_panel(panel, |panel| panel.with_title(title))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_subtitle(panel: *mut RichPanel, subtitle: RichStr) -> RichResult {
    ffi_call(|| {
        let subtitle = unsafe { rich_str(subtitle)? }.to_owned();
        map_panel(panel, |panel| panel.with_subtitle(subtitle))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_box(panel: *mut RichPanel, box_id: u32) -> RichResult {
    ffi_call(|| {
        let drawing = box_from_id(box_id)?;
        map_panel(panel, |panel| panel.with_box(drawing))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_expand(panel: *mut RichPanel, expand: u8) -> RichResult {
    ffi_call(|| {
        let expand = strict_bool(expand)?;
        map_panel(panel, |panel| panel.with_expand(expand))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_width(panel: *mut RichPanel, width: u32) -> RichResult {
    ffi_call(|| {
        let width = usize::try_from(width).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?;
        map_panel(panel, |panel| panel.with_width(width))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_padding(
    panel: *mut RichPanel,
    top: u32,
    right: u32,
    bottom: u32,
    left: u32,
) -> RichResult {
    ffi_call(|| {
        let padding = (
            usize::try_from(top).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?,
            usize::try_from(right).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?,
            usize::try_from(bottom).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?,
            usize::try_from(left).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?,
        );
        map_panel(panel, |panel| panel.with_padding(padding))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_style(panel: *mut RichPanel, style: RichStr) -> RichResult {
    ffi_call(|| {
        let style = parse_style(unsafe { rich_str(style)? })?;
        map_panel(panel, |panel| panel.with_style(style))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_panel_set_border_style(panel: *mut RichPanel, style: RichStr) -> RichResult {
    ffi_call(|| {
        let style = parse_style(unsafe { rich_str(style)? })?;
        map_panel(panel, |panel| panel.with_border_style(style))
    })
}

#[unsafe(no_mangle)]
/// Consume a live panel on success and write an owned renderable.
pub extern "C" fn rich_panel_finish(
    panel: *mut *mut RichPanel,
    out_renderable: *mut *mut RichRenderable,
) -> RichResult {
    unsafe { finish(panel, out_renderable, |v| v.0.is_some(), |v| v.0) }
}

#[unsafe(no_mangle)]
/// Destroy an unfinished panel. NULL is accepted.
pub extern "C" fn rich_panel_free(panel: *mut RichPanel) {
    unsafe { destroy(panel) }
}
