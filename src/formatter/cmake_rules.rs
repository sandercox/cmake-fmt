use crate::cst::ArgumentList;
use crate::syntax_kind::SyntaxKind;
use pretty::RcDoc;
use rowan::NodeOrToken;

use super::config::FormatConfig;

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
}

/// Parse an argument list into keyword sections
pub fn parse_keyword_sections(arg_list: &ArgumentList) -> Vec<KeywordSection> {
    let mut sections = Vec::new();
    let mut current_section = KeywordSection {
        keyword: None,
        args: Vec::new(),
    };

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
                    // Check if this argument is a keyword
                    if is_cmake_keyword(&text) {
                        // Start a new section
                        if !current_section.args.is_empty() || current_section.keyword.is_some() {
                            sections.push(current_section);
                        }
                        current_section = KeywordSection {
                            keyword: Some(text),
                            args: Vec::new(),
                        };
                    } else {
                        // Add as argument to current section
                        current_section.args.push(text);
                    }
                }
                // Skip whitespace and comments (handled separately)
                SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE | SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {}
                _ => {}
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
    sections: Vec<KeywordSection>,
    config: &FormatConfig,
) -> RcDoc<'static, ()> {
    if sections.is_empty() {
        return RcDoc::nil();
    }

    // Check if we have any actual keywords (not just pre-keyword args)
    let has_keywords = sections.iter().any(|s| s.keyword.is_some());

    if !has_keywords {
        // No keywords found, fall back to simple formatting
        return format_simple_args(&sections, config);
    }

    // Build keyword-aware Doc structure
    let mut docs = vec![RcDoc::line_()]; // Soft line at start (empty when flat)

    for (i, section) in sections.iter().enumerate() {
        if let Some(keyword) = &section.keyword {
            // Keyword on its own line at 1 indent level
            docs.push(RcDoc::line());
            docs.push(RcDoc::text(keyword.clone()));

            // Values under the keyword at 2 indent levels (extra nest)
            if !section.args.is_empty() {
                let mut value_docs = Vec::new();
                for arg in &section.args {
                    value_docs.push(RcDoc::line());
                    value_docs.push(RcDoc::text(arg.clone()));
                }
                docs.push(
                    RcDoc::concat(value_docs)
                        .nest(config.indent_width as isize)
                );
            }
        } else {
            // Pre-keyword arguments (e.g., target name)
            // These go on the same line as the command
            for (j, arg) in section.args.iter().enumerate() {
                // First arg of first section: no separator (line_() at start handles it)
                // Other args: add space/newline separator
                if i > 0 || j > 0 {
                    docs.push(RcDoc::line());
                }
                docs.push(RcDoc::text(arg.clone()));
            }
        }
    }

    // Add soft line at the end
    docs.push(RcDoc::line_());

    // Nest everything and wrap in group
    // Group tries flat first; if too long, breaks with proper indentation
    RcDoc::concat(docs)
        .nest(config.indent_width as isize)
        .group()
}

/// Format arguments without keyword awareness (simple line breaking)
fn format_simple_args(sections: &[KeywordSection], config: &FormatConfig) -> RcDoc<'static, ()> {
    let mut docs = vec![RcDoc::line_()];

    // Collect all args from all sections
    let all_args: Vec<&String> = sections
        .iter()
        .flat_map(|s| s.args.iter())
        .collect();

    for (i, arg) in all_args.iter().enumerate() {
        docs.push(RcDoc::text((*arg).clone()));

        if i < all_args.len() - 1 {
            docs.push(RcDoc::line());
        }
    }

    docs.push(RcDoc::line_());

    RcDoc::concat(docs)
        .nest(config.indent_width as isize)
        .group()
}
