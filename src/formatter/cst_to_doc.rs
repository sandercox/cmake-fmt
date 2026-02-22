use std::collections::HashMap;
use std::collections::HashSet;

use crate::cst::{ArgumentList, CSTRoot, CommandInvocation};
use crate::syntax_kind::SyntaxKind;
use crate::SyntaxNode;
use pretty::RcDoc;
use rowan::NodeOrToken;

use super::builtins;
use super::config::{ClosingStyle, CommandCase, FormatConfig, UserCommandCase};
use super::cmake_rules;
use super::comments;
use super::suppression::{parse_directive, line_number_at_offset, SuppressionTracker, SuppressionWarning};
use super::grammar::GrammarRegistry;

// Import post-processing function from parent module
use super::post_process_rendered_output;

/// Signals detected in argument list that affect formatting
pub(crate) struct ArgumentFormatSignals {
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
    user_defs: &'a HashMap<String, String>,
    user_grammars: &'a HashMap<String, super::grammar::CommandGrammar>,
}

impl<'a> FormatContext<'a> {
    fn new(
        config: &'a FormatConfig,
        user_defs: &'a HashMap<String, String>,
        user_grammars: &'a HashMap<String, super::grammar::CommandGrammar>,
    ) -> Self {
        Self {
            config,
            indent_level: 0,
            user_defs,
            user_grammars,
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

/// Entry point: convert CST to Doc IR and render to String
pub fn format_cst(
    cst: &CSTRoot,
    config: &FormatConfig,
    source: &str,
    user_defs: &HashMap<String, String>,
    user_grammars: &HashMap<String, super::grammar::CommandGrammar>,
) -> (String, Vec<SuppressionWarning>) {
    let ctx = FormatContext::new(config, user_defs, user_grammars);
    format_file(&cst.root, &ctx, source)
}

/// Render a batch of docs to a String
///
/// `width` is the target line length. A value of 0 means unlimited (never wrap).
fn render_batch(docs: Vec<RcDoc<'static, ()>>, width: usize) -> String {
    if docs.is_empty() {
        return String::new();
    }
    let doc = RcDoc::concat(docs);
    let mut output = Vec::new();
    // 0 means unlimited — use usize::MAX so the pretty printer never wraps
    let effective_width = if width == 0 { usize::MAX } else { width };
    doc.render(effective_width, &mut output)
        .expect("rendering to Vec should not fail");
    String::from_utf8(output)
        .expect("formatted output should be valid UTF-8")
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

/// Identify source lines that are part of multi-line comment blocks (2+ consecutive comment lines).
/// Returns a HashSet of 1-indexed line numbers.
/// Comment blocks are sequences of consecutive lines where each trimmed line starts with '#'.
/// Only non-comment, non-blank lines break a block.
fn comment_block_lines(source: &str) -> HashSet<usize> {
    let mut result = HashSet::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut run_start: Option<usize> = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') && !trimmed.starts_with("#[") {
            if run_start.is_none() {
                run_start = Some(i);
            }
        } else {
            if let Some(start) = run_start {
                if i - start >= 2 {
                    // 2+ consecutive comment lines = block
                    for j in start..i {
                        result.insert(j + 1); // 1-indexed line numbers
                    }
                }
            }
            run_start = None;
        }
    }
    // Handle block at end of file
    if let Some(start) = run_start {
        if lines.len() - start >= 2 {
            for j in start..lines.len() {
                result.insert(j + 1);
            }
        }
    }

    result
}

/// Format the FILE node
fn format_file(node: &SyntaxNode, ctx: &FormatContext, source: &str) -> (String, Vec<SuppressionWarning>) {
    // Clone config so we can modify it based on style directives
    let mut config = ctx.config.clone();

    // Pre-scan source for consecutive comment blocks (2+ adjacent comment lines)
    let block_comment_lines = comment_block_lines(source);

    let mut docs = Vec::new();
    let mut batch_strings = Vec::new();
    let mut current_indent: usize = 0;
    let mut blank_line_count = 0;
    let mut scope_stack: Vec<ScopeFrame> = Vec::new();
    let mut tracker = SuppressionTracker::new();

    // Batch size: render every 500 docs to prevent stack overflow
    const BATCH_SIZE: usize = 500;

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

                                // Handle style directives separately
                                match &directive {
                                    super::suppression::Directive::Style { key, value } => {
                                        if let Err(msg) = config.apply_override(key, value) {
                                            eprintln!("Warning: {}", msg);
                                        }
                                    }
                                    _ => {
                                        // Only pass suppression directives to the tracker
                                        tracker.process_directive(directive, line);
                                    }
                                }
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
                        if blank_line_count >= 2 && (!docs.is_empty() || !batch_strings.is_empty()) && !has_emittable_leading {
                            let blank_lines_to_emit = std::cmp::min(blank_line_count - 1, config.max_blank_lines);
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
                                let indent_str = indent_string(current_indent, &config);
                                for lc in &leading_comments {
                                    if !trailing_comments.contains(&lc.text) {
                                        if lc.blank_line_before && (!docs.is_empty() || !batch_strings.is_empty()) {
                                            docs.push(RcDoc::hardline());
                                        }
                                        // Normalize line comments, but skip block comments and bracket comments
                                        let lc_line = source[..cmd_start]
                                            .rfind(&lc.text[..])
                                            .map(|offset| line_number_at_offset(source, offset))
                                            .unwrap_or(0);
                                        let text = if !lc.text.starts_with("#[") && !block_comment_lines.contains(&lc_line) {
                                            cmake_rules::normalize_comment_whitespace(&lc.text, config.comment_style)
                                        } else {
                                            lc.text.clone()
                                        };
                                        docs.push(RcDoc::text(format!("{}{}", indent_str, text)));
                                        docs.push(RcDoc::hardline());
                                    }
                                }
                            } else {
                                // Suppressed region: emit raw leading comments
                                let indent_str = indent_string(current_indent, &config);
                                for lc in &leading_comments {
                                    if !trailing_comments.contains(&lc.text) {
                                        if lc.blank_line_before && (!docs.is_empty() || !batch_strings.is_empty()) {
                                            docs.push(RcDoc::hardline());
                                        }
                                        // Suppressed: preserve comment text as-is
                                        let text = lc.text.clone();
                                        docs.push(RcDoc::text(format!("{}{}", indent_str, text)));
                                        docs.push(RcDoc::hardline());
                                    }
                                }
                            }

                            // Emit blank lines between last leading comment and command
                            if has_emittable_leading && blank_line_count >= 2 {
                                let blank_lines_to_emit = std::cmp::min(blank_line_count - 1, config.max_blank_lines);
                                for _ in 0..blank_lines_to_emit {
                                    docs.push(RcDoc::hardline());
                                }
                            }

                            // Emit the command itself as raw text
                            let raw_text = child_node.text().to_string();
                            let indent_str = indent_string(current_indent, &config);
                            let mut raw_doc = RcDoc::text(format!("{}{}", indent_str, raw_text.trim()));

                            // Preserve trailing comment (not part of command node)
                            if let Some(trailing_comment) = comments::extract_trailing_comment(&child_node) {
                                // Normalize line comments, but not when suppressed
                                let text = if !is_suppressed && !trailing_comment.starts_with("#[") {
                                    cmake_rules::normalize_comment_whitespace(&trailing_comment, config.comment_style)
                                } else {
                                    trailing_comment
                                };
                                raw_doc = raw_doc.append(RcDoc::space()).append(RcDoc::text(text));
                            }

                            docs.push(raw_doc);
                            docs.push(RcDoc::hardline());
                            blank_line_count = 0;
                            continue;
                        }

                        // Normal formatting path: emit leading comments (skip if already handled as trailing)
                        let indent_str = indent_string(current_indent, &config);
                        for lc in &leading_comments {
                            // Only emit if not already handled as a trailing comment
                            if !trailing_comments.contains(&lc.text) {
                                if lc.blank_line_before && (!docs.is_empty() || !batch_strings.is_empty()) {
                                    docs.push(RcDoc::hardline());
                                }
                                // Normalize line comments, but skip block comments and bracket comments
                                let lc_line = source[..cmd_start]
                                    .rfind(&lc.text[..])
                                    .map(|offset| line_number_at_offset(source, offset))
                                    .unwrap_or(0);
                                let text = if !lc.text.starts_with("#[") && !block_comment_lines.contains(&lc_line) {
                                    cmake_rules::normalize_comment_whitespace(&lc.text, config.comment_style)
                                } else {
                                    lc.text.clone()
                                };
                                docs.push(RcDoc::text(format!("{}{}", indent_str, text)));
                                docs.push(RcDoc::hardline());
                            }
                        }

                        // Emit blank lines between last leading comment and command
                        if has_emittable_leading && blank_line_count >= 2 {
                            let blank_lines_to_emit = std::cmp::min(blank_line_count - 1, config.max_blank_lines);
                            for _ in 0..blank_lines_to_emit {
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

                            // Handle no-sort directive: temporarily disable sorting for this command
                            let should_skip_sort = tracker.should_skip_sort_next();
                            let temp_config;
                            let effective_config = if should_skip_sort {
                                tracker.clear_skip_sort();
                                temp_config = FormatConfig {
                                    sort_sources: super::config::SortSources::None,
                                    ..config.clone()
                                };
                                &temp_config
                            } else {
                                &config
                            };

                            // Create context for this command
                            let cmd_ctx = FormatContext {
                                config: effective_config,
                                indent_level: cmd_indent,
                                user_defs: ctx.user_defs,
                                user_grammars: ctx.user_grammars,
                            };

                            // Format and emit command
                            let mut cmd_doc = format_command(&cmd, &cmd_ctx, closer_context.as_ref());

                            // Check for trailing comment
                            if let Some(trailing_comment) = comments::extract_trailing_comment(&child_node) {
                                // Normalize line comments (not bracket comments)
                                let text = if !trailing_comment.starts_with("#[") {
                                    cmake_rules::normalize_comment_whitespace(&trailing_comment, config.comment_style)
                                } else {
                                    trailing_comment
                                };
                                cmd_doc = cmd_doc.append(RcDoc::space()).append(RcDoc::text(text));
                            }

                            docs.push(cmd_doc);
                            docs.push(RcDoc::hardline());

                            // Check if we should render a batch to prevent deep nesting
                            if docs.len() >= BATCH_SIZE {
                                let batch = render_batch(std::mem::take(&mut docs), config.max_line_length);
                                batch_strings.push(batch);
                            }

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
                        if blank_line_count >= 2 && (!docs.is_empty() || !batch_strings.is_empty()) {
                            let blank_lines_to_emit = std::cmp::min(blank_line_count - 1, config.max_blank_lines);
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

                                // Handle style directives separately
                                match &directive {
                                    super::suppression::Directive::Style { key, value } => {
                                        if let Err(msg) = config.apply_override(key, value) {
                                            eprintln!("Warning: {}", msg);
                                        }
                                    }
                                    _ => {
                                        // Only pass suppression directives to the tracker
                                        tracker.process_directive(directive, line);
                                    }
                                }
                            }
                            // Emit accumulated blank lines before standalone comment
                            // But only if not first content
                            if blank_line_count >= 2 && (!docs.is_empty() || !batch_strings.is_empty()) {
                                let blank_lines_to_emit = std::cmp::min(blank_line_count - 1, config.max_blank_lines);
                                for _ in 0..blank_lines_to_emit {
                                    docs.push(RcDoc::hardline());
                                }
                            }

                            let indent_str = indent_string(current_indent, &config);
                            // Normalize line comments (not bracket comments), but not when suppressed or in a comment block
                            let comment_line = line_number_at_offset(source, token.text_range().start().into());
                            let text = if token.kind() == SyntaxKind::COMMENT
                                && !tracker.is_suppressed()
                                && !block_comment_lines.contains(&comment_line)
                            {
                                cmake_rules::normalize_comment_whitespace(&comment_text, config.comment_style)
                            } else {
                                comment_text
                            };
                            docs.push(RcDoc::text(format!("{}{}", indent_str, text)));
                            docs.push(RcDoc::hardline());
                            blank_line_count = 0;
                        } else {
                            // Handled comment: blank lines before it are captured in
                            // extract_leading_comments() metadata. Reset so post-comment
                            // newlines are tracked independently.
                            blank_line_count = 0;
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

    // Render any remaining docs in the final batch
    if !docs.is_empty() {
        let batch = render_batch(docs, config.max_line_length);
        batch_strings.push(batch);
    }

    // Finalize suppression tracking and collect warnings
    tracker.finalize();
    let warnings = tracker.into_warnings();

    // Join all batch strings and apply post-processing
    let result = batch_strings.join("");
    let result = post_process_rendered_output(&result, config.final_newline, source.ends_with('\n'));

    (result, warnings)
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
    let name_lower = name.to_lowercase();
    let formatted_name = if builtins::is_builtin_command(&name_lower) {
        match ctx.config.command_case {
            CommandCase::Lowercase => name_lower.clone(),
            CommandCase::Uppercase => name.to_uppercase(),
            CommandCase::Leave => name.clone(),
        }
    } else {
        match ctx.config.user_command_case {
            UserCommandCase::Lowercase => name_lower.clone(),
            UserCommandCase::Uppercase => name.to_uppercase(),
            UserCommandCase::Leave => name.clone(),
            UserCommandCase::Infer => {
                // Look up from function()/macro() definitions; if not found, leave as-is
                ctx.user_defs.get(&name_lower).cloned().unwrap_or(name.clone())
            }
        }
    };

    // Handle block closers and mid-block commands based on closing_style
    let args_doc = if let Some(closer_ctx) = closer_context {
        if closer_ctx.is_mid_block && name.to_lowercase() == "elseif" {
            // elseif carries a condition — always preserve its arguments
            if let Some(arg_list) = cmd.argument_list() {
                let is_custom = !builtins::is_builtin_command(&name_lower);
                format_argument_list(&arg_list, ctx, is_custom)
            } else {
                RcDoc::nil()
            }
        } else {
            // True closers (endif, endforeach, etc.) — apply closing_style
            match ctx.config.closing_style {
                ClosingStyle::Leave => {
                    // Leave mode: format normally, ignore closer_context
                    if let Some(arg_list) = cmd.argument_list() {
                        let grammar_enum = GrammarRegistry::global().get(&name_lower);
                        // Resolve multi-mode grammar
                        let first_keyword = grammar_enum.as_ref()
                            .filter(|g| g.is_multi_mode())
                            .and_then(|_| detect_mode_keyword(&arg_list));
                        let grammar = grammar_enum.and_then(|g| g.resolve(first_keyword.as_deref()));
                        // If no builtin grammar, check user grammars
                        let user_grammar = if grammar.is_none() {
                            ctx.user_grammars.get(&name_lower)
                        } else {
                            None
                        };
                        let effective_grammar = grammar.or(user_grammar);
                        // Skip keyword-aware formatting for unrecognized modes in multi-mode commands
                        let is_unrecognized_mode = grammar_enum.as_ref()
                            .map_or(false, |g| g.is_multi_mode() && grammar.is_none());
                        if !is_unrecognized_mode && (effective_grammar.is_some() || cmake_rules::is_keyword_aware_command(&name)) {
                            let sections = cmake_rules::parse_keyword_sections_with_grammar(&arg_list, effective_grammar, ctx.config.comment_style);
                            let is_multi_mode = grammar_enum.as_ref().map_or(false, |g| g.is_multi_mode());
                            let has_builtin_grammar = grammar.is_some();
                            let force_args_on_new_line = effective_grammar.map_or(false, |g| g.force_args_on_new_line);
                            let sub_kws = effective_grammar.map(|g| &g.sub_keywords).filter(|s| !s.is_empty());
                            cmake_rules::format_keyword_aware_args(&arg_list, sections, ctx.config, ctx.indent_level, is_multi_mode, has_builtin_grammar, force_args_on_new_line, sub_kws, name.len())
                        } else {
                            let is_custom = !builtins::is_builtin_command(&name_lower);
                            format_argument_list(&arg_list, ctx, is_custom)
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
            let grammar_enum = GrammarRegistry::global().get(&name_lower);
            // Resolve multi-mode grammar
            let first_keyword = grammar_enum.as_ref()
                .filter(|g| g.is_multi_mode())
                .and_then(|_| detect_mode_keyword(&arg_list));
            let grammar = grammar_enum.and_then(|g| g.resolve(first_keyword.as_deref()));
            // If no builtin grammar, check user grammars (builtins take precedence)
            let user_grammar = if grammar.is_none() {
                ctx.user_grammars.get(&name_lower)
            } else {
                None
            };
            let effective_grammar = grammar.or(user_grammar);
            // Skip keyword-aware formatting for unrecognized modes in multi-mode commands
            let is_unrecognized_mode = grammar_enum.as_ref()
                .map_or(false, |g| g.is_multi_mode() && grammar.is_none());
            if !is_unrecognized_mode && (effective_grammar.is_some() || cmake_rules::is_keyword_aware_command(&name)) {
                let sections = cmake_rules::parse_keyword_sections_with_grammar(&arg_list, effective_grammar, ctx.config.comment_style);
                let is_multi_mode = grammar_enum.as_ref().map_or(false, |g| g.is_multi_mode());
                let has_builtin_grammar = grammar.is_some();
                let force_args_on_new_line = effective_grammar.map_or(false, |g| g.force_args_on_new_line);
                let sub_kws = effective_grammar.map(|g| &g.sub_keywords).filter(|s| !s.is_empty());
                cmake_rules::format_keyword_aware_args(&arg_list, sections, ctx.config, ctx.indent_level, is_multi_mode, has_builtin_grammar, force_args_on_new_line, sub_kws, name.len())
            } else {
                let is_custom = !builtins::is_builtin_command(&name_lower);
                format_argument_list(&arg_list, ctx, is_custom)
            }
        } else {
            RcDoc::nil()
        }
    };

    // Format as: indent + name + ( + args + )
    // Conditionally insert a space before ( for block/control-flow commands
    // and a space after ( if space_between_command_parens is enabled
    let space_before = if ctx.config.control_flow_space_before_paren && is_block_command(&name_lower) {
        " "
    } else {
        ""
    };
    let has_args = cmd.argument_list().map_or(false, |al| {
        al.syntax().children_with_tokens().any(|c| {
            matches!(c.kind(), SyntaxKind::UNQUOTED_ARGUMENT
                | SyntaxKind::QUOTED_ARGUMENT
                | SyntaxKind::BRACKET_ARGUMENT
                | SyntaxKind::VARIABLE_REF
                | SyntaxKind::ENV_VAR_REF
                | SyntaxKind::CACHE_VAR_REF
                | SyntaxKind::GENERATOR_EXPR)
        })
    });
    let space_after = if ctx.config.space_between_command_parens && has_args {
        " "
    } else {
        ""
    };
    let paren_open = format!("{}({}", space_before, space_after);
    let cmd_doc = RcDoc::text(formatted_name)
        .append(RcDoc::text(paren_open))
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
        force_multiline,
    }
}

/// Format an argument list with intelligent line breaking
fn format_argument_list(arg_list: &ArgumentList, ctx: &FormatContext, is_custom_command: bool) -> RcDoc<'static, ()> {
    let args = collect_logical_args(arg_list);

    if args.is_empty() {
        return RcDoc::nil();
    }

    // Detect formatting signals
    let signals = detect_argument_formatting_signals(arg_list);

    // If no multiline signals, use auto-layout (flat_alt + group)
    // ARGL-03: For builtin commands, first arg stays on same line when broken
    // For custom commands, ALL args break to new lines when broken
    // Guard: if args > 200, skip auto-layout to avoid deep RcDoc::concat trees
    // (a 200+ arg command will never fit on one line anyway)
    if !signals.force_multiline && args.len() <= 200 {
        if args.len() == 1 {
            // Single argument: simple case
            return RcDoc::text(args[0].clone());
        }

        // Use explicit text indentation via flat_alt instead of nest()
        // This correctly handles tabs and respects the command's nesting depth
        let inner_indent = indent_string(ctx.indent_level + 1, ctx.config);

        if is_custom_command {
            // Custom command: ALL args break to new lines when broken
            // When flat: "arg1 arg2 arg3"
            // When broken: "\n<inner>arg1\n<inner>arg2\n<inner>arg3\n<base>"
            let mut all_docs = Vec::new();

            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    // Separator before arg (except first): flat → space, broken → newline + indent
                    all_docs.push(RcDoc::flat_alt(
                        RcDoc::hardline().append(RcDoc::text(inner_indent.clone())),
                        RcDoc::space(),
                    ));
                } else {
                    // First arg: flat → no separator, broken → newline + indent
                    all_docs.push(RcDoc::flat_alt(
                        RcDoc::hardline().append(RcDoc::text(inner_indent.clone())),
                        RcDoc::nil(),
                    ));
                }
                all_docs.push(RcDoc::text(arg.clone()));
            }

            // Closing paren position
            all_docs.push(closing_paren_position(ctx.config, ctx.indent_level, false));

            // Group all arguments together - when it doesn't fit flat, all break
            return RcDoc::concat(all_docs).group();
        } else {
            // Builtin command: first arg stays inline, rest break
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

            // Closing paren position
            rest_docs.push(closing_paren_position(ctx.config, ctx.indent_level, false));

            // When flat: "first rest1 rest2"
            // When broken: "first\n<inner>rest1\n<inner>rest2\n<base>"
            return RcDoc::text(first_text)
                .append(
                    RcDoc::concat(rest_docs)
                        .group()
                );
        }
    }

    // Force multiline: walk tokens and build output
    // Use explicit text indentation instead of nest() for correct tab/space handling
    let base_indent = indent_string(ctx.indent_level, ctx.config);
    let inner_indent = indent_string(ctx.indent_level + 1, ctx.config);

    // Strategy: For builtin commands (ARGL-03), first arg not indented, rest indented
    // For custom commands, ALL args indented (including first)
    //
    // Build directly as a String to avoid deeply-nested RcDoc::concat trees
    // that overflow the stack on Drop for commands with 1000+ arguments.
    let mut first_arg: Option<String> = None;
    let mut rest_parts = String::new();
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
                        let text = token.text();

                        if !saw_separator && seen_first_arg {
                            // Adjacent to previous token (no whitespace) — merge
                            rest_parts.push_str(text);
                        } else if !seen_first_arg {
                            if is_custom_command {
                                // Custom command: first arg gets newline + indent
                                rest_parts.push('\n');
                                rest_parts.push_str(&inner_indent);
                                rest_parts.push_str(text);
                            } else {
                                // Builtin command: first argument no indent, no break before
                                first_arg = Some(text.to_string());
                            }
                            seen_first_arg = true;
                        } else {
                            // Subsequent arguments: newline + indent
                            rest_parts.push('\n');

                            // If there were blank lines before this arg, emit extra newlines
                            if consecutive_newline_count >= 2 {
                                let blank_lines = consecutive_newline_count - 1;
                                let blank_lines_to_emit = std::cmp::min(blank_lines, ctx.config.max_blank_lines);
                                for _ in 0..blank_lines_to_emit {
                                    rest_parts.push('\n');
                                }
                            }

                            rest_parts.push_str(&inner_indent);
                            rest_parts.push_str(text);
                        }
                        saw_separator = false;
                        consecutive_newline_count = 0;
                    }
                    SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
                        // Comments at same indent level as arguments
                        let text = token.text();
                        rest_parts.push('\n');

                        // If there were blank lines before this comment, emit extra newlines
                        if consecutive_newline_count >= 2 {
                            let blank_lines = consecutive_newline_count - 1;
                            let blank_lines_to_emit = std::cmp::min(blank_lines, ctx.config.max_blank_lines);
                            for _ in 0..blank_lines_to_emit {
                                rest_parts.push('\n');
                            }
                        }

                        rest_parts.push_str(&inner_indent);
                        rest_parts.push_str(text);
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

    // Compute closing indent for force-multiline path
    let closing_indent = if ctx.config.indent_closing_paren {
        indent_string(ctx.indent_level + 1, ctx.config)
    } else {
        base_indent.clone()
    };

    // Build final Doc IR from pre-rendered string
    // Using RcDoc::text with pre-rendered content avoids deeply-nested concat trees
    if is_custom_command {
        if !rest_parts.is_empty() {
            RcDoc::text(rest_parts)
                .append(RcDoc::text(format!("\n{}", closing_indent)))
        } else {
            RcDoc::nil()
        }
    } else {
        if let Some(first) = first_arg {
            if !rest_parts.is_empty() {
                RcDoc::text(first)
                    .append(RcDoc::text(rest_parts))
                    .append(RcDoc::text(format!("\n{}", closing_indent)))
            } else {
                RcDoc::text(first)
            }
        } else {
            RcDoc::nil()
        }
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

/// Detect the mode keyword for multi-mode commands
/// Returns the first unquoted argument, or None if the first arg is a variable reference
fn detect_mode_keyword(arg_list: &ArgumentList) -> Option<String> {
    for child in arg_list.syntax().children_with_tokens() {
        if let NodeOrToken::Token(token) = child {
            match token.kind() {
                SyntaxKind::UNQUOTED_ARGUMENT => {
                    // First unquoted argument is the mode keyword
                    return Some(token.text().to_string());
                }
                SyntaxKind::VARIABLE_REF
                | SyntaxKind::ENV_VAR_REF
                | SyntaxKind::CACHE_VAR_REF
                | SyntaxKind::GENERATOR_EXPR => {
                    // Variable reference in mode position - fallback to simple formatting
                    return None;
                }
                SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE | SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
                    // Skip whitespace and comments
                    continue;
                }
                _ => {
                    // Other token types (quoted args, etc.) shouldn't be mode keywords
                    continue;
                }
            }
        }
    }
    None
}

/// Build an indentation string for the given level, respecting tabs/spaces config
pub(crate) fn indent_string(level: usize, config: &FormatConfig) -> String {
    if config.use_tabs {
        "\t".repeat(level)
    } else {
        " ".repeat(level * config.indent_width)
    }
}

/// Build the Doc IR for the closing paren position in a command.
///
/// In flat mode: nothing (or a space if space_between_command_parens is set).
/// In broken mode: newline + appropriate indent (base or base+1 depending on indent_closing_paren).
pub(crate) fn closing_paren_position(
    config: &FormatConfig,
    indent_level: usize,
    force_multiline: bool,
) -> RcDoc<'static, ()> {
    let closing_indent = if config.indent_closing_paren {
        indent_string(indent_level + 1, config)
    } else {
        indent_string(indent_level, config)
    };
    let flat_text = if config.space_between_command_parens {
        RcDoc::text(" ")
    } else {
        RcDoc::nil()
    };

    if force_multiline {
        RcDoc::hardline().append(RcDoc::text(closing_indent))
    } else {
        RcDoc::flat_alt(
            RcDoc::hardline().append(RcDoc::text(closing_indent)),
            flat_text,
        )
    }
}

/// Check if command is a block opener
fn is_block_opener(name: &str) -> bool {
    matches!(name, "if" | "foreach" | "while" | "function" | "macro")
}

/// Check if command is a control flow / block statement (union of opener, mid, closer, plus block/endblock)
fn is_block_command(name: &str) -> bool {
    matches!(
        name,
        "if" | "elseif"
            | "else"
            | "endif"
            | "foreach"
            | "endforeach"
            | "while"
            | "endwhile"
            | "macro"
            | "endmacro"
            | "function"
            | "endfunction"
            | "block"
            | "endblock"
    )
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
