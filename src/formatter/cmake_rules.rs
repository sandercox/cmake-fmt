use crate::cst::ArgumentList;
use crate::syntax_kind::SyntaxKind;
use pretty::RcDoc;
use rowan::NodeOrToken;
use std::collections::HashMap;

use super::config::FormatConfig;
use super::cst_to_doc::detect_argument_formatting_signals;
use super::grammar::{CommandGrammar, KeywordType};

/// Recognized header extensions
const HEADER_EXTS: &[&str] = &["h", "hh", "hpp", "hxx", "h++", "H"];
/// Recognized source extensions
const SOURCE_EXTS: &[&str] = &["c", "cc", "cpp", "cxx", "c++", "C", "m", "mm"];

/// Check if a filename has a header extension
fn is_header_file(name: &str) -> bool {
    name.rsplit('.').next().map_or(false, |ext| HEADER_EXTS.contains(&ext))
}

/// Check if a filename has a source extension
fn is_source_file(name: &str) -> bool {
    name.rsplit('.').next().map_or(false, |ext| SOURCE_EXTS.contains(&ext))
}

/// Extract the base name (without extension) from a file path
fn base_name(name: &str) -> Option<&str> {
    // Handle paths: take the last component, then strip extension
    let filename = name.rsplit('/').next().unwrap_or(name);
    let filename = filename.rsplit('\\').next().unwrap_or(filename);
    filename.rfind('.').map(|pos| &filename[..pos])
}

/// Group source files into pairs based on matching base names
///
/// Takes a list of file arguments and returns a new list where matching
/// header/source pairs are placed adjacent to each other and joined as a
/// single string (space-separated) so they render on the same line.
///
/// Arguments that are not source/header files pass through unchanged.
/// Files without a matching pair pass through unchanged.
pub fn group_source_pairs(
    args: &[String],
    grouping: super::config::SourceGrouping,
) -> Vec<String> {
    use super::config::SourceGrouping;

    if grouping == SourceGrouping::None {
        return args.to_vec();
    }

    // Build index: base_name -> (header_indices, source_indices)
    let mut base_map: HashMap<String, (Vec<usize>, Vec<usize>)> = HashMap::new();
    let mut is_paired = vec![false; args.len()];

    for (i, arg) in args.iter().enumerate() {
        if let Some(base) = base_name(arg) {
            let base_lower = base.to_lowercase();
            let entry = base_map.entry(base_lower).or_insert_with(|| (vec![], vec![]));
            if is_header_file(arg) {
                entry.0.push(i);
            } else if is_source_file(arg) {
                entry.1.push(i);
            }
        }
    }

    // Identify pairs (greedily match first header with first source)
    let mut pairs: Vec<(usize, usize)> = Vec::new(); // (first_idx, second_idx) in desired order
    for (_base, (headers, sources)) in &base_map {
        let pair_count = headers.len().min(sources.len());
        for k in 0..pair_count {
            let (first, second) = match grouping {
                SourceGrouping::HeadersFirst => (headers[k], sources[k]),
                SourceGrouping::SourcesFirst => (sources[k], headers[k]),
                SourceGrouping::None => unreachable!(),
            };
            pairs.push((first, second));
            is_paired[headers[k]] = true;
            is_paired[sources[k]] = true;
        }
    }

    // Build result: emit paired files together, unpaired files unchanged
    // Maintain original order: when we encounter the first file of a pair,
    // emit the pair as a single space-joined string. Skip the second file.
    let pair_lookup: HashMap<usize, usize> = pairs.iter()
        .flat_map(|&(a, b)| vec![(a, b), (b, a)])
        .collect();
    let mut emitted = vec![false; args.len()];
    let mut result = Vec::new();

    for (i, arg) in args.iter().enumerate() {
        if emitted[i] {
            continue;
        }
        if is_paired[i] {
            if let Some(&partner) = pair_lookup.get(&i) {
                // Determine which goes first based on grouping mode
                let (first_idx, second_idx) = if pairs.iter().any(|&(a, _)| a == i) {
                    (i, partner)
                } else {
                    (partner, i)
                };
                // Emit pair as single space-joined string (renders as one unit on one line)
                result.push(format!("{} {}", args[first_idx], args[second_idx]));
                emitted[first_idx] = true;
                emitted[second_idx] = true;
            } else {
                result.push(arg.clone());
                emitted[i] = true;
            }
        } else {
            result.push(arg.clone());
            emitted[i] = true;
        }
    }

    result
}

