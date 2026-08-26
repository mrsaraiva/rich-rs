use rich_rs::markdown::Markdown;
use rich_rs::{Json, Syntax};

use crate::common::{
    destroy, ffi_call, finish, init_out, justify_from_id, required_mut, rich_str, strict_bool,
};
use crate::{
    RICH_ERROR_INVALID_ARGUMENT, RICH_ERROR_INVALID_STATE, RICH_ERROR_IO, RichRenderable,
    RichResult, RichStr,
};

macro_rules! map_builder {
    ($handle:expr, $transform:expr) => {{
        let handle = unsafe { required_mut($handle)? };
        let value = handle.0.take().ok_or(RICH_ERROR_INVALID_STATE)?;
        handle.0 = Some(($transform)(value));
        Ok(())
    }};
}

/// Owning syntax-highlighting builder. File construction distinguishes invalid
/// UTF-8/path arguments from filesystem I/O failures. Finish consumes on success.
pub struct RichSyntax(Option<Syntax>);

fn validate_theme(theme: &str) -> Result<(), RichResult> {
    let normalized = theme.to_ascii_lowercase().replace('_', "-");
    let known_alias = matches!(
        normalized.as_str(),
        "one-dark"
            | "onedark"
            | "one-light"
            | "onelight"
            | "github-dark"
            | "github-light"
            | "solarized-dark"
            | "solarized-light"
            | "gruvbox"
    );
    let available = Syntax::available_themes().iter().any(|candidate| {
        candidate
            .replace('_', "-")
            .eq_ignore_ascii_case(&normalized)
    });
    if known_alias || available {
        Ok(())
    } else {
        Err(RICH_ERROR_INVALID_ARGUMENT)
    }
}

