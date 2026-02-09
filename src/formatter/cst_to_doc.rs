use crate::cst::{ArgumentList, CSTRoot, CommandInvocation};
use crate::syntax_kind::SyntaxKind;
use crate::SyntaxNode;
use pretty::RcDoc;
use rowan::NodeOrToken;

use super::config::{CommandCase, FormatConfig};
use super::cmake_rules;
use super::comments;

/// Signals detected in argument list that affect formatting
struct ArgumentFormatSignals {
    has_comments: bool,
    has_blank_lines: bool,
    has_newlines: bool,
    force_multiline: bool,
}

/// Format context tracking indentation level
struct FormatContext<'a> {
    config: &'a FormatConfig,
    indent_level: usize,
}

impl<'a> FormatContext<'a> {
    fn new(config: &'a FormatConfig) -> Self {
        Self {
            config,
            indent_level: 0,
        }
    }

    /// Get the indentation string for the current level
    fn indent_str(&self) -> String {
        let count = self.indent_level * self.config.indent_width;
        if self.config.use_tabs {
            "\t".repeat(self.indent_level)
        } else {
            " ".repeat(count)
        }
    }
}

/// Entry point: convert CST to Doc IR
pub fn format_cst(cst: &CSTRoot, config: &FormatConfig) -> RcDoc<'static, ()> {
    let ctx = FormatContext::new(config);
    format_file(&cst.root, &ctx)
}

/// Collect all trailing comments in the file
fn collect_trailing_comments(node: &SyntaxNode) -> std::collections::HashSet<String> {
    let mut trailing_comments = std::collections::HashSet::new();
    for child in node.children_with_tokens() {
        if let NodeOrToken::Node(child_node) = &child {
            if child_node.kind() == SyntaxKind::COMMAND_INVOCATION {
                if let Some(trailing) = comments::extract_trailing_comment(child_node) {
                    trailing_comments.insert(trailing);
                }
            }
        }
    }
    trailing_comments
}

/// Format the FILE node
fn format_file(node: &SyntaxNode, ctx: &FormatContext) -> RcDoc<'static, ()> {
    let mut docs = Vec::new();
    let mut current_indent: usize = 0;
    let mut blank_line_count = 0;

    // First pass: collect all comments that will be handled as leading/trailing
    // Trailing comments take precedence over leading comments (to avoid duplication)
    let mut handled_comments = std::collections::HashSet::new();
    let trailing_comments = collect_trailing_comments(node);

    // Add trailing comments to handled set
    for trailing in &trailing_comments {
        handled_comments.insert(trailing.clone());
    }

    // Then collect leading comments, but skip those that are trailing comments
    for child in node.children_with_tokens() {
        if let NodeOrToken::Node(child_node) = &child {
            if child_node.kind() == SyntaxKind::COMMAND_INVOCATION {
                for comment in comments::extract_leading_comments(child_node) {
                    // Don't mark as handled if it's a trailing comment
                    if !trailing_comments.contains(&comment) {
                        handled_comments.insert(comment);
                    }
                }
            }
        }
    }

    // Second pass: emit formatted output
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Node(child_node) => {
                match child_node.kind() {
                    SyntaxKind::COMMAND_INVOCATION => {
                        // Check for leading comments FIRST
                        let leading_comments = comments::extract_leading_comments(&child_node);

                        // Emit accumulated blank lines before command/comments
                        // But only if not first content
                        if blank_line_count >= 2 && !docs.is_empty() {
                            let blank_lines_to_emit = std::cmp::min(blank_line_count - 1, ctx.config.max_blank_lines);
                            for _ in 0..blank_lines_to_emit {
                                docs.push(RcDoc::hardline());
                            }
                        }

                        // Emit leading comments (skip if already handled as trailing)
                        let indent_str = indent_string(current_indent, ctx.config);
                        for comment in &leading_comments {
                            // Only emit if not already handled as a trailing comment
                            if !trailing_comments.contains(comment) {
                                docs.push(RcDoc::text(format!("{}{}", indent_str, comment)));
                                docs.push(RcDoc::hardline());
                            }
                        }

                        // Determine command name and adjust indentation
                        if let Some(cmd) = CommandInvocation::cast(child_node.clone()) {
                            let cmd_name = cmd.name_text()
                                .map(|s| s.to_lowercase())
                                .unwrap_or_default();

                            // Handle block closers (dedent before emitting)
                            if is_block_closer(&cmd_name) {
                                current_indent = current_indent.saturating_sub(1);
                            }

                            // Handle mid-block commands (dedent for this line only)
                            let temp_dedent = is_block_mid(&cmd_name);
                            let cmd_indent = if temp_dedent {
                                current_indent.saturating_sub(1)
                            } else {
                                current_indent
                            };

                            // Create context for this command
                            let cmd_ctx = FormatContext {
                                config: ctx.config,
                                indent_level: cmd_indent,
                            };

                            // Format and emit command
                            let mut cmd_doc = format_command(&cmd, &cmd_ctx);

                            // Check for trailing comment
                            if let Some(trailing_comment) = comments::extract_trailing_comment(&child_node) {
                                cmd_doc = cmd_doc.append(RcDoc::space()).append(RcDoc::text(trailing_comment));
                            }

                            docs.push(cmd_doc);
                            docs.push(RcDoc::hardline());

                            // Handle block openers (indent after emitting)
                            if is_block_opener(&cmd_name) {
                                current_indent += 1;
                            }

                            blank_line_count = 0;
                        }
                    }
                    SyntaxKind::ERROR => {
                        // Emit accumulated blank lines before error
                        // But only if not first content
                        if blank_line_count >= 2 && !docs.is_empty() {
                            let blank_lines_to_emit = std::cmp::min(blank_line_count - 1, ctx.config.max_blank_lines);
                            for _ in 0..blank_lines_to_emit {
                                docs.push(RcDoc::hardline());
                            }
                        }

                        // Preserve error nodes verbatim
                        let text = child_node.text().to_string();
                        if !text.trim().is_empty() {
                            docs.push(RcDoc::text(text));
                            docs.push(RcDoc::hardline());
                        }
                        blank_line_count = 0;
                    }
                    _ => {}
                }
            }
            NodeOrToken::Token(token) => {
                match token.kind() {
                    SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
                        // Only emit standalone comments (not already handled)
                        let comment_text = token.text().to_string();
                        if !handled_comments.contains(&comment_text) {
                            // Emit accumulated blank lines before standalone comment
                            // But only if not first content
                            if blank_line_count >= 2 && !docs.is_empty() {
                                let blank_lines_to_emit = std::cmp::min(blank_line_count - 1, ctx.config.max_blank_lines);
                                for _ in 0..blank_lines_to_emit {
                                    docs.push(RcDoc::hardline());
                                }
                            }

                            let indent_str = indent_string(current_indent, ctx.config);
                            docs.push(RcDoc::text(format!("{}{}", indent_str, comment_text)));
                            docs.push(RcDoc::hardline());
                            blank_line_count = 0;
                        }
                        // Don't reset blank_line_count for handled comments - they're part of leading comments
                    }
                    SyntaxKind::NEWLINE => {
                        blank_line_count += 1;
                    }
                    SyntaxKind::WHITESPACE => {
                        // Skip - formatter decides whitespace
                    }
                    _ => {}
                }
            }
        }
    }

    RcDoc::concat(docs)
}