/// Check if a command name requires keyword-aware formatting
pub fn is_keyword_aware_command(name: &str) -> bool {
    use super::grammar::GrammarRegistry;
    GrammarRegistry::global().get(&name.to_lowercase()).is_some()
}

/// Check if a text token is a CMake keyword (case-sensitive)
pub fn is_cmake_keyword(text: &str) -> bool {
    matches!(
        text,
        "PUBLIC"
            | "PRIVATE"
            | "INTERFACE"
            | "IMPORTED"
            | "ALIAS"
            | "OBJECT"
            | "STATIC"
            | "SHARED"
            | "MODULE"
            | "PROPERTIES"
            | "COMMAND"
            | "DEPENDS"
            | "OUTPUT"
            | "WORKING_DIRECTORY"
            | "ARCHIVE"
            | "LIBRARY"
            | "RUNTIME"
            | "DESTINATION"
            | "DIRECTORY"
            | "TARGETS"
            | "FILES"
            | "PROGRAMS"
            | "REQUIRED"
            | "COMPONENTS"
            | "OPTIONAL_COMPONENTS"
            | "CONFIG"
            | "SOURCES"
            | "COMPILE_OPTIONS"
            | "COMPILE_DEFINITIONS"
            | "INCLUDE_DIRECTORIES"
            | "LINK_LIBRARIES"
            | "VERSION"
            | "LANGUAGES"
            | "FATAL_ERROR"
            | "SEND_ERROR"
            | "WARNING"
            | "AUTHOR_WARNING"
            | "STATUS"
            | "VERBOSE"
            | "DEBUG"
            | "BOOL"
            | "FILEPATH"
            | "PATH"
            | "STRING"
            | "INTERNAL"
            | "APPEND"
            | "APPEND_STRING"
            | "REMOVE_ITEM"
            | "REMOVE_DUPLICATES"
            | "SORT"
            | "CACHE"
            | "PARENT_SCOPE"
            | "FORCE"
    )
}

/// A section of arguments grouped by a keyword
#[derive(Debug, Clone)]
pub struct KeywordSection {
    /// The keyword (e.g., "PUBLIC"), or None for args before any keyword
    pub keyword: Option<String>,
    /// Arguments belonging to this section
    pub args: Vec<String>,
    /// Comments with their positions: (position_after_arg_index, comment_text)
    /// position_after_arg_index = 0 means before first arg, 1 means after first arg, etc.
    pub comments: Vec<(usize, String)>,
    /// Blank line positions: indices after which a blank line appears
    pub blank_lines: Vec<usize>,
    /// The type of the keyword (if known from grammar)
    pub keyword_type: Option<KeywordType>,
    /// Whether a newline appeared between the keyword and its first value
    /// (i.e., values were written on separate lines from the keyword)
    pub values_on_new_line: bool,
}

