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

/// Normalize whitespace in line comments to a single space after `#`.
/// Only affects line comments (not bracket comments).
/// Examples: "#\t\tfoo" -> "# foo", "#no-space" -> "# no-space", "#" -> "#"
fn normalize_comment_whitespace(comment: &str) -> String {
    if let Some(content) = comment.strip_prefix('#') {
        let trimmed = content.trim_start();
        if trimmed.is_empty() {
            "#".to_string()
        } else {
            format!("# {}", trimmed)
        }
    } else {
        comment.to_string()
    }
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

/// Apply source grouping while preserving blank line boundaries.
/// Groups files within segments (between blank lines) independently,
/// then adjusts blank line positions for the shorter grouped segments.
fn group_source_pairs_preserving_blanks(
    args: &[String],
    blank_lines: &[usize],
    grouping: super::config::SourceGrouping,
) -> (Vec<String>, Vec<usize>) {
    if blank_lines.is_empty() {
        return (group_source_pairs(args, grouping), Vec::new());
    }

    // Split args into segments at blank line boundaries
    let mut segments: Vec<&[String]> = Vec::new();
    let mut start = 0;

    for &bl_pos in blank_lines {
        let end = bl_pos.min(args.len());
        segments.push(&args[start..end]);
        start = end;
    }
    // Final segment
    if start < args.len() {
        segments.push(&args[start..]);
    }

    // Group each segment independently and track new blank line positions
    let mut result = Vec::new();
    let mut new_blank_lines = Vec::new();

    for (i, segment) in segments.iter().enumerate() {
        if i > 0 {
            new_blank_lines.push(result.len());
        }
        let grouped = group_source_pairs(segment, grouping);
        result.extend(grouped);
    }

    (result, new_blank_lines)
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
    /// Trailing inline comments: (arg_index, comment_text) - comment on same line after arg
    pub trailing_comments: Vec<(usize, String)>,
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
        trailing_comments: Vec::new(),
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
                            trailing_comments: Vec::new(),
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
                        // Enforce SingleValue consumption: if the current keyword is
                        // SingleValue and already has its one value, overflow to a new
                        // positional section. This mirrors cmake_parse_arguments behavior
                        // where one_value_keywords consume exactly one argument.
                        let at_capacity = matches!(
                            current_section.keyword_type,
                            Some(KeywordType::SingleValue)
                        ) && current_section.args.len() >= 1;

                        if at_capacity {
                            sections.push(current_section);
                            current_section = KeywordSection {
                                keyword: None,
                                args: vec![text],
                                comments: Vec::new(),
                                trailing_comments: Vec::new(),
                                blank_lines: Vec::new(),
                                keyword_type: None,
                                values_on_new_line: false,
                            };
                        } else {
                            // Track if first value is on a new line from its keyword
                            if current_section.args.is_empty() && current_section.keyword.is_some() && saw_newline_since_keyword {
                                current_section.values_on_new_line = true;
                            }
                            // Add as argument to current section
                            current_section.args.push(text);
                        }
                        saw_separator = false;
                    }
                }
                // Track comments
                SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
                    saw_separator = true;
                    // Normalize whitespace in line comments (not bracket comments)
                    let text = if token.kind() == SyntaxKind::COMMENT {
                        normalize_comment_whitespace(&text)
                    } else {
                        text
                    };
                    if consecutive_newlines == 0 && !current_section.args.is_empty() {
                        // Same line as previous arg (trailing inline comment)
                        let arg_index = current_section.args.len() - 1;
                        current_section.trailing_comments.push((arg_index, text));
                    } else {
                        // Own line (leading comment before next arg)
                        let position = current_section.args.len();
                        current_section.comments.push((position, text));
                    }
                    consecutive_newlines = 0;
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

/// Check if a string looks like a filename (has extension or path separator)
fn looks_like_filename(s: &str) -> bool {
    // Must contain a dot with extension, or path separators
    // Exclude CMake variables like ${VAR}, generator expressions $<...>
    if s.starts_with("${") || s.starts_with("$<") || s.starts_with("$ENV{") || s.starts_with("$CACHE{") {
        return false;
    }
    // Check for file extension (dot followed by at least 1 char, not at start)
    if let Some(dot_pos) = s.rfind('.') {
        if dot_pos > 0 && dot_pos < s.len() - 1 {
            return true;
        }
    }
    // Check for path separators
    s.contains('/') || s.contains('\\')
}

