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
//! The comparison is over tokens, not characters, because four settings
//! legitimately change characters:
//!
//! - `command_case` and `user_command_case` re-case a command's name
//! - `closing_style` adds or removes a block closer's arguments
//! - `comment_style` moves the space after a `#`
//! - `sort_sources` and `source_grouping` permute an unordered list
//!
//! Each of those is a hole in the check, so each is cut as narrowly as the
//! setting that needs it: a closer is compared against what `closing_style`
//! will actually emit for it rather than having its arguments discarded, and a
//! list is compared as a multiset only over the argument runs a grammar marks
//! as unordered. Anything this module cannot model is compared verbatim, so an
//! unrecognised shape reads as a difference rather than as agreement.

use std::collections::HashMap;

use crate::cst::{ArgumentList, CSTRoot, parse_text};
use crate::syntax_kind::SyntaxKind;
use rowan::NodeOrToken;

use super::config::{
    ClosingStyle, CommandCase, CommentStyle, FormatConfig, SortSources, SourceGrouping,
    UserCommandCase,
};
use super::grammar::{CommandGrammar, GrammarRegistry};

/// Grammars discovered from the project and the config, as the formatter saw them.
type UserGrammars = HashMap<String, CommandGrammar>;

/// Which text a [`Content`] was read from.
///
/// `closing_style` rewrites a closer's arguments in one direction only — from
/// the opener's, or to nothing — so the two sides are not normalised the same
/// way. The input is normalised to what the setting says the output should
/// contain; the output is read as written, so output that says something else
/// is still a difference.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Side {
    Input,
    Output,
}

/// What a file says, with the things the formatter is allowed to change removed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Content {
    entries: Vec<Entry>,
}

#[derive(Debug, PartialEq, Eq)]
enum Entry {
    /// A command and its arguments.
    Command {
        name: String,
        /// The invocation's own tokens other than its name: its parentheses,
        /// and anything the parser left between the name and the `(` — a
        /// comment written there lands here rather than in the argument list.
        /// Never reordered, and compared verbatim, so a synthesized or dropped
        /// paren is a difference.
        shape: Vec<String>,
        /// The argument list's contents.
        args: Vec<String>,
    },
    /// Anything else carrying content: a top-level comment, or the stray tokens
    /// an error-recovery region leaves behind.
    Loose(String),
}

/// Describes a difference, for the warning the caller prints.
pub(crate) struct Difference {
    pub(crate) summary: String,
}

/// The exemptions in force for one comparison.
struct Rules<'a> {
    config: &'a FormatConfig,
    grammars: &'a UserGrammars,
    /// Either casing setting can rewrite a command's name, so the name is
    /// compared case-folded unless both are set to preserve it.
    fold_case: bool,
    /// Either reordering pass can permute an unordered list.
    reorders: bool,
    side: Side,
}

impl<'a> Rules<'a> {
    fn new(config: &'a FormatConfig, grammars: &'a UserGrammars, side: Side) -> Self {
        Self {
            config,
            grammars,
            fold_case: config.command_case != CommandCase::Preserve
                || config.user_command_case != UserCommandCase::Preserve,
            reorders: config.sort_sources != SortSources::None
                || config.source_grouping != SourceGrouping::None,
            side,
        }
    }
}

impl Content {
    /// Read the content of `source` under `config`.
    pub(crate) fn read(
        source: &str,
        config: &FormatConfig,
        grammars: &UserGrammars,
        side: Side,
    ) -> Self {
        Self::from_cst(&parse_text(source), &Rules::new(config, grammars, side))
    }