/// Parse an argument list into keyword sections with optional grammar guidance
pub fn parse_keyword_sections_with_grammar(
    arg_list: &ArgumentList,
    grammar: Option<&CommandGrammar>
) -> Vec<KeywordSection> {
    let mut sections = Vec::new();
    let mut current_section = KeywordSection {
        keyword: None,
        args: Vec::new(),
        comments: Vec::new(),
        blank_lines: Vec::new(),
        keyword_type: None,
        values_on_new_line: false,
    };

    let mut consecutive_newlines = 0;
    let mut saw_separator = true; // tracks whitespace for adjacent token merging
    let mut saw_newline_since_keyword = false; // tracks newlines between keyword and first value

    // Iterate through all tokens in the argument list
    for child in arg_list.syntax().children_with_tokens() {
        if let NodeOrToken::Token(token) = child {
            let text = token.text().to_string();

            match token.kind() {
                // Argument tokens
                SyntaxKind::UNQUOTED_ARGUMENT
                | SyntaxKind::QUOTED_ARGUMENT
                | SyntaxKind::BRACKET_ARGUMENT
                | SyntaxKind::VARIABLE_REF
                | SyntaxKind::ENV_VAR_REF
                | SyntaxKind::CACHE_VAR_REF
                | SyntaxKind::GENERATOR_EXPR => {
                    // Reset newline counter when we see an argument
                    consecutive_newlines = 0;

                    // Check if this argument is a keyword
                    let is_kw = if let Some(g) = grammar {
                        // Use grammar to determine if this is a keyword
                        g.keyword_type(&text).is_some()
                    } else {
                        // Fall back to hardcoded keyword check
                        is_cmake_keyword(&text)
                    };

                    if is_kw {
                        // Get the keyword type from grammar if available
                        let kw_type = grammar.and_then(|g| g.keyword_type(&text));

                        // Start a new section
                        if !current_section.args.is_empty() || current_section.keyword.is_some() {
                            sections.push(current_section);
                        }
                        current_section = KeywordSection {
                            keyword: Some(text),
                            args: Vec::new(),
                            comments: Vec::new(),
                            blank_lines: Vec::new(),
                            keyword_type: kw_type,
                            values_on_new_line: false,
                        };
                        saw_separator = true;
                        saw_newline_since_keyword = false;
                    } else if !saw_separator && !current_section.args.is_empty() {
                        // Adjacent to previous token (no whitespace) — merge
                        // e.g. ${VAR}/suffix is two tokens but one logical argument
                        current_section.args.last_mut().unwrap().push_str(&text);
                        saw_separator = false;
                    } else {
                        // Track if first value is on a new line from its keyword
                        if current_section.args.is_empty() && current_section.keyword.is_some() && saw_newline_since_keyword {
                            current_section.values_on_new_line = true;
                        }
                        // Add as argument to current section
                        current_section.args.push(text);
                        saw_separator = false;
                    }
                }
                // Track comments
                SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
                    consecutive_newlines = 0;
                    saw_separator = true;
                    // Position is after the current arg count
                    let position = current_section.args.len();
                    current_section.comments.push((position, text));
                }
                // Track newlines for blank line detection
                SyntaxKind::NEWLINE => {
                    saw_separator = true;
                    saw_newline_since_keyword = true;
                    consecutive_newlines += 1;
                    if consecutive_newlines >= 2 {
                        // Blank line detected - record position after last arg
                        let position = current_section.args.len();
                        if !current_section.blank_lines.contains(&position) {
                            current_section.blank_lines.push(position);
                        }
                    }
                }
                // Whitespace doesn't reset newline counter but marks separation
                SyntaxKind::WHITESPACE => {
                    saw_separator = true;
                }
                _ => {
                    saw_separator = true;
                    consecutive_newlines = 0;
                }
            }
        }
    }

    // Push the last section if it has content
    if !current_section.args.is_empty() || current_section.keyword.is_some() {
        sections.push(current_section);
    }

    sections
}

/// Parse an argument list into keyword sections (backward compatibility wrapper)
#[allow(dead_code)]
pub fn parse_keyword_sections(arg_list: &ArgumentList) -> Vec<KeywordSection> {
    parse_keyword_sections_with_grammar(arg_list, None)
}

