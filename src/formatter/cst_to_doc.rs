use crate::cst::{ArgumentList, CSTRoot, CommandInvocation};
use crate::syntax_kind::SyntaxKind;
use crate::SyntaxNode;
use pretty::RcDoc;
use rowan::NodeOrToken;

use super::config::{ClosingStyle, CommandCase, FormatConfig};
use super::cmake_rules;
use super::comments;
use super::suppression::{parse_directive, line_number_at_offset, SuppressionTracker, SuppressionWarning};

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
pub fn format_cst(cst: &CSTRoot, config: &FormatConfig, source: &str) -> (RcDoc<'static, ()>, Vec<SuppressionWarning>) {
    let ctx = FormatContext::new(config);
    format_file(&cst.root, &ctx, source)
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
fn format_file(node: &SyntaxNode, ctx: &FormatContext, source: &str) -> (RcDoc<'static, ()>, Vec<SuppressionWarning>) {
    let mut docs = Vec::new();
    let mut current_indent: usize = 0;
    let mut blank_line_count = 0;
    let mut scope_stack: Vec<ScopeFrame> = Vec::new();
    let mut tracker = SuppressionTracker::new();

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
                for lc in comments::extract_leading_comments(child_node) {
                    // Don't mark as handled if it's a trailing comment
                    if !trailing_comments.contains(&lc.text) {
                        handled_comments.insert(lc.text);
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

                        // Process directives in leading comments
                        // We need to find the actual comment position by searching backwards in source
                        let cmd_start: usize = child_node.text_range().start().into();
                        for lc in &leading_comments {
                            if let Some(directive) = parse_directive(&lc.text) {
                                // Find comment position by searching backwards from command
                                let comment_offset = source[..cmd_start]
                                    .rfind(&lc.text[..])
                                    .unwrap_or(cmd_start);
                                let line = line_number_at_offset(source, comment_offset);
                                tracker.process_directive(directive, line);
                            }
                        }

                        // Check if we should skip this command or if we're in a suppressed region
                        let should_skip = tracker.should_skip_next();
                        let is_suppressed = tracker.is_suppressed();

                        if should_skip {
                            tracker.clear_skip();
                        }

                        // Check if any leading comments will actually be emitted
                        let has_emittable_leading = leading_comments.iter()
                            .any(|lc| !trailing_comments.contains(&lc.text));

                        // Emit accumulated blank lines before command/comments
                        // When leading comments exist, blank_line_before handles all gaps
                        // (including before the first comment), so skip blank_line_count.
                        if blank_line_count >= 2 && !docs.is_empty() && !has_emittable_leading {
                            let blank_lines_to_emit = std::cmp::min(blank_line_count - 1, ctx.config.max_blank_lines);
                            for _ in 0..blank_lines_to_emit {
                                docs.push(RcDoc::hardline());
                            }
                        }

                        if should_skip || is_suppressed {
                            // Emit the command as raw text
                            // For skip: the leading comments are formatted (outside suppression)
                            // For suppressed: everything is raw

                            if !is_suppressed {
                                // Skip mode: emit formatted leading comments
                                let indent_str = indent_string(current_indent, ctx.config);
                                for lc in &leading_comments {
                                    if !trailing_comments.contains(&lc.text) {
                                        if lc.blank_line_before && !docs.is_empty() {
                                            docs.push(RcDoc::hardline());
                                        }
                                        docs.push(RcDoc::text(format!("{}{}", indent_str, lc.text)));
                                        docs.push(RcDoc::hardline());
                                    }
                                }
                            } else {
                                // Suppressed region: emit raw leading comments
                                let indent_str = indent_string(current_indent, ctx.config);
                                for lc in &leading_comments {
                                    if !trailing_comments.contains(&lc.text) {
                                        if lc.blank_line_before && !docs.is_empty() {
                                            docs.push(RcDoc::hardline());
                                        }
                                        docs.push(RcDoc::text(format!("{}{}", indent_str, lc.text)));
                                        docs.push(RcDoc::hardline());
                                    }
                                }
                            }

                            // Emit the command itself as raw text
                            let raw_text = child_node.text().to_string();
                            let indent_str = indent_string(current_indent, ctx.config);
                            docs.push(RcDoc::text(format!("{}{}", indent_str, raw_text.trim())));
                            docs.push(RcDoc::hardline());
                            blank_line_count = 0;
                            continue;
                        }

                        // Normal formatting path: emit leading comments (skip if already handled as trailing)
                        let indent_str = indent_string(current_indent, ctx.config);
                        for lc in &leading_comments {
                            // Only emit if not already handled as a trailing comment
                            if !trailing_comments.contains(&lc.text) {
                                if lc.blank_line_before && !docs.is_empty() {
                                    docs.push(RcDoc::hardline());
                                }
                                docs.push(RcDoc::text(format!("{}{}", indent_str, lc.text)));
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
                            // Process directives in standalone comments (not leading/trailing)
                            if let Some(directive) = parse_directive(&comment_text) {
                                let line = line_number_at_offset(source, token.text_range().start().into());
                                tracker.process_directive(directive, line);
                            }
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
                        } else {
                            // Handled comment (leading/trailing of a command): it occupies
                            // a line, so its structural newlines shouldn't count as blank
                            // lines. Reset to 0 only if no blank line was already detected;
                            // if blank_line_count >= 2, a real blank line preceded this
                            // comment block and should be preserved.
                            if blank_line_count < 2 {
                                blank_line_count = 0;
                            }
                        }
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

    // Finalize suppression tracking and collect warnings
    tracker.finalize();
    let warnings = tracker.into_warnings();

    (RcDoc::concat(docs), warnings)
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
        CommandCase::Leave => name.clone(),
    };

    // Handle block closers and mid-block commands based on closing_style
    let args_doc = if let Some(closer_ctx) = closer_context {
        if closer_ctx.is_mid_block && name.to_lowercase() == "elseif" {
            // elseif carries a condition — always preserve its arguments
            if let Some(arg_list) = cmd.argument_list() {
                format_argument_list(&arg_list, ctx)
            } else {
                RcDoc::nil()
            }
        } else {
            // True closers (endif, endforeach, etc.) — apply closing_style
            match ctx.config.closing_style {
                ClosingStyle::Leave => {
                    // Leave mode: format normally, ignore closer_context
                    if let Some(arg_list) = cmd.argument_list() {
                        if cmake_rules::is_keyword_aware_command(&name) {
                            let sections = cmake_rules::parse_keyword_sections(&arg_list);
                            cmake_rules::format_keyword_aware_args(&arg_list, sections, ctx.config, ctx.indent_level)
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
        }
    } else {
        // Not a closer/mid-block command: format normally
        if let Some(arg_list) = cmd.argument_list() {
            // Check if this command should use keyword-aware formatting
            if cmake_rules::is_keyword_aware_command(&name) {
                let sections = cmake_rules::parse_keyword_sections(&arg_list);
                cmake_rules::format_keyword_aware_args(&arg_list, sections, ctx.config, ctx.indent_level)
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
    let args = collect_logical_args(arg_list);

    if args.is_empty() {
        return RcDoc::nil();
    }

    // Detect formatting signals
    let signals = detect_argument_formatting_signals(arg_list);

    // If no multiline signals, use auto-layout (flat_alt + group)
    // But follow ARGL-03: first arg should be on same line as command when broken
    if !signals.force_multiline {
        if args.len() == 1 {
            // Single argument: simple case
            return RcDoc::text(args[0].clone());
        }

        // Use explicit text indentation via flat_alt instead of nest()
        // This correctly handles tabs and respects the command's nesting depth
        let base_indent = indent_string(ctx.indent_level, ctx.config);
        let inner_indent = indent_string(ctx.indent_level + 1, ctx.config);

        let first_text = args[0].clone();
        let mut rest_docs = Vec::new();

        for arg in args.iter().skip(1) {
            // flat_alt: broken → newline + indent text, flat → space
            rest_docs.push(RcDoc::flat_alt(
                RcDoc::hardline().append(RcDoc::text(inner_indent.clone())),
                RcDoc::space(),
            ));
            rest_docs.push(RcDoc::text(arg.clone()));
        }

        // Closing paren position: broken → newline + base indent, flat → nothing
        rest_docs.push(RcDoc::flat_alt(
            RcDoc::hardline().append(RcDoc::text(base_indent)),
            RcDoc::nil(),
        ));

        // When flat: "first rest1 rest2"
        // When broken: "first\n<inner>rest1\n<inner>rest2\n<base>"
        return RcDoc::text(first_text)
            .append(
                RcDoc::concat(rest_docs)
                    .group()
            );
    }

    // Force multiline: walk tokens and build Doc IR with hardlines
    // Use explicit text indentation instead of nest() for correct tab/space handling
    let base_indent = indent_string(ctx.indent_level, ctx.config);
    let inner_indent = indent_string(ctx.indent_level + 1, ctx.config);

    // Strategy for ARGL-03: first arg not indented, rest indented
    let mut first_arg_doc: Option<RcDoc<'static, ()>> = None;
    let mut rest_docs = Vec::new();
    let mut consecutive_newline_count = 0;
    let mut seen_first_arg = false;
    let mut saw_separator = true; // tracks whitespace between tokens for adjacency

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

                        if !saw_separator && seen_first_arg {
                            // Adjacent to previous token (no whitespace) — merge
                            // e.g. ${VAR}/suffix is two tokens but one logical argument
                            rest_docs.push(RcDoc::text(text));
                        } else if !seen_first_arg {
                            // First argument: no indent, no break before
                            first_arg_doc = Some(RcDoc::text(text));
                            seen_first_arg = true;
                        } else {
                            // Subsequent arguments: hardline + explicit indent
                            rest_docs.push(RcDoc::hardline());

                            // If there were blank lines before this arg, emit extra hardlines
                            if consecutive_newline_count >= 2 {
                                let blank_lines = consecutive_newline_count - 1;
                                let blank_lines_to_emit = std::cmp::min(blank_lines, ctx.config.max_blank_lines);
                                for _ in 0..blank_lines_to_emit {
                                    rest_docs.push(RcDoc::hardline());
                                }
                            }

                            rest_docs.push(RcDoc::text(inner_indent.clone()));
                            rest_docs.push(RcDoc::text(text));
                        }
                        saw_separator = false;
                        consecutive_newline_count = 0;
                    }
                    SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
                        // Comments at same indent level as arguments
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

                        rest_docs.push(RcDoc::text(inner_indent.clone()));
                        rest_docs.push(RcDoc::text(text));
                        saw_separator = true;
                        consecutive_newline_count = 0;
                    }
                    SyntaxKind::NEWLINE => {
                        saw_separator = true;
                        consecutive_newline_count += 1;
                    }
                    SyntaxKind::WHITESPACE | SyntaxKind::LPAREN | SyntaxKind::RPAREN => {
                        saw_separator = true;
                    }
                    _ => {
                        saw_separator = true;
                        consecutive_newline_count = 0;
                    }
                }
            }
            NodeOrToken::Node(_) => {
                saw_separator = true;
                consecutive_newline_count = 0;
            }
        }
    }

    // Build final Doc IR
    if let Some(first) = first_arg_doc {
        // We have a first arg
        if !rest_docs.is_empty() {
            // First arg + rest with explicit indentation + closing paren indent
            first
                .append(RcDoc::concat(rest_docs))
                .append(RcDoc::hardline())
                .append(RcDoc::text(base_indent))
        } else {
            first
        }
    } else {
        // No arguments at all
        RcDoc::nil()
    }
}

/// Collect logical arguments from an argument list, merging adjacent tokens
/// (no whitespace between them) into single strings.
/// For example, `${CMAKE_CURRENT_SOURCE_DIR}` + `/src` (two CST tokens)
/// becomes one logical argument `${CMAKE_CURRENT_SOURCE_DIR}/src`.
pub(crate) fn collect_logical_args(arg_list: &ArgumentList) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    let mut saw_separator = true;

    for child in arg_list.syntax().children_with_tokens() {
        if let NodeOrToken::Token(token) = child {
            match token.kind() {
                SyntaxKind::UNQUOTED_ARGUMENT
                | SyntaxKind::QUOTED_ARGUMENT
                | SyntaxKind::BRACKET_ARGUMENT
                | SyntaxKind::VARIABLE_REF
                | SyntaxKind::ENV_VAR_REF
                | SyntaxKind::CACHE_VAR_REF
                | SyntaxKind::GENERATOR_EXPR => {
                    let text = token.text();
                    if !saw_separator && !args.is_empty() {
                        args.last_mut().unwrap().push_str(text);
                    } else {
                        args.push(text.to_string());
                    }
                    saw_separator = false;
                }
                SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE => {
                    saw_separator = true;
                }
                _ => {
                    saw_separator = true;
                }
            }
        }
    }
    args
}

/// Build an indentation string for the given level, respecting tabs/spaces config
pub(crate) fn indent_string(level: usize, config: &FormatConfig) -> String {
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