    fn from_cst(cst: &CSTRoot, rules: &Rules) -> Self {
        let mut entries = Vec::new();
        // Arguments of the block openers still open, innermost last. A forced
        // closer echoes the innermost one, exactly as the formatter builds it.
        let mut openers: Vec<Vec<String>> = Vec::new();

        for child in cst.root.children_with_tokens() {
            match child {
                NodeOrToken::Node(node) if node.kind() == SyntaxKind::COMMAND_INVOCATION => {
                    let Some(entry) = command_entry(&node, rules, &mut openers) else {
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
                        if let Some(text) = significant(&token, rules.config.comment_style) {
                            entries.push(Entry::Loose(text));
                        }
                    }
                }
                NodeOrToken::Token(token) => {
                    if let Some(text) = significant(&token, rules.config.comment_style) {
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
        Entry::Command { name, shape, args } => {
            // `shape` holds the parens and anything written before them, so a
            // command that lost one reads as having lost one rather than as a
            // well-formed call
            let mut rendered = name.clone();
            for stray in shape.iter().filter(|t| *t != "(" && *t != ")") {
                rendered.push(' ');
                rendered.push_str(stray);
            }
            if !shape.iter().any(|t| t == "(") {
                rendered.push_str(" with no parentheses");
                return truncate(&rendered);
            }
            rendered.push('(');
            rendered.push_str(&args.join(" "));
            if shape.iter().any(|t| t == ")") {
                rendered.push(')');
            } else {
                rendered.push_str(" — unclosed");
            }
            rendered
        }
        Entry::Loose(text) => text.clone(),
    };
    truncate(&raw)
}

/// One readable line, whatever the entry held.
fn truncate(raw: &str) -> String {
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
    rules: &Rules,
    openers: &mut Vec<Vec<String>>,
) -> Option<Entry> {
    let mut name = None;
    let mut shape = Vec::new();
    let mut arg_list = None;

    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(token) if token.kind() == SyntaxKind::COMMAND_NAME => {
                name = Some(token.text().to_string());
            }
            NodeOrToken::Token(token) => {
                // The parens, and any comment the parser bumped out of the
                // argument list. Nothing here is the formatter's to change.
                if let Some(text) = significant(&token, rules.config.comment_style) {
                    shape.push(text);
                }
            }
            NodeOrToken::Node(list) if list.kind() == SyntaxKind::ARGUMENT_LIST => {
                arg_list = ArgumentList::cast(list);
            }
            NodeOrToken::Node(other) => {
                // Not a shape this module models: compare its tokens verbatim
                // rather than discarding them.
                for token in other
                    .descendants_with_tokens()
                    .filter_map(|it| it.into_token())
                {
                    if let Some(text) = significant(&token, rules.config.comment_style) {
                        shape.push(text);
                    }
                }
            }
        }
    }

    let name = name?;
    let name_lower = name.to_lowercase();

    let mut args = arg_list
        .as_ref()
        .map(|list| comparable_args(list, &name_lower, rules))
        .unwrap_or_default();

    // Block bookkeeping mirrors the formatter's: an opener's arguments are what
    // a forced closer is built from.
    if is_block_opener(&name_lower) {
        openers.push(args.clone());
    } else if let Some(governed) = closer_kind(&name_lower) {
        let opener = match governed {
            Governed::Closer => openers.pop(),
            Governed::MidBlock => openers.last().cloned(),
        };
        // An unmatched closer keeps its own arguments, because the formatter
        // has no opener to rebuild it from and leaves it alone too.
        if let Some(opener) = opener {
            match (rules.config.closing_style, rules.side) {
                (ClosingStyle::Preserve, _) | (_, Side::Output) => {}
                (ClosingStyle::Remove, Side::Input) => args.clear(),
                (ClosingStyle::Force, Side::Input) => args = opener,
            }
        }
    }

    Some(Entry::Command {
        name: if rules.fold_case { name_lower } else { name },
        shape,
        args,
    })
}

/// The arguments of one command, with the runs a grammar marks as unordered
/// canonicalised so a permutation of them is not a difference.
fn comparable_args(arg_list: &ArgumentList, name_lower: &str, rules: &Rules) -> Vec<String> {
    if rules.reorders
        && let Some(canonical) = reorderable_args(arg_list, name_lower, rules)
    {
        return canonical;
    }

    // Nothing here may move, so order is part of the content.
    let mut args = Vec::new();
    collect_args(arg_list.syntax(), rules.config.comment_style, &mut args);
    args
}

/// Walk one argument list, rendering a nested `( … )` group as a single atom.
///
/// Flattening a group would let the multiset comparison move an argument across
/// its parentheses — `(a.cpp b.cpp) c.cpp` and `(a.cpp c.cpp) b.cpp` hold the
/// same tokens — which is exactly the reordering a group exists to prevent.
fn collect_args(node: &crate::SyntaxNode, comment_style: CommentStyle, out: &mut Vec<String>) {
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(token) => {
                if let Some(text) = significant(&token, comment_style) {
                    out.push(text);
                }
            }
            NodeOrToken::Node(nested) => {
                let mut inner = Vec::new();
                collect_args(&nested, comment_style, &mut inner);
                out.push(format!("({})", inner.join(" ")));
            }
        }
    }
}

