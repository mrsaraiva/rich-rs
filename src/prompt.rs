//! Interactive prompts for user input.
//!
//! This module provides prompt types for getting user input with validation,
//! choices, and default values. It matches the Python Rich `prompt.py` module.
//!
//! # Example
//!
//! ```ignore
//! use rich_rs::prompt::{Prompt, IntPrompt, Confirm};
//!
//! // Simple string prompt
//! let name = Prompt::ask("Enter your name")?;
//!
//! // Prompt with default value
//! let name = Prompt::new("Enter your name")
//!     .with_default("Anonymous")
//!     .ask()?;
//!
//! // Integer prompt
//! let count: i32 = IntPrompt::ask("How many?")?;
//!
//! // Confirmation prompt
//! if Confirm::ask("Continue?")? {
//!     println!("Continuing...");
//! }
//! ```

use std::io;

use crate::Console;
use crate::text::Text;

// ============================================================================
// Errors
// ============================================================================

/// Errors that can occur during prompting.
#[derive(Debug, Clone)]
pub enum PromptError {
    /// An I/O error occurred.
    Io(String),
    /// The user provided an invalid response.
    InvalidResponse(InvalidResponse),
    /// Input was interrupted (e.g., Ctrl+C or EOF).
    Interrupted,
}

impl std::fmt::Display for PromptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PromptError::Io(msg) => write!(f, "I/O error: {}", msg),
            PromptError::InvalidResponse(err) => write!(f, "{}", err.message),
            PromptError::Interrupted => write!(f, "Input interrupted"),
        }
    }
}

impl std::error::Error for PromptError {}

impl From<io::Error> for PromptError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::UnexpectedEof | io::ErrorKind::Interrupted => PromptError::Interrupted,
            _ => PromptError::Io(err.to_string()),
        }
    }
}

impl From<InvalidResponse> for PromptError {
    fn from(err: InvalidResponse) -> Self {
        PromptError::InvalidResponse(err)
    }
}

/// Indicates that a response was invalid.
///
/// Raise this within processing logic to indicate an error and provide an error message.
#[derive(Debug, Clone)]
pub struct InvalidResponse {
    /// The error message to display.
    pub message: String,
}