/// Format arguments for a keyword-aware command
pub fn format_keyword_aware_args(
    arg_list: &ArgumentList,
    sections: Vec<KeywordSection>,
    config: &FormatConfig,
    indent_level: usize,
) -> RcDoc<'static, ()> {
    if sections.is_empty() {
        return RcDoc::nil();
    }

    // Detect formatting signals from the input (same as non-grammar path).
    // Single-line input → force_multiline=false → flat_alt + group() tries flat first
    // Already-multiline input (has newlines/comments/blank lines) → force_multiline=true → preserves multiline
    let mut signals = detect_argument_formatting_signals(arg_list);

    // Config override: force_break_keywords always forces multiline
    if config.force_break_keywords {
        signals.force_multiline = true;
    }

    // Check if we have any actual keywords (not just pre-keyword args)
    let has_keywords = sections.iter().any(|s| s.keyword.is_some());

    if !has_keywords {
        // No keywords found, fall back to simple formatting
        return format_simple_args(&sections, config, signals.force_multiline, indent_level);
    }

    // Explicit indentation strings for correct tab/space handling at any nesting depth
    let base_indent = super::cst_to_doc::indent_string(indent_level, config);
    let keyword_indent = super::cst_to_doc::indent_string(indent_level + 1, config);
    let value_indent = super::cst_to_doc::indent_string(indent_level + 2, config);

    // Build keyword-aware Doc structure
    // ARGL-03: first arg should stay on same line as command (no separator before it)
    let mut docs = Vec::new();
    let mut is_first_arg = true;

    for (i, section) in sections.iter().enumerate() {
        // Check if previous section had a trailing blank line (blank line between sections)
        if i > 0 && signals.force_multiline {
            let prev_section = &sections[i - 1];
            if prev_section.blank_lines.contains(&prev_section.args.len()) {
                // Extra blank line between sections
                docs.push(RcDoc::hardline());
            }
        }

        if let Some(keyword) = &section.keyword {
            // Handle different keyword types
            match section.keyword_type {
                // Flag keywords: group consecutive flags together
                Some(KeywordType::Flag) => {
                    // Flags typically have no values, but section.args may contain
                    // non-keyword arguments that follow before the next keyword
                    // Add separator before the flag keyword
                    if is_first_arg {
                        is_first_arg = false;
                        // First keyword in command: drop to next line when multiline
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.clone()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::nil(),
                            ));
                        }
                    } else {
                        // Consecutive flags or flag after pre-keyword args: group with space
                        let prev_is_flag = matches!(
                            sections.get(i.saturating_sub(1)),
                            Some(prev) if prev.keyword_type == Some(KeywordType::Flag)
                        );
                        let prev_is_pre_keyword = matches!(
                            sections.get(i.saturating_sub(1)),
                            Some(prev) if prev.keyword.is_none()
                        );
                        if prev_is_flag || prev_is_pre_keyword {
                            docs.push(RcDoc::space());
                        } else if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.clone()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::space(),
                            ));
                        }
                    }
                    docs.push(RcDoc::text(keyword.clone()));

                    // Output any trailing non-keyword arguments in this section
                    if !section.args.is_empty() {
                        // Use per-line when values were explicitly on new lines,
                        // or when there are comments/blank lines that can't go inline
                        let use_per_line = section.values_on_new_line
                            || !section.comments.is_empty()
                            || !section.blank_lines.is_empty();

                        if use_per_line {
                            let mut comment_iter = section.comments.iter().peekable();
                            for (arg_idx, arg) in section.args.iter().enumerate() {
                                while let Some((pos, comment)) = comment_iter.peek() {
                                    if *pos == arg_idx {
                                        if signals.force_multiline {
                                            docs.push(RcDoc::hardline());
                                            docs.push(RcDoc::text(keyword_indent.clone()));
                                        } else {
                                            docs.push(RcDoc::flat_alt(
                                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                                RcDoc::space(),
                                            ));
                                        }
                                        docs.push(RcDoc::text(comment.clone()));
                                        comment_iter.next();
                                    } else {
                                        break;
                                    }
                                }
                                if section.blank_lines.contains(&arg_idx) && signals.force_multiline {
                                    docs.push(RcDoc::hardline());
                                }
                                if signals.force_multiline {
                                    docs.push(RcDoc::hardline());
                                    docs.push(RcDoc::text(keyword_indent.clone()));
                                } else {
                                    docs.push(RcDoc::flat_alt(
                                        RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                        RcDoc::space(),
                                    ));
                                }
                                docs.push(RcDoc::text(arg.clone()));
                            }
                            while let Some((_, comment)) = comment_iter.next() {
                                if signals.force_multiline {
                                    docs.push(RcDoc::hardline());
                                    docs.push(RcDoc::text(keyword_indent.clone()));
                                } else {
                                    docs.push(RcDoc::flat_alt(
                                        RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                        RcDoc::space(),
                                    ));
                                }
                                docs.push(RcDoc::text(comment.clone()));
                            }
                        } else {
                            // Values on same line as keyword: flat_alt inherits from outer group
                            for arg in &section.args {
                                docs.push(RcDoc::flat_alt(
                                    RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                    RcDoc::space(),
                                ));
                                docs.push(RcDoc::text(arg.clone()));
                            }
                        }
                    }
                }

                // SingleValue keywords: keep value inline (ignore force_multiline for idempotency)
                Some(KeywordType::SingleValue) if section.args.len() == 1 => {
                    // Add separator before the keyword
                    if is_first_arg {
                        is_first_arg = false;
                        // First keyword in command: drop to next line when multiline
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.clone()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::nil(),
                            ));
                        }
                    } else {
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                            RcDoc::space(),
                        ));
                    }
                    docs.push(RcDoc::text(keyword.clone()));
                    // Add the single value inline
                    docs.push(RcDoc::space());
                    docs.push(RcDoc::text(section.args[0].clone()));
                }

                // PairValue keywords: format as key-value pairs
                Some(KeywordType::PairValue) => {
                    // Add separator before the keyword
                    if is_first_arg {
                        is_first_arg = false;
                        // First keyword in command: drop to next line when multiline
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.clone()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::nil(),
                            ));
                        }
                    } else if signals.force_multiline {
                        docs.push(RcDoc::hardline());
                        docs.push(RcDoc::text(keyword_indent.clone()));
                    } else {
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                            RcDoc::space(),
                        ));
                    }
                    docs.push(RcDoc::text(keyword.clone()));

                    // Format values as key-value pairs
                    if !section.args.is_empty() {
                        let pairs: Vec<_> = section.args.chunks(2).collect();
                        let use_per_line = section.values_on_new_line
                            || !section.comments.is_empty()
                            || !section.blank_lines.is_empty();

                        if pairs.len() == 1 {
                            // Single pair: keep inline with keyword (e.g., PROPERTIES KEY VALUE)
                            docs.push(RcDoc::space());
                            docs.push(RcDoc::text(pairs[0][0].clone()));
                            if pairs[0].len() > 1 {
                                docs.push(RcDoc::space());
                                docs.push(RcDoc::text(pairs[0][1].clone()));
                            }
                        } else if use_per_line || signals.force_multiline {
                            // Per-line pairs
                            for chunk in pairs {
                                if signals.force_multiline {
                                    docs.push(RcDoc::hardline());
                                    docs.push(RcDoc::text(value_indent.clone()));
                                } else {
                                    docs.push(RcDoc::flat_alt(
                                        RcDoc::hardline().append(RcDoc::text(value_indent.clone())),
                                        RcDoc::space(),
                                    ));
                                }
                                // key
                                docs.push(RcDoc::text(chunk[0].clone()));
                                // value (if present — odd number of args means last key has no value)
                                if chunk.len() > 1 {
                                    docs.push(RcDoc::space());
                                    docs.push(RcDoc::text(chunk[1].clone()));
                                }
                            }
                        } else {
                            // Auto-layout: flat_alt pairs inherit from outer group
                            for chunk in pairs {
                                docs.push(RcDoc::flat_alt(
                                    RcDoc::hardline().append(RcDoc::text(value_indent.clone())),
                                    RcDoc::space(),
                                ));
                                docs.push(RcDoc::text(chunk[0].clone()));
                                if chunk.len() > 1 {
                                    docs.push(RcDoc::space());
                                    docs.push(RcDoc::text(chunk[1].clone()));
                                }
                            }
                        }
                    }
                }

                // MultiValue or SingleValue with >1 arg in force_multiline mode: vertical layout
                _ => {
                    // Standard vertical keyword formatting
                    if is_first_arg {
                        is_first_arg = false;
                        // First keyword in command: drop to next line when multiline
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.clone()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::nil(),
                            ));
                        }
                    } else if signals.force_multiline {
                        docs.push(RcDoc::hardline());
                        docs.push(RcDoc::text(keyword_indent.clone()));
                    } else {
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                            RcDoc::space(),
                        ));
                    }
                    docs.push(RcDoc::text(keyword.clone()));

                    // Values under the keyword with explicit indentation
                    if !section.args.is_empty() {
                        // Apply source grouping if enabled
                        // Disable grouping when comments or blank lines are present to preserve their positions
                        let effective_args = if config.source_grouping != super::config::SourceGrouping::None
                            && matches!(section.keyword_type, Some(KeywordType::MultiValue) | None)
                            && section.comments.is_empty()
                            && section.blank_lines.is_empty()
                        {
                            group_source_pairs(&section.args, config.source_grouping)
                        } else {
                            section.args.clone()
                        };

                        // Use per-line when values were explicitly on new lines,
                        // or when there are comments/blank lines that can't go inline
                        let use_per_line = section.values_on_new_line
                            || !section.comments.is_empty()
                            || !section.blank_lines.is_empty();

                        if use_per_line {
                            // Values on separate lines or has comments: keep per-line behavior
                            let mut comment_iter = section.comments.iter().peekable();

                            for (arg_idx, arg) in effective_args.iter().enumerate() {
                                while let Some((pos, comment)) = comment_iter.peek() {
                                    if *pos == arg_idx {
                                        if signals.force_multiline {
                                            docs.push(RcDoc::hardline());
                                            docs.push(RcDoc::text(value_indent.clone()));
                                        } else {
                                            docs.push(RcDoc::flat_alt(
                                                RcDoc::hardline().append(RcDoc::text(value_indent.clone())),
                                                RcDoc::space(),
                                            ));
                                        }
                                        docs.push(RcDoc::text(comment.clone()));
                                        comment_iter.next();
                                    } else {
                                        break;
                                    }
                                }

                                if section.blank_lines.contains(&arg_idx) && signals.force_multiline {
                                    docs.push(RcDoc::hardline());
                                }

                                if signals.force_multiline {
                                    docs.push(RcDoc::hardline());
                                    docs.push(RcDoc::text(value_indent.clone()));
                                } else {
                                    docs.push(RcDoc::flat_alt(
                                        RcDoc::hardline().append(RcDoc::text(value_indent.clone())),
                                        RcDoc::space(),
                                    ));
                                }
                                docs.push(RcDoc::text(arg.clone()));
                            }

                            while let Some((_, comment)) = comment_iter.next() {
                                if signals.force_multiline {
                                    docs.push(RcDoc::hardline());
                                    docs.push(RcDoc::text(value_indent.clone()));
                                } else {
                                    docs.push(RcDoc::flat_alt(
                                        RcDoc::hardline().append(RcDoc::text(value_indent.clone())),
                                        RcDoc::space(),
                                    ));
                                }
                                docs.push(RcDoc::text(comment.clone()));
                            }
                        } else {
                            // Values on same line as keyword: flat_alt inherits from outer group
                            for arg in &effective_args {
                                docs.push(RcDoc::flat_alt(
                                    RcDoc::hardline().append(RcDoc::text(value_indent.clone())),
                                    RcDoc::space(),
                                ));
                                docs.push(RcDoc::text(arg.clone()));
                            }
                        }
                    }
                }
            }
        } else {
            // Pre-keyword arguments (e.g., target name, or file list)
            // ARGL-03 refined: first arg stays inline only when it's a single
            // pre-keyword arg (e.g., target name). When there are multiple
            // pre-keyword args (a file list), all go on separate lines.

            // Apply source grouping if enabled
            // Disable grouping when comments or blank lines are present to preserve their positions
            let effective_args = if config.source_grouping != super::config::SourceGrouping::None
                && section.comments.is_empty()
                && section.blank_lines.is_empty()
            {
                group_source_pairs(&section.args, config.source_grouping)
            } else {
                section.args.clone()
            };

            let is_list = effective_args.len() > 1;
            for (_j, arg) in effective_args.iter().enumerate() {
                if is_first_arg && !is_list {
                    // Single pre-keyword arg: keep inline with command
                    is_first_arg = false;
                } else if is_first_arg {
                    // First arg of a list: treat like subsequent args
                    is_first_arg = false;
                    if signals.force_multiline {
                        docs.push(RcDoc::hardline());
                        docs.push(RcDoc::text(keyword_indent.clone()));
                    } else {
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                            RcDoc::nil(),
                        ));
                    }
                } else {
                    // Subsequent args: add separator with explicit indentation
                    if signals.force_multiline {
                        docs.push(RcDoc::hardline());
                        docs.push(RcDoc::text(keyword_indent.clone()));
                    } else {
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                            RcDoc::space(),
                        ));
                    }
                }
                docs.push(RcDoc::text(arg.clone()));
            }
        }
    }

    // Closing paren position: base indent
    if signals.force_multiline {
        docs.push(RcDoc::hardline());
        docs.push(RcDoc::text(base_indent));
    } else {
        docs.push(RcDoc::flat_alt(
            RcDoc::hardline().append(RcDoc::text(base_indent)),
            RcDoc::nil(),
        ));
    }

    let combined = RcDoc::concat(docs);

    if signals.force_multiline {
        combined
    } else {
        combined.group()
    }
}

