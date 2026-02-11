use serde::Deserialize;
use std::collections::HashMap;

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
    /// Force keyword-aware commands to use multiline layout regardless of line length (default: false)
    pub force_break_keywords: bool,

    /// Manual command grammar definitions
    /// Map of command name -> grammar definition
    #[serde(default)]
    pub command_grammars: HashMap<String, CommandGrammarConfig>,

    /// Source file grouping mode for .h/.cpp pairs (default: None)
    pub source_grouping: SourceGrouping,
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
            force_break_keywords: false,
            command_grammars: HashMap::new(),
            source_grouping: SourceGrouping::None,
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

/// Source file grouping mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceGrouping {
    /// No grouping (default) - each file on its own line
    None,
    /// Group header/source pairs on same line, headers listed first
    HeadersFirst,
    /// Group header/source pairs on same line, sources listed first
    SourcesFirst,
}

impl Default for SourceGrouping {
    fn default() -> Self {
        Self::None
    }
}

/// Grammar configuration for a custom command, as specified in config file
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct CommandGrammarConfig {
    /// Flag keywords (no values consumed)
    #[serde(default)]
    pub options: Vec<String>,
    /// Single-value keywords (consumes exactly one value)
    #[serde(default)]
    pub one_value_keywords: Vec<String>,
    /// Multi-value keywords (consumes multiple values until next keyword)
    #[serde(default)]
    pub multi_value_keywords: Vec<String>,
    /// Pair-value keywords (consumes alternating key/value pairs)
    #[serde(default)]
    pub pair_value_keywords: Vec<String>,
}