impl InvalidResponse {
    /// Create a new invalid response error.
    pub fn new(message: impl Into<String>) -> Self {
        InvalidResponse {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for InvalidResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for InvalidResponse {}

/// Result type for prompt operations.
pub type Result<T> = std::result::Result<T, PromptError>;

// ============================================================================
// PromptBase - Common prompt functionality
// ============================================================================

/// Base trait for prompt implementations.
pub trait PromptBase<T> {
    /// The error message for invalid values.
    const VALIDATE_ERROR_MESSAGE: &'static str = "[prompt.invalid]Please enter a valid value";

    /// The error message for invalid choices.
    const ILLEGAL_CHOICE_MESSAGE: &'static str =
        "[prompt.invalid.choice]Please select one of the available options";

    /// The prompt suffix.
    const PROMPT_SUFFIX: &'static str = ": ";

    /// Process the raw string response into the target type.
    fn process_response(&self, value: &str) -> std::result::Result<T, InvalidResponse>;

    /// Render the default value for display.
    fn render_default(&self, default: &T) -> String;
}

// ============================================================================
// Prompt - String prompts
// ============================================================================

/// A prompt that returns a string.
///
/// # Example
///
/// ```ignore
/// use rich_rs::prompt::Prompt;
///
/// // Simple usage
/// let name = Prompt::ask("Enter your name")?;
///
/// // With options
/// let name = Prompt::new("Enter your name")
///     .with_default("Anonymous")
///     .with_choices(&["Alice", "Bob", "Charlie"])
///     .ask()?;
/// ```
#[derive(Debug, Clone)]
pub struct Prompt {
    /// The prompt text.
    prompt: String,
    /// Optional default value.
    default: Option<String>,
    /// Optional list of valid choices.
    choices: Option<Vec<String>>,
    /// Whether choice matching is case-sensitive.
    case_sensitive: bool,
    /// Whether to show the default value in the prompt.
    show_default: bool,
    /// Whether to show the choices in the prompt.
    show_choices: bool,
    /// Whether this is a password prompt (input will be masked).
    password: bool,
}

impl Default for Prompt {
    fn default() -> Self {
        Self::new("")
    }
}

impl Prompt {
    /// Create a new prompt with the given text.
    pub fn new(prompt: impl Into<String>) -> Self {
        Prompt {
            prompt: prompt.into(),
            default: None,
            choices: None,
            case_sensitive: true,
            show_default: true,
            show_choices: true,
            password: false,
        }
    }

    /// Set the default value.
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Set the list of valid choices.
    pub fn with_choices(mut self, choices: &[&str]) -> Self {
        self.choices = Some(choices.iter().map(|s| s.to_string()).collect());
        self
    }

    /// Set whether choice matching is case-sensitive (default: true).
    pub fn case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }

    /// Set whether to show the default value in the prompt (default: true).
    pub fn show_default(mut self, show: bool) -> Self {
        self.show_default = show;
        self
    }

    /// Set whether to show the choices in the prompt (default: true).
    pub fn show_choices(mut self, show: bool) -> Self {
        self.show_choices = show;
        self
    }

    /// Set whether this is a password prompt (default: false).
    pub fn password(mut self, is_password: bool) -> Self {
        self.password = is_password;
        self
    }

    /// Shortcut to create and run a prompt, returning the result.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let name = Prompt::ask("Enter your name")?;
    /// ```
    pub fn ask(prompt: impl Into<String>) -> Result<String> {
        Prompt::new(prompt).run()
    }

    /// Run the prompt loop and return the result.
    pub fn run(&self) -> Result<String> {
        let mut console = Console::new();
        self.run_with_console(&mut console)
    }

    /// Run the prompt loop with a specific console.
    pub fn run_with_console(&self, console: &mut Console) -> Result<String> {
        loop {
            let prompt_text = self.make_prompt();
            let value = console.input(&prompt_text, self.password)?;

            // Use default if input is empty
            if value.is_empty() {
                if let Some(ref default) = self.default {
                    return Ok(default.clone());
                }
            }

            // Process the response
            match self.process_response(&value) {
                Ok(result) => return Ok(result),
                Err(err) => {
                    // Display error and continue loop
                    let error_text = Text::from_markup(&err.message, false)
                        .unwrap_or_else(|_| Text::plain(&err.message));
                    let _ = console.print(&error_text, None, None, None, false, "\n");
                }
            }
        }
    }

    /// Build the prompt text including choices and default.
    fn make_prompt(&self) -> Text {
        let mut parts = vec![self.prompt.clone()];

        // Add choices
        if self.show_choices {
            if let Some(ref choices) = self.choices {
                let choices_str = choices.join("/");
                parts.push(format!(" [prompt.choices]\\[{}][/]", choices_str));
            }
        }

        // Add default
        if self.show_default {
            if let Some(ref default) = self.default {
                parts.push(format!(" [prompt.default]({})[/]", default));
            }
        }

        // Add suffix
        parts.push(": ".to_string());

        let markup = parts.join("");
        Text::from_markup(&markup, false).unwrap_or_else(|_| Text::plain(&markup))
    }

    /// Check if the value is in the list of valid choices.
    fn check_choice(&self, value: &str) -> bool {
        if let Some(ref choices) = self.choices {
            if self.case_sensitive {
                choices.iter().any(|c| c == value)
            } else {
                let value_lower = value.to_lowercase();
                choices.iter().any(|c| c.to_lowercase() == value_lower)
            }
        } else {
            true
        }
    }

    /// Get the original choice (for case-insensitive matching).
    fn get_original_choice(&self, value: &str) -> String {
        if let Some(ref choices) = self.choices {
            if !self.case_sensitive {
                let value_lower = value.to_lowercase();
                for choice in choices {
                    if choice.to_lowercase() == value_lower {
                        return choice.clone();
                    }
                }
            }
        }
        value.to_string()
    }

    /// Process the response.
    fn process_response(&self, value: &str) -> std::result::Result<String, InvalidResponse> {
        let value = value.trim();

        // Check choice
        if self.choices.is_some() && !self.check_choice(value) {
            return Err(InvalidResponse::new(
                "[prompt.invalid.choice]Please select one of the available options",
            ));
        }

        // Return original choice for case-insensitive matching
        if !self.case_sensitive && self.choices.is_some() {
            Ok(self.get_original_choice(value))
        } else {
            Ok(value.to_string())
        }
    }
}

// ============================================================================
// IntPrompt - Integer prompts
// ============================================================================

/// A prompt that returns an integer.
///
/// # Example
///
/// ```ignore
/// use rich_rs::prompt::IntPrompt;
///
/// let count: i32 = IntPrompt::ask("How many?")?;
///
/// // With default
/// let count = IntPrompt::new("How many?")
///     .with_default(5)
///     .ask()?;
/// ```
#[derive(Debug, Clone)]
pub struct IntPrompt {
    /// The prompt text.
    prompt: String,
    /// Optional default value.
    default: Option<i64>,
    /// Whether to show the default value in the prompt.
    show_default: bool,
}

impl Default for IntPrompt {
    fn default() -> Self {
        Self::new("")
    }
}

impl IntPrompt {
    /// Create a new integer prompt with the given text.
    pub fn new(prompt: impl Into<String>) -> Self {
        IntPrompt {
            prompt: prompt.into(),
            default: None,
            show_default: true,
        }
    }

    /// Set the default value.
    pub fn with_default(mut self, default: i64) -> Self {
        self.default = Some(default);
        self
    }

    /// Set whether to show the default value in the prompt (default: true).
    pub fn show_default(mut self, show: bool) -> Self {
        self.show_default = show;
        self
    }

    /// Shortcut to create and run a prompt, returning the result.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let count: i64 = IntPrompt::ask("How many?")?;
    /// ```
    pub fn ask(prompt: impl Into<String>) -> Result<i64> {
        IntPrompt::new(prompt).run()
    }

    /// Run the prompt loop and return the result.
    pub fn run(&self) -> Result<i64> {
        let mut console = Console::new();
        self.run_with_console(&mut console)
    }

    /// Run the prompt loop with a specific console.
    pub fn run_with_console(&self, console: &mut Console) -> Result<i64> {
        loop {
            let prompt_text = self.make_prompt();
            let value = console.input(&prompt_text, false)?;

            // Use default if input is empty
            if value.is_empty() {
                if let Some(default) = self.default {
                    return Ok(default);
                }
            }

            // Process the response
            match self.process_response(&value) {
                Ok(result) => return Ok(result),
                Err(err) => {
                    // Display error and continue loop
                    let error_text = Text::from_markup(&err.message, false)
                        .unwrap_or_else(|_| Text::plain(&err.message));
                    let _ = console.print(&error_text, None, None, None, false, "\n");
                }
            }
        }
    }

    /// Build the prompt text including default.
    fn make_prompt(&self) -> Text {
        let mut parts = vec![self.prompt.clone()];

        // Add default
        if self.show_default {
            if let Some(default) = self.default {
                parts.push(format!(" [prompt.default]({})[/]", default));
            }
        }

        // Add suffix
        parts.push(": ".to_string());

        let markup = parts.join("");
        Text::from_markup(&markup, false).unwrap_or_else(|_| Text::plain(&markup))
    }

    /// Process the response.
    fn process_response(&self, value: &str) -> std::result::Result<i64, InvalidResponse> {
        let value = value.trim();
        value.parse::<i64>().map_err(|_| {
            InvalidResponse::new("[prompt.invalid]Please enter a valid integer number")
        })
    }
}

// ============================================================================
// FloatPrompt - Float prompts
// ============================================================================

/// A prompt that returns a floating-point number.
///
/// # Example
///
/// ```ignore
/// use rich_rs::prompt::FloatPrompt;
///
/// let temperature: f64 = FloatPrompt::ask("Enter temperature")?;
///
/// // With default
/// let temp = FloatPrompt::new("Enter temperature")
///     .with_default(98.6)
///     .ask()?;
/// ```
#[derive(Debug, Clone)]
pub struct FloatPrompt {
    /// The prompt text.
    prompt: String,
    /// Optional default value.
    default: Option<f64>,
    /// Whether to show the default value in the prompt.
    show_default: bool,
}

impl Default for FloatPrompt {
    fn default() -> Self {
        Self::new("")
    }
}

impl FloatPrompt {
    /// Create a new float prompt with the given text.
    pub fn new(prompt: impl Into<String>) -> Self {
        FloatPrompt {
            prompt: prompt.into(),
            default: None,
            show_default: true,
        }
    }

    /// Set the default value.
    pub fn with_default(mut self, default: f64) -> Self {
        self.default = Some(default);
        self
    }

    /// Set whether to show the default value in the prompt (default: true).
    pub fn show_default(mut self, show: bool) -> Self {
        self.show_default = show;
        self
    }

    /// Shortcut to create and run a prompt, returning the result.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let temp: f64 = FloatPrompt::ask("Enter temperature")?;
    /// ```
    pub fn ask(prompt: impl Into<String>) -> Result<f64> {
        FloatPrompt::new(prompt).run()
    }

    /// Run the prompt loop and return the result.
    pub fn run(&self) -> Result<f64> {
        let mut console = Console::new();
        self.run_with_console(&mut console)
    }

    /// Run the prompt loop with a specific console.
    pub fn run_with_console(&self, console: &mut Console) -> Result<f64> {
        loop {
            let prompt_text = self.make_prompt();
            let value = console.input(&prompt_text, false)?;

            // Use default if input is empty
            if value.is_empty() {
                if let Some(default) = self.default {
                    return Ok(default);
                }
            }

            // Process the response
            match self.process_response(&value) {
                Ok(result) => return Ok(result),
                Err(err) => {
                    // Display error and continue loop
                    let error_text = Text::from_markup(&err.message, false)
                        .unwrap_or_else(|_| Text::plain(&err.message));
                    let _ = console.print(&error_text, None, None, None, false, "\n");
                }
            }
        }
    }

    /// Build the prompt text including default.
    fn make_prompt(&self) -> Text {
        let mut parts = vec![self.prompt.clone()];

        // Add default
        if self.show_default {
            if let Some(default) = self.default {
                parts.push(format!(" [prompt.default]({})[/]", default));
            }
        }

        // Add suffix
        parts.push(": ".to_string());

        let markup = parts.join("");
        Text::from_markup(&markup, false).unwrap_or_else(|_| Text::plain(&markup))
    }

    /// Process the response.
    fn process_response(&self, value: &str) -> std::result::Result<f64, InvalidResponse> {
        let value = value.trim();
        value
            .parse::<f64>()
            .map_err(|_| InvalidResponse::new("[prompt.invalid]Please enter a number"))
    }
}

// ============================================================================
// Confirm - Yes/No prompts
// ============================================================================

/// A yes/no confirmation prompt.
///
/// # Example
///
/// ```ignore
/// use rich_rs::prompt::Confirm;
///
/// if Confirm::ask("Continue?")? {
///     println!("Continuing...");
/// }
///
/// // With default
/// if Confirm::new("Continue?").with_default(true).ask()? {
///     println!("Continuing...");
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Confirm {
    /// The prompt text.
    prompt: String,
    /// Optional default value.
    default: Option<bool>,
    /// Whether to show the default value in the prompt.
    show_default: bool,
    /// The yes choice string.
    yes_choice: String,
    /// The no choice string.
    no_choice: String,
}

impl Default for Confirm {
    fn default() -> Self {
        Self::new("")
    }
}

impl Confirm {
    /// Create a new confirmation prompt with the given text.
    pub fn new(prompt: impl Into<String>) -> Self {
        Confirm {
            prompt: prompt.into(),
            default: None,
            show_default: true,
            yes_choice: "y".to_string(),
            no_choice: "n".to_string(),
        }
    }

    /// Set the default value.
    pub fn with_default(mut self, default: bool) -> Self {
        self.default = Some(default);
        self
    }

    /// Set whether to show the default value in the prompt (default: true).
    pub fn show_default(mut self, show: bool) -> Self {
        self.show_default = show;
        self
    }

    /// Set custom yes/no choices (default: "y"/"n").
    pub fn with_choices(mut self, yes: impl Into<String>, no: impl Into<String>) -> Self {
        self.yes_choice = yes.into();
        self.no_choice = no.into();
        self
    }

    /// Shortcut to create and run a prompt, returning the result.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if Confirm::ask("Continue?")? {
    ///     println!("Continuing...");
    /// }
    /// ```
    pub fn ask(prompt: impl Into<String>) -> Result<bool> {
        Confirm::new(prompt).run()
    }

    /// Run the prompt loop and return the result.
    pub fn run(&self) -> Result<bool> {
        let mut console = Console::new();
        self.run_with_console(&mut console)
    }

    /// Run the prompt loop with a specific console.
    pub fn run_with_console(&self, console: &mut Console) -> Result<bool> {
        loop {
            let prompt_text = self.make_prompt();
            let value = console.input(&prompt_text, false)?;

            // Use default if input is empty
            if value.is_empty() {
                if let Some(default) = self.default {
                    return Ok(default);
                }
            }

            // Process the response
            match self.process_response(&value) {
                Ok(result) => return Ok(result),
                Err(err) => {
                    // Display error and continue loop
                    let error_text = Text::from_markup(&err.message, false)
                        .unwrap_or_else(|_| Text::plain(&err.message));
                    let _ = console.print(&error_text, None, None, None, false, "\n");
                }
            }
        }
    }

    /// Build the prompt text including choices and default.
    fn make_prompt(&self) -> Text {
        let mut parts = vec![self.prompt.clone()];

        // Add choices
        let choices_str = format!("{}/{}", self.yes_choice, self.no_choice);
        parts.push(format!(" [prompt.choices]\\[{}][/]", choices_str));

        // Add default as y/n
        if self.show_default {
            if let Some(default) = self.default {
                let default_str = if default {
                    &self.yes_choice
                } else {
                    &self.no_choice
                };
                parts.push(format!(" [prompt.default]({})[/]", default_str));
            }
        }

        // Add suffix
        parts.push(": ".to_string());

        let markup = parts.join("");
        Text::from_markup(&markup, false).unwrap_or_else(|_| Text::plain(&markup))
    }

    /// Process the response.
    fn process_response(&self, value: &str) -> std::result::Result<bool, InvalidResponse> {
        let value = value.trim().to_lowercase();
        let yes_lower = self.yes_choice.to_lowercase();
        let no_lower = self.no_choice.to_lowercase();

        if value == yes_lower {
            Ok(true)
        } else if value == no_lower {
            Ok(false)
        } else {
            Err(InvalidResponse::new("[prompt.invalid]Please enter Y or N"))
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_new() {
        let prompt = Prompt::new("Enter name");
        assert_eq!(prompt.prompt, "Enter name");
        assert!(prompt.default.is_none());
        assert!(prompt.choices.is_none());
        assert!(prompt.case_sensitive);
        assert!(prompt.show_default);
        assert!(prompt.show_choices);
        assert!(!prompt.password);
    }

    #[test]
    fn test_prompt_with_options() {
        let prompt = Prompt::new("Choose")
            .with_default("Alice")
            .with_choices(&["Alice", "Bob", "Charlie"])
            .case_sensitive(false)
            .password(true);

        assert_eq!(prompt.default, Some("Alice".to_string()));
        assert_eq!(
            prompt.choices,
            Some(vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string()
            ])
        );
        assert!(!prompt.case_sensitive);
        assert!(prompt.password);
    }

    #[test]
    fn test_prompt_check_choice_case_sensitive() {
        let prompt = Prompt::new("Choose")
            .with_choices(&["Alice", "Bob"])
            .case_sensitive(true);

        assert!(prompt.check_choice("Alice"));
        assert!(prompt.check_choice("Bob"));
        assert!(!prompt.check_choice("alice"));
        assert!(!prompt.check_choice("Charlie"));
    }

    #[test]
    fn test_prompt_check_choice_case_insensitive() {
        let prompt = Prompt::new("Choose")
            .with_choices(&["Alice", "Bob"])
            .case_sensitive(false);

        assert!(prompt.check_choice("Alice"));
        assert!(prompt.check_choice("alice"));
        assert!(prompt.check_choice("ALICE"));
        assert!(!prompt.check_choice("Charlie"));
    }

    #[test]
    fn test_prompt_get_original_choice() {
        let prompt = Prompt::new("Choose")
            .with_choices(&["Alice", "Bob"])
            .case_sensitive(false);

        assert_eq!(prompt.get_original_choice("alice"), "Alice");
        assert_eq!(prompt.get_original_choice("ALICE"), "Alice");
        assert_eq!(prompt.get_original_choice("bob"), "Bob");
    }

    #[test]
    fn test_prompt_process_response_valid() {
        let prompt = Prompt::new("Enter name");
        let result = prompt.process_response("  John  ");
        assert_eq!(result.unwrap(), "John");
    }

    #[test]
    fn test_prompt_process_response_invalid_choice() {
        let prompt = Prompt::new("Choose").with_choices(&["Alice", "Bob"]);
        let result = prompt.process_response("Charlie");
        assert!(result.is_err());
    }

    #[test]
    fn test_int_prompt_new() {
        let prompt = IntPrompt::new("Enter number");
        assert_eq!(prompt.prompt, "Enter number");
        assert!(prompt.default.is_none());
        assert!(prompt.show_default);
    }

    #[test]
    fn test_int_prompt_with_default() {
        let prompt = IntPrompt::new("Enter number").with_default(42);
        assert_eq!(prompt.default, Some(42));
    }

    #[test]
    fn test_int_prompt_process_response_valid() {
        let prompt = IntPrompt::new("Enter number");
        assert_eq!(prompt.process_response("42").unwrap(), 42);
        assert_eq!(prompt.process_response("  -10  ").unwrap(), -10);
    }

    #[test]
    fn test_int_prompt_process_response_invalid() {
        let prompt = IntPrompt::new("Enter number");
        assert!(prompt.process_response("abc").is_err());
        assert!(prompt.process_response("3.14").is_err());
    }

    #[test]
    fn test_float_prompt_new() {
        let prompt = FloatPrompt::new("Enter number");
        assert_eq!(prompt.prompt, "Enter number");
        assert!(prompt.default.is_none());
    }

    #[test]
    fn test_float_prompt_with_default() {
        let prompt = FloatPrompt::new("Enter number").with_default(3.14);
        assert_eq!(prompt.default, Some(3.14));
    }

    #[test]
    fn test_float_prompt_process_response_valid() {
        let prompt = FloatPrompt::new("Enter number");
        assert!((prompt.process_response("3.14").unwrap() - 3.14).abs() < f64::EPSILON);
        assert!((prompt.process_response("  -2.5  ").unwrap() - (-2.5)).abs() < f64::EPSILON);
        assert!((prompt.process_response("42").unwrap() - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_float_prompt_process_response_invalid() {
        let prompt = FloatPrompt::new("Enter number");
        assert!(prompt.process_response("abc").is_err());
    }

    #[test]
    fn test_confirm_new() {
        let confirm = Confirm::new("Continue?");
        assert_eq!(confirm.prompt, "Continue?");
        assert!(confirm.default.is_none());
        assert_eq!(confirm.yes_choice, "y");
        assert_eq!(confirm.no_choice, "n");
    }

    #[test]
    fn test_confirm_with_default() {
        let confirm = Confirm::new("Continue?").with_default(true);
        assert_eq!(confirm.default, Some(true));
    }

    #[test]
    fn test_confirm_with_choices() {
        let confirm = Confirm::new("Continue?").with_choices("yes", "no");
        assert_eq!(confirm.yes_choice, "yes");
        assert_eq!(confirm.no_choice, "no");
    }

    #[test]
    fn test_confirm_process_response_yes() {
        let confirm = Confirm::new("Continue?");
        assert!(confirm.process_response("y").unwrap());
        assert!(confirm.process_response("Y").unwrap());
    }

    #[test]
    fn test_confirm_process_response_no() {
        let confirm = Confirm::new("Continue?");
        assert!(!confirm.process_response("n").unwrap());
        assert!(!confirm.process_response("N").unwrap());
    }

    #[test]
    fn test_confirm_process_response_invalid() {
        let confirm = Confirm::new("Continue?");
        assert!(confirm.process_response("x").is_err());
        assert!(confirm.process_response("yes").is_err()); // Default choices are y/n
    }

    #[test]
    fn test_confirm_custom_choices() {
        let confirm = Confirm::new("Continue?").with_choices("yes", "no");
        assert!(confirm.process_response("yes").unwrap());
        assert!(!confirm.process_response("no").unwrap());
        assert!(confirm.process_response("y").is_err()); // Custom choices don't include y
    }

    #[test]
    fn test_invalid_response() {
        let err = InvalidResponse::new("Test error");
        assert_eq!(err.message, "Test error");
        assert_eq!(format!("{}", err), "Test error");
    }

    #[test]
    fn test_prompt_error_display() {
        let io_err = PromptError::Io("test error".to_string());
        assert!(format!("{}", io_err).contains("test error"));

        let invalid_err = PromptError::InvalidResponse(InvalidResponse::new("invalid"));
        assert!(format!("{}", invalid_err).contains("invalid"));

        let interrupted = PromptError::Interrupted;
        assert!(format!("{}", interrupted).contains("interrupted"));
    }

    #[test]
    fn test_prompt_make_prompt() {
        let prompt = Prompt::new("Enter name")
            .with_default("John")
            .with_choices(&["John", "Jane"]);

        let text = prompt.make_prompt();
        let plain = text.plain_text();
        assert!(plain.contains("Enter name"));
        assert!(plain.contains(":")); // suffix
    }
}
