//! Error types for rich-rs.

use thiserror::Error;

/// Errors that can occur when parsing styles, colors, or markup.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Invalid color specification.
    #[error("invalid color: {0}")]
    InvalidColor(String),

    /// Invalid style specification.
    #[error("invalid style: {0}")]
    InvalidStyle(String),

    /// Invalid markup syntax.
    #[error("invalid markup: {0}")]
    InvalidMarkup(String),

    /// Unclosed tag in markup.
    #[error("unclosed tag: {0}")]
    UnclosedTag(String),

    /// Unexpected closing tag in markup.
    #[error("unexpected closing tag: {0}")]
    UnexpectedClosingTag(String),
}

impl ParseError {
    /// Create an invalid color error.
    pub fn invalid_color(s: impl Into<String>) -> Self {
        ParseError::InvalidColor(s.into())
    }

    /// Create an invalid style error.
    pub fn invalid_style(s: impl Into<String>) -> Self {
        ParseError::InvalidStyle(s.into())
    }

    /// Create an invalid markup error.
    pub fn invalid_markup(s: impl Into<String>) -> Self {
        ParseError::InvalidMarkup(s.into())
    }
}
