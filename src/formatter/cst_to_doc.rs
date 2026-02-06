use crate::cst::{ArgumentList, CSTRoot, CommandInvocation};
use crate::syntax_kind::SyntaxKind;
use crate::SyntaxNode;
use pretty::RcDoc;
use rowan::NodeOrToken;

use super::config::{CommandCase, FormatConfig};

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

    /// Create a new context with increased indentation
    fn indent(&self) -> Self {
        Self {
            config: self.config,
            indent_level: self.indent_level + 1,
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

/// Format the FILE node
fn format_file(node: &SyntaxNode, ctx: &FormatContext) -> RcDoc<'static, ()> {
    let mut docs = Vec::new();
    let mut current_indent: usize = 0;
    let mut blank_line_count = 0;
    let mut pending_comment: Option<RcDoc<'static, ()>> = None;

    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Node(child_node) => {
                match child_node.kind() {
                    SyntaxKind::COMMAND_INVOCATION => {
                        // Emit any pending comment before the command
                        if let Some(comment) = pending_comment.take() {
                            docs.push(comment);
                            docs.push(RcDoc::hardline());
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
                            let cmd_doc = format_command(&cmd, &cmd_ctx);
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
                    SyntaxKind::COMMENT => {
                        let text = token.text();
                        let indent_str = " ".repeat(current_indent * ctx.config.indent_width);
                        pending_comment = Some(RcDoc::text(format!("{}{}", indent_str, text)));
                        blank_line_count = 0;
                    }
                    SyntaxKind::BRACKET_COMMENT => {
                        let text = token.text();
                        let indent_str = " ".repeat(current_indent * ctx.config.indent_width);
                        // Bracket comments can be multi-line, preserve as-is with indentation
                        let lines: Vec<_> = text.lines().collect();
                        if let Some((first, rest)) = lines.split_first() {
                            let mut comment_doc = RcDoc::text(format!("{}{}", indent_str, first));
                            for line in rest {
                                comment_doc = comment_doc.append(RcDoc::hardline())
                                    .append(RcDoc::text(format!("{}{}", indent_str, line)));
                            }
                            docs.push(comment_doc);
                            docs.push(RcDoc::hardline());
                        }
                        blank_line_count = 0;
                    }
                    SyntaxKind::NEWLINE => {
                        blank_line_count += 1;
                        // Emit blank lines up to max_blank_lines
                        if blank_line_count > 1 && blank_line_count <= ctx.config.max_blank_lines + 1 {
                            docs.push(RcDoc::hardline());
                        }
                    }
                    SyntaxKind::WHITESPACE => {
                        // Skip - formatter decides whitespace
                    }
                    _ => {}
                }
            }
        }
    }

    // Emit any trailing comment
    if let Some(comment) = pending_comment {
        docs.push(comment);
        docs.push(RcDoc::hardline());
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
        CommandCase::Preserve => name,
    };

    // Get arguments
    let args_doc = if let Some(arg_list) = cmd.argument_list() {
        format_argument_list(&arg_list, ctx)
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

/// Format an argument list with intelligent line breaking
fn format_argument_list(arg_list: &ArgumentList, ctx: &FormatContext) -> RcDoc<'static, ()> {
    let args: Vec<_> = arg_list.arguments().collect();

    if args.is_empty() {
        return RcDoc::nil();
    }

    // Build argument documents with separators
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
    // When broken, nest the arguments
    RcDoc::concat(docs)
        .nest(ctx.config.indent_width as isize)
        .group()
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