/// Format arguments without keyword awareness (simple line breaking)
fn format_simple_args(sections: &[KeywordSection], config: &FormatConfig, force_multiline: bool, indent_level: usize) -> RcDoc<'static, ()> {
    let base_indent = super::cst_to_doc::indent_string(indent_level, config);
    let inner_indent = super::cst_to_doc::indent_string(indent_level + 1, config);

    let mut docs = Vec::new();
    let mut is_first_arg = true;

    // In flat mode (auto-layout), start with nothing before first arg
    if !force_multiline {
        docs.push(RcDoc::flat_alt(RcDoc::nil(), RcDoc::nil()));
    }

    // Collect all args and comments from all sections
    for section in sections {
        // Apply source grouping if enabled
        // Disable grouping when comments or blank lines are present to preserve their positions
        let effective_args = if config.source_grouping != super::config::SourceGrouping::None
            && section.comments.is_empty()
            && section.blank_lines.is_empty()
        {
            group_source_pairs(&section.args, config.source_grouping)
        } else {
            section.args.clone()
        };

        let mut comment_iter = section.comments.iter().peekable();

        for (arg_idx, arg) in effective_args.iter().enumerate() {
            // Emit comments before this argument
            while let Some((pos, comment)) = comment_iter.peek() {
                if *pos == arg_idx {
                    if force_multiline {
                        docs.push(RcDoc::hardline());
                        docs.push(RcDoc::text(inner_indent.clone()));
                    } else {
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(inner_indent.clone())),
                            RcDoc::space(),
                        ));
                    }
                    docs.push(RcDoc::text(comment.clone()));
                    comment_iter.next();
                    is_first_arg = false;
                } else {
                    break;
                }
            }

            // Check for blank line before this argument
            if section.blank_lines.contains(&arg_idx) && force_multiline {
                docs.push(RcDoc::hardline());
                is_first_arg = false;
            }

            // Add separator before arg (except for the very first arg)
            if !is_first_arg {
                if force_multiline {
                    docs.push(RcDoc::hardline());
                    docs.push(RcDoc::text(inner_indent.clone()));
                } else {
                    docs.push(RcDoc::flat_alt(
                        RcDoc::hardline().append(RcDoc::text(inner_indent.clone())),
                        RcDoc::space(),
                    ));
                }
            }
            docs.push(RcDoc::text(arg.clone()));
            is_first_arg = false;
        }

        // Emit trailing comments (after last argument)
        while let Some((_, comment)) = comment_iter.next() {
            if force_multiline {
                docs.push(RcDoc::hardline());
                docs.push(RcDoc::text(inner_indent.clone()));
            } else {
                docs.push(RcDoc::flat_alt(
                    RcDoc::hardline().append(RcDoc::text(inner_indent.clone())),
                    RcDoc::space(),
                ));
            }
            docs.push(RcDoc::text(comment.clone()));
            is_first_arg = false;
        }
    }

    // Closing paren position
    if force_multiline {
        docs.push(RcDoc::hardline());
        docs.push(RcDoc::text(base_indent));
    } else {
        docs.push(RcDoc::flat_alt(
            RcDoc::hardline().append(RcDoc::text(base_indent)),
            RcDoc::nil(),
        ));
    }

    let combined = RcDoc::concat(docs);

    if force_multiline {
        combined
    } else {
        combined.group()
    }
}
