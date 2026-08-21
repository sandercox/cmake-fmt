use serde::Deserialize;
use std::collections::HashMap;

/// Final newline handling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FinalNewline {
    /// Preserve original file's trailing newline state (default)
    #[default]
    Preserve,
    /// Strip trailing newline from output
    Remove,
    /// Ensure output ends with trailing newline
    Force,
}

impl<'de> Deserialize<'de> for FinalNewline {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct FinalNewlineVisitor;

        impl<'de> serde::de::Visitor<'de> for FinalNewlineVisitor {
            type Value = FinalNewline;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a boolean (true/false) or string (\"preserve\", \"leave\", \"remove\", \"force\")")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v {
                    Ok(FinalNewline::Force)
                } else {
                    Ok(FinalNewline::Remove)
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    "preserve" | "leave" => Ok(FinalNewline::Preserve),
                    "remove" => Ok(FinalNewline::Remove),
                    "force" => Ok(FinalNewline::Force),
                    other => Err(serde::de::Error::unknown_variant(
                        other,
                        &["preserve", "leave", "remove", "force"],
                    )),
                }
            }
        }

        deserializer.deserialize_any(FinalNewlineVisitor)
    }
}

/// Configuration for CMake formatting
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct FormatConfig {
    /// Completely skip formatting and return input unchanged (default: false)
    pub disable_format: bool,
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
    /// Final newline handling (default: Preserve)
    pub final_newline: FinalNewline,
    /// Comment whitespace normalization style (default: HashSpace)
    pub comment_style: CommentStyle,

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

    /// Collapse consecutive no-argument flags onto the same line (default: true)
    /// When true: `add_library(mylib STATIC ...)` keeps STATIC inline with target name
    /// When false: flags get their own indented line like other keywords
    pub collapse_empty_flags: bool,

    /// When true and a keyword command has only one keyword section, keep the keyword inline with args (default: false)
    /// E.g., `target_sources(mylib PUBLIC\n    src/a.cpp\n)` instead of `target_sources(mylib\n    PUBLIC\n        src/a.cpp\n)`
    pub inline_single_keyword: bool,

    /// Insert a space before '(' in control flow / block statements (default: false)
    pub control_flow_space_before_paren: bool,

    /// Insert a space inside command parentheses: `set( VAR value )` (default: false)
    pub space_between_command_parens: bool,

    /// Indent the closing paren by one level in multiline commands (default: false)
    pub indent_closing_paren: bool,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            disable_format: false,
            max_line_length: 80,
            indent_width: 4,
            use_tabs: true,
            command_case: CommandCase::Lowercase,
            user_command_case: UserCommandCase::Infer,
            max_blank_lines: 1,
            line_ending: LineEnding::Auto,
            closing_style: ClosingStyle::Remove,
            force_break_keywords: false,
            final_newline: FinalNewline::Preserve,
            comment_style: CommentStyle::HashSpace,
            command_grammars: HashMap::new(),
            grammar_files: Vec::new(),
            source_grouping: SourceGrouping::None,
            sort_sources: SortSources::None,
            collapse_empty_flags: true,
            inline_single_keyword: false,
            control_flow_space_before_paren: false,
            space_between_command_parens: false,
            indent_closing_paren: false,
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
    /// Preserve original casing unchanged
    #[serde(rename = "preserve", alias = "leave")]
    Preserve,
}

/// User-defined command name casing options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserCommandCase {
    /// Convert to lowercase
    Lowercase,
    /// Convert to uppercase
    Uppercase,
    /// Preserve original casing unchanged
    #[serde(rename = "preserve", alias = "leave")]
    Preserve,
    /// Infer casing from function()/macro() definitions; if not found, leave as-is
    Infer,
}

/// Block closer argument handling options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ClosingStyle {
    /// Preserve arguments as written in input
    #[serde(rename = "preserve", alias = "leave")]
    Preserve,
    /// Remove arguments from closers (default, modernize)
    #[default]
    Remove,
    /// Add arguments to match openers (enforce explicit)
    Force,
}

