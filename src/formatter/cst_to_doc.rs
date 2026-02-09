use crate::cst::{ArgumentList, CSTRoot, CommandInvocation};
use crate::syntax_kind::SyntaxKind;
use crate::SyntaxNode;
use pretty::RcDoc;
use rowan::NodeOrToken;

use super::config::{ClosingStyle, CommandCase, FormatConfig};
use super::cmake_rules;
use super::comments;

/// Signals detected in argument list that affect formatting
pub(crate) struct ArgumentFormatSignals {
    pub(crate) has_comments: bool,
    pub(crate) has_blank_lines: bool,
    pub(crate) has_newlines: bool,
    pub(crate) force_multiline: bool,
}

/// Scope frame for tracking block opener arguments
struct ScopeFrame {
    opener_args: Vec<String>,
}

/// Context for formatting block closers and mid-block commands
struct CloserContext {
    opener_args: Vec<String>,
    is_mid_block: bool,
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
    let mut scope_stack: Vec<ScopeFrame> = Vec::new();

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

                            // Determine if this is a closer or mid-block command
                            let is_closer = is_block_closer(&cmd_name);
                            let is_mid = is_block_mid(&cmd_name);

                            // Prepare closer_context before dedenting/formatting
                            let closer_context = if is_closer {
                                // Pop scope for closers
                                scope_stack.pop().map(|frame| CloserContext {
                                    opener_args: frame.opener_args,
                                    is_mid_block: false,
                                })
                            } else if is_mid {
                                // Peek scope for mid-block commands
                                scope_stack.last().map(|frame| CloserContext {
                                    opener_args: frame.opener_args.clone(),
                                    is_mid_block: true,
                                })
                            } else {
                                None
                            };

                            // Handle block closers (dedent before emitting)
                            if is_closer {
                                current_indent = current_indent.saturating_sub(1);
                            }