/// Format a command invocation
fn format_command(cmd: &CommandInvocation, ctx: &FormatContext) -> RcDoc<'static, ()> {
    let indent_str = ctx.indent_str();

    // Get command name and apply casing
    let name = cmd.name_text().unwrap_or_else(|| "unknown".to_string());
    let formatted_name = match ctx.config.command_case {
        CommandCase::Lowercase => name.to_lowercase(),
        CommandCase::Uppercase => name.to_uppercase(),
        CommandCase::Preserve => name.clone(),
    };

    // Get arguments and check for keyword-aware formatting
    let args_doc = if let Some(arg_list) = cmd.argument_list() {
        // Check if this command should use keyword-aware formatting
        if cmake_rules::is_keyword_aware_command(&name) {
            let sections = cmake_rules::parse_keyword_sections(&arg_list);
            cmake_rules::format_keyword_aware_args(sections, ctx.config)
        } else {
            format_argument_list(&arg_list, ctx)
        }
    } else {
        RcDoc::nil()
    };

    // Format as: indent + name + ( + args + )
    let cmd_doc = RcDoc::text(formatted_name)
        .append(RcDoc::text("("))
        .append(args_doc)
        .append(RcDoc::text(")"));

    RcDoc::text(indent_str).append(cmd_doc)
}

/// Detect formatting signals in argument list (comments, blank lines, newlines)
fn detect_argument_formatting_signals(arg_list: &ArgumentList) -> ArgumentFormatSignals {
    let mut has_comments = false;
    let mut has_blank_lines = false;
    let mut has_newlines = false;
    let mut consecutive_newline_count = 0;

    for child in arg_list.syntax().children_with_tokens() {
        match child {
            NodeOrToken::Token(token) => {
                match token.kind() {
                    SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
                        has_comments = true;
                        consecutive_newline_count = 0;
                    }
                    SyntaxKind::NEWLINE => {
                        has_newlines = true;
                        consecutive_newline_count += 1;
                        if consecutive_newline_count >= 2 {
                            has_blank_lines = true;
                        }
                    }
                    SyntaxKind::WHITESPACE => {
                        // Whitespace doesn't reset newline count
                    }
                    _ => {
                        // Any other token resets newline count
                        consecutive_newline_count = 0;
                    }
                }
            }
            NodeOrToken::Node(_) => {
                // Nodes reset newline count
                consecutive_newline_count = 0;
            }
        }
    }

    let force_multiline = has_comments || has_blank_lines || has_newlines;

    ArgumentFormatSignals {
        has_comments,
        has_blank_lines,
        has_newlines,
        force_multiline,
    }
}

