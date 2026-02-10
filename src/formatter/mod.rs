pub mod config;
mod builtins;
pub mod grammar;
mod cst_to_doc;
mod cmake_rules;
mod comments;
mod suppression;
mod user_commands;

pub use config::{ClosingStyle, CommandCase, FormatConfig, LineEnding, UserCommandCase};
pub use grammar::{CommandGrammar, GrammarRegistry, KeywordType};
pub use suppression::SuppressionWarning;

use crate::cst::parse_text;
use std::path::Path;

/// Detect the line ending style used in the input text.
///
/// Counts CRLF (`\r\n`) vs lone LF (`\n`) occurrences. The majority wins.
/// Returns `LineEnding::Lf` if there are no newlines.
pub fn detect_line_ending(input: &str) -> LineEnding {
    let crlf_count = input.matches("\r\n").count();
    // Lone LF = total LF minus those that are part of CRLF
    let total_lf = input.matches('\n').count();
    let lone_lf_count = total_lf - crlf_count;

    if crlf_count == 0 && lone_lf_count == 0 {
        LineEnding::Lf
    } else if crlf_count > lone_lf_count {
        LineEnding::CrLf
    } else {
        LineEnding::Lf
    }
}

/// Format CMake code with the given configuration and return diagnostics
///
/// # Arguments
/// * `input` - The CMake source code to format
/// * `config` - Formatting configuration
/// * `file_path` - Optional file path for project-wide user command scanning
///
/// # Returns
/// A tuple of (formatted_code, warnings) where formatted_code is guaranteed
/// to end with a single newline and warnings contains any suppression-related
/// diagnostics
pub fn format_text_with_diagnostics_and_path(
    input: &str,
    config: &FormatConfig,
    file_path: Option<&Path>,
) -> (String, Vec<SuppressionWarning>) {
    // Resolve effective line ending
    let effective_line_ending = match config.line_ending {
        LineEnding::Auto => detect_line_ending(input),
        other => other,
    };

    // Normalize input: strip \r so the parser/formatter sees only \n
    let normalized;
    let parse_input = if input.contains('\r') {
        normalized = input.replace('\r', "");
        &normalized
    } else {
        input
    };

    let cst = parse_text(parse_input);

    // Get single-file user definitions
    let single_file_defs = user_commands::scan_user_command_definitions(&cst.root);

    // Merge with project-wide definitions if file_path is provided
    let user_defs = if let Some(path) = file_path {
        let mut merged = grammar::get_project_user_commands(path);
        // Single-file definitions override project-wide (local wins)
        merged.extend(single_file_defs);
        merged
    } else {
        // Stdin case: use only single-file definitions
        single_file_defs
    };

    // Get project-wide user grammars if file_path is provided
    let user_grammars = if let Some(path) = file_path {
        grammar::get_project_user_grammars(path)
    } else {
        std::collections::HashMap::new()
    };

    let (mut result, warnings) = cst_to_doc::format_cst(&cst, config, parse_input, &user_defs, &user_grammars);

    // Apply CRLF if needed
    if effective_line_ending == LineEnding::CrLf && !result.is_empty() {
        result = result.replace('\n', "\r\n");
    }

    (result, warnings)
}

/// Format CMake code with the given configuration and return diagnostics
///
/// # Arguments
/// * `input` - The CMake source code to format
/// * `config` - Formatting configuration
///
/// # Returns
/// A tuple of (formatted_code, warnings) where formatted_code is guaranteed
/// to end with a single newline and warnings contains any suppression-related
/// diagnostics
pub fn format_text_with_diagnostics(input: &str, config: &FormatConfig) -> (String, Vec<SuppressionWarning>) {
    format_text_with_diagnostics_and_path(input, config, None)
}

/// Format CMake code with the given configuration
///
/// # Arguments
/// * `input` - The CMake source code to format
/// * `config` - Formatting configuration
///
/// # Returns
/// Formatted CMake code as a String, guaranteed to end with a single newline
pub fn format_text(input: &str, config: &FormatConfig) -> String {
    let (result, _warnings) = format_text_with_diagnostics(input, config);
    result
}

/// Post-process rendered output: strip trailing whitespace and normalize ending
pub(crate) fn post_process_rendered_output(result: &str) -> String {
    // Strip trailing whitespace from each line (the pretty crate can produce
    // indentation on otherwise-blank lines when nest() wraps line() breaks)
    let result: String = result
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");

    // Trim the result and ensure proper ending
    let trimmed = result.trim();

    // If nothing meaningful, return empty
    if trimmed.is_empty() {
        String::new()
    } else if !trimmed.ends_with('\n') {
        format!("{}\n", trimmed)
    } else {
        trimmed.to_string()
    }
}
