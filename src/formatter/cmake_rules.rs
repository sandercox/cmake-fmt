use crate::cst::ArgumentList;
use crate::syntax_kind::SyntaxKind;
use pretty::RcDoc;
use rowan::NodeOrToken;

use super::config::FormatConfig;
use super::cst_to_doc::detect_argument_formatting_signals;

/// Check if a command name requires keyword-aware formatting
pub fn is_keyword_aware_command(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "target_link_libraries"
            | "target_sources"
            | "target_compile_options"
            | "target_include_directories"
            | "target_compile_definitions"
            | "add_library"
            | "add_executable"
            | "install"
            | "set_target_properties"
            | "set_property"
            | "get_property"
    )
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
}

/// Parse an argument list into keyword sections
pub fn parse_keyword_sections(arg_list: &ArgumentList) -> Vec<KeywordSection> {
    let mut sections = Vec::new();
    let mut current_section = KeywordSection {
        keyword: None,
        args: Vec::new(),
        comments: Vec::new(),
        blank_lines: Vec::new(),
    };

    let mut consecutive_newlines = 0;
    let mut saw_separator = true; // tracks whitespace for adjacent token merging

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
                    if is_cmake_keyword(&text) {
                        // Start a new section
                        if !current_section.args.is_empty() || current_section.keyword.is_some() {
                            sections.push(current_section);
                        }
                        current_section = KeywordSection {
                            keyword: Some(text),
                            args: Vec::new(),
                            comments: Vec::new(),
                            blank_lines: Vec::new(),
                        };
                        saw_separator = true;
                    } else if !saw_separator && !current_section.args.is_empty() {
                        // Adjacent to previous token (no whitespace) — merge
                        // e.g. ${VAR}/suffix is two tokens but one logical argument
                        current_section.args.last_mut().unwrap().push_str(&text);
                        saw_separator = false;
                    } else {
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

    // Detect formatting signals from the argument list
    let signals = detect_argument_formatting_signals(arg_list);

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
            // Keyword: ARGL-03 first arg stays on same line as command
            if is_first_arg {
                // First keyword is right after `(` — no separator
                is_first_arg = false;
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
                let mut comment_iter = section.comments.iter().peekable();

                for (arg_idx, arg) in section.args.iter().enumerate() {
                    // Emit comments before this argument
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

                    // Check for blank line before this argument
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

                // Emit trailing comments (after last argument)
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
            }
        } else {
            // Pre-keyword arguments (e.g., target name)
            // ARGL-03: first arg stays on same line as command
            for (_j, arg) in section.args.iter().enumerate() {
                if is_first_arg {
                    // First arg: no separator
                    is_first_arg = false;
                } else {
                    // Other args: add separator with explicit indentation
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
        let mut comment_iter = section.comments.iter().peekable();

        for (arg_idx, arg) in section.args.iter().enumerate() {
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