impl FormatConfig {
    /// Apply a single style override by key=value.
    /// Returns Ok(()) on success, Err(warning_message) for invalid key/value.
    pub fn apply_override(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "indent_width" => match value.parse::<usize>() {
                Ok(v) => {
                    self.indent_width = v;
                    Ok(())
                }
                Err(_) => Err(format!("Invalid value for indent_width: {}", value)),
            },
            "max_line_length" => match value.parse::<usize>() {
                Ok(v) => {
                    self.max_line_length = v;
                    Ok(())
                }
                Err(_) => Err(format!("Invalid value for max_line_length: {}", value)),
            },
            "use_tabs" => match value.parse::<bool>() {
                Ok(v) => {
                    self.use_tabs = v;
                    Ok(())
                }
                Err(_) => Err(format!("Invalid value for use_tabs: {}", value)),
            },
            "command_case" => match value {
                "lowercase" => {
                    self.command_case = CommandCase::Lowercase;
                    Ok(())
                }
                "uppercase" => {
                    self.command_case = CommandCase::Uppercase;
                    Ok(())
                }
                "preserve" | "leave" => {
                    self.command_case = CommandCase::Preserve;
                    Ok(())
                }
                _ => Err(format!(
                    "Invalid value for command_case (expected lowercase, uppercase, or preserve): {}",
                    value
                )),
            },
            "max_blank_lines" => match value.parse::<usize>() {
                Ok(v) => {
                    self.max_blank_lines = v;
                    Ok(())
                }
                Err(_) => Err(format!("Invalid value for max_blank_lines: {}", value)),
            },
            "line_ending" => match value {
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
            },
            "user_command_case" => match value {
                "lowercase" => {
                    self.user_command_case = UserCommandCase::Lowercase;
                    Ok(())
                }
                "uppercase" => {
                    self.user_command_case = UserCommandCase::Uppercase;
                    Ok(())
                }
                "preserve" | "leave" => {
                    self.user_command_case = UserCommandCase::Preserve;
                    Ok(())
                }
                "infer" => {
                    self.user_command_case = UserCommandCase::Infer;
                    Ok(())
                }
                _ => Err(format!(
                    "Invalid value for user_command_case (expected lowercase, uppercase, preserve, or infer): {}",
                    value
                )),
            },
            "closing_style" => match value {
                "preserve" | "leave" => {
                    self.closing_style = ClosingStyle::Preserve;
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
                    "Invalid value for closing_style (expected preserve, remove, or force): {}",
                    value
                )),
            },
            "source_grouping" => match value {
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
            },
            "sort_sources" => match value {
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
            },
            "disable_format" => match value.parse::<bool>() {
                Ok(v) => {
                    self.disable_format = v;
                    Ok(())
                }
                Err(_) => Err(format!("Invalid value for disable_format: {}", value)),
            },
            "force_break_keywords" => match value.parse::<bool>() {
                Ok(v) => {
                    self.force_break_keywords = v;
                    Ok(())
                }
                Err(_) => Err(format!("Invalid value for force_break_keywords: {}", value)),
            },
            "final_newline" => match value {
                "preserve" | "leave" => {
                    self.final_newline = FinalNewline::Preserve;
                    Ok(())
                }
                "remove" | "false" => {
                    self.final_newline = FinalNewline::Remove;
                    Ok(())
                }
                "force" | "true" => {
                    self.final_newline = FinalNewline::Force;
                    Ok(())
                }
                _ => Err(format!(
                    "Invalid value for final_newline (expected preserve, remove, or force): {}",
                    value
                )),
            },
            "comment_style" => match value {
                "preserve" | "leave" => {
                    self.comment_style = CommentStyle::Preserve;
                    Ok(())
                }
                "hash_space" => {
                    self.comment_style = CommentStyle::HashSpace;
                    Ok(())
                }
                "hash_no_space" => {
                    self.comment_style = CommentStyle::HashNoSpace;
                    Ok(())
                }
                _ => Err(format!(
                    "Invalid value for comment_style (expected preserve, hash_space, or hash_no_space): {}",
                    value
                )),
            },
            "collapse_empty_flags" => match value.parse::<bool>() {
                Ok(v) => {
                    self.collapse_empty_flags = v;
                    Ok(())
                }
                Err(_) => Err(format!("Invalid value for collapse_empty_flags: {}", value)),
            },
            "inline_single_keyword" => match value.parse::<bool>() {
                Ok(v) => {
                    self.inline_single_keyword = v;
                    Ok(())
                }
                Err(_) => Err(format!(
                    "Invalid value for inline_single_keyword: {}",
                    value
                )),
            },
            "control_flow_space_before_paren" => match value.parse::<bool>() {
                Ok(v) => {
                    self.control_flow_space_before_paren = v;
                    Ok(())
                }
                Err(_) => Err(format!(
                    "Invalid value for control_flow_space_before_paren: {}",
                    value
                )),
            },
            "space_between_command_parens" => match value.parse::<bool>() {
                Ok(v) => {
                    self.space_between_command_parens = v;
                    Ok(())
                }
                Err(_) => Err(format!(
                    "Invalid value for space_between_command_parens: {}",
                    value
                )),
            },
            "indent_closing_paren" => match value.parse::<bool>() {
                Ok(v) => {
                    self.indent_closing_paren = v;
                    Ok(())
                }
                Err(_) => Err(format!("Invalid value for indent_closing_paren: {}", value)),
            },
            _ => Err(format!("Unknown config key: {}", key)),
        }
    }
}

/// Source file grouping mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SourceGrouping {
    /// No grouping (default) - each file on its own line
    #[default]
    None,
    /// Group header/source pairs on same line, headers listed first
    HeadersFirst,
    /// Group header/source pairs on same line, sources listed first
    SourcesFirst,
}

/// Source file sorting mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SortSources {
    /// No sorting (default) - preserve original order
    #[default]
    None,
    /// Sort filenames alphabetically (case-insensitive)
    Alphabetical,
}

/// Comment whitespace normalization style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum CommentStyle {
    /// Preserve original whitespace after # unchanged
    #[serde(rename = "preserve", alias = "leave")]
    Preserve,
    /// Normalize to "# text" - single space after hash (default, backward compat)
    #[default]
    HashSpace,
    /// Normalize to "#text" - no space after hash
    HashNoSpace,
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
    /// Keywords whose values are an unordered list, so `sort_sources` and
    /// `source_grouping` may reorder them (e.g. SOURCES for a wrapper command).
    /// Reordering is opt-in: keywords not listed here are left alone.
    #[serde(default)]
    pub sortable_keywords: Vec<String>,
    /// True when the command's keyword-less arguments are an unordered list,
    /// as in `set(VAR a.cpp b.cpp)`. The first argument is always pinned.
    #[serde(default)]
    pub sortable_positional: bool,
}