/// Sort source file arguments within a section, respecting blank line boundaries
/// and keeping comments in sync with their associated filenames.
///
/// Rules:
/// - Only sort items that look like filenames (have extensions or path separators)
/// - Blank lines create separate sortable segments
/// - Comments at position N are associated with the filename at position N
///   (i.e., comment before a filename moves with that filename)
/// - Paired entries from source_grouping (e.g., "foo.h foo.cpp") sort as a unit
///   using their first component as sort key
/// - Case-insensitive sort
pub fn sort_source_args(section: &mut KeywordSection) {
    if section.args.is_empty() {
        return;
    }

    // Determine the range of args to sort
    // For keyword sections (keyword is Some), sort all args
    // For pre-keyword sections (keyword is None):
    //   - If all args are filenames, sort all
    //   - If first arg is not a filename but rest are (e.g., add_executable(target src1 src2)),
    //     sort starting from index 1 (preserve target name)
    //   - Otherwise, skip sorting
    let (sort_start, _all_filenames) = if section.keyword.is_some() {
        // Keyword section: sort all if any are filenames
        let has_filenames = section.args.iter().any(|a| looks_like_filename(a));
        if !has_filenames {
            return;
        }
        (0, true)
    } else {
        // Pre-keyword section: check if we should sort
        let all_filenames = section.args.iter().all(|a| looks_like_filename(a));
        if all_filenames {
            // All are filenames, sort all
            (0, true)
        } else if section.args.len() > 1 {
            // Check if first is not a filename but rest are (common pattern: target_name + sources)
            let first_not_filename = !looks_like_filename(&section.args[0]);
            let rest_are_filenames = section.args[1..].iter().all(|a| looks_like_filename(a));
            if first_not_filename && rest_are_filenames {
                // Sort starting from index 1 (preserve first arg as target name)
                (1, false)
            } else {
                // Mixed content, skip sorting
                return;
            }
        } else {
            // Only one arg, nothing to sort
            return;
        }
    };

    // Split into segments by blank lines, but only for the sortable range
    let mut segments: Vec<std::ops::Range<usize>> = Vec::new();
    let mut seg_start = sort_start;
    for &bl in &section.blank_lines {
        if bl > seg_start && bl <= section.args.len() {
            segments.push(seg_start..bl);
            seg_start = bl;
        }
    }
    if seg_start < section.args.len() {
        segments.push(seg_start..section.args.len());
    }

    // For each segment, build sortable entries (arg + associated comments)
    // then sort and reassemble
    let mut new_args: Vec<String> = Vec::with_capacity(section.args.len());
    let mut new_comments: Vec<(usize, String)> = Vec::new();
    let mut new_trailing_comments: Vec<(usize, String)> = Vec::new();

    // First, preserve args before sort_start (e.g., target name in add_executable)
    for idx in 0..sort_start {
        let comments_at_pos: Vec<String> = section.comments.iter()
            .filter(|(pos, _)| *pos == idx)
            .map(|(_, text)| text.clone())
            .collect();
        for comment in comments_at_pos {
            new_comments.push((new_args.len(), comment));
        }
        new_args.push(section.args[idx].clone());
        // Preserve trailing comments for this arg
        for (tc_idx, tc_text) in &section.trailing_comments {
            if *tc_idx == idx {
                new_trailing_comments.push((new_args.len() - 1, tc_text.clone()));
            }
        }
    }

    // Now process sortable segments
    for seg in &segments {
        // Collect entries: each entry is (sort_key, arg, comments_before, trailing_comment)
        struct SortEntry {
            sort_key: String,
            arg: String,
            comments: Vec<String>, // comments positioned before this arg
            trailing_comment: Option<String>, // trailing comment on same line as this arg
        }

        let mut entries: Vec<SortEntry> = Vec::new();

        for idx in seg.clone() {
            // Collect comments at this position (positioned before this arg)
            let comments_at_pos: Vec<String> = section.comments.iter()
                .filter(|(pos, _)| *pos == idx)
                .map(|(_, text)| text.clone())
                .collect();

            // Collect trailing comment at this arg index
            let trailing_comment = section.trailing_comments.iter()
                .find(|(tc_idx, _)| *tc_idx == idx)
                .map(|(_, text)| text.clone());

            let arg = &section.args[idx];

            // Check if this is a commented-out filename in the comments
            // (handled by keeping comments associated with args)

            // Sort key: case-insensitive, use first component for paired entries
            let sort_key = if arg.contains(' ') {
                // Paired entry (e.g., "foo.h foo.cpp") - sort by first component
                arg.split_whitespace().next().unwrap_or(arg).to_lowercase()
            } else {
                arg.to_lowercase()
            };

            entries.push(SortEntry {
                sort_key,
                arg: arg.clone(),
                comments: comments_at_pos,
                trailing_comment,
            });
        }

        // Also collect comments that are at position == seg.end (trailing comments of segment)
        // These go after the last entry

        // Sort entries by sort key (stable sort preserves order of equal keys)
        entries.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));

        // Reassemble into new_args, new_comments, and new_trailing_comments
        let base = new_args.len();
        for (i, entry) in entries.iter().enumerate() {
            let pos = base + i;
            for comment in &entry.comments {
                new_comments.push((pos, comment.clone()));
            }
            new_args.push(entry.arg.clone());
            if let Some(trailing) = &entry.trailing_comment {
                new_trailing_comments.push((pos, trailing.clone()));
            }
        }
    }

    // Handle trailing comments (comments at position == args.len())
    for (pos, text) in &section.comments {
        if *pos == section.args.len() {
            new_comments.push((new_args.len(), text.clone()));
        }
    }

    section.args = new_args;
    section.comments = new_comments;
    section.trailing_comments = new_trailing_comments;
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

    // Apply sorting if enabled (do this BEFORE source grouping)
    let mut sections = sections; // make mutable
    if config.sort_sources != super::config::SortSources::None {
        for section in &mut sections {
            sort_source_args(section);
        }
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
                            || !section.trailing_comments.is_empty()
                            || !section.blank_lines.is_empty();

                        if use_per_line {
                            let mut comment_iter = section.comments.iter().peekable();
                            for (arg_idx, arg) in section.args.iter().enumerate() {
                                // Blank line before comments to preserve ordering
                                if section.blank_lines.contains(&arg_idx) && signals.force_multiline {
                                    docs.push(RcDoc::hardline());
                                }
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
                                // Emit trailing comment if present
                                for (tc_idx, tc_text) in &section.trailing_comments {
                                    if *tc_idx == arg_idx {
                                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                                    }
                                }
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
                            for (arg_idx, arg) in section.args.iter().enumerate() {
                                docs.push(RcDoc::flat_alt(
                                    RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                    RcDoc::space(),
                                ));
                                docs.push(RcDoc::text(arg.clone()));
                                // Emit trailing comment if present
                                for (tc_idx, tc_text) in &section.trailing_comments {
                                    if *tc_idx == arg_idx {
                                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                                    }
                                }
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
                            || !section.trailing_comments.is_empty()
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

                // MultiValue with exactly 1 arg: keep inline like SingleValue
                Some(KeywordType::MultiValue) if section.args.len() == 1 => {
                    // Add separator before the keyword
                    if is_first_arg {
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
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                            RcDoc::space(),
                        ));
                    }
                    docs.push(RcDoc::text(keyword.clone()));
                    // Single value inline with keyword
                    docs.push(RcDoc::space());
                    docs.push(RcDoc::text(section.args[0].clone()));
                }

                // BinPack keywords (e.g., COMMAND): pack values to fill lines
                Some(KeywordType::BinPack) => {
                    // Add separator before the keyword (same pattern as other keywords)
                    if is_first_arg {
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

                    // Values: bin-pack using per-value groups
                    if !section.args.is_empty() {
                        let has_annotations = !section.comments.is_empty() || !section.trailing_comments.is_empty() || !section.blank_lines.is_empty();

                        if has_annotations {
                            // Fall back to per-line (same as MultiValue) to preserve comment positions
                            for arg in &section.args {
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
                        } else {
                            // Bin-pack: each value in its own group()
                            // The pretty printer checks each inner group independently:
                            // if flat form (space + arg) fits remaining line width, stays flat;
                            // otherwise breaks to new line with value_indent
                            for arg in &section.args {
                                docs.push(
                                    RcDoc::group(
                                        RcDoc::flat_alt(
                                            RcDoc::hardline().append(RcDoc::text(value_indent.clone())),
                                            RcDoc::space(),
                                        )
                                        .append(RcDoc::text(arg.clone()))
                                    )
                                );
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
                        // Disable grouping when comments are present to preserve their positions
                        // Blank lines are preserved as segment boundaries
                        let (effective_args, effective_blank_lines) = if config.source_grouping != super::config::SourceGrouping::None
                            && matches!(section.keyword_type, Some(KeywordType::MultiValue) | Some(KeywordType::BinPack) | None)
                            && section.comments.is_empty()
                            && section.trailing_comments.is_empty()
                        {
                            group_source_pairs_preserving_blanks(&section.args, &section.blank_lines, config.source_grouping)
                        } else {
                            (section.args.clone(), section.blank_lines.clone())
                        };

                        // Use per-line when values were explicitly on new lines,
                        // or when there are comments/blank lines that can't go inline
                        let use_per_line = section.values_on_new_line
                            || !section.comments.is_empty()
                            || !effective_blank_lines.is_empty();

                        if use_per_line {
                            // Values on separate lines or has comments: keep per-line behavior
                            let mut comment_iter = section.comments.iter().peekable();

                            for (arg_idx, arg) in effective_args.iter().enumerate() {
                                // Blank line before comments to preserve ordering
                                if effective_blank_lines.contains(&arg_idx) && signals.force_multiline {
                                    docs.push(RcDoc::hardline());
                                }

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
                                // Emit trailing comment if present
                                for (tc_idx, tc_text) in &section.trailing_comments {
                                    if *tc_idx == arg_idx {
                                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                                    }
                                }
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
                            for (arg_idx, arg) in effective_args.iter().enumerate() {
                                docs.push(RcDoc::flat_alt(
                                    RcDoc::hardline().append(RcDoc::text(value_indent.clone())),
                                    RcDoc::space(),
                                ));
                                docs.push(RcDoc::text(arg.clone()));
                                // Emit trailing comment if present (should be rare since use_per_line checks trailing_comments)
                                for (tc_idx, tc_text) in &section.trailing_comments {
                                    if *tc_idx == arg_idx {
                                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                                    }
                                }
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
            // Disable grouping when comments or trailing comments are present to preserve their positions
            // Blank lines are preserved as segment boundaries
            let (effective_args, effective_blank_lines) = if config.source_grouping != super::config::SourceGrouping::None
                && section.comments.is_empty()
                && section.trailing_comments.is_empty()
            {
                group_source_pairs_preserving_blanks(&section.args, &section.blank_lines, config.source_grouping)
            } else {
                (section.args.clone(), section.blank_lines.clone())
            };

            let is_list = effective_args.len() > 1;
            let mut comment_iter = section.comments.iter().peekable();

            for (arg_idx, arg) in effective_args.iter().enumerate() {
                // Check for blank line before this argument (before comments to preserve ordering)
                if effective_blank_lines.contains(&arg_idx) && signals.force_multiline {
                    docs.push(RcDoc::hardline());
                    is_first_arg = false;
                }

                // Emit comments before this argument
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
                        is_first_arg = false;
                    } else {
                        break;
                    }
                }

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
                // Emit trailing comment if present
                for (tc_idx, tc_text) in &section.trailing_comments {
                    if *tc_idx == arg_idx {
                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                    }
                }
            }

            // Emit trailing comments (after last argument)
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

    // Apply sorting if enabled (do this BEFORE source grouping)
    let mut sections_owned: Vec<KeywordSection> = sections.to_vec();
    if config.sort_sources != super::config::SortSources::None {
        for section in &mut sections_owned {
            sort_source_args(section);
        }
    }
    let sections = &sections_owned;

    // Collect all args and comments from all sections
    for section in sections {
        // Apply source grouping if enabled
        // Disable grouping when comments are present to preserve their positions
        // Blank lines are preserved as segment boundaries
        let (effective_args, effective_blank_lines) = if config.source_grouping != super::config::SourceGrouping::None
            && section.comments.is_empty()
        {
            group_source_pairs_preserving_blanks(&section.args, &section.blank_lines, config.source_grouping)
        } else {
            (section.args.clone(), section.blank_lines.clone())
        };

        let mut comment_iter = section.comments.iter().peekable();

        for (arg_idx, arg) in effective_args.iter().enumerate() {
            // Check for blank line before this argument (before comments to preserve ordering)
            if effective_blank_lines.contains(&arg_idx) && force_multiline {
                docs.push(RcDoc::hardline());
                is_first_arg = false;
            }

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
            // Emit trailing comment if present
            for (tc_idx, tc_text) in &section.trailing_comments {
                if *tc_idx == arg_idx {
                    docs.push(RcDoc::text(format!(" {}", tc_text)));
                }
            }
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
