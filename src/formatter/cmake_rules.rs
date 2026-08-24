use crate::cst::ArgumentList;
use crate::syntax_kind::SyntaxKind;
use pretty::RcDoc;
use rowan::NodeOrToken;
use std::collections::{HashMap, HashSet};

use super::config::FormatConfig;
use super::cst_to_doc::detect_argument_formatting_signals;
use super::grammar::{CommandGrammar, KeywordType};

/// Recognized header extensions
const HEADER_EXTS: &[&str] = &["h", "hh", "hpp", "hxx", "ipp", "h++", "H", "vert"];
/// Recognized source extensions
const SOURCE_EXTS: &[&str] = &["c", "cc", "cpp", "cxx", "c++", "C", "m", "mm", "frag"];

/// Check if a filename has a header extension
fn is_header_file(name: &str) -> bool {
    name.rsplit('.')
        .next()
        .is_some_and(|ext| HEADER_EXTS.contains(&ext))
}

/// Check if a filename has a source extension
fn is_source_file(name: &str) -> bool {
    name.rsplit('.')
        .next()
        .is_some_and(|ext| SOURCE_EXTS.contains(&ext))
}

/// Normalize whitespace in line comments according to the specified style.
/// Only affects line comments (not bracket comments).
/// Multi-hash comments (##, ###, etc.) are always preserved as-is.
/// Examples:
///   - Preserve: "#\t\tfoo" -> "#\t\tfoo" (unchanged)
///   - HashSpace: "#\t\tfoo" -> "# foo", "#no-space" -> "# no-space", "#" -> "#"
///   - HashNoSpace: "#  foo" -> "#foo", "#" -> "#"
///   - Any style: "## heading" -> "## heading" (multi-hash preserved)
pub fn normalize_comment_whitespace(comment: &str, style: super::config::CommentStyle) -> String {
    use super::config::CommentStyle;

    // Multi-hash comments (##, ###, etc.) are preserved as-is
    if comment.starts_with("##") {
        return comment.to_string();
    }

    match style {
        CommentStyle::Preserve => comment.to_string(),
        CommentStyle::HashSpace => {
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
        CommentStyle::HashNoSpace => {
            if let Some(content) = comment.strip_prefix('#') {
                let trimmed = content.trim_start();
                if trimmed.is_empty() {
                    "#".to_string()
                } else {
                    format!("#{}", trimmed)
                }
            } else {
                comment.to_string()
            }
        }
    }
}

/// Extract the base name (without extension) from a file path
fn base_name(name: &str) -> Option<&str> {
    // Handle paths: take the last component, then strip extension
    let filename = name.rsplit('/').next().unwrap_or(name);
    let filename = filename.rsplit('\\').next().unwrap_or(filename);
    filename.rfind('.').map(|pos| &filename[..pos])
}

/// Get extension priority for HeadersFirst ordering.
/// Lower numbers come first. Header extensions get lower priorities than source extensions.
fn ext_priority(name: &str) -> usize {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        // Header extensions (lower priorities = earlier in order)
        "h" => 0,
        "hh" => 1,
        "hpp" => 2,
        "hxx" => 3,
        "ipp" => 4,
        "h++" => 5,
        "H" => 6,
        "vert" => 7,
        // Source extensions (higher priorities = later in order)
        "c" => 8,
        "cc" => 9,
        "cpp" => 10,
        "cxx" => 11,
        "c++" => 12,
        "C" => 13,
        "m" => 14,
        "mm" => 15,
        "frag" => 16,
        // Unknown extensions go at the end
        _ => 99,
    }
}