/// The canonical form of an argument list that holds at least one unordered run,
/// or `None` when the grammar marks nothing here as unordered.
///
/// Keywords hold their place; only the values a section marks sortable are
/// sorted, and only within the runs a barrier leaves them. That is coarser than
/// the sorting pass in one respect — it ignores the blank-line segmentation, so
/// it permits a few permutations the formatter would not produce — which costs
/// tightness, never correctness: the same canonical form is applied to both
/// sides.
fn reorderable_args(
    arg_list: &ArgumentList,
    name_lower: &str,
    rules: &Rules,
) -> Option<Vec<String>> {
    let grammar = resolve_grammar(arg_list, name_lower, rules)?;
    let sections = super::cmake_rules::parse_keyword_sections_with_grammar(
        arg_list,
        Some(grammar),
        rules.config.comment_style,
    );

    let mut out = Vec::new();
    let mut anything_moves = false;

    for section in &sections {
        if let Some(keyword) = &section.keyword {
            out.push(keyword.clone());
        }
        match section.sort_from {
            Some(from) if from < section.args.len() => {
                out.extend(section.args[..from].iter().cloned());
                for run in
                    super::cmake_rules::sortable_runs(&section.args, from..section.args.len())
                {
                    let mut values: Vec<String> = section.args[run].to_vec();
                    if values.len() > 1 {
                        anything_moves = true;
                        values.sort();
                    }
                    out.extend(values);
                }
            }
            _ => out.extend(section.args.iter().cloned()),
        }
    }

    if !anything_moves {
        return None;
    }

    // The sections carry the arguments but not the comments between them, and
    // the sorting pass moves a comment with the argument it belongs to. So a
    // comment's content is compared, its place in the list is not.
    let mut comments: Vec<String> = Vec::new();
    collect_comments(arg_list.syntax(), rules.config.comment_style, &mut comments);
    comments.sort();
    out.extend(comments);

    Some(out)
}

fn collect_comments(node: &crate::SyntaxNode, comment_style: CommentStyle, out: &mut Vec<String>) {
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(token)
                if matches!(
                    token.kind(),
                    SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT
                ) =>
            {
                if let Some(text) = significant(&token, comment_style) {
                    out.push(text);
                }
            }
            NodeOrToken::Node(nested) => collect_comments(&nested, comment_style, out),
            _ => {}
        }
    }
}

/// The grammar the formatter would have used for this command, or `None` when it
/// would have used none.
fn resolve_grammar<'a>(
    arg_list: &ArgumentList,
    name_lower: &str,
    rules: &'a Rules,
) -> Option<&'a CommandGrammar> {
    let builtin = GrammarRegistry::global().get(name_lower);
    let mode = builtin
        .filter(|g| g.is_multi_mode())
        .and_then(|_| super::cst_to_doc::detect_mode_keyword(arg_list));
    if let Some(grammar) = builtin.and_then(|g| g.resolve(mode.as_deref())) {
        // Borrowed from the process-wide registry, which outlives everything
        return Some(grammar);
    }
    // A builtin takes precedence over a user grammar of the same name, which is
    // what the formatter does too.
    rules.grammars.get(name_lower)
}

/// Commands that open a block whose closer `closing_style` governs.
fn is_block_opener(name_lower: &str) -> bool {
    matches!(
        name_lower,
        "if" | "foreach" | "while" | "function" | "macro" | "block"
    )
}

enum Governed {
    /// Ends the block, so it consumes the opener.
    Closer,
    /// Sits inside it, so the opener stays open.
    MidBlock,
}

/// Whether `closing_style` rewrites this command's arguments, and how it reaches
/// its opener.
///
/// `elseif` is deliberately absent: it carries a condition of its own, which the
/// formatter emits as written under every `closing_style`. Exempting it would
/// have let `elseif(B)` become `elseif(NOT B)` unnoticed.
fn closer_kind(name_lower: &str) -> Option<Governed> {
    match name_lower {
        "endif" | "endforeach" | "endwhile" | "endfunction" | "endmacro" | "endblock" => {
            Some(Governed::Closer)
        }
        "else" => Some(Governed::MidBlock),
        _ => None,
    }
}

/// The comparable text of a token, or `None` when it carries no content.
fn significant(
    token: &rowan::SyntaxToken<crate::syntax_kind::CMakeLang>,
    comment_style: CommentStyle,
) -> Option<String> {
    match token.kind() {
        SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE => None,
        SyntaxKind::COMMENT => Some(normalize_comment(token.text(), comment_style)),
        _ => Some(token.text().to_string()),
    }
}

