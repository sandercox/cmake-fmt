use serde::Deserialize;

/// Configuration for CMake formatting
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct FormatConfig {
    /// Maximum line length before breaking (default: 80)
    pub max_line_length: usize,
    /// Number of spaces/tabs per indent level (default: 2)
    pub indent_width: usize,
    /// Use tabs instead of spaces for indentation (default: false)
    pub use_tabs: bool,
    /// Command name casing transformation (default: Lowercase)
    pub command_case: CommandCase,
    /// Maximum consecutive blank lines allowed (default: 1)
    pub max_blank_lines: usize,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            max_line_length: 80,
            indent_width: 2,
            use_tabs: false,
            command_case: CommandCase::Lowercase,
            max_blank_lines: 1,
        }
    }
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
