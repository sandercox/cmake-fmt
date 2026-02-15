use serde::Deserialize;
use std::collections::HashMap;

/// Configuration for CMake formatting
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct FormatConfig {
    /// Maximum line length before breaking (default: 80, 0 = unlimited)
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
    /// Ensure file ends with a newline (default: true)
    pub final_newline: bool,

    /// Manual command grammar definitions
    /// Map of command name -> grammar definition
    #[serde(default)]
    pub command_grammars: HashMap<String, CommandGrammarConfig>,

    /// External grammar files to import (CLI or config)
    #[serde(default)]
    pub grammar_files: Vec<std::path::PathBuf>,

    /// Source file grouping mode for .h/.cpp pairs (default: None)
    pub source_grouping: SourceGrouping,
    /// Source file sorting mode (default: None)
    pub sort_sources: SortSources,
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
            final_newline: true,
            command_grammars: HashMap::new(),
            grammar_files: Vec::new(),
            source_grouping: SourceGrouping::None,
            sort_sources: SortSources::None,
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

impl FormatConfig {
    /// Apply a single style override by key=value.
    /// Returns Ok(()) on success, Err(warning_message) for invalid key/value.
    pub fn apply_override(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "indent_width" => {
                match value.parse::<usize>() {
                    Ok(v) => {
                        self.indent_width = v;
                        Ok(())
                    }
                    Err(_) => Err(format!("Invalid value for indent_width: {}", value)),
                }
            }
            "max_line_length" => {
                match value.parse::<usize>() {
                    Ok(v) => {
                        self.max_line_length = v;
                        Ok(())
                    }
                    Err(_) => Err(format!("Invalid value for max_line_length: {}", value)),
                }
            }
            "use_tabs" => {
                match value.parse::<bool>() {
                    Ok(v) => {
                        self.use_tabs = v;
                        Ok(())
                    }
                    Err(_) => Err(format!("Invalid value for use_tabs: {}", value)),
                }
            }
            "command_case" => {
                match value {
                    "lowercase" => {
                        self.command_case = CommandCase::Lowercase;
                        Ok(())
                    }
                    "uppercase" => {
                        self.command_case = CommandCase::Uppercase;
                        Ok(())
                    }
                    "leave" => {
                        self.command_case = CommandCase::Leave;
                        Ok(())
                    }
                    _ => Err(format!(
                        "Invalid value for command_case (expected lowercase, uppercase, or leave): {}",
                        value
                    )),
                }
            }
            "max_blank_lines" => {
                match value.parse::<usize>() {
                    Ok(v) => {
                        self.max_blank_lines = v;
                        Ok(())
                    }
                    Err(_) => Err(format!("Invalid value for max_blank_lines: {}", value)),
                }
            }
            "line_ending" => {
                match value {
                    "auto" => {
                        self.line_ending = LineEnding::Auto;
                        Ok(())
                    }
                    "lf" => {
                        self.line_ending = LineEnding::Lf;
                        Ok(())
                    }
                    "crlf" => {
                        self.line_ending = LineEnding::CrLf;
                        Ok(())
                    }
                    _ => Err(format!(
                        "Invalid value for line_ending (expected auto, lf, or crlf): {}",
                        value
                    )),
                }
            }
            "user_command_case" => {
                match value {
                    "lowercase" => {
                        self.user_command_case = UserCommandCase::Lowercase;
                        Ok(())
                    }
                    "uppercase" => {
                        self.user_command_case = UserCommandCase::Uppercase;
                        Ok(())
                    }
                    "leave" => {
                        self.user_command_case = UserCommandCase::Leave;
                        Ok(())
                    }
                    "infer" => {
                        self.user_command_case = UserCommandCase::Infer;
                        Ok(())
                    }
                    _ => Err(format!(
                        "Invalid value for user_command_case (expected lowercase, uppercase, leave, or infer): {}",
                        value
                    )),
                }
            }
            "closing_style" => {
                match value {
                    "leave" => {
                        self.closing_style = ClosingStyle::Leave;
                        Ok(())
                    }
                    "remove" => {
                        self.closing_style = ClosingStyle::Remove;
                        Ok(())
                    }
                    "force" => {
                        self.closing_style = ClosingStyle::Force;
                        Ok(())
                    }
                    _ => Err(format!(
                        "Invalid value for closing_style (expected leave, remove, or force): {}",
                        value
                    )),
                }
            }
            "source_grouping" => {
                match value {
                    "none" => {
                        self.source_grouping = SourceGrouping::None;
                        Ok(())
                    }
                    "headers_first" => {
                        self.source_grouping = SourceGrouping::HeadersFirst;
                        Ok(())
                    }
                    "sources_first" => {
                        self.source_grouping = SourceGrouping::SourcesFirst;
                        Ok(())
                    }
                    _ => Err(format!(
                        "Invalid value for source_grouping (expected none, headers_first, or sources_first): {}",
                        value
                    )),
                }
            }
            "sort_sources" => {
                match value {
                    "none" => {
                        self.sort_sources = SortSources::None;
                        Ok(())
                    }
                    "alphabetical" => {
                        self.sort_sources = SortSources::Alphabetical;
                        Ok(())
                    }
                    _ => Err(format!(
                        "Invalid value for sort_sources (expected none or alphabetical): {}",
                        value
                    )),
                }
            }
            "force_break_keywords" => {
                match value.parse::<bool>() {
                    Ok(v) => {
                        self.force_break_keywords = v;
                        Ok(())
                    }
                    Err(_) => Err(format!("Invalid value for force_break_keywords: {}", value)),
                }
            }
            "final_newline" => {
                match value.parse::<bool>() {
                    Ok(v) => {
                        self.final_newline = v;
                        Ok(())
                    }
                    Err(_) => Err(format!("Invalid value for final_newline: {}", value)),
                }
            }
            _ => Err(format!("Unknown config key: {}", key)),
        }
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

/// Source file sorting mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortSources {
    /// No sorting (default) - preserve original order
    None,
    /// Sort filenames alphabetically (case-insensitive)
    Alphabetical,
}

impl Default for SortSources {
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
    /// Bin-pack keywords (packs values to fill lines)
    #[serde(default)]
    pub bin_pack_keywords: Vec<String>,
}