/// Format an argument list with intelligent line breaking
fn format_argument_list(arg_list: &ArgumentList, ctx: &FormatContext) -> RcDoc<'static, ()> {
    let args: Vec<_> = arg_list.arguments().collect();

    if args.is_empty() {
        return RcDoc::nil();
    }

    // Detect formatting signals
    let signals = detect_argument_formatting_signals(arg_list);

    // If no multiline signals, use original behavior (soft lines + group)
    if !signals.force_multiline {
        let mut docs = vec![RcDoc::line_()]; // Soft line at start

        for (i, arg) in args.iter().enumerate() {
            let text = arg.text().to_string();
            docs.push(RcDoc::text(text));

            // Add separator between arguments (not after last)
            if i < args.len() - 1 {
                docs.push(RcDoc::line());
            }
        }

        docs.push(RcDoc::line_()); // Soft line at end

        // Group everything together: tries flat first, breaks if too long
        return RcDoc::concat(docs)
            .nest(ctx.config.indent_width as isize)
            .group();
    }

    // Force multiline: walk tokens and build Doc IR with hardlines
    let mut docs = vec![RcDoc::line_()]; // First break - collapses to nothing (ARGL-03)
    let mut last_was_arg = false;
    let mut consecutive_newline_count = 0;
    let mut first_arg = true;

    for child in arg_list.syntax().children_with_tokens() {
        match child {
            NodeOrToken::Token(token) => {
                match token.kind() {
                    SyntaxKind::UNQUOTED_ARGUMENT
                    | SyntaxKind::QUOTED_ARGUMENT
                    | SyntaxKind::BRACKET_ARGUMENT
                    | SyntaxKind::VARIABLE_REF
                    | SyntaxKind::ENV_VAR_REF
                    | SyntaxKind::CACHE_VAR_REF
                    | SyntaxKind::GENERATOR_EXPR => {
                        // Add hardline separator before this arg (if not first)
                        if last_was_arg && !first_arg {
                            docs.push(RcDoc::hardline());
                        }

                        let text = token.text().to_string();
                        docs.push(RcDoc::text(text));
                        last_was_arg = true;
                        first_arg = false;
                        consecutive_newline_count = 0;
                    }
                    SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
                        // Emit comment with hardline after
                        let text = token.text().to_string();
                        docs.push(RcDoc::hardline());
                        docs.push(RcDoc::text(text));
                        docs.push(RcDoc::hardline());
                        last_was_arg = false;
                        consecutive_newline_count = 0;
                    }
                    SyntaxKind::NEWLINE => {
                        consecutive_newline_count += 1;
                        // If this is a blank line (2+ consecutive newlines), emit extra hardline
                        if consecutive_newline_count >= 2 {
                            // Respect max_blank_lines config
                            let blank_lines_to_emit = consecutive_newline_count - 1;
                            let max_blank = ctx.config.max_blank_lines;
                            if blank_lines_to_emit <= max_blank {
                                docs.push(RcDoc::hardline());
                            }
                            // Reset so we don't emit multiple times for same blank line run
                            consecutive_newline_count = 1;
                        }
                    }
                    SyntaxKind::WHITESPACE | SyntaxKind::LPAREN | SyntaxKind::RPAREN => {
                        // Skip - whitespace is formatter's job, parens are handled by format_command
                    }
                    _ => {
                        // Reset state for other tokens
                        consecutive_newline_count = 0;
                    }
                }
            }
            NodeOrToken::Node(_) => {
                // Nested nodes - skip for now
                consecutive_newline_count = 0;
            }
        }
    }

    docs.push(RcDoc::line_()); // Last break - collapses to nothing

    // Nest without group (forced multiline must not collapse)
    RcDoc::concat(docs).nest(ctx.config.indent_width as isize)
}

/// Build an indentation string for the given level, respecting tabs/spaces config
fn indent_string(level: usize, config: &FormatConfig) -> String {
    if config.use_tabs {
        "\t".repeat(level)
    } else {
        " ".repeat(level * config.indent_width)
    }
}

/// Check if command is a block opener
fn is_block_opener(name: &str) -> bool {
    matches!(name, "if" | "foreach" | "while" | "function" | "macro")
}

/// Check if command is a mid-block command (else/elseif)
fn is_block_mid(name: &str) -> bool {
    matches!(name, "else" | "elseif")
}

/// Check if command is a block closer
fn is_block_closer(name: &str) -> bool {
    matches!(
        name,
        "endif" | "endforeach" | "endwhile" | "endfunction" | "endmacro"
    )
}
