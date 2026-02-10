use serde::Deserialize;

/// Configuration for CMake formatting
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct FormatConfig {
    /// Maximum line length before breaking (default: 80)
    pub max_line_length: usize,
    /// Number of spaces/tabs per indent level (default: 4)
    pub indent_width: usize,
    /// Use tabs instead of spaces for indentation (default: true)
    pub use_tabs: bool,
    /// Builtin command name casing transformation (default: Lowercase)
    pub command_case: CommandCase,
    /// User-defined command name casing (default: Infer)
    pub user_command_case: UserCommandCase,

    /// Maximum consecutive blank lines allowed (default: 1)
    pub max_blank_lines: usize,
    /// Line ending style (default: Auto)
    pub line_ending: LineEnding,
    /// Block closer argument handling (default: Remove)
    pub closing_style: ClosingStyle,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            max_line_length: 80,
            indent_width: 4,
            use_tabs: true,
            command_case: CommandCase::Lowercase,
            user_command_case: UserCommandCase::Infer,
            max_blank_lines: 1,
            line_ending: LineEnding::Auto,
            closing_style: ClosingStyle::Remove,
        }
    }
}

/// Line ending style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineEnding {
    /// Detect from input (majority wins, default to LF)
    Auto,
    /// Always use LF (\n)
    Lf,
    /// Always use CRLF (\r\n)
    #[serde(rename = "crlf")]
    CrLf,
}

/// Command name casing options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandCase {
    /// Convert to lowercase (e.g., "set")
    Lowercase,
    /// Convert to uppercase (e.g., "SET")
    Uppercase,
    /// Leave original casing unchanged
    Leave,
}

/// User-defined command name casing options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserCommandCase {
    /// Convert to lowercase
    Lowercase,
    /// Convert to uppercase
    Uppercase,
    /// Leave original casing unchanged
    Leave,
    /// Infer casing from function()/macro() definitions; if not found, leave as-is
    Infer,
}

/// Block closer argument handling options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClosingStyle {
    /// Leave arguments as written in input
    Leave,
    /// Remove arguments from closers (default, modernize)
    Remove,
    /// Add arguments to match openers (enforce explicit)
    Force,
}

impl Default for ClosingStyle {
    fn default() -> Self {
        Self::Remove
    }
}