#[unsafe(no_mangle)]
/// Create syntax content from borrowed code and lexer strings.
pub extern "C" fn rich_syntax_new(
    code: RichStr,
    lexer: RichStr,
    out_syntax: *mut *mut RichSyntax,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_syntax)? };
        let code = unsafe { rich_str(code)? };
        let lexer = unsafe { rich_str(lexer)? };
        if lexer.is_empty() {
            return Err(RICH_ERROR_INVALID_ARGUMENT);
        }
        unsafe {
            *out_syntax = Box::into_raw(Box::new(RichSyntax(Some(Syntax::new(code, lexer)))))
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Read syntax content from a UTF-8 path. Embedded NUL is invalid; filesystem
/// failures return `RICH_ERROR_IO`.
pub extern "C" fn rich_syntax_from_path(
    path: RichStr,
    out_syntax: *mut *mut RichSyntax,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_syntax)? };
        let path = unsafe { rich_str(path)? };
        if path.is_empty() || path.contains('\0') {
            return Err(RICH_ERROR_INVALID_ARGUMENT);
        }
        let syntax = Syntax::from_path(path).map_err(|_| RICH_ERROR_IO)?;
        unsafe { *out_syntax = Box::into_raw(Box::new(RichSyntax(Some(syntax)))) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_syntax_set_theme(syntax: *mut RichSyntax, theme: RichStr) -> RichResult {
    ffi_call(|| {
        let theme = unsafe { rich_str(theme)? }.to_owned();
        validate_theme(&theme)?;
        map_builder!(syntax, |syntax: Syntax| syntax.with_theme(theme))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_syntax_set_line_numbers(syntax: *mut RichSyntax, enabled: u8) -> RichResult {
    ffi_call(|| {
        let enabled = strict_bool(enabled)?;
        map_builder!(syntax, |syntax: Syntax| syntax.with_line_numbers(enabled))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_syntax_set_word_wrap(syntax: *mut RichSyntax, enabled: u8) -> RichResult {
    ffi_call(|| {
        let enabled = strict_bool(enabled)?;
        map_builder!(syntax, |syntax: Syntax| syntax.with_word_wrap(enabled))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_syntax_set_line_range(
    syntax: *mut RichSyntax,
    has_start: u8,
    start: u32,
    has_end: u8,
    end: u32,
) -> RichResult {
    ffi_call(|| {
        let start = if strict_bool(has_start)? {
            Some(usize::try_from(start).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?)
        } else {
            None
        };
        let end = if strict_bool(has_end)? {
            Some(usize::try_from(end).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?)
        } else {
            None
        };
        if let (Some(start), Some(end)) = (start, end)
            && (start == 0 || end == 0 || start > end)
        {
            return Err(RICH_ERROR_INVALID_ARGUMENT);
        }
        map_builder!(syntax, |syntax: Syntax| syntax.with_line_range(start, end))
    })
}

#[unsafe(no_mangle)]
/// Consume live syntax content on success and write an owned renderable.
pub extern "C" fn rich_syntax_finish(
    syntax: *mut *mut RichSyntax,
    out: *mut *mut RichRenderable,
) -> RichResult {
    unsafe { finish(syntax, out, |v| v.0.is_some(), |v| v.0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_syntax_free(syntax: *mut RichSyntax) {
    unsafe { destroy(syntax) }
}

/// Owning Markdown builder over borrowed UTF-8 input copied by Rust. Successful
/// finish consumes and NULLs the builder; free destroys unfinished builders.
pub struct RichMarkdown(Option<Markdown>);

#[unsafe(no_mangle)]
/// Create Markdown content through `out_markdown`.
pub extern "C" fn rich_markdown_new(
    source: RichStr,
    out_markdown: *mut *mut RichMarkdown,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_markdown)? };
        let source = unsafe { rich_str(source)? };
        unsafe {
            *out_markdown = Box::into_raw(Box::new(RichMarkdown(Some(Markdown::new(source)))))
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_markdown_set_code_theme(
    markdown: *mut RichMarkdown,
    theme: RichStr,
) -> RichResult {
    ffi_call(|| {
        let theme = unsafe { rich_str(theme)? }.to_owned();
        validate_theme(&theme)?;
        map_builder!(markdown, |markdown: Markdown| markdown
            .with_code_theme(theme))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_markdown_set_hyperlinks(
    markdown: *mut RichMarkdown,
    enabled: u8,
) -> RichResult {
    ffi_call(|| {
        let enabled = strict_bool(enabled)?;
        map_builder!(markdown, |markdown: Markdown| markdown
            .with_hyperlinks(enabled))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_markdown_set_justify(
    markdown: *mut RichMarkdown,
    justify: u32,
) -> RichResult {
    ffi_call(|| {
        let justify = justify_from_id(justify)?;
        map_builder!(markdown, |markdown: Markdown| markdown
            .with_justify(justify))
    })
}

#[unsafe(no_mangle)]
/// Consume live Markdown on success and write an owned renderable.
pub extern "C" fn rich_markdown_finish(
    markdown: *mut *mut RichMarkdown,
    out: *mut *mut RichRenderable,
) -> RichResult {
    unsafe { finish(markdown, out, |v| v.0.is_some(), |v| v.0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_markdown_free(markdown: *mut RichMarkdown) {
    unsafe { destroy(markdown) }
}

/// Owning validated-JSON builder. Malformed JSON and invalid booleans are
/// argument errors. Successful finish consumes and NULLs the builder.
pub struct RichJson(Option<Json>);

#[unsafe(no_mangle)]
/// Validate and create JSON content through `out_json`.
pub extern "C" fn rich_json_new(
    data: RichStr,
    indent: u32,
    highlight: u8,
    sort_keys: u8,
    out_json: *mut *mut RichJson,
) -> RichResult {
    ffi_call(|| {
        unsafe { init_out(out_json)? };
        let data = unsafe { rich_str(data)? };
        serde_json::from_str::<serde_json::Value>(data).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?;
        let indent = usize::try_from(indent).map_err(|_| RICH_ERROR_INVALID_ARGUMENT)?;
        let highlight = strict_bool(highlight)?;
        let sort_keys = strict_bool(sort_keys)?;
        unsafe {
            *out_json = Box::into_raw(Box::new(RichJson(Some(Json::new(
                data, indent, highlight, sort_keys,
            )))))
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
/// Consume live JSON content on success and write an owned renderable.
pub extern "C" fn rich_json_finish(
    json: *mut *mut RichJson,
    out: *mut *mut RichRenderable,
) -> RichResult {
    unsafe { finish(json, out, |v| v.0.is_some(), |v| v.0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rich_json_free(json: *mut RichJson) {
    unsafe { destroy(json) }
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
    fn syntax_path_rejects_embedded_nul_as_argument_error() {
        let mut syntax = ptr::dangling_mut();
        assert_eq!(
            rich_syntax_from_path(s("missing\0file.rs"), &mut syntax),
            RICH_ERROR_INVALID_ARGUMENT
        );
        assert!(syntax.is_null());
    }

    #[test]
    fn syntax_path_maps_filesystem_failure_to_io_error() {
        let mut syntax = ptr::dangling_mut();
        assert_eq!(
            rich_syntax_from_path(
                s("/this/path/must/not/exist/rich-ffi-foundation.rs"),
                &mut syntax
            ),
            RICH_ERROR_IO
        );
        assert!(syntax.is_null());
    }
}