/// A line comment with the whitespace the formatter may move taken out.
///
/// Two changes are permitted: trailing whitespace is always trimmed, and
/// `comment_style` sets the whitespace between the leading `#` run and the text.
/// Everything else — the number of `#`s, and every space inside the text — is
/// content. Stripping all whitespace instead, as this first did, made `# a b`
/// and `# ab` compare equal.
fn normalize_comment(text: &str, comment_style: CommentStyle) -> String {
    let text = text.trim_end();
    let hashes = text.len() - text.trim_start_matches('#').len();
    let (hashes, rest) = text.split_at(hashes);
    if comment_style == CommentStyle::Preserve {
        return format!("{}{}", hashes, rest);
    }
    format!("{}{}", hashes, rest.trim_start())
}

/// Compare what `output` says against what `input` said.
///
/// `None` means the formatter only moved characters around.
pub(crate) fn check(
    input: &str,
    output: &str,
    config: &FormatConfig,
    grammars: &UserGrammars,
) -> Option<Difference> {
    // The CRLF pass runs after formatting as a blanket replacement, so it
    // rewrites newlines inside a bracket argument or bracket comment as well as
    // between lines. Comparing on \n accepts that; it would otherwise be a
    // difference in every token on the line.
    let input = input.replace("\r\n", "\n");
    let output = output.replace("\r\n", "\n");

    Content::read(&input, config, grammars, Side::Input).diff(&Content::read(
        &output,
        config,
        grammars,
        Side::Output,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::config::LineEnding;
    use super::*;

    /// Whether the guard would let this input/output pair through.
    fn accepts(input: &str, output: &str, config: &FormatConfig) -> bool {
        check(input, output, config, &UserGrammars::new()).is_none()
    }

    fn with_closing(style: ClosingStyle) -> FormatConfig {
        FormatConfig {
            closing_style: style,
            ..Default::default()
        }
    }

    #[test]
    fn test_an_elseif_condition_is_never_exempt() {
        // `elseif` carries a condition of its own, which the formatter emits as
        // written under every closing_style. Exempting it alongside the real
        // closers let an inverted condition through.
        for style in [
            ClosingStyle::Preserve,
            ClosingStyle::Remove,
            ClosingStyle::Force,
        ] {
            let config = with_closing(style);
            assert!(
                !accepts(
                    "if(A)\nelseif(B)\nendif()\n",
                    "if(A)\nelseif(NOT B)\nendif()\n",
                    &config
                ),
                "{:?} accepted an inverted elseif condition",
                style
            );
            assert!(
                !accepts(
                    "if(A)\nelseif(B AND C)\nendif()\n",
                    "if(A)\nelseif()\nendif()\n",
                    &config
                ),
                "{:?} accepted a dropped elseif condition",
                style
            );
        }
    }

    #[test]
    fn test_a_forced_closer_is_compared_against_its_opener() {
        let config = with_closing(ClosingStyle::Force);
        // What the setting exists to do
        assert!(accepts("if(A)\nendif()\n", "if(A)\nendif(A)\n", &config));
        assert!(accepts(
            "foreach(x ${l})\nendforeach()\n",
            "foreach(x ${l})\nendforeach(x ${l})\n",
            &config
        ));
        // Anything else it emits is not the opener's condition
        assert!(!accepts(
            "if(A)\nendif()\n",
            "if(A)\nendif(GARBAGE)\n",
            &config
        ));
        // `else` reaches its opener without consuming it
        assert!(accepts(
            "if(A)\nelse()\nendif()\n",
            "if(A)\nelse(A)\nendif(A)\n",
            &config
        ));
    }

    #[test]
    fn test_a_removed_closer_is_exempt_only_where_the_formatter_touches_it() {
        let config = with_closing(ClosingStyle::Remove);
        assert!(accepts("if(A)\nendif(A)\n", "if(A)\nendif()\n", &config));
        // An unmatched closer has no opener to rebuild it from, so the formatter
        // leaves its arguments alone and so must the check
        assert!(!accepts("endif(A)\n", "endif()\n", &config));
    }

    #[test]
    fn test_the_reordering_exemption_is_confined_to_sortable_lists() {
        let config = FormatConfig {
            sort_sources: SortSources::Alphabetical,
            ..Default::default()
        };
        // The list the setting is for
        assert!(accepts(
            "set(SRCS b.cpp a.cpp)\n",
            "set(SRCS a.cpp b.cpp)\n",
            &config
        ));
        // Lists where order is the meaning. One setting anywhere in the config
        // used to downgrade every command in the file to a multiset.
        assert!(!accepts("if(A LESS B)\n", "if(B LESS A)\n", &config));
        assert!(!accepts(
            "list(INSERT L 0 x)\n",
            "list(0 INSERT L x)\n",
            &config
        ));
        assert!(!accepts(
            "set(V x CACHE PATH \"d\")\n",
            "set(V x CACHE \"d\" PATH)\n",
            &config
        ));
        // A keyword may not swap places with its own values
        assert!(!accepts(
            "set(SOURCES a.cpp b.cpp)\n",
            "set(a.cpp SOURCES b.cpp)\n",
            &config
        ));
    }

    #[test]
    fn test_a_group_holds_its_contents_under_reordering() {
        // `(a.cpp b.cpp) c.cpp` and `(a.cpp c.cpp) b.cpp` hold the same tokens,
        // so flattening the group let the multiset comparison move an argument
        // across parentheses — the reordering a group exists to prevent.
        let config = FormatConfig {
            sort_sources: SortSources::Alphabetical,
            ..Default::default()
        };
        assert!(!accepts(
            "set(SOURCES (a.cpp b.cpp) c.cpp)\n",
            "set(SOURCES (a.cpp c.cpp) b.cpp)\n",
            &config
        ));
    }

    #[test]
    fn test_a_comments_content_is_compared_but_its_layout_is_not() {
        let hash_space = FormatConfig::default();
        // What comment_style does
        assert!(accepts("#foo\n", "# foo\n", &hash_space));
        assert!(accepts("#   foo\n", "# foo\n", &hash_space));
        // Trailing whitespace is always the formatter's to trim
        assert!(accepts("# foo   \n", "# foo\n", &hash_space));
        // The text itself is content. Stripping every space, as this first did,
        // made these compare equal.
        assert!(!accepts("# a b\n", "# ab\n", &hash_space));
        assert!(!accepts("#[[a b]]\n", "#[[ab]]\n", &hash_space));
        assert!(!accepts("## foo\n", "# foo\n", &hash_space));
        assert!(!accepts("# foo\n", "# bar\n", &hash_space));

        // Under preserve the formatter may not move that space either
        let preserve = FormatConfig {
            comment_style: CommentStyle::Preserve,
            ..Default::default()
        };
        assert!(!accepts("#foo\n", "# foo\n", &preserve));
        assert!(accepts("# foo  \n", "# foo\n", &preserve));
    }

    #[test]
    fn test_a_commands_own_parentheses_are_compared() {
        let config = FormatConfig::default();
        // The parser hangs these off the invocation rather than the argument
        // list, so a model that only read the argument list could not see an
        // invented closer — one of the bugs this module was written for.
        assert!(!accepts("set(A b", "set(A b)\n", &config));
        assert!(!accepts("set", "set()\n", &config));
        assert!(!accepts("set()\n", "set\n", &config));
        // A comment written between the name and the `(` lands there too
        assert!(!accepts("set # c\n(A b)\n", "set(A b)\n", &config));
        assert!(accepts("set(A b)\n", "set(A  b)\n", &config));
    }

    #[test]
    fn test_case_folding_follows_both_casing_settings() {
        // Only one of the two casing settings was consulted, so a config that
        // preserved builtin casing while re-casing user commands reported every
        // user command as a content change and stopped formatting the file.
        let both_preserve = FormatConfig {
            command_case: CommandCase::Preserve,
            user_command_case: UserCommandCase::Preserve,
            ..Default::default()
        };
        assert!(!accepts("SET(A b)\n", "set(A b)\n", &both_preserve));

        let user_only = FormatConfig {
            command_case: CommandCase::Preserve,
            user_command_case: UserCommandCase::Lowercase,
            ..Default::default()
        };
        assert!(accepts("MY_HELPER(x)\n", "my_helper(x)\n", &user_only));

        let builtin_only = FormatConfig {
            command_case: CommandCase::Lowercase,
            user_command_case: UserCommandCase::Preserve,
            ..Default::default()
        };
        assert!(accepts("SET(A b)\n", "set(A b)\n", &builtin_only));
        // Folding case is not licence to rename
        assert!(!accepts("set(A b)\n", "unset(A b)\n", &builtin_only));
    }

    #[test]
    fn test_crlf_is_not_a_difference() {
        let config = FormatConfig {
            line_ending: LineEnding::CrLf,
            ..Default::default()
        };
        assert!(accepts("set(A b)\n", "set(A b)\r\n", &config));
    }
}
