//! A self-check that the formatter only moved characters around.
//!
//! A formatter may re-indent, re-wrap and re-case. It may not invent an argument
//! the author never wrote, and it may not drop one. Both have happened: a
//! parenthesised condition used to be deleted outright, and a file whose last
//! group ran off the end of the file grew a closing paren on every run.
//!
//! So after formatting, the output is re-parsed and its content compared against
//! the input's. On a mismatch the caller leaves the file alone rather than
//! writing something the author did not write.
//!
//! The comparison is over tokens, not characters, for two reasons. Several
//! settings legitimately change characters — `command_case` re-cases, and
//! `closing_style` adds or removes a closer's arguments — so a character
//! comparison would reject a plain `SET(A b)`. And the reordering settings
//! change the order of content without changing the content, which only a
//! token-level view can express.

use crate::cst::{CSTRoot, parse_text};
use crate::syntax_kind::SyntaxKind;
use rowan::NodeOrToken;

use super::config::{ClosingStyle, CommandCase, FormatConfig, SortSources, SourceGrouping};

/// What a file says, with the things the formatter is allowed to change removed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Content {
    entries: Vec<Entry>,
}

#[derive(Debug, PartialEq, Eq)]
enum Entry {
    /// A command and its arguments.
    Command { name: String, args: Vec<String> },
    /// Anything else carrying content: a top-level comment, or the stray tokens
    /// an error-recovery region leaves behind. Compared too, because a
    /// synthesized paren lands here rather than inside any command.
    Loose(String),
}

/// Describes a difference, for the warning the caller prints.
pub(crate) struct Difference {
    pub(crate) summary: String,
}

impl Content {
    /// Read the content of `source` under `config`.
    pub(crate) fn read(source: &str, config: &FormatConfig) -> Self {
        Self::from_cst(&parse_text(source), config)
    }

    fn from_cst(cst: &CSTRoot, config: &FormatConfig) -> Self {
        let fold_case = config.command_case != CommandCase::Preserve;
        // Either reordering pass permutes arguments, so order stops being
        // comparable and only the multiset is.
        let reorders = config.sort_sources != SortSources::None
            || config.source_grouping != SourceGrouping::None;

        let mut entries = Vec::new();

        for child in cst.root.children_with_tokens() {
            match child {
                NodeOrToken::Node(node) if node.kind() == SyntaxKind::COMMAND_INVOCATION => {
                    let Some(entry) = command_entry(&node, config, fold_case, reorders) else {
                        continue;
                    };
                    entries.push(entry);
                }
                NodeOrToken::Node(node) => {
                    // An ERROR node, or anything else the parser could not make
                    // a command of
                    for token in node
                        .descendants_with_tokens()
                        .filter_map(|it| it.into_token())
                    {
                        if let Some(text) = significant(&token) {
                            entries.push(Entry::Loose(text));
                        }
                    }
                }
                NodeOrToken::Token(token) => {
                    if let Some(text) = significant(&token) {
                        entries.push(Entry::Loose(text));
                    }
                }
            }
        }

        Self { entries }
    }

    /// The first difference from `other`, or `None` when they say the same thing.
    pub(crate) fn diff(&self, other: &Self) -> Option<Difference> {
        for (index, (before, after)) in self.entries.iter().zip(other.entries.iter()).enumerate() {
            if before != after {
                return Some(Difference {
                    summary: format!(
                        "at item {}: {} became {}",
                        index + 1,
                        describe(before),
                        describe(after)
                    ),
                });
            }
        }

        match self.entries.len().cmp(&other.entries.len()) {
            std::cmp::Ordering::Less => Some(Difference {
                summary: format!("gained {}", describe(&other.entries[self.entries.len()])),
            }),
            std::cmp::Ordering::Greater => Some(Difference {
                summary: format!("lost {}", describe(&self.entries[other.entries.len()])),
            }),
            std::cmp::Ordering::Equal => None,
        }
    }
}

/// A short, single-line rendering — this ends up in a warning, so a whole
/// multi-line command would drown it.
fn describe(entry: &Entry) -> String {
    let raw = match entry {
        Entry::Command { name, args } if args.is_empty() => format!("{}()", name),
        Entry::Command { name, args } => format!("{}({})", name, args.join(" ")),
        Entry::Loose(text) => text.clone(),
    };

    let flattened: String = raw
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect();

    const LIMIT: usize = 48;
    if flattened.chars().count() > LIMIT {
        let head: String = flattened.chars().take(LIMIT).collect();
        format!("`{}…`", head.trim_end())
    } else {
        format!("`{}`", flattened)
    }
}

fn command_entry(
    node: &crate::SyntaxNode,
    config: &FormatConfig,
    fold_case: bool,
    reorders: bool,
) -> Option<Entry> {
    let mut name = None;
    let mut args = Vec::new();

    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(token) if token.kind() == SyntaxKind::COMMAND_NAME => {
                name = Some(token.text().to_string());
            }
            NodeOrToken::Node(list) if list.kind() == SyntaxKind::ARGUMENT_LIST => {
                for token in list
                    .descendants_with_tokens()
                    .filter_map(|it| it.into_token())
                {
                    if let Some(text) = significant(&token) {
                        args.push(text);
                    }
                }
            }
            _ => {}
        }
    }

    let name = name?;
    let comparable = if fold_case {
        name.to_lowercase()
    } else {
        name.clone()
    };

    // `closing_style` exists to add or remove a closer's arguments, so those are
    // not comparable unless the author asked for them to be preserved.
    if config.closing_style != ClosingStyle::Preserve && is_block_closer(&name.to_lowercase()) {
        args.clear();
    }

    if reorders {
        args.sort();
    }

    Some(Entry::Command {
        name: comparable,
        args,
    })
}

/// A closer or mid-block command, whose arguments `closing_style` governs.
fn is_block_closer(name_lower: &str) -> bool {
    matches!(
        name_lower,
        "endif"
            | "else"
            | "elseif"
            | "endforeach"
            | "endwhile"
            | "endfunction"
            | "endmacro"
            | "endblock"
    )
}

/// The comparable text of a token, or `None` when it carries no content.
fn significant(token: &rowan::SyntaxToken<crate::syntax_kind::CMakeLang>) -> Option<String> {
    match token.kind() {
        SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE => None,
        // `comment_style` inserts or removes the space after `#`, so a comment's
        // own whitespace is not comparable — its content is.
        SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
            let stripped: String = token
                .text()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            Some(stripped)
        }
        _ => Some(token.text().to_string()),
    }
}

/// Compare what `output` says against what `input` said.
///
/// `None` means the formatter only moved characters around.
pub(crate) fn check(input: &str, output: &str, config: &FormatConfig) -> Option<Difference> {
    // The CRLF pass runs after formatting; compare on \n so it isn't a
    // difference in every token.
    let input = input.replace("\r\n", "\n");
    let output = output.replace("\r\n", "\n");

    Content::read(&input, config).diff(&Content::read(&output, config))
}