                            // Handle mid-block commands (dedent for this line only)
                            let cmd_indent = if is_mid {
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
                            let mut cmd_doc = format_command(&cmd, &cmd_ctx, closer_context.as_ref());

                            // Check for trailing comment
                            if let Some(trailing_comment) = comments::extract_trailing_comment(&child_node) {
                                cmd_doc = cmd_doc.append(RcDoc::space()).append(RcDoc::text(trailing_comment));
                            }

                            docs.push(cmd_doc);
                            docs.push(RcDoc::hardline());

                            // Handle block openers (indent and push scope after emitting)
                            if is_block_opener(&cmd_name) {
                                current_indent += 1;
                                // Extract opener arguments for scope tracking
                                let opener_args: Vec<String> = cmd.argument_list()
                                    .map(|al| al.arguments()
                                        .map(|t| t.text().to_string())
                                        .collect())
                                    .unwrap_or_default();
                                scope_stack.push(ScopeFrame { opener_args });
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
fn format_command(
    cmd: &CommandInvocation,
    ctx: &FormatContext,
    closer_context: Option<&CloserContext>
) -> RcDoc<'static, ()> {
    let indent_str = ctx.indent_str();

    // Get command name and apply casing
    let name = cmd.name_text().unwrap_or_else(|| "unknown".to_string());
    let formatted_name = match ctx.config.command_case {
        CommandCase::Lowercase => name.to_lowercase(),
        CommandCase::Uppercase => name.to_uppercase(),
        CommandCase::Preserve => name.clone(),
    };

    // Handle block closers and mid-block commands based on closing_style
    let args_doc = if let Some(closer_ctx) = closer_context {
        match ctx.config.closing_style {
            ClosingStyle::Keep => {
                // Keep mode: format normally, ignore closer_context
                if let Some(arg_list) = cmd.argument_list() {
                    if cmake_rules::is_keyword_aware_command(&name) {
                        let sections = cmake_rules::parse_keyword_sections(&arg_list);
                        cmake_rules::format_keyword_aware_args(&arg_list, sections, ctx.config)
                    } else {
                        format_argument_list(&arg_list, ctx)
                    }
                } else {
                    RcDoc::nil()
                }
            }
            ClosingStyle::Remove => {
                // Remove mode: emit empty argument list
                RcDoc::nil()
            }
            ClosingStyle::Force => {
                // Force mode: emit opener's arguments
                if closer_ctx.opener_args.is_empty() {
                    RcDoc::nil()
                } else {
                    RcDoc::text(closer_ctx.opener_args.join(" "))
                }
            }
        }
    } else {
        // Not a closer/mid-block command: format normally
        if let Some(arg_list) = cmd.argument_list() {
            // Check if this command should use keyword-aware formatting
            if cmake_rules::is_keyword_aware_command(&name) {
                let sections = cmake_rules::parse_keyword_sections(&arg_list);
                cmake_rules::format_keyword_aware_args(&arg_list, sections, ctx.config)
            } else {
                format_argument_list(&arg_list, ctx)
            }
        } else {
            RcDoc::nil()
        }
    };

    // Format as: indent + name + ( + args + )
    let cmd_doc = RcDoc::text(formatted_name)
        .append(RcDoc::text("("))
        .append(args_doc)
        .append(RcDoc::text(")"));

    RcDoc::text(indent_str).append(cmd_doc)
}

/// Detect formatting signals in argument list (comments, blank lines, newlines)
pub(crate) fn detect_argument_formatting_signals(arg_list: &ArgumentList) -> ArgumentFormatSignals {
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

    // If no multiline signals, use auto-layout (soft lines + group)
    // But follow ARGL-03: first arg should be on same line as command when broken
    if !signals.force_multiline {
        if args.len() == 1 {
            // Single argument: simple case
            let text = args[0].text().to_string();
            return RcDoc::text(text);
        }

        // Multiple arguments: first arg not nested, rest nested
        // This ensures ARGL-03 even when auto-layout breaks
        let first_text = args[0].text().to_string();
        let mut rest_docs = Vec::new();

        for (i, arg) in args.iter().enumerate().skip(1) {
            rest_docs.push(RcDoc::line());
            let text = arg.text().to_string();
            rest_docs.push(RcDoc::text(text));
        }

        rest_docs.push(RcDoc::line_()); // Soft line before closing paren

        // Build: first + grouped(nested(rest))
        // When flat: "first rest1 rest2"
        // When broken: "first\n  rest1\n  rest2\n"
        return RcDoc::text(first_text)
            .append(
                RcDoc::concat(rest_docs)
                    .nest(ctx.config.indent_width as isize)
                    .group()
            );
    }

    // Force multiline: walk tokens and build Doc IR with hardlines
    // Strategy for ARGL-03: first arg not indented, rest indented
    // Build first_arg_doc separately, then build rest_docs with nesting
    let mut first_arg_doc: Option<RcDoc<'static, ()>> = None;
    let mut rest_docs = Vec::new();
    let mut consecutive_newline_count = 0;
    let mut seen_first_arg = false;

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
                        let text = token.text().to_string();

                        if !seen_first_arg {
                            // First argument: no indent, no break before
                            first_arg_doc = Some(RcDoc::text(text));
                            seen_first_arg = true;
                        } else {
                            // Subsequent arguments: add hardline before each
                            rest_docs.push(RcDoc::hardline());

                            // If there were blank lines before this arg, emit extra hardlines
                            if consecutive_newline_count >= 2 {
                                // consecutive_newline_count includes the line-ending newline
                                // So 2 newlines = 1 blank line, 3 newlines = 2 blank lines, etc.
                                let blank_lines = consecutive_newline_count - 1;
                                let blank_lines_to_emit = std::cmp::min(blank_lines, ctx.config.max_blank_lines);
                                for _ in 0..blank_lines_to_emit {
                                    rest_docs.push(RcDoc::hardline());
                                }
                            }

                            rest_docs.push(RcDoc::text(text));
                        }
                        consecutive_newline_count = 0;
                    }
                    SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
                        // Comments always go in rest_docs (indented)
                        let text = token.text().to_string();
                        rest_docs.push(RcDoc::hardline());

                        // If there were blank lines before this comment, emit extra hardlines
                        if consecutive_newline_count >= 2 {
                            let blank_lines = consecutive_newline_count - 1;
                            let blank_lines_to_emit = std::cmp::min(blank_lines, ctx.config.max_blank_lines);
                            for _ in 0..blank_lines_to_emit {
                                rest_docs.push(RcDoc::hardline());
                            }
                        }

                        rest_docs.push(RcDoc::text(text));
                        consecutive_newline_count = 0;
                    }
                    SyntaxKind::NEWLINE => {
                        consecutive_newline_count += 1;
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

    // Build final Doc IR
    if let Some(first) = first_arg_doc {
        // We have a first arg
        if !rest_docs.is_empty() {
            // First arg + nested rest + hardline before closing paren
            first
                .append(RcDoc::concat(rest_docs).nest(ctx.config.indent_width as isize))
                .append(RcDoc::hardline())
        } else {
            // Only first arg - but since multiline was forced, there must be a newline somewhere
            // This shouldn't happen in practice (if only 1 arg and no newlines, not forced multiline)
            first
        }
    } else {
        // No arguments at all
        RcDoc::nil()
    }
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
