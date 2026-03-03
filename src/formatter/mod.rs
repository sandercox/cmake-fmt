mod builtins;
mod cmake_rules;
mod comments;
pub mod config;
mod cst_to_doc;
pub mod grammar;
pub mod line_ranges;
mod suppression;
mod user_commands;

pub use config::{
    ClosingStyle, CommandCase, CommandGrammarConfig, CommentStyle, FinalNewline, FormatConfig,
    LineEnding, SortSources, SourceGrouping, UserCommandCase,
};
pub use grammar::{
    CommandGrammar, GrammarFormat, GrammarRegistry, KeywordType, detect_grammar_format,
    export_command_grammars, export_command_grammars_to_toml, export_command_grammars_to_yaml,
    export_grammars, export_grammars_to_toml, export_grammars_to_yaml, import_grammar_file,
};
pub use line_ranges::{LineRange, format_with_line_ranges, parse_line_ranges};
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
/// * `verbose` - Show verbose output during file scanning (default: false)
///
/// # Returns
/// A tuple of (formatted_code, warnings) where formatted_code is guaranteed
/// to end with a single newline (when final_newline is true) and warnings
/// contains any suppression-related diagnostics
pub fn format_text_with_diagnostics_and_path(
    input: &str,
    config: &FormatConfig,
    file_path: Option<&Path>,
    verbose: bool,
) -> (String, Vec<SuppressionWarning>) {
    // Early return if formatting is disabled
    if config.disable_format {
        return (input.to_string(), Vec::new());
    }

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
        let mut merged = grammar::get_project_user_commands(path, verbose);
        // Single-file definitions override project-wide (local wins)
        merged.extend(single_file_defs);
        merged
    } else {
        // Stdin case: use only single-file definitions
        single_file_defs
    };

    // Get single-file grammars from cmake_parse_arguments in current file
    let single_file_grammars = grammar::user_scanner::extract_grammars_from_file(&cst.root);

    // Get project-wide user grammars if file_path is provided
    let user_grammars = if let Some(path) = file_path {
        let mut merged = grammar::get_project_user_grammars(path, verbose);
        // Single-file grammars override project-wide (local wins, matching user_defs behavior)
        merged.extend(single_file_grammars);
        merged
    } else {
        // Stdin case: use only single-file grammars
        single_file_grammars
    };

    // Convert config grammars and merge (config overrides auto-detected)
    let config_grammar_map = grammar::config_grammars_to_map(&config.command_grammars);
    // Start with auto-detected grammars, then override with config grammars
    let mut merged_grammars = user_grammars;
    merged_grammars.extend(config_grammar_map);

    // Load external grammar files (precedence: config > grammar_files > auto-detected)
    for grammar_path in &config.grammar_files {
        match std::fs::read_to_string(grammar_path) {
            Ok(content) => {
                match grammar::import_grammar_file(&content) {
                    Ok(imported) => {
                        // Imported grammars: insert only if not already present (config takes precedence)
                        for (name, cg) in imported {
                            merged_grammars.entry(name).or_insert(cg);
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to parse grammar file {}: {}",
                            grammar_path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to read grammar file {}: {}",
                    grammar_path.display(),
                    e
                );
            }
        }
    }

    let user_grammars = merged_grammars;

    let (mut result, warnings) =
        cst_to_doc::format_cst(&cst, config, parse_input, &user_defs, &user_grammars);

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
/// to end with a single newline (when final_newline is true) and warnings
/// contains any suppression-related diagnostics
pub fn format_text_with_diagnostics(
    input: &str,
    config: &FormatConfig,
) -> (String, Vec<SuppressionWarning>) {
    format_text_with_diagnostics_and_path(input, config, None, false)
}

/// Format CMake code with the given configuration
///
/// # Arguments
/// * `input` - The CMake source code to format
/// * `config` - Formatting configuration
///
/// # Returns
/// Formatted CMake code as a String, guaranteed to end with a single newline
/// (when final_newline is true)
pub fn format_text(input: &str, config: &FormatConfig) -> String {
    let (result, _warnings) = format_text_with_diagnostics(input, config);
    result
}

/// Post-process rendered output: strip trailing whitespace and normalize ending
pub(crate) fn post_process_rendered_output(
    result: &str,
    final_newline: config::FinalNewline,
    input_had_trailing_newline: bool,
) -> String {
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
    } else {
        match final_newline {
            config::FinalNewline::Force => {
                // Always ensure output ends with newline
                if trimmed.ends_with('\n') {
                    trimmed.to_string()
                } else {
                    format!("{}\n", trimmed)
                }
            }
            config::FinalNewline::Remove => {
                // Never append trailing newline
                trimmed.to_string()
            }
            config::FinalNewline::Preserve => {
                // Preserve input's trailing newline state
                if input_had_trailing_newline {
                    if trimmed.ends_with('\n') {
                        trimmed.to_string()
                    } else {
                        format!("{}\n", trimmed)
                    }
                } else {
                    // Input had no trailing newline, strip any added by trimming
                    trimmed.trim_end_matches('\n').to_string()
                }
            }
        }
    }
}