/// Group source files into N-way groups based on matching base names
///
/// Takes a list of file arguments and returns a new list where files with
/// matching base names are placed adjacent to each other and joined as a
/// single string (space-separated) so they render on the same line.
///
/// Files are ordered within each group by extension priority (see ext_priority).
/// Arguments that are not source/header files pass through unchanged.
/// Files without a matching pair pass through unchanged.
///
/// Returns: (grouped_args, old_to_new_index_mapping)
/// The index mapping maps each original index to its new position in the result.
/// When multiple files are grouped, they all map to the same result index.
pub fn group_source_pairs(
    args: &[String],
    grouping: super::config::SourceGrouping,
) -> (Vec<String>, Vec<usize>) {
    use super::config::SourceGrouping;

    if grouping == SourceGrouping::None {
        let identity_map: Vec<usize> = (0..args.len()).collect();
        return (args.to_vec(), identity_map);
    }

    // Build index: base_name -> all indices with that base name
    let mut base_map: HashMap<String, Vec<usize>> = HashMap::new();

    for (i, arg) in args.iter().enumerate() {
        if let Some(base) = base_name(arg) {
            let base_lower = base.to_lowercase();
            // Only include files that are recognized headers or sources
            if is_header_file(arg) || is_source_file(arg) {
                base_map.entry(base_lower).or_default().push(i);
            }
        }
    }

    // Identify groups: any base name with 2+ files forms a group
    let mut is_grouped = vec![false; args.len()];
    let mut groups: Vec<Vec<usize>> = Vec::new();

    for indices in base_map.values() {
        if indices.len() >= 2 {
            // Mark all indices as grouped
            for &idx in indices {
                is_grouped[idx] = true;
            }
            groups.push(indices.clone());
        }
    }

    // Build result: emit groups at the position of their earliest index
    let mut emitted = vec![false; args.len()];
    let mut result = Vec::new();
    let mut old_to_new = vec![0; args.len()];

    // Create a lookup: index -> group containing that index
    let mut index_to_group: HashMap<usize, usize> = HashMap::new();
    for (group_id, group) in groups.iter().enumerate() {
        for &idx in group {
            index_to_group.insert(idx, group_id);
        }
    }

    for (i, arg) in args.iter().enumerate() {
        if emitted[i] {
            continue;
        }

        if is_grouped[i] {
            // This file is part of a group
            let group_id = index_to_group[&i];
            let group = &groups[group_id];

            // Emit the entire group (only once, at the earliest index position)
            let earliest = *group.iter().min().unwrap();
            if i == earliest {
                // Collect all files in the group
                let mut group_files: Vec<(usize, &String)> =
                    group.iter().map(|&idx| (idx, &args[idx])).collect();

                // Sort by extension priority
                match grouping {
                    SourceGrouping::HeadersFirst => {
                        // Lower priority numbers first (headers before sources)
                        group_files.sort_by_key(|(_, file)| ext_priority(file));
                    }
                    SourceGrouping::SourcesFirst => {
                        // Higher priority numbers first (sources before headers)
                        group_files.sort_by(|(_, a), (_, b)| ext_priority(b).cmp(&ext_priority(a)));
                    }
                    SourceGrouping::None => unreachable!(),
                }

                // Join the group as a space-separated string
                let group_str = group_files
                    .iter()
                    .map(|(_, file)| file.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");

                let result_idx = result.len();
                result.push(group_str);

                // Map all original indices in this group to the same result index
                for &idx in group {
                    old_to_new[idx] = result_idx;
                    emitted[idx] = true;
                }
            }
        } else {
            // Not grouped, pass through unchanged
            let result_idx = result.len();
            result.push(arg.clone());
            old_to_new[i] = result_idx;
            emitted[i] = true;
        }
    }

    (result, old_to_new)
}

/// A section's arguments as they should be emitted, grouped when
/// `source_grouping` is on and the section's own allowlist entry permits it.
///
/// Every place that renders a section's arguments goes through this. Five copies
/// of the decision meant the `Flag` arm — which carries the positional run after
/// a flag, as in `add_library(l STATIC a.cpp a.h)` — never grouped at all, so
/// the two reordering passes disagreed about a list the allowlist owns; and
/// three of the five copies had no test.
fn grouped_section(
    section: &KeywordSection,
    grouping: super::config::SourceGrouping,
) -> GroupedSection {
    if grouping != super::config::SourceGrouping::None && section.trailing_comments.is_empty() {
        group_source_pairs_preserving_blanks(
            &section.args,
            section.sort_from,
            &section.blank_lines,
            &section.comments,
            &section.post_comment_blanks,
            &section.comment_blank_indices,
            grouping,
        )
    } else {
        (
            section.args.clone(),
            section.blank_lines.clone(),
            section.comments.clone(),
            section.post_comment_blanks.clone(),
            section.comment_blank_indices.clone(),
        )
    }
}

/// Emit the comments of a keyword section that has no values.
///
/// Every arm that renders a section guards its comment machinery on the section
/// having arguments, so a comment attached to a keyword with no values had
/// nowhere to go and was deleted outright — `target_sources(t\n\tPRIVATE # note)`
/// lost `# note`, and so did the `PairValue`, `BinPack` and inline equivalents.
///
/// They are always own-line comments: the section parser only records a trailing
/// comment for a section that already holds an argument, so a comment written on
/// the same line as a valueless keyword arrives at position 0 of `comments`.
/// Whether the section before `index` has already put a comment on its line.
///
/// A line comment runs to the end of its line, so nothing may follow that
/// section on the same line. Every separator that collapses a keyword onto the
/// previous line has to ask this first: collapsing regardless put the keyword
/// *inside* the comment — `# note QUIET` — and the keyword was gone from the
/// file, permanently, at a stable fixed point and exit 0.
///
/// Three arms have made that same mistake in three consecutive rounds, each time
/// one arm over from the last fix, so the question lives in one place and every
/// separator consults it.
fn previous_section_ended_in_a_comment(sections: &[KeywordSection], index: usize) -> bool {
    index
        .checked_sub(1)
        .and_then(|previous| sections.get(previous))
        .is_some_and(|previous| {
            !previous.comments.is_empty() || !previous.trailing_comments.is_empty()
        })
}

/// Emit the comments of a keyword section that has no values.
///
/// At `keyword_indent`, not `value_indent`: there are no values for the comment
/// to sit under, and the comment belongs to the keyword. Three of the five arms
/// used the deeper level, so `find_package(Foo REQUIRED # n)` put its comment one
/// tab in and `target_sources(t PRIVATE # n)` put an identical construct two.
fn push_valueless_section_comments(
    docs: &mut Vec<RcDoc<'static, ()>>,
    comments: &[(usize, String)],
    indent: &str,
) {
    for (_, comment) in comments {
        docs.push(RcDoc::hardline());
        docs.push(RcDoc::text(indent.to_string()));
        docs.push(RcDoc::text(comment.clone()));
    }
}

/// Group only the runs the sorting pass is allowed to permute: from `sort_from`
/// onward, and never across a barrier.
///
/// `source_grouping` reorders on its own — it hoists a later header to its
/// pair's index whether or not `sort_sources` is on — so without this it moved a
/// file across a `${...}` that `sort_source_args` refuses to cross, and past the
/// target name of an `add_library`. `set(SRCS b.cpp ${GENERATED} ${OTHER} b.h)`
/// became `set(SRCS b.h b.cpp ${GENERATED} ${OTHER})`, which is a reorder of a
/// list whose contents nobody can read.
fn group_sortable_runs(
    args: &[String],
    sort_from: usize,
    grouping: super::config::SourceGrouping,
) -> (Vec<String>, Vec<usize>) {
    // A section with no sortable range pins everything, so `min` here is what
    // makes `sort_from = usize::MAX` mean "group nothing"
    let pinned = sort_from.min(args.len());
    let mut out: Vec<String> = args[..pinned].to_vec();
    let mut old_to_new: Vec<usize> = (0..pinned).collect();

    for run in split_at_barriers(args, pinned..args.len()) {
        let base = out.len();
        let (grouped, local) = group_source_pairs(&args[run.clone()], grouping);
        out.extend(grouped);
        old_to_new.extend(local.into_iter().map(|new| base + new));
    }

    (out, old_to_new)
}

/// A section after grouping: the arguments, the blank-line positions, the
/// comments with their new positions, the post-comment blanks, and the indices
/// of comments that carry a blank line.
type GroupedSection = (
    Vec<String>,
    Vec<usize>,
    Vec<(usize, String)>,
    Vec<usize>,
    Vec<usize>,
);

/// Group within each blank-line segment independently, adjusting the blank-line
/// and comment positions to the shorter grouped segments.
fn group_source_pairs_preserving_blanks(
    args: &[String],
    sort_from: Option<usize>,
    blank_lines: &[usize],
    comments: &[(usize, String)],
    post_comment_blanks: &[usize],
    comment_blank_indices: &[usize],
    grouping: super::config::SourceGrouping,
) -> GroupedSection {
    // A section with no sortable run groups nothing, so `None` pins everything
    let sort_from = sort_from.unwrap_or(usize::MAX);

    if blank_lines.is_empty() {
        let (grouped_args, old_to_new) = group_sortable_runs(args, sort_from, grouping);
        // Remap comment positions using the index mapping
        let new_comments = comments
            .iter()
            .map(|(pos, text)| {
                let new_pos = if *pos < old_to_new.len() {
                    old_to_new[*pos]
                } else {
                    // Comment after last arg - map to new length
                    grouped_args.len()
                };
                (new_pos, text.clone())
            })
            .collect();
        return (
            grouped_args,
            Vec::new(),
            new_comments,
            Vec::new(),
            comment_blank_indices.to_vec(),
        );
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
    let mut new_post_comment_blanks = Vec::new();
    let mut global_old_to_new = vec![0; args.len()];

    let mut segment_start = 0;
    for (i, segment) in segments.iter().enumerate() {
        if i > 0 {
            let new_bl_pos = result.len();
            new_blank_lines.push(new_bl_pos);
            // Check if the original blank_line at this boundary was post-comment
            if i - 1 < blank_lines.len() && post_comment_blanks.contains(&blank_lines[i - 1]) {
                new_post_comment_blanks.push(new_bl_pos);
            }
        }
        let (grouped, segment_old_to_new) =
            group_sortable_runs(segment, sort_from.saturating_sub(segment_start), grouping);

        // Map segment-local indices to global indices
        for (local_idx, &local_new) in segment_old_to_new.iter().enumerate() {
            let global_idx = segment_start + local_idx;
            let global_new = result.len() + local_new;
            if global_idx < global_old_to_new.len() {
                global_old_to_new[global_idx] = global_new;
            }
        }

        result.extend(grouped);
        segment_start += segment.len();
    }

    // Remap comment positions using the global index mapping
    let new_comments = comments
        .iter()
        .map(|(pos, text)| {
            let new_pos = if *pos < global_old_to_new.len() {
                global_old_to_new[*pos]
            } else {
                // Comment after last arg - map to new length
                result.len()
            };
            (new_pos, text.clone())
        })
        .collect();

    (
        result,
        new_blank_lines,
        new_comments,
        new_post_comment_blanks,
        comment_blank_indices.to_vec(),
    )
}

/// Check if a command name requires keyword-aware formatting
pub fn is_keyword_aware_command(name: &str) -> bool {
    use super::grammar::GrammarRegistry;
    GrammarRegistry::global()
        .get(&name.to_lowercase())
        .is_some()
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
    /// Blank line positions where the blank line appears AFTER comments at the same position
    /// (as opposed to the default where blank lines come before comments)
    pub post_comment_blanks: Vec<usize>,
    /// Indices into the `comments` vector where a blank line should appear BEFORE this comment.
    /// Tracks blank lines between comment groups at the same arg position, which blank_lines
    /// cannot represent (since blank_lines is position-based and deduplicates).
    pub comment_blank_indices: Vec<usize>,
    /// The type of the keyword (if known from grammar)
    pub keyword_type: Option<KeywordType>,
    /// `Some(n)` when this section's arguments are an unordered list that
    /// `sort_sources` and `source_grouping` may reorder, starting at index `n`.
    /// `None` means the order is meaningful and must be left alone.
    ///
    /// Decided by the grammar while sections are parsed — see
    /// [`CommandGrammar::sortable_keywords`] and
    /// [`CommandGrammar::sortable_positional`].
    pub sort_from: Option<usize>,
    /// Whether a newline appeared between the keyword and its first value
    /// (i.e., values were written on separate lines from the keyword)
    pub values_on_new_line: bool,
}

/// Parse an argument list into keyword sections with optional grammar guidance
pub fn parse_keyword_sections_with_grammar(
    arg_list: &ArgumentList,
    grammar: Option<&CommandGrammar>,
    comment_style: super::config::CommentStyle,
) -> Vec<KeywordSection> {
    let mut sections = Vec::new();
    let mut current_section = KeywordSection {
        keyword: None,
        args: Vec::new(),
        comments: Vec::new(),
        trailing_comments: Vec::new(),
        blank_lines: Vec::new(),
        post_comment_blanks: Vec::new(),
        comment_blank_indices: Vec::new(),
        keyword_type: None,
        // Leading positional run: index 0 is the variable or target name
        sort_from: grammar.is_some_and(|g| g.sortable_positional).then_some(1),
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

                    // Check if this keyword should be consumed by the current BinPack or MultiValue section
                    // (e.g., DESTINATION inside LIBRARY BinPack, or PATTERN inside FILES_MATCHING MultiValue)
                    let consumed_as_sub_keyword = is_kw
                        && grammar.is_some_and(|g| g.sub_keywords.contains(&text))
                        && match current_section.keyword_type {
                            Some(KeywordType::BinPack) => true,
                            Some(KeywordType::MultiValue) => {
                                // Only consume sub_keywords in MultiValue sections that are
                                // explicitly marked as collection keywords (e.g., FILES_MATCHING)
                                current_section.keyword.as_ref().is_some_and(|kw| {
                                    grammar.is_some_and(|g| {
                                        g.collection_keywords.contains(kw.as_str())
                                    })
                                })
                            }
                            _ => false,
                        };

                    if is_kw && !consumed_as_sub_keyword {
                        // Get the keyword type from grammar if available
                        let kw_type = grammar.and_then(|g| g.keyword_type(&text));
                        // Whether this keyword's values are an unordered list
                        let kw_sort_from = grammar
                            .is_some_and(|g| g.is_sortable_keyword(&text))
                            .then_some(0);

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
                            post_comment_blanks: Vec::new(),
                            comment_blank_indices: Vec::new(),
                            keyword_type: kw_type,
                            sort_from: kw_sort_from,
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
                        let at_capacity =
                            matches!(current_section.keyword_type, Some(KeywordType::SingleValue))
                                && !current_section.args.is_empty();

                        if at_capacity {
                            // A leading mode keyword that consumed the list
                            // variable opens an unordered run, as in
                            // `list(APPEND var a b)`. Only the command's first
                            // section qualifies: anywhere later, this is a stray
                            // positional argument rather than the command's
                            // argument list. Computed before the push, which is
                            // what makes `sections` still empty here.
                            let overflow_sortable = sections.is_empty()
                                && grammar.is_some_and(|g| g.sortable_positional);
                            sections.push(current_section);
                            current_section = KeywordSection {
                                keyword: None,
                                args: vec![text],
                                comments: Vec::new(),
                                trailing_comments: Vec::new(),
                                blank_lines: Vec::new(),
                                post_comment_blanks: Vec::new(),
                                comment_blank_indices: Vec::new(),
                                keyword_type: None,
                                sort_from: overflow_sortable.then_some(0),
                                values_on_new_line: false,
                            };
                        } else {
                            // Track if first value is on a new line from its keyword
                            if current_section.args.is_empty()
                                && current_section.keyword.is_some()
                                && saw_newline_since_keyword
                            {
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
                        normalize_comment_whitespace(&text, comment_style)
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
                            // Check if comments already exist at this position
                            // If so, the blank line comes AFTER those comments in the source
                            if current_section
                                .comments
                                .iter()
                                .any(|(pos, _)| *pos == position)
                            {
                                current_section.post_comment_blanks.push(position);
                            }
                        } else {
                            // Already have a blank line at this position -- this is a blank line
                            // between comment groups. Track which comment index gets a blank before it.
                            let next_comment_idx = current_section.comments.len();
                            current_section.comment_blank_indices.push(next_comment_idx);
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
        } else if let NodeOrToken::Node(node) = child {
            // A nested `( ... )` group is one logical argument, never a keyword,
            // e.g. the grouped sub-expression in `if((A AND B) OR C)`.
            if let Some(nested) = ArgumentList::cast(node) {
                consecutive_newlines = 0;
                let text = super::cst_to_doc::render_nested_group(&nested);

                if !saw_separator && !current_section.args.is_empty() {
                    // Adjacent to previous token (no whitespace) — merge,
                    // e.g. `NOT(TRUE)`
                    current_section.args.last_mut().unwrap().push_str(&text);
                } else if matches!(current_section.keyword_type, Some(KeywordType::SingleValue))
                    && !current_section.args.is_empty()
                {
                    // Same SingleValue overflow the token path applies: the
                    // keyword already has its one value, so this starts a new
                    // positional section. `sort_from` is decided the same way
                    // too — before the push, while `sections` is still empty
                    // for a leading mode keyword. The group itself cannot move,
                    // because `is_variable_like` treats it as a barrier.
                    let overflow_sortable =
                        sections.is_empty() && grammar.is_some_and(|g| g.sortable_positional);
                    sections.push(current_section);
                    current_section = KeywordSection {
                        keyword: None,
                        args: vec![text],
                        comments: Vec::new(),
                        trailing_comments: Vec::new(),
                        blank_lines: Vec::new(),
                        post_comment_blanks: Vec::new(),
                        comment_blank_indices: Vec::new(),
                        sort_from: overflow_sortable.then_some(0),
                        keyword_type: None,
                        values_on_new_line: false,
                    };
                } else {
                    if current_section.args.is_empty()
                        && current_section.keyword.is_some()
                        && saw_newline_since_keyword
                    {
                        current_section.values_on_new_line = true;
                    }
                    current_section.args.push(text);
                }
                saw_separator = false;
            }
        }
    }

    // Push the last section if it has content
    if !current_section.args.is_empty() || current_section.keyword.is_some() {
        sections.push(current_section);
    }

    // `add_library`/`add_executable` name a target in their first positional,
    // not the variable that governs the list
    let first_positional_governs = owning_command_name(arg_list)
        .map(|name| !matches!(name.as_str(), "add_library" | "add_executable"))
        .unwrap_or(true);
    unmark_unsortable_positional_runs(&mut sections, first_positional_governs);

    sections
}

/// Variables whose element order is a search precedence, not a set.
///
/// `list(APPEND CMAKE_MODULE_PATH cmake/overrides cmake/defaults)` resolves
/// first match wins, so sorting it silently picks a different module. Only ever
/// disables reordering, so a false positive costs a little tidiness and a false
/// negative costs correctness.
fn is_search_path_variable(name: &str) -> bool {
    // Compared case-insensitively: CMake variable names are case-sensitive, and
    // project-local lists are conventionally lowercase (`warning_flags`), so a
    // byte comparison would miss most of them. Unquoted first, as every sibling
    // predicate does — `"CMAKE_MODULE_PATH"` is as ordinary a spelling as the
    // bare one, and only the `FLAGS` substring test survived the quotes.
    let name = name.trim_matches('"').to_ascii_uppercase();

    const EXACT: &[&str] = &[
        "CMAKE_MODULE_PATH",
        "CMAKE_PREFIX_PATH",
        "CMAKE_FIND_ROOT_PATH",
        "CMAKE_INCLUDE_PATH",
        "CMAKE_LIBRARY_PATH",
        "CMAKE_PROGRAM_PATH",
    ];
    const SUFFIXES: &[&str] = &[
        "_PATH",
        "_PATHS",
        "_DIRS",
        "_DIRECTORIES",
        "_OPTIONS",
        // Argument lists feeding a COMMAND: `list(APPEND ARGS -o out.png)` then
        // `add_custom_command(COMMAND tool ${ARGS})` is an argv one level of
        // indirection away
        "_ARGS",
        "_ARGUMENTS",
        // Static archive link order is significant
        "_LIBS",
        "_LIBRARIES",
        // Glob/regex lists are evaluated in order, and that order decides the
        // order of what they match — see OCV_GLOB_PATTERNS in the OpenCV corpus
        "_PATTERNS",
    ];

    if EXACT.contains(&name.as_str()) || SUFFIXES.iter().any(|suffix| name.ends_with(suffix)) {
        return true;
    }

    // Compiler and linker flag lists, where the last flag usually wins.
    // `contains` rather than `ends_with`, because the common spelling is
    // `CMAKE_CXX_FLAGS_RELEASE` / `CMAKE_EXE_LINKER_FLAGS_DEBUG`, and because
    // pkg-config names are `GTK_CFLAGS` / `MY_LDFLAGS` with no underscore.
    name.contains("FLAGS")
}

/// True for a value that may be reordered inside a positional run.
///
/// A keyword names what its values are, so a keyword section trusts the grammar
/// alone. A positional run has no such label — `set(VAR …)` holds sources in one
/// project and compiler flags in the next — so its values have to look like
/// source files before anything moves. This sits behind the grammar allowlist
/// and only ever narrows it, so it cannot reopen the `CACHE`, `FILES_MATCHING`
/// or `file(RENAME)` cases: the first two are keyword sections, and `file` is a
/// recognised multi-mode command whose `RENAME` mode opts nothing in.
fn is_sortable_positional_value(arg: &str) -> bool {
    // Unquote first: a quoted flag is still a flag, and `"-I/usr/inc/a.h"`
    // would otherwise pass the prefix test and then look like a header
    let name = arg.trim_matches('"');

    // Flags and options: -Wall, --input, /O2, /wd4100, -I/usr/include. This
    // also rejects absolute POSIX paths, deliberately: protecting an MSVC flag
    // list is worth more than sorting a list of literal /usr/src/… paths.
    if name.starts_with('-') || name.starts_with('/') {
        return false;
    }
    // Definitions and assignments: -DVERSION=1.0, A=1
    if name.contains('=') {
        return false;
    }

    let Some(extension) = name.rsplit('.').next().filter(|ext| *ext != name) else {
        // No extension: a bare word like README or a target name. Sorting those
        // is fine where a keyword vouches for them, but not here.
        return false;
    };

    // A version number is not a file: `set(PYTHON_VERSIONS 3.9 3.12)` reads as
    // extension "9", and version lists are usually precedence lists.
    if extension.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    // Libraries, where link order decides symbol resolution
    const LINKABLE_EXTS: &[&str] = &["a", "so", "dylib", "lib", "dll", "o", "obj"];
    !LINKABLE_EXTS.contains(&extension.to_ascii_lowercase().as_str())
}

/// The command this argument list belongs to, when the tree above it says so.
///
/// Whether the leading positional is a variable or a target name is a property
/// of the command, and the section parser is otherwise given only the grammar,
/// which does not carry it.
fn owning_command_name(arg_list: &ArgumentList) -> Option<String> {
    let invocation = arg_list.syntax().parent()?;
    if invocation.kind() != SyntaxKind::COMMAND_INVOCATION {
        return None;
    }
    invocation
        .children_with_tokens()
        .filter_map(|child| child.into_token())
        .find(|token| token.kind() == SyntaxKind::COMMAND_NAME)
        .map(|token| token.text().to_lowercase())
}

/// Clear `sort_from` on positional runs that must not be reordered after all.
///
/// A positional run is opted in by the command's grammar, which cannot know what
/// the list actually holds. Two things disqualify one:
///
/// - its governing variable names a search path, flag list or argv — the name is
///   the run's own first argument (`set(VAR …)`) or the value of the preceding
///   single-value mode keyword (`list(APPEND VAR …)`);
/// - any of its values does not look like a source file.
///
/// `first_positional_governs` says whether the leading run's own first argument
/// is that variable. It is not for `add_library`/`add_executable`, where it
/// names a target: reading a target name as a variable name stopped
/// `add_library(my_libs z.cpp a.cpp)` sorting, because the name ends in `_LIBS`.
fn unmark_unsortable_positional_runs(
    sections: &mut [KeywordSection],
    first_positional_governs: bool,
) {
    for idx in 0..sections.len() {
        let Some(sort_from) = sections[idx].sort_from else {
            continue;
        };
        if sections[idx].keyword.is_some() {
            continue;
        }

        // At index 0 the first argument names the thing being defined — a
        // variable for `set`, but a *target* for add_library/add_executable.
        //
        // Which of the two is still decided by builtin command name, inside a
        // path that is otherwise grammar-driven, so a user wrapper cannot say
        // "my first positional is a target" and gets the conservative answer:
        // `my_add_library(my_LIBS z.cpp a.cpp)` holds where `add_library` sorts.
        // Saying it needs a third grammar field; until there is one, a wrapper
        // declines to sort where the builtin would, which is the safe direction.
        // A later run is opened by a mode keyword that already consumed the
        // list variable, so there the name provably governs the list.
        let (governing, governs_the_list) = if idx == 0 {
            (sections[idx].args.first(), first_positional_governs)
        } else {
            let previous = &sections[idx - 1];
            let name = previous
                .keyword
                .is_some()
                .then(|| previous.args.first())
                .flatten();
            (name, true)
        };

        // A list variable we cannot read is one we cannot vet — but only when
        // the whole token is a reference. `${PREFIX}_SOURCES` has a readable
        // suffix, and a dynamic *target* name says nothing about its sources.
        let blocked_by_name = governing.is_some_and(|name| {
            governs_the_list && (is_whole_variable_reference(name) || is_search_path_variable(name))
        });

        let blocked_by_value = sections[idx]
            .args
            .iter()
            .skip(sort_from)
            .any(|arg| !is_variable_like(arg) && !is_sortable_positional_value(arg));

        if blocked_by_name || blocked_by_value {
            sections[idx].sort_from = None;
        }
    }
}

/// Parse an argument list into keyword sections (backward compatibility wrapper)
#[allow(dead_code)]
pub fn parse_keyword_sections(arg_list: &ArgumentList) -> Vec<KeywordSection> {
    parse_keyword_sections_with_grammar(arg_list, None, super::config::CommentStyle::HashSpace)
}

/// Split `seg` into runs of adjacent sortable args, with each variable-like arg
/// becoming its own single-element run — a barrier that can neither move nor let
/// its neighbours move across it.
fn split_at_barriers(args: &[String], seg: std::ops::Range<usize>) -> Vec<std::ops::Range<usize>> {
    let mut runs: Vec<std::ops::Range<usize>> = Vec::new();
    let mut run_start = seg.start;

    for (offset, arg) in args[seg.start..seg.end].iter().enumerate() {
        let idx = seg.start + offset;
        if is_variable_like(arg) {
            if run_start < idx {
                runs.push(run_start..idx);
            }
            runs.push(idx..idx + 1);
            run_start = idx + 1;
        }
    }

    if run_start < seg.end {
        runs.push(run_start..seg.end);
    }

    runs
}

/// True for arguments that expand to something unknown at format time, so their
/// position may be meaningful even inside a list that is otherwise unordered.
///
/// A leading quote is stripped first: `"${GENERATED}"` is as common as the bare
/// spelling, and it would otherwise sort ahead of everything because `"` (0x22)
/// precedes every letter.
fn is_variable_like(s: &str) -> bool {
    // Tested before the quotes are stripped: a rendered group always starts at
    // its own `(`, so a leading quote means this is a quoted *value* that
    // merely begins with a paren, not a group.
    if s.starts_with('(') {
        return true;
    }

    let s = s.trim_start_matches('"');
    // A parenthesized group is one rendered argument holding several real ones,
    // so its position is meaningful for the same reason a variable's is — and
    // the value heuristics cannot read it: `(b c.cpp)` looks like a file with
    // extension "cpp)", and its leading '(' hides any flag inside it.
    s.starts_with("${") || s.starts_with("$<") || s.starts_with("$ENV{") || s.starts_with("$CACHE{")
}

/// True when the whole token is a single variable reference, so nothing about
/// the name can be read. `${PREFIX}_SOURCES` is not one: its suffix is readable
/// and can be vetted against the search-path list.
fn is_whole_variable_reference(s: &str) -> bool {
    let s = s.trim_matches('"');
    ((s.starts_with("${") || s.starts_with("$ENV{") || s.starts_with("$CACHE{"))
        && s.ends_with('}'))
        || (s.starts_with("$<") && s.ends_with('>'))
}

/// Sort the arguments of a section the grammar marked as an unordered list,
/// respecting blank line boundaries and keeping comments with their arguments.
///
/// Rules:
/// - Only sections with `sort_from` set are touched; whether a list may be
///   reordered is the grammar's decision, not a guess from the argument text
/// - Arguments before `sort_from` are pinned (the variable or target name)
/// - Variable references and generator expressions hold their index and keep
///   arguments from moving across them
/// - Blank lines create separate sortable segments
/// - Comments at position N are associated with the argument at position N
///   (i.e., comment before a filename moves with that filename)
/// - Paired entries from source_grouping (e.g., "foo.h foo.cpp") sort as a unit
///   using their first component as sort key
/// - Case-insensitive sort
pub fn sort_source_args(section: &mut KeywordSection) {
    let Some(sort_start) = section.sort_from else {
        return;
    };

    if section.args.len() <= sort_start + 1 {
        // Nothing to reorder
        return;
    }

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

    // Split each segment further at variable-like arguments, which hold their
    // index and act as a barrier for the arguments around them
    let segments: Vec<std::ops::Range<usize>> = segments
        .into_iter()
        .flat_map(|seg| split_at_barriers(&section.args, seg))
        .collect();

    // For each segment, build sortable entries (arg + associated comments)
    // then sort and reassemble
    let mut new_args: Vec<String> = Vec::with_capacity(section.args.len());
    let mut new_comments: Vec<(usize, String)> = Vec::new();
    let mut new_trailing_comments: Vec<(usize, String)> = Vec::new();

    // First, preserve args before sort_start (e.g., target name in add_executable)
    for idx in 0..sort_start {
        let comments_at_pos: Vec<String> = section
            .comments
            .iter()
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
            let comments_at_pos: Vec<String> = section
                .comments
                .iter()
                .filter(|(pos, _)| *pos == idx)
                .map(|(_, text)| text.clone())
                .collect();

            // Collect trailing comment at this arg index
            let trailing_comment = section
                .trailing_comments
                .iter()
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

/// Group args that start with sub-keywords into logical groups.
///
/// Each sub_keyword and its following non-sub-keyword args form one logical group.
/// For example, ["PATTERN", "*.h", "EXCLUDE", "PATTERN", "internal/*"] with
/// sub_keywords {"PATTERN", "EXCLUDE"} yields ["PATTERN *.h", "EXCLUDE", "PATTERN internal/*"].
///
/// Used by FILES_MATCHING (MultiValue) to render each PATTERN/REGEX/EXCLUDE rule
/// as a single "logical line" in the output.
pub fn group_sub_keyword_args(args: &[String], sub_keywords: &HashSet<String>) -> Vec<String> {
    let mut groups: Vec<String> = Vec::new();
    let mut current_group = String::new();
    // Tracks whether the current group has any non-sub-keyword args yet.
    // When false, the group is still a "keyword-only prefix" and a following
    // sub_keyword should be appended (e.g., EXCLUDE followed by PATTERN).
    let mut current_has_value = false;

    for arg in args {
        if sub_keywords.contains(arg.as_str()) {
            if !current_group.is_empty() && current_has_value {
                // Current group has a value: flush and start a new group
                groups.push(std::mem::replace(&mut current_group, arg.clone()));
                current_has_value = false;
            } else if !current_group.is_empty() {
                // Current group is keyword-only prefix (e.g., EXCLUDE): append the next sub_keyword
                current_group.push(' ');
                current_group.push_str(arg);
                // Still no value appended
            } else {
                // No current group: start one
                current_group = arg.clone();
                current_has_value = false;
            }
        } else if current_group.is_empty() {
            // Arg before any sub_keyword: treat as its own group
            groups.push(arg.clone());
        } else {
            // Append value to current group
            current_group.push(' ');
            current_group.push_str(arg);
            current_has_value = true;
        }
    }
    if !current_group.is_empty() {
        groups.push(current_group);
    }
    groups
}

/// Format arguments for a keyword-aware command
///
/// `first_keyword_inline`: when true, the first keyword (SingleValue or Flag) stays on the
/// same line as the command name (used for multi-mode commands like `list(APPEND var)`
/// or `define_property(TEST PROPERTY name)`).
/// `builtin_grammar`: when true, Flag keywords after positional args stay inline
/// (e.g., `add_library(mylib STATIC ...)`). When false (user/auto-detected grammars),
/// flags break to new lines like other keywords.
/// `sub_keywords`: optional set of keywords that should be grouped within collection keywords
/// like FILES_MATCHING. When present, MultiValue sections that contain sub_keywords will
/// render logical groups (each sub_keyword + its values) per line.
#[allow(clippy::too_many_arguments)]
pub fn format_keyword_aware_args(
    arg_list: &ArgumentList,
    sections: Vec<KeywordSection>,
    config: &FormatConfig,
    indent_level: usize,
    first_keyword_inline: bool,
    builtin_grammar: bool,
    force_args_on_new_line: bool,
    sub_keywords: Option<&HashSet<String>>,
    command_name_len: usize,
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
        return format_simple_args(
            &sections,
            config,
            signals.force_multiline,
            indent_level,
            force_args_on_new_line,
        );
    }

    // Explicit indentation strings for correct tab/space handling at any nesting depth
    let base_indent = super::cst_to_doc::indent_string(indent_level, config);
    let keyword_indent = super::cst_to_doc::indent_string(indent_level + 1, config);
    let value_indent = super::cst_to_doc::indent_string(indent_level + 2, config);

    // inline_single_keyword: when enabled and there is exactly one keyword section,
    // emit the keyword inline with the pre-keyword args and use single indentation for values.
    // Guard: only inline if the pre-keyword args + keyword fit on the command's opening line.
    // If pre-keyword args are multiline (don't fit), fall through to normal keyword formatting.
    let keyword_section_count = sections.iter().filter(|s| s.keyword.is_some()).count();
    if config.inline_single_keyword && keyword_section_count == 1 {
        // Compute whether pre-keyword args + keyword fit on the command's opening line.
        // If they don't fit, the pre-keyword args will break to multiple lines and the
        // keyword should NOT be inlined — fall through to normal formatting.
        let pre_kw_section = sections.iter().find(|s| s.keyword.is_none());
        let pre_kw_fits = if let Some(pre_section) = pre_kw_section {
            // Check: would "indent + cmd_name( + pre_args_joined_by_space + space + keyword" fit?
            let indent_width = config.indent_width; // used for both tab and space modes
            let paren_extra = if config.space_between_command_parens {
                2
            } else {
                1
            }; // '(' plus optional space
            let prefix_len = indent_level * indent_width + command_name_len + paren_extra;
            let pre_args_len: usize = pre_section.args.iter().map(|a| a.len()).sum::<usize>()
                + pre_section.args.len().saturating_sub(1); // spaces between args
            let keyword_section = sections.iter().find(|s| s.keyword.is_some()).unwrap();
            let keyword_len = keyword_section.keyword.as_ref().unwrap().len();
            let total = prefix_len + pre_args_len + 1 + keyword_len; // +1 for space before keyword
            // Also reject if pre-keyword section has comments (they force line breaks)
            pre_section.comments.is_empty() && total <= config.max_line_length
        } else {
            true // No pre-keyword args — keyword is first thing, always OK to inline
        };

        if pre_kw_fits {
            return format_keyword_aware_args_inline_single(
                &sections,
                config,
                &signals,
                &base_indent,
                &keyword_indent,
            );
        }
        // Otherwise fall through to normal keyword formatting
    }

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
                    // Grouping applies here too. A `Flag` section carries the
                    // positional run that follows the flag —
                    // `add_library(l STATIC a.cpp a.h)` — and `sort_sources`
                    // already reorders that run, so skipping `source_grouping`
                    // made the two passes disagree about a list the allowlist
                    // owns.
                    let (
                        flag_args,
                        flag_blank_lines,
                        flag_comments,
                        _flag_post_comment_blanks,
                        flag_comment_blank_indices,
                    ) = grouped_section(section, config.source_grouping);
                    // Flags typically have no values, but flag_args may contain
                    // non-keyword arguments that follow before the next keyword
                    // Add separator before the flag keyword
                    if is_first_arg {
                        is_first_arg = false;
                        if first_keyword_inline {
                            // Multi-mode: first keyword stays inline with command name
                            // (no separator emitted)
                        } else if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.clone()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::nil(),
                            ));
                        }
                    } else {
                        // Consecutive flags group with space; builtin flags after positional args stay inline
                        let prev_is_flag = matches!(
                            sections.get(i.saturating_sub(1)),
                            Some(prev) if prev.keyword_type == Some(KeywordType::Flag)
                        );
                        let prev_is_pre_keyword = builtin_grammar
                            && matches!(
                                sections.get(i.saturating_sub(1)),
                                Some(prev) if prev.keyword.is_none()
                            );
                        let flag_has_trailing_args = !flag_args.is_empty();
                        if !previous_section_ended_in_a_comment(&sections, i)
                            && ((prev_is_pre_keyword && flag_has_trailing_args)
                                || ((prev_is_flag || prev_is_pre_keyword)
                                    && config.collapse_empty_flags))
                        {
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

                    // A flag with no values still carries its comments — the
                    // whole comment machinery below lives inside the
                    // `!flag_args.is_empty()` arm, so `find_package(Foo REQUIRED
                    // # note\n COMPONENTS ...)` lost `# note` outright.
                    //
                    // They are all own-line comments here whatever the author
                    // wrote: the section parser only fills `trailing_comments`
                    // for a section that already has an argument, so a comment on
                    // the same line as a valueless flag arrives at position 0 of
                    // `comments`. Each therefore takes its own line, and the
                    // separator for the *next* section refuses to collapse after
                    // one — a line comment runs to end of line, and collapsing
                    // put the next keyword inside it.
                    if flag_args.is_empty() {
                        for (_, comment) in &flag_comments {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.clone()));
                            docs.push(RcDoc::text(comment.clone()));
                        }
                    }

                    // Output any trailing non-keyword arguments in this section
                    if !flag_args.is_empty() {
                        // Use per-line when values were explicitly on new lines,
                        // or when there are comments/blank lines that can't go inline
                        let use_per_line = section.values_on_new_line
                            || !flag_comments.is_empty()
                            || !section.trailing_comments.is_empty()
                            || !flag_blank_lines.is_empty();

                        if use_per_line {
                            let mut comment_iter = flag_comments.iter().peekable();
                            let mut comment_index = 0usize;
                            for (arg_idx, arg) in flag_args.iter().enumerate() {
                                // Blank line before comments to preserve ordering
                                if flag_blank_lines.contains(&arg_idx) && signals.force_multiline {
                                    docs.push(RcDoc::hardline());
                                }
                                while let Some((pos, comment)) = comment_iter.peek() {
                                    if *pos == arg_idx {
                                        // Blank line between comment groups at same position
                                        if flag_comment_blank_indices.contains(&comment_index)
                                            && signals.force_multiline
                                        {
                                            docs.push(RcDoc::hardline());
                                        }
                                        if signals.force_multiline {
                                            docs.push(RcDoc::hardline());
                                            docs.push(RcDoc::text(keyword_indent.clone()));
                                        } else {
                                            docs.push(RcDoc::flat_alt(
                                                RcDoc::hardline()
                                                    .append(RcDoc::text(keyword_indent.clone())),
                                                RcDoc::space(),
                                            ));
                                        }
                                        docs.push(RcDoc::text(comment.clone()));
                                        comment_iter.next();
                                        comment_index += 1;
                                    } else {
                                        break;
                                    }
                                }
                                if signals.force_multiline {
                                    docs.push(RcDoc::hardline());
                                    docs.push(RcDoc::text(keyword_indent.clone()));
                                } else {
                                    docs.push(RcDoc::flat_alt(
                                        RcDoc::hardline()
                                            .append(RcDoc::text(keyword_indent.clone())),
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
                            // Blank line before trailing comments at end of section
                            if comment_iter.peek().is_some()
                                && flag_blank_lines.contains(&flag_args.len())
                                && signals.force_multiline
                            {
                                docs.push(RcDoc::hardline());
                            }
                            for (_, comment) in comment_iter {
                                // Blank line between comment groups at same position
                                if flag_comment_blank_indices.contains(&comment_index)
                                    && signals.force_multiline
                                {
                                    docs.push(RcDoc::hardline());
                                }
                                if signals.force_multiline {
                                    docs.push(RcDoc::hardline());
                                    docs.push(RcDoc::text(keyword_indent.clone()));
                                } else {
                                    docs.push(RcDoc::flat_alt(
                                        RcDoc::hardline()
                                            .append(RcDoc::text(keyword_indent.clone())),
                                        RcDoc::space(),
                                    ));
                                }
                                docs.push(RcDoc::text(comment.clone()));
                                comment_index += 1;
                            }
                        } else {
                            // Values on same line as keyword: flat_alt inherits from outer group
                            for (arg_idx, arg) in flag_args.iter().enumerate() {
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

                // SingleValue keywords: keep value inline (ignore force_multiline for
                // idempotency).
                //
                // A comment used to demote the whole section into the catch-all
                // arm, which puts the keyword on its own line and its value one
                // level deeper. That cost the fixed point as well as the layout:
                // the parser gives a comment written after this keyword's one
                // value to *this* section, while a `list(APPEND V …)` run's
                // elements live in the following keyword-less section — so
                // sorting the run to put the commented element first moved the
                // comment into this slot, and the next pass laid the command out
                // differently. `--check` then rejected freshly formatted output.
                //
                // The comments are emitted after the value instead, so the arm
                // renders the same shape with them as without.
                Some(KeywordType::SingleValue) if section.args.len() == 1 => {
                    // Add separator before the keyword
                    if is_first_arg && first_keyword_inline {
                        is_first_arg = false;
                        // Multi-mode command (e.g., list(APPEND <var>)):
                        // keep inline like ARGL-03 for first positional arg
                    } else if is_first_arg {
                        is_first_arg = false;
                        // Regular command: first keyword drops to next line when multiline
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
                        // In multi-mode commands with first_keyword_inline, group the first empty Flag
                        // with the immediately following SingleValue (e.g., TEST PROPERTY name).
                        // Only apply to section index 1 (right after first Flag at index 0).
                        let prev_is_first_empty_flag = first_keyword_inline
                            && i == 1
                            && matches!(
                                sections.first(),
                                Some(first) if first.keyword_type == Some(KeywordType::Flag) && first.args.is_empty()
                            );
                        if prev_is_first_empty_flag
                            && !previous_section_ended_in_a_comment(&sections, i)
                        {
                            docs.push(RcDoc::space());
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::space(),
                            ));
                        }
                    }
                    docs.push(RcDoc::text(keyword.clone()));
                    // Add the single value inline
                    docs.push(RcDoc::space());
                    docs.push(RcDoc::text(section.args[0].clone()));
                    // Then anything written about it, on its own line — a
                    // comment runs to end of line, so it cannot precede the
                    // value, and it must not be dropped
                    for (_, comment) in &section.trailing_comments {
                        docs.push(RcDoc::text(format!(" {}", comment)));
                    }
                    push_valueless_section_comments(&mut docs, &section.comments, &keyword_indent);
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
                    if section.args.is_empty() {
                        push_valueless_section_comments(
                            &mut docs,
                            &section.comments,
                            &keyword_indent,
                        );
                    }
                    if !section.args.is_empty() {
                        let pairs: Vec<_> = section.args.chunks(2).collect();
                        let use_per_line = section.values_on_new_line
                            || !section.comments.is_empty()
                            || !section.trailing_comments.is_empty()
                            || !section.blank_lines.is_empty();

                        if pairs.len() == 1
                            && section.comments.is_empty()
                            && section.trailing_comments.is_empty()
                        {
                            // Single pair: keep inline with keyword (e.g., PROPERTIES KEY VALUE).
                            // Guarded on comments like every other shortcut that
                            // emits the keyword and its values and nothing else —
                            // and this one is tested *before* `use_per_line`, so
                            // the comments that set that flag never reached it.
                            docs.push(RcDoc::space());
                            docs.push(RcDoc::text(pairs[0][0].clone()));
                            if pairs[0].len() > 1 {
                                docs.push(RcDoc::space());
                                docs.push(RcDoc::text(pairs[0][1].clone()));
                            }
                        } else if use_per_line || signals.force_multiline {
                            // Per-line pairs. The comments are what put us on
                            // this branch — `use_per_line` is set by them — and
                            // they were then never emitted, so a `PROPERTIES`
                            // run with comments lost every one of them.
                            let mut comment_iter = section.comments.iter().peekable();
                            for (pair_idx, chunk) in pairs.iter().enumerate() {
                                let key_index = pair_idx * 2;

                                // Own-line comments written before this pair
                                while let Some((position, comment)) = comment_iter.peek() {
                                    if *position > key_index {
                                        break;
                                    }
                                    docs.push(RcDoc::hardline());
                                    docs.push(RcDoc::text(value_indent.clone()));
                                    docs.push(RcDoc::text((*comment).clone()));
                                    comment_iter.next();
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
                                // key
                                docs.push(RcDoc::text(chunk[0].clone()));
                                // value (if present — odd number of args means last key has no value)
                                if chunk.len() > 1 {
                                    docs.push(RcDoc::space());
                                    docs.push(RcDoc::text(chunk[1].clone()));
                                }
                                // Both trailing comments come after the value. A comment runs to
                                // the end of its line, so emitting the key's before the value put
                                // the value *inside* the comment — `CXX_STANDARD # note 17` — and
                                // the next pass swallowed the following key too, losing a token per
                                // run. The inline_single_keyword twin has always emitted them in
                                // this order.
                                for (tc_index, tc_text) in &section.trailing_comments {
                                    if *tc_index == key_index
                                        || (chunk.len() > 1 && *tc_index == key_index + 1)
                                    {
                                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                                    }
                                }
                            }

                            // Anything written after the last pair
                            for (_, comment) in comment_iter {
                                docs.push(RcDoc::hardline());
                                docs.push(RcDoc::text(value_indent.clone()));
                                docs.push(RcDoc::text(comment.clone()));
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
                // A single value goes inline with its keyword — but only when
                // there is nothing else to place. This arm emits the keyword and
                // the value and nothing else, so a comment written on its own
                // line inside the section was silently deleted:
                // `target_sources(t PRIVATE\n\t# impl\n\tb.cpp)` lost `# impl`
                // entirely. The `inline_single_keyword` path already guards its
                // equivalent shortcut this way; the general path did not.
                Some(KeywordType::MultiValue)
                    if section.args.len() == 1
                        && section.comments.is_empty()
                        && section.trailing_comments.is_empty() =>
                {
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
                    if section.args.is_empty() {
                        push_valueless_section_comments(
                            &mut docs,
                            &section.comments,
                            &keyword_indent,
                        );
                    }
                    if !section.args.is_empty() {
                        let has_annotations = !section.comments.is_empty()
                            || !section.trailing_comments.is_empty()
                            || !section.blank_lines.is_empty();

                        if has_annotations {
                            // Path A: per-line with full comment support (same pattern as MultiValue use_per_line)
                            let mut comment_iter = section.comments.iter().peekable();
                            let mut comment_index = 0usize;

                            for (arg_idx, arg) in section.args.iter().enumerate() {
                                // Blank line BEFORE comments (unless this is a post-comment blank line)
                                let is_post_comment =
                                    section.post_comment_blanks.contains(&arg_idx);
                                if !is_post_comment
                                    && section.blank_lines.contains(&arg_idx)
                                    && signals.force_multiline
                                {
                                    docs.push(RcDoc::hardline());
                                }

                                while let Some((pos, comment)) = comment_iter.peek() {
                                    if *pos == arg_idx {
                                        // Blank line between comment groups at same position
                                        if section.comment_blank_indices.contains(&comment_index)
                                            && signals.force_multiline
                                        {
                                            docs.push(RcDoc::hardline());
                                        }
                                        if signals.force_multiline {
                                            docs.push(RcDoc::hardline());
                                            docs.push(RcDoc::text(value_indent.clone()));
                                        } else {
                                            docs.push(RcDoc::flat_alt(
                                                RcDoc::hardline()
                                                    .append(RcDoc::text(value_indent.clone())),
                                                RcDoc::space(),
                                            ));
                                        }
                                        docs.push(RcDoc::text(comment.clone()));
                                        comment_iter.next();
                                        comment_index += 1;
                                    } else {
                                        break;
                                    }
                                }

                                // Blank line AFTER comments (when comments preceded the blank line in source)
                                if is_post_comment
                                    && section.blank_lines.contains(&arg_idx)
                                    && signals.force_multiline
                                {
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
                                // Emit trailing comment if present
                                for (tc_idx, tc_text) in &section.trailing_comments {
                                    if *tc_idx == arg_idx {
                                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                                    }
                                }
                            }

                            // Trailing comments at end of section
                            if comment_iter.peek().is_some()
                                && section.blank_lines.contains(&section.args.len())
                                && signals.force_multiline
                            {
                                docs.push(RcDoc::hardline());
                            }
                            for (_, comment) in comment_iter {
                                // Blank line between comment groups at same position
                                if section.comment_blank_indices.contains(&comment_index)
                                    && signals.force_multiline
                                {
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
                                docs.push(RcDoc::text(comment.clone()));
                                comment_index += 1;
                            }
                        } else if signals.force_multiline {
                            // Path B: manual bin-packing with width tracking
                            // The pretty printer's group(flat_alt(...)) doesn't break inner groups
                            // when the parent is already broken, so we manually track line width.
                            let mut current_line_width = keyword_indent.len() + keyword.len();
                            for arg in &section.args {
                                let needed = 1 + arg.len();
                                if current_line_width + needed <= config.max_line_length {
                                    docs.push(RcDoc::space());
                                    docs.push(RcDoc::text(arg.clone()));
                                    current_line_width += needed;
                                } else {
                                    docs.push(RcDoc::hardline());
                                    docs.push(RcDoc::text(value_indent.clone()));
                                    docs.push(RcDoc::text(arg.clone()));
                                    current_line_width = value_indent.len() + arg.len();
                                }
                            }
                        } else {
                            // Path C: group(flat_alt(...)) for non-multiline input
                            // The pretty printer checks each inner group independently:
                            // if flat form (space + arg) fits remaining line width, stays flat;
                            // otherwise breaks to new line with value_indent
                            for arg in &section.args {
                                docs.push(RcDoc::group(
                                    RcDoc::flat_alt(
                                        RcDoc::hardline().append(RcDoc::text(value_indent.clone())),
                                        RcDoc::space(),
                                    )
                                    .append(RcDoc::text(arg.clone())),
                                ));
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
                    if section.args.is_empty() {
                        push_valueless_section_comments(
                            &mut docs,
                            &section.comments,
                            &keyword_indent,
                        );
                    }
                    if !section.args.is_empty() {
                        // Check if this is a collection keyword with sub_keyword grouping
                        // (e.g., FILES_MATCHING with PATTERN/REGEX/EXCLUDE sub-items)
                        // Only apply when: section has sub_keywords, no interleaved comments/blank lines
                        let grouped_collection = if let Some(sub_kws) = sub_keywords {
                            if !sub_kws.is_empty()
                                && section.comments.is_empty()
                                && section.trailing_comments.is_empty()
                                && section.blank_lines.is_empty()
                                && section.args.iter().any(|a| sub_kws.contains(a.as_str()))
                            {
                                Some(group_sub_keyword_args(&section.args, sub_kws))
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(groups) = grouped_collection {
                            // Collection keyword: render logical groups
                            if groups.len() == 1 {
                                // Single logical group: keep inline with the keyword
                                docs.push(RcDoc::space());
                                docs.push(RcDoc::text(groups[0].clone()));
                            } else {
                                // Multiple groups: each on its own indented line below the keyword
                                for group in &groups {
                                    if signals.force_multiline {
                                        docs.push(RcDoc::hardline());
                                        docs.push(RcDoc::text(value_indent.clone()));
                                    } else {
                                        docs.push(RcDoc::flat_alt(
                                            RcDoc::hardline()
                                                .append(RcDoc::text(value_indent.clone())),
                                            RcDoc::space(),
                                        ));
                                    }
                                    docs.push(RcDoc::text(group.clone()));
                                }
                            }
                            continue; // Section fully rendered, skip to next section
                        }

                        // Apply source grouping if enabled
                        // Disable grouping only when trailing comments are present (can't merge inline comments)
                        // Leading comments can be remapped to their new positions
                        // Blank lines are preserved as segment boundaries
                        let (
                            effective_args,
                            effective_blank_lines,
                            effective_comments,
                            effective_post_comment_blanks,
                            effective_comment_blank_indices,
                        ) = grouped_section(section, config.source_grouping);

                        // Use per-line when values were explicitly on new lines,
                        // or when there are comments/blank lines that can't go inline
                        let use_per_line = section.values_on_new_line
                            || !effective_comments.is_empty()
                            || !effective_blank_lines.is_empty();

                        if use_per_line {
                            // Values on separate lines or has comments: keep per-line behavior
                            let mut comment_iter = effective_comments.iter().peekable();
                            let mut comment_index = 0usize;

                            for (arg_idx, arg) in effective_args.iter().enumerate() {
                                // Blank line BEFORE comments (unless this is a post-comment blank line)
                                let is_post_comment =
                                    effective_post_comment_blanks.contains(&arg_idx);
                                if !is_post_comment
                                    && effective_blank_lines.contains(&arg_idx)
                                    && signals.force_multiline
                                {
                                    docs.push(RcDoc::hardline());
                                }

                                while let Some((pos, comment)) = comment_iter.peek() {
                                    if *pos == arg_idx {
                                        // Blank line between comment groups at same position
                                        if effective_comment_blank_indices.contains(&comment_index)
                                            && signals.force_multiline
                                        {
                                            docs.push(RcDoc::hardline());
                                        }
                                        if signals.force_multiline {
                                            docs.push(RcDoc::hardline());
                                            docs.push(RcDoc::text(value_indent.clone()));
                                        } else {
                                            docs.push(RcDoc::flat_alt(
                                                RcDoc::hardline()
                                                    .append(RcDoc::text(value_indent.clone())),
                                                RcDoc::space(),
                                            ));
                                        }
                                        docs.push(RcDoc::text(comment.clone()));
                                        comment_iter.next();
                                        comment_index += 1;
                                    } else {
                                        break;
                                    }
                                }

                                // Blank line AFTER comments (when comments preceded the blank line in source)
                                if is_post_comment
                                    && effective_blank_lines.contains(&arg_idx)
                                    && signals.force_multiline
                                {
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
                                // Emit trailing comment if present
                                for (tc_idx, tc_text) in &section.trailing_comments {
                                    if *tc_idx == arg_idx {
                                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                                    }
                                }
                            }

                            // Blank line before trailing comments at end of section
                            if comment_iter.peek().is_some()
                                && effective_blank_lines.contains(&effective_args.len())
                                && signals.force_multiline
                            {
                                docs.push(RcDoc::hardline());
                            }
                            for (_, comment) in comment_iter {
                                // Blank line between comment groups at same position
                                if effective_comment_blank_indices.contains(&comment_index)
                                    && signals.force_multiline
                                {
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
                                docs.push(RcDoc::text(comment.clone()));
                                comment_index += 1;
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
            // Disable grouping only when trailing comments are present (can't merge inline comments)
            // Leading comments can be remapped to their new positions
            // Blank lines are preserved as segment boundaries
            let (
                effective_args,
                effective_blank_lines,
                effective_comments,
                effective_post_comment_blanks,
                effective_comment_blank_indices,
            ) = grouped_section(section, config.source_grouping);

            let is_list = effective_args.len() > 1 || force_args_on_new_line;
            let mut comment_iter = effective_comments.iter().peekable();
            let mut comment_index = 0usize;

            for (arg_idx, arg) in effective_args.iter().enumerate() {
                // Blank line BEFORE comments (unless this is a post-comment blank line)
                let is_post_comment = effective_post_comment_blanks.contains(&arg_idx);
                if !is_post_comment
                    && effective_blank_lines.contains(&arg_idx)
                    && signals.force_multiline
                {
                    docs.push(RcDoc::hardline());
                    is_first_arg = false;
                }

                // Emit comments before this argument
                while let Some((pos, comment)) = comment_iter.peek() {
                    if *pos == arg_idx {
                        // Blank line between comment groups at same position
                        if effective_comment_blank_indices.contains(&comment_index)
                            && signals.force_multiline
                        {
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
                        docs.push(RcDoc::text(comment.clone()));
                        comment_iter.next();
                        comment_index += 1;
                        is_first_arg = false;
                    } else {
                        break;
                    }
                }

                // Blank line AFTER comments (when comments preceded the blank line in source)
                if is_post_comment
                    && effective_blank_lines.contains(&arg_idx)
                    && signals.force_multiline
                {
                    docs.push(RcDoc::hardline());
                    is_first_arg = false;
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

            // Blank line before trailing comments at end of section
            if comment_iter.peek().is_some()
                && effective_blank_lines.contains(&effective_args.len())
                && signals.force_multiline
            {
                docs.push(RcDoc::hardline());
            }
            // Emit trailing comments (after last argument)
            for (_, comment) in comment_iter {
                // Blank line between comment groups at same position
                if effective_comment_blank_indices.contains(&comment_index)
                    && signals.force_multiline
                {
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
                docs.push(RcDoc::text(comment.clone()));
                comment_index += 1;
            }
        }
    }

    // Closing paren position
    docs.push(super::cst_to_doc::closing_paren_position(
        config,
        indent_level,
        signals.force_multiline,
    ));

    let combined = RcDoc::concat(docs);

    if signals.force_multiline {
        combined
    } else {
        combined.group()
    }
}

/// Format keyword-aware args with a single keyword section using inline layout.
///
/// When `inline_single_keyword = true` and there is exactly one keyword section:
/// - The keyword is emitted on the SAME line as the pre-keyword args (with a space separator).
/// - Values under the keyword are indented at `keyword_indent` level (one level, not two).
/// - The closing paren is at `base_indent` level as usual.
///
/// When the command fits on one line, flat rendering keeps everything inline (unchanged).
fn format_keyword_aware_args_inline_single(
    sections: &[KeywordSection],
    config: &FormatConfig,
    signals: &super::cst_to_doc::ArgumentFormatSignals,
    base_indent: &str,
    keyword_indent: &str,
) -> RcDoc<'static, ()> {
    let mut docs = Vec::new();
    let mut is_first_arg = true;

    for (i, section) in sections.iter().enumerate() {
        if let Some(keyword) = &section.keyword {
            // There is exactly one keyword section — emit keyword INLINE with preceding args.
            // Separator: space (flat and broken both use space here)
            if is_first_arg {
                // Keyword is the very first thing (no pre-keyword args).
                // In inline_single_keyword mode the keyword appears directly after '(' — no separator.
                // This is correct for both flat and broken (force_multiline) rendering since the
                // keyword's position on the opening line is the whole point of this function.
                is_first_arg = false;
                if !signals.force_multiline {
                    docs.push(RcDoc::flat_alt(
                        RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                        RcDoc::nil(),
                    ));
                }
                // force_multiline=true: emit nothing — keyword stays on the opening line after '('
            } else if previous_section_ended_in_a_comment(sections, i) {
                // The pre-keyword args ended in a comment, so this keyword cannot
                // share their line — it would become part of the comment.
                is_first_arg = false;
                docs.push(RcDoc::hardline());
                docs.push(RcDoc::text(keyword_indent.to_string()));
            } else {
                // Keyword follows pre-keyword args — stays on same line as the last pre-keyword arg.
                // In flat mode this is already inline; in broken mode we want a space (not a newline).
                is_first_arg = false;
                docs.push(RcDoc::space());
            }
            docs.push(RcDoc::text(keyword.clone()));

            // For SingleValue keywords with exactly one arg, render the value inline (same line as
            // the keyword) rather than indented below. This keeps "APPEND SOURCES" together on the
            // opening line when the overflow positional args follow in the next section.
            let is_single_value_with_one_arg = section.keyword_type
                == Some(KeywordType::SingleValue)
                && section.args.len() == 1
                && section.comments.is_empty()
                && section.trailing_comments.is_empty()
                && section.blank_lines.is_empty();

            let is_pair_value = section.keyword_type == Some(KeywordType::PairValue);

            if is_single_value_with_one_arg {
                // Always emit inline with a space — even when force_multiline is true.
                docs.push(RcDoc::space());
                docs.push(RcDoc::text(section.args[0].clone()));
            } else if is_pair_value && !section.args.is_empty() {
                // PairValue keywords (e.g., PROPERTIES): format as key-value pairs
                // Use keyword_indent (single indent) since the keyword is inlined on the command line
                let pairs: Vec<_> = section.args.chunks(2).collect();
                let use_per_line = section.values_on_new_line
                    || !section.comments.is_empty()
                    || !section.trailing_comments.is_empty()
                    || !section.blank_lines.is_empty();

                if pairs.len() == 1
                    && section.comments.is_empty()
                    && section.trailing_comments.is_empty()
                {
                    // Single pair: keep inline with keyword. Same guard as the
                    // general path's twin, for the same reason.
                    docs.push(RcDoc::space());
                    docs.push(RcDoc::text(pairs[0][0].clone()));
                    if pairs[0].len() > 1 {
                        docs.push(RcDoc::space());
                        docs.push(RcDoc::text(pairs[0][1].clone()));
                    }
                } else if use_per_line || signals.force_multiline {
                    let mut pair_comments = section.comments.iter().peekable();
                    for (pair_idx, chunk) in pairs.iter().enumerate() {
                        if section.blank_lines.contains(&(pair_idx * 2)) && signals.force_multiline
                        {
                            docs.push(RcDoc::hardline());
                        }
                        // Own-line comments written before this pair. This loop
                        // was missing entirely, so the twin emitted only trailing
                        // comments and deleted every own-line one.
                        while let Some((position, comment)) = pair_comments.peek() {
                            if *position > pair_idx * 2 {
                                break;
                            }
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.to_string()));
                            docs.push(RcDoc::text((*comment).clone()));
                            pair_comments.next();
                        }
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.to_string()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                                RcDoc::space(),
                            ));
                        }
                        docs.push(RcDoc::text(chunk[0].clone()));
                        if chunk.len() > 1 {
                            docs.push(RcDoc::space());
                            docs.push(RcDoc::text(chunk[1].clone()));
                        }
                        // Trailing comments for key (even index) and value (odd index)
                        for (tc_idx, tc_text) in &section.trailing_comments {
                            let key_idx = pair_idx * 2;
                            if *tc_idx == key_idx || (chunk.len() > 1 && *tc_idx == key_idx + 1) {
                                docs.push(RcDoc::text(format!(" {}", tc_text)));
                            }
                        }
                    }
                    // Anything written after the last pair
                    for (_, comment) in pair_comments {
                        docs.push(RcDoc::hardline());
                        docs.push(RcDoc::text(keyword_indent.to_string()));
                        docs.push(RcDoc::text(comment.clone()));
                    }
                } else {
                    for chunk in pairs {
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                            RcDoc::space(),
                        ));
                        docs.push(RcDoc::text(chunk[0].clone()));
                        if chunk.len() > 1 {
                            docs.push(RcDoc::space());
                            docs.push(RcDoc::text(chunk[1].clone()));
                        }
                    }
                }
            } else if section.args.is_empty() {
                push_valueless_section_comments(&mut docs, &section.comments, keyword_indent);
            } else {
                // Apply source grouping to keyword section args (e.g., source files after PUBLIC)
                let (
                    effective_args,
                    effective_blank_lines,
                    effective_comments,
                    _effective_post_comment_blanks,
                    effective_comment_blank_indices,
                ) = grouped_section(section, config.source_grouping);

                // Values are indented at keyword_indent level (single indent, not double)
                let use_per_line = section.values_on_new_line
                    || !effective_comments.is_empty()
                    || !section.trailing_comments.is_empty()
                    || !effective_blank_lines.is_empty();

                if use_per_line {
                    let mut comment_iter = effective_comments.iter().peekable();
                    let mut comment_index = 0usize;
                    for (arg_idx, arg) in effective_args.iter().enumerate() {
                        if effective_blank_lines.contains(&arg_idx) && signals.force_multiline {
                            docs.push(RcDoc::hardline());
                        }
                        while let Some((pos, comment)) = comment_iter.peek() {
                            if *pos == arg_idx {
                                if effective_comment_blank_indices.contains(&comment_index)
                                    && signals.force_multiline
                                {
                                    docs.push(RcDoc::hardline());
                                }
                                if signals.force_multiline {
                                    docs.push(RcDoc::hardline());
                                    docs.push(RcDoc::text(keyword_indent.to_string()));
                                } else {
                                    docs.push(RcDoc::flat_alt(
                                        RcDoc::hardline()
                                            .append(RcDoc::text(keyword_indent.to_string())),
                                        RcDoc::space(),
                                    ));
                                }
                                docs.push(RcDoc::text(comment.clone()));
                                comment_iter.next();
                                comment_index += 1;
                            } else {
                                break;
                            }
                        }
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.to_string()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                                RcDoc::space(),
                            ));
                        }
                        docs.push(RcDoc::text(arg.clone()));
                        for (tc_idx, tc_text) in &section.trailing_comments {
                            if *tc_idx == arg_idx {
                                docs.push(RcDoc::text(format!(" {}", tc_text)));
                            }
                        }
                    }
                    if comment_iter.peek().is_some()
                        && effective_blank_lines.contains(&effective_args.len())
                        && signals.force_multiline
                    {
                        docs.push(RcDoc::hardline());
                    }
                    for (_, comment) in comment_iter {
                        if effective_comment_blank_indices.contains(&comment_index)
                            && signals.force_multiline
                        {
                            docs.push(RcDoc::hardline());
                        }
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.to_string()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                                RcDoc::space(),
                            ));
                        }
                        docs.push(RcDoc::text(comment.clone()));
                        comment_index += 1;
                    }
                } else {
                    // Flat layout: values go on new lines below keyword (single indent) when broken
                    for (arg_idx, arg) in effective_args.iter().enumerate() {
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                            RcDoc::space(),
                        ));
                        docs.push(RcDoc::text(arg.clone()));
                        for (tc_idx, tc_text) in &section.trailing_comments {
                            if *tc_idx == arg_idx {
                                docs.push(RcDoc::text(format!(" {}", tc_text)));
                            }
                        }
                    }
                }
            }
        } else {
            // Pre-keyword section: same as the main loop's pre-keyword handling
            let (
                effective_args,
                effective_blank_lines,
                effective_comments,
                effective_post_comment_blanks,
                effective_comment_blank_indices,
            ) = grouped_section(section, config.source_grouping);

            let is_list = effective_args.len() > 1;
            let mut comment_iter = effective_comments.iter().peekable();
            let mut comment_index = 0usize;

            for (arg_idx, arg) in effective_args.iter().enumerate() {
                let is_post_comment = effective_post_comment_blanks.contains(&arg_idx);
                if !is_post_comment
                    && effective_blank_lines.contains(&arg_idx)
                    && signals.force_multiline
                {
                    docs.push(RcDoc::hardline());
                    is_first_arg = false;
                }

                while let Some((pos, comment)) = comment_iter.peek() {
                    if *pos == arg_idx {
                        if effective_comment_blank_indices.contains(&comment_index)
                            && signals.force_multiline
                        {
                            docs.push(RcDoc::hardline());
                        }
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.to_string()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                                RcDoc::space(),
                            ));
                        }
                        docs.push(RcDoc::text(comment.clone()));
                        comment_iter.next();
                        comment_index += 1;
                        is_first_arg = false;
                    } else {
                        break;
                    }
                }

                if is_post_comment
                    && effective_blank_lines.contains(&arg_idx)
                    && signals.force_multiline
                {
                    docs.push(RcDoc::hardline());
                    is_first_arg = false;
                }

                if is_first_arg && !is_list {
                    is_first_arg = false;
                } else if is_first_arg {
                    is_first_arg = false;
                    if signals.force_multiline {
                        docs.push(RcDoc::hardline());
                        docs.push(RcDoc::text(keyword_indent.to_string()));
                    } else {
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                            RcDoc::nil(),
                        ));
                    }
                } else if signals.force_multiline {
                    docs.push(RcDoc::hardline());
                    docs.push(RcDoc::text(keyword_indent.to_string()));
                } else {
                    docs.push(RcDoc::flat_alt(
                        RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                        RcDoc::space(),
                    ));
                }
                docs.push(RcDoc::text(arg.clone()));
                for (tc_idx, tc_text) in &section.trailing_comments {
                    if *tc_idx == arg_idx {
                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                    }
                }
            }

            if comment_iter.peek().is_some()
                && effective_blank_lines.contains(&effective_args.len())
                && signals.force_multiline
            {
                docs.push(RcDoc::hardline());
            }
            for (_, comment) in comment_iter {
                if effective_comment_blank_indices.contains(&comment_index)
                    && signals.force_multiline
                {
                    docs.push(RcDoc::hardline());
                }
                if signals.force_multiline {
                    docs.push(RcDoc::hardline());
                    docs.push(RcDoc::text(keyword_indent.to_string()));
                } else {
                    docs.push(RcDoc::flat_alt(
                        RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                        RcDoc::space(),
                    ));
                }
                docs.push(RcDoc::text(comment.clone()));
                comment_index += 1;
            }
        }
    }

    // Closing paren position
    // Need indent_level to compute closing indent — derive from base_indent length
    // The base_indent is already computed for us; use closing_paren_position with a reconstructed level.
    // Since format_keyword_aware_args_inline_single doesn't take indent_level directly,
    // we push the closing paren directly using the same logic as closing_paren_position.
    let closing_indent = if config.indent_closing_paren {
        // We need to add one extra indent level. base_indent is the current level string.
        // Compute by checking config to get a single indent unit.
        let one_indent = if config.use_tabs {
            "\t".to_string()
        } else {
            " ".repeat(config.indent_width)
        };
        format!("{}{}", base_indent, one_indent)
    } else {
        base_indent.to_string()
    };
    let flat_text = if config.space_between_command_parens {
        RcDoc::text(" ")
    } else {
        RcDoc::nil()
    };
    if signals.force_multiline {
        docs.push(RcDoc::hardline());
        docs.push(RcDoc::text(closing_indent));
    } else {
        docs.push(RcDoc::flat_alt(
            RcDoc::hardline().append(RcDoc::text(closing_indent)),
            flat_text,
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
fn format_simple_args(
    sections: &[KeywordSection],
    config: &FormatConfig,
    force_multiline: bool,
    indent_level: usize,
    force_args_on_new_line: bool,
) -> RcDoc<'static, ()> {
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
        // Disable grouping only when trailing comments are present (can't merge inline comments)
        // Leading comments can be remapped to their new positions
        // Blank lines are preserved as segment boundaries
        let (
            effective_args,
            effective_blank_lines,
            effective_comments,
            effective_post_comment_blanks,
            effective_comment_blank_indices,
        ) = grouped_section(section, config.source_grouping);

        let mut comment_iter = effective_comments.iter().peekable();
        let mut comment_index = 0usize;

        for (arg_idx, arg) in effective_args.iter().enumerate() {
            // Blank line BEFORE comments (unless this is a post-comment blank line)
            let is_post_comment = effective_post_comment_blanks.contains(&arg_idx);
            if !is_post_comment && effective_blank_lines.contains(&arg_idx) && force_multiline {
                docs.push(RcDoc::hardline());
                is_first_arg = false;
            }

            // Emit comments before this argument
            while let Some((pos, comment)) = comment_iter.peek() {
                if *pos == arg_idx {
                    // Blank line between comment groups at same position
                    if effective_comment_blank_indices.contains(&comment_index) && force_multiline {
                        docs.push(RcDoc::hardline());
                    }
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
                    comment_index += 1;
                    is_first_arg = false;
                } else {
                    break;
                }
            }

            // Blank line AFTER comments (when comments preceded the blank line in source)
            if is_post_comment && effective_blank_lines.contains(&arg_idx) && force_multiline {
                docs.push(RcDoc::hardline());
                is_first_arg = false;
            }

            // Add separator before arg (except for the very first arg, unless force_args_on_new_line)
            if !is_first_arg || force_args_on_new_line {
                if force_multiline {
                    docs.push(RcDoc::hardline());
                    docs.push(RcDoc::text(inner_indent.clone()));
                } else {
                    // For the first arg with force_args_on_new_line, use nil in flat mode
                    // (no space before first arg when everything fits on one line)
                    let flat = if is_first_arg {
                        RcDoc::nil()
                    } else {
                        RcDoc::space()
                    };
                    docs.push(RcDoc::flat_alt(
                        RcDoc::hardline().append(RcDoc::text(inner_indent.clone())),
                        flat,
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

        // Blank line before trailing comments at end of section
        if comment_iter.peek().is_some()
            && effective_blank_lines.contains(&effective_args.len())
            && force_multiline
        {
            docs.push(RcDoc::hardline());
        }
        // Emit trailing comments (after last argument)
        for (_, comment) in comment_iter {
            // Blank line between comment groups at same position
            if effective_comment_blank_indices.contains(&comment_index) && force_multiline {
                docs.push(RcDoc::hardline());
            }
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
            comment_index += 1;
            is_first_arg = false;
        }
    }

    // Closing paren position
    docs.push(super::cst_to_doc::closing_paren_position(
        config,
        indent_level,
        force_multiline,
    ));

    let combined = RcDoc::concat(docs);

    if force_multiline {
        combined
    } else {
        combined.group()
    }
}
