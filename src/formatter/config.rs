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
    /// Command name casing transformation (default: Lowercase)
    pub command_case: CommandCase,
    /// Maximum consecutive blank lines allowed (default: 1)
    pub max_blank_lines: usize,
    /// Line ending style (default: Auto)
    pub line_ending: LineEnding,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            max_line_length: 80,
            indent_width: 4,
            use_tabs: true,
            command_case: CommandCase::Lowercase,
            max_blank_lines: 1,
            line_ending: LineEnding::Auto,
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
    /// Keep original casing
    Preserve,
}
