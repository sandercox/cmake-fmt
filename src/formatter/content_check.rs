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
//! setting that needs it: a closer may say either what its author wrote or what
//! `closing_style` would emit for it — which is what makes comparing a text
//! against itself impossible to fail — and a list is compared as a multiset only
//! over the argument runs a grammar marks as unordered. Anything this module cannot model is compared verbatim, so an
//! unrecognised shape reads as a difference rather than as agreement.
//!
//! Known holes, all in the direction of missing a change rather than inventing
//! one: a comment's position within a command is not compared, because the
//! formatter re-places one legitimately; and neither the reordering canonicalisation nor the closer exemption knows
//! about suppression, so inside a `# cmake-fmt: off` region — or after
//! `no-sort` or `skip` — this would accept a permutation or a rewritten closer
//! that the formatter must never produce there. Closing them means giving this
//! module the same region tracking the formatter has.

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
/// Only the input side carries the readings a closer is allowed to have, since
/// only it can see the opener. Nothing else differs between the sides, so this
/// is an optimisation — computing the alternatives for the output too would be
/// harmless and unused.
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
        /// The argument list's contents, exactly as this side wrote them.
        args: Vec<String>,
        /// For a block closer on the input side: the arguments `closing_style`
        /// may replace `args` with. The output is accepted if it wrote either.
        ///
        /// Normalising the input to what the setting *will* emit was wrong,
        /// because plenty of closers are not touched even when the setting is
        /// on: inside a `# cmake-fmt: off` region, on a line `--line-ranges`
        /// did not select, on a command the formatter's own block lists do not
        /// carry, and wherever the opener's arguments hold something the
        /// formatter drops. Every one of those refused a file the formatter had
        /// written correctly. Offering both readings is reflexive by
        /// construction — `check(x, x)` cannot fire — while still refusing a
        /// closer that says something neither the author nor the setting asked
        /// for.
        closer_alternatives: Vec<Vec<String>>,
        /// Byte offset of the command in its source, for the warning's line
        /// number.
        offset: usize,
    },
    /// Anything else carrying content: a top-level comment, or the stray tokens
    /// an error-recovery region leaves behind.
    Loose(String),
}

impl Entry {
    /// Whether `output` says what this input entry said.
    fn agrees_with(&self, output: &Entry) -> bool {
        match (self, output) {
            (
                Entry::Command {
                    name: input_name,
                    shape: input_shape,
                    args: input_args,
                    closer_alternatives,
                    ..
                },
                Entry::Command {
                    name: output_name,
                    shape: output_shape,
                    args: output_args,
                    ..
                },
            ) => {
                input_name == output_name
                    && input_shape == output_shape
                    && (input_args == output_args
                        || closer_alternatives
                            .iter()
                            .any(|allowed| allowed == output_args))
            }
            (Entry::Loose(input), Entry::Loose(output)) => input == output,
            _ => false,
        }
    }
}

/// Describes a difference, for the warning the caller prints.
pub(crate) struct Difference {
    pub(crate) summary: String,
}

/// The exemptions in force for one comparison.
///
/// Every one of them is keyed to a setting, and a file can change its own
/// settings with `# cmake-fmt: <key>=<value>` — which the formatter applies as
/// it goes. So the settings here are the *union* of the file-level config and
/// every override the file sets on itself: an exemption the formatter had open
/// while this one was closed refused the file outright, at default settings, for
/// a documented feature of the tool. Taking the union can miss a change; keying
/// off a config the formatter has already discarded invents one.
struct Rules<'a> {
    config: &'a FormatConfig,
    grammars: &'a UserGrammars,
    /// Either casing setting can rewrite a command's name, so the name is
    /// compared case-folded unless both preserve it everywhere in the file.
    fold_case: bool,
    /// Either reordering pass can permute an unordered list.
    reorders: bool,
    /// A closer's arguments may be rebuilt from its opener's.
    force_possible: bool,
    /// A closer's arguments may be dropped.
    remove_possible: bool,
    /// A comment's whitespace may be rewritten, so it is not comparable.
    /// `normalize_comment` only asks whether the style is `Preserve`, so any
    /// other value stands for "the formatter may move it".
    comment_style: CommentStyle,
    side: Side,
}

impl<'a> Rules<'a> {
    fn new(
        config: &'a FormatConfig,
        overrides: &StyleOverrides,
        grammars: &'a UserGrammars,
        side: Side,
    ) -> Self {
        Self {
            config,
            grammars,
            fold_case: config.command_case != CommandCase::Preserve
                || config.user_command_case != UserCommandCase::Preserve
                || overrides.recases,
            reorders: config.sort_sources != SortSources::None
                || config.source_grouping != SourceGrouping::None
                || overrides.reorders,
            force_possible: config.closing_style == ClosingStyle::Force || overrides.forces_closers,
            remove_possible: config.closing_style == ClosingStyle::Remove
                || overrides.removes_closers,
            comment_style: if config.comment_style != CommentStyle::Preserve
                || overrides.restyles_comments
            {
                CommentStyle::HashSpace
            } else {
                CommentStyle::Preserve
            },
            side,
        }
    }
}

/// What the file's own `# cmake-fmt: <key>=<value>` directives can turn on.
#[derive(Default)]
pub(crate) struct StyleOverrides {
    recases: bool,
    reorders: bool,
    forces_closers: bool,
    removes_closers: bool,
    restyles_comments: bool,
}

impl StyleOverrides {
    /// Read every style directive in the file, whether or not the formatter
    /// reached it — a directive applies from where it appears, and there is no
    /// way to know from here which ones the formatter will reach. The cost is
    /// looseness, not strictness: a `sort_sources=alphabetical` directive
    /// inside a `# cmake-fmt: off` region opens that exemption for the whole
    /// file. Deliberate, and the direction to err in — the alternative refuses
    /// files the formatter wrote correctly.
    ///
    /// Each directive is applied to a copy of the config through the same
    /// `apply_override` the formatter uses, and the *result* is inspected. Doing
    /// it by matching keys and values by hand missed `comment_style` entirely —
    /// one of this module's own four exemptions — and accepted values
    /// `apply_override` rejects, so a typo opened an exemption for the whole
    /// file.
    ///
    /// The comments come from the tree, not from scanning lines for a `#`: the
    /// first `#` on a line can be inside a quoted argument, a bracket argument
    /// or a bracket comment, and a directive after one of those was invisible.
    fn read(cst: &CSTRoot, config: &FormatConfig) -> Self {
        let mut overrides = Self::default();

        for token in cst
            .root
            .descendants_with_tokens()
            .filter_map(|child| child.into_token())
            .filter(|token| token.kind() == SyntaxKind::COMMENT)
        {
            let Some(super::suppression::Directive::Style { key, value }) =
                super::suppression::parse_directive(token.text().trim())
            else {
                continue;
            };

            let mut probe = config.clone();
            // A value the formatter rejects is a value the formatter never
            // applied, so it must not widen anything
            if probe.apply_override(&key, &value).is_err() {
                continue;
            }

            overrides.recases |= probe.command_case != CommandCase::Preserve
                || probe.user_command_case != UserCommandCase::Preserve;
            overrides.reorders |= probe.sort_sources != SortSources::None
                || probe.source_grouping != SourceGrouping::None;
            overrides.forces_closers |= probe.closing_style == ClosingStyle::Force;
            overrides.removes_closers |= probe.closing_style == ClosingStyle::Remove;
            overrides.restyles_comments |= probe.comment_style != CommentStyle::Preserve;
        }

        overrides
    }
}

impl Content {
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
                        if let Some(text) = significant(&token, rules.comment_style) {
                            entries.push(Entry::Loose(text));
                        }
                    }
                }
                NodeOrToken::Token(token) => {
                    if let Some(text) = significant(&token, rules.comment_style) {
                        entries.push(Entry::Loose(text));
                    }
                }
            }
        }

        Self { entries }
    }

    /// The first difference from `other`, or `None` when they say the same thing.
    pub(crate) fn diff(&self, other: &Self, source: &str) -> Option<Difference> {
        for (before, after) in self.entries.iter().zip(other.entries.iter()) {
            if !before.agrees_with(after) {
                let (before_text, after_text) = describe_pair(before, after);
                return Some(Difference {
                    summary: format!(
                        "{}: {} became {}",
                        locate(before, source),
                        before_text,
                        after_text
                    ),
                });
            }
        }

        match self.entries.len().cmp(&other.entries.len()) {
            std::cmp::Ordering::Less => {
                let gained = &other.entries[self.entries.len()];
                Some(Difference {
                    summary: format!("gained {}", describe(gained)),
                })
            }
            std::cmp::Ordering::Greater => {
                let lost = &self.entries[other.entries.len()];
                Some(Difference {
                    summary: format!("{}: lost {}", locate(lost, source), describe(lost)),
                })
            }
            std::cmp::Ordering::Equal => None,
        }
    }
}

/// Where in the input an entry sits, for the warning.
fn locate(entry: &Entry, source: &str) -> String {
    match entry {
        Entry::Command { offset, .. } => {
            let line = source[..(*offset).min(source.len())]
                .bytes()
                .filter(|b| *b == b'\n')
                .count()
                + 1;
            format!("line {}", line)
        }
        Entry::Loose(_) => "somewhere".to_string(),
    }
}

/// Render two entries so their difference is visible.
///
/// Truncating each to its first 48 characters independently rendered both
/// corpus firings identically — `set(msvc_warning_flags /wd4141 #'modifier'…`
/// on both sides — which told the reader nothing at all. This keeps a little
/// context before the point where they diverge.
fn describe_pair(before: &Entry, after: &Entry) -> (String, String) {
    let before_full = render(before);
    let after_full = render(after);

    let diverge = before_full
        .char_indices()
        .zip(after_full.char_indices())
        .find(|((_, a), (_, b))| a != b)
        .map(|((index, _), _)| index)
        .unwrap_or_else(|| before_full.len().min(after_full.len()));

    const CONTEXT: usize = 16;
    const WIDTH: usize = 48;
    let start = before_full[..diverge]
        .char_indices()
        .rev()
        .nth(CONTEXT)
        .map(|(index, _)| index)
        .unwrap_or(0);

    (
        window(&before_full, start, WIDTH),
        window(&after_full, start, WIDTH),
    )
}

fn window(text: &str, start: usize, width: usize) -> String {
    let flattened: String = text
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    let start = flattened
        .char_indices()
        .map(|(index, _)| index)
        .find(|index| *index >= start)
        .unwrap_or(0);
    let head = if start > 0 { "…" } else { "" };
    let body: String = flattened[start..].chars().take(width).collect();
    let tail = if flattened[start..].chars().count() > width {
        "…"
    } else {
        ""
    };
    format!("`{}{}{}`", head, body.trim_end(), tail)
}

/// A short, single-line rendering — this ends up in a warning, so a whole
/// multi-line command would drown it.
fn describe(entry: &Entry) -> String {
    window(&render(entry), 0, 48)
}

/// The full text of an entry, before any windowing.
fn render(entry: &Entry) -> String {
    match entry {
        Entry::Command {
            name, shape, args, ..
        } => {
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
                return rendered;
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
                if let Some(text) = significant(&token, rules.comment_style) {
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
                    if let Some(text) = significant(&token, rules.comment_style) {
                        shape.push(text);
                    }
                }
            }
        }
    }

    let name = name?;
    let name_lower = name.to_lowercase();

    let args = arg_list
        .as_ref()
        .map(|list| comparable_args(list, &name_lower, rules))
        .unwrap_or_default();

    // Block bookkeeping mirrors the formatter's: an opener's arguments are what
    // a forced closer is built from. Comments are left out of them because the
    // formatter's own `collect_logical_args` drops comment tokens, so a closer
    // rebuilt from `if(A # why)` carries only `A`.
    let opener_args = if is_block_opener(&name_lower) {
        openers.push(
            arg_list
                .as_ref()
                .map(|list| logical_values(list, rules.comment_style))
                .unwrap_or_default(),
        );
        None
    } else {
        match closer_kind(&name_lower) {
            // An unmatched closer has no opener to be rebuilt from, and the
            // formatter leaves those alone too
            Some(Governed::Closer) => openers.pop(),
            Some(Governed::MidBlock) => openers.last().cloned(),
            None => None,
        }
    };

    // Every reading the output may have, given what any part of the file may
    // have turned on. The input's own arguments are always accepted separately,
    // which is what makes the comparison reflexive.
    let mut closer_alternatives = Vec::new();
    if rules.side == Side::Input
        && let Some(opener) = opener_args
    {
        if rules.force_possible {
            closer_alternatives.push(opener);
        }
        if rules.remove_possible {
            closer_alternatives.push(Vec::new());
        }
    }

    Some(Entry::Command {
        name: if rules.fold_case { name_lower } else { name },
        shape,
        args,
        closer_alternatives,
        offset: usize::from(node.text_range().start()),
    })
}

/// The argument list's values with comments left out, the way the formatter
/// collects an opener's arguments to rebuild a closer from.
///
/// At every depth: the formatter renders a group normalized for this purpose, so
/// a comment written *inside* the opener's group is dropped from the closer too.
/// Removing only the top-level ones left the group's atom carrying its comment,
/// and the guard then refused a correct forced closer.
fn logical_values(arg_list: &ArgumentList, comment_style: CommentStyle) -> Vec<String> {
    let mut values = Vec::new();
    collect_values(arg_list.syntax(), comment_style, &mut values);
    values
}

fn collect_values(node: &crate::SyntaxNode, comment_style: CommentStyle, out: &mut Vec<String>) {
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(token) => {
                if matches!(
                    token.kind(),
                    SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT
                ) {
                    continue;
                }
                if let Some(text) = significant(&token, comment_style) {
                    out.push(text);
                }
            }
            NodeOrToken::Node(nested) => {
                let mut inner = Vec::new();
                collect_values(&nested, comment_style, &mut inner);
                out.push(inner.join(" "));
            }
        }
    }
}

/// The arguments of one command, with the runs a grammar marks as unordered
/// canonicalised so a permutation of them is not a difference.
fn comparable_args(arg_list: &ArgumentList, name_lower: &str, rules: &Rules) -> Vec<String> {
    if rules.reorders
        && let Some(canonical) = reorderable_args(arg_list, name_lower, rules)
    {
        return canonical;
    }

    // Argument order is part of the content here. A comment's *position* within
    // the command is not: the formatter re-places one legitimately — a trailing
    // comment on a property key moves past the value it precedes, and a comment
    // before a keyword is re-attached inside its section — so comparing where it
    // sits refused files the formatter had written correctly. Its text and its
    // count are still compared, and a comment moving between commands still
    // shows up as one command losing it and another gaining it.
    let mut args = Vec::new();
    collect_args_without_comments(arg_list.syntax(), rules.comment_style, &mut args);
    let mut comments = Vec::new();
    collect_comments(arg_list.syntax(), rules.comment_style, &mut comments);
    comments.sort();
    args.extend(comments);
    args
}

/// The same walk as [`collect_args`], with comment tokens left out.
fn collect_args_without_comments(
    node: &crate::SyntaxNode,
    comment_style: CommentStyle,
    out: &mut Vec<String>,
) {
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(token) => {
                if matches!(
                    token.kind(),
                    SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT
                ) {
                    continue;
                }
                if let Some(text) = significant(&token, comment_style) {
                    out.push(text);
                }
            }
            NodeOrToken::Node(nested) => {
                let mut inner = Vec::new();
                collect_args_without_comments(&nested, comment_style, &mut inner);
                out.push(inner.join(" "));
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
    collect_comments(arg_list.syntax(), rules.comment_style, &mut comments);
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
///
/// The same list the formatter uses, and it has to stay the same list: `block`
/// and `endblock` were here and not there, so on an unbalanced `endblock()` this
/// stack popped a frame the formatter kept and the two disagreed about which
/// opener a later closer belonged to.
fn is_block_opener(name_lower: &str) -> bool {
    matches!(
        name_lower,
        "if" | "foreach" | "while" | "function" | "macro"
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
        "endif" | "endforeach" | "endwhile" | "endfunction" | "endmacro" => Some(Governed::Closer),
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
    // Strip every `\r`, exactly as the formatter does before parsing. Handling
    // only `\r\n` here meant a lone `\r` reached the lexer for the first time
    // through this function — and the lexer loops on one, so `set(A b)\rset(C
    // d)` allocated until the process was killed.
    //
    // It also accepts the CRLF pass, which runs after formatting as a blanket
    // replacement and so rewrites newlines inside a bracket argument or bracket
    // comment as well as between lines.
    let input = strip_carriage_returns(input);
    let output = strip_carriage_returns(output);

    // Nothing to compare when nothing changed, and `--check` over an
    // already-formatted tree is the common case
    if input == output {
        return None;
    }

    compare(&input, &output, config, grammars)
}

/// The same, for a caller that has already parsed what the formatter parsed.
///
/// `input` must be `\r`-stripped and `input_cst` must be its parse — the
/// formatter's own `parse_input` and `cst`. Get that wrong and the two sides are
/// read from different texts. It saves the third parse of a file that changed,
/// which is where this check costs anything at all.
pub(crate) fn check_parsed(
    input_cst: &CSTRoot,
    input: &str,
    output: &str,
    config: &FormatConfig,
    grammars: &UserGrammars,
) -> Option<Difference> {
    debug_assert!(
        !input.contains('\r'),
        "check_parsed takes the text the caller parsed, which is \\r-stripped"
    );
    let output = strip_carriage_returns(output);
    if input == output {
        return None;
    }
    compare_parsed(input_cst, input, &output, config, grammars)
}

/// `\r`-stripped, and borrowed when there was nothing to strip — this runs over
/// every formatted file, and two whole-file copies to find no `\r` was the
/// measured cost of the guard on an already-formatted tree.
fn strip_carriage_returns(text: &str) -> std::borrow::Cow<'_, str> {
    if text.contains('\r') {
        std::borrow::Cow::Owned(text.replace('\r', ""))
    } else {
        std::borrow::Cow::Borrowed(text)
    }
}

/// The comparison itself, on text the caller has already canonicalized.
///
/// Split out from `check` so a test can reach it with `input == output`: the
/// short-circuit above is an optimization, and with the comparison behind it the
/// reflexivity property — comparing a text against itself never fires — was
/// asserted of a `return None` rather than of the code it is meant to hold for.
fn compare(
    input: &str,
    output: &str,
    config: &FormatConfig,
    grammars: &UserGrammars,
) -> Option<Difference> {
    compare_parsed(&parse_text(input), input, output, config, grammars)
}

fn compare_parsed(
    input_cst: &CSTRoot,
    input: &str,
    output: &str,
    config: &FormatConfig,
    grammars: &UserGrammars,
) -> Option<Difference> {
    // The input is what carries the directives; the output should carry the same
    // ones, but reading them from the input is what makes a dropped directive a
    // difference rather than a silent widening.
    let overrides = StyleOverrides::read(input_cst, config);

    Content::from_cst(
        input_cst,
        &Rules::new(config, &overrides, grammars, Side::Input),
    )
    .diff(
        &Content::from_cst(
            &parse_text(output),
            &Rules::new(config, &overrides, grammars, Side::Output),
        ),
        input,
    )
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
    fn test_the_reordering_exemption_needs_a_reordering_setting() {
        // The exemption was confined by command but nothing asserted it is off
        // entirely when neither pass is enabled — so a permutation with both
        // settings at their defaults would have been accepted.
        let config = FormatConfig::default();
        assert!(!accepts(
            "set(SRCS b.cpp a.cpp)\n",
            "set(SRCS a.cpp b.cpp)\n",
            &config
        ));
        assert!(!accepts(
            "target_sources(t PRIVATE b.cpp a.cpp)\n",
            "target_sources(t PRIVATE a.cpp b.cpp)\n",
            &config
        ));
    }

    #[test]
    fn test_the_block_lists_match_the_formatters() {
        // `block`/`endblock` were here and not in the formatter's lists, so on an
        // unbalanced `endblock()` this stack popped a frame the formatter kept
        // and then disagreed about which opener a later closer belonged to.
        let config = FormatConfig {
            closing_style: ClosingStyle::Force,
            ..Default::default()
        };
        // The formatter leaves `endblock` alone, so its arguments are its own
        assert!(accepts(
            "if(A)\nendblock()\nendif()\n",
            "if(A)\nendblock()\nendif(A)\n",
            &config
        ));
        // And an `endblock` must not be offered the `if`'s arguments
        assert!(!accepts(
            "if(A)\nendblock()\nendif()\n",
            "if(A)\nendblock(A)\nendif(A)\n",
            &config
        ));
        // The opener half of the same desync: `block` is not a block opener for
        // the formatter, so it pushes no frame — and a closer after it must be
        // offered the enclosing opener's arguments, not the `block`'s. Adding
        // `block` to this side's list is exactly the drift the test is for, and
        // only an opener can show it.
        assert!(accepts(
            "block(PROPAGATE x)\nendif()\n",
            "block(PROPAGATE x)\nendif()\n",
            &config
        ));
        assert!(!accepts(
            "block(PROPAGATE x)\nendif()\n",
            "block(PROPAGATE x)\nendif(PROPAGATE x)\n",
            &config
        ));
        assert!(accepts(
            "if(A)\nblock(PROPAGATE x)\nendif()\n",
            "if(A)\nblock(PROPAGATE x)\nendif(A)\n",
            &config
        ));
    }

    #[test]
    fn test_reordering_never_crosses_a_barrier() {
        // Sorting is allowed inside a run of ordinary arguments and nowhere
        // else: an argument whose value is unknown at format time holds its
        // index, and the arguments on either side of it are separate runs.
        // Comparing the whole section as one sorted run accepts a file whose
        // arguments crossed one, which changes what it says.
        let config = FormatConfig {
            sort_sources: SortSources::Alphabetical,
            ..Default::default()
        };
        // Sorting within a run is fine
        assert!(accepts(
            "set(SRCS b.cpp a.cpp ${V} d.cpp c.cpp)\n",
            "set(SRCS a.cpp b.cpp ${V} c.cpp d.cpp)\n",
            &config
        ));
        // Crossing the barrier is not
        assert!(!accepts(
            "set(SRCS a.cpp ${V} b.cpp)\n",
            "set(SRCS b.cpp ${V} a.cpp)\n",
            &config
        ));
        // Nor is moving the barrier itself
        assert!(!accepts(
            "set(SRCS a.cpp ${V} b.cpp)\n",
            "set(SRCS ${V} a.cpp b.cpp)\n",
            &config
        ));
        // A generator expression and an environment reference bound runs too
        assert!(!accepts(
            "set(SRCS a.cpp $<TARGET_OBJECTS:o> b.cpp)\n",
            "set(SRCS b.cpp $<TARGET_OBJECTS:o> a.cpp)\n",
            &config
        ));
        assert!(!accepts(
            "set(SRCS a.cpp $ENV{E} b.cpp)\n",
            "set(SRCS b.cpp $ENV{E} a.cpp)\n",
            &config
        ));
    }

    #[test]
    fn test_a_closer_reading_is_offered_only_when_its_setting_is_reachable() {
        // The two alternatives are gated on the setting being possible somewhere
        // in the file. Offering them unconditionally accepts a closer rewritten
        // under a setting nobody asked for.
        let preserve = FormatConfig {
            closing_style: ClosingStyle::Preserve,
            ..Default::default()
        };
        assert!(!accepts("if(A)\nendif()\n", "if(A)\nendif(A)\n", &preserve));
        assert!(!accepts("if(A)\nendif(A)\n", "if(A)\nendif()\n", &preserve));
    }

    #[test]
    fn test_a_gained_or_lost_command_is_a_difference() {
        // The length comparison is what catches a whole command appearing or
        // disappearing; nothing else looks at the entry count.
        let config = FormatConfig::default();
        assert!(!accepts("set(A b)\n", "set(A b)\nset(C d)\n", &config));
        assert!(!accepts("set(A b)\nset(C d)\n", "set(A b)\n", &config));
    }

    #[test]
    fn test_an_in_file_directive_widens_the_exemptions() {
        // A file can turn a setting on for itself, and the formatter applies it.
        // Keying the exemptions off the file-level config alone refused every
        // such file — at default settings, for a documented feature.
        let config = FormatConfig::default();
        assert!(accepts(
            "# cmake-fmt: sort_sources=alphabetical\nset(SRCS b.cpp a.cpp)\n",
            "# cmake-fmt: sort_sources=alphabetical\nset(SRCS a.cpp b.cpp)\n",
            &config
        ));
        assert!(accepts(
            "# cmake-fmt: closing_style=force\nif(A)\nendif()\n",
            "# cmake-fmt: closing_style=force\nif(A)\nendif(A)\n",
            &config
        ));
        assert!(accepts(
            "# cmake-fmt: closing_style=remove\nif(A)\nendif(A)\n",
            "# cmake-fmt: closing_style=remove\nif(A)\nendif()\n",
            &config
        ));
        // ...and again against a config that does *not* already remove closers.
        // `FormatConfig::default()` has `closing_style = Remove`, so the
        // assertion above holds whether the directive is read or not — the same
        // trap the casing case below sidesteps with `preserving`.
        let keeping = FormatConfig {
            closing_style: ClosingStyle::Preserve,
            ..Default::default()
        };
        assert!(accepts(
            "# cmake-fmt: closing_style=remove\nif(A)\nendif(A)\n",
            "# cmake-fmt: closing_style=remove\nif(A)\nendif()\n",
            &keeping
        ));
        assert!(!accepts("if(A)\nendif(A)\n", "if(A)\nendif()\n", &keeping));
        // With both casing settings preserving, only the directive can license
        // a re-casing — the default config already folds case, so asserting it
        // there would prove nothing
        let preserving = FormatConfig {
            command_case: CommandCase::Preserve,
            user_command_case: UserCommandCase::Preserve,
            ..Default::default()
        };
        assert!(accepts(
            "# cmake-fmt: command_case=uppercase\nset(C d)\n",
            "# cmake-fmt: command_case=uppercase\nSET(C d)\n",
            &preserving
        ));
        assert!(!accepts("set(C d)\n", "SET(C d)\n", &preserving));
        // Widening is not licence: a file that says nothing still holds
        assert!(!accepts(
            "set(SRCS b.cpp a.cpp)\n",
            "set(SRCS a.cpp b.cpp)\n",
            &config
        ));
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
        // so a comparison that flattened the group would let the multiset
        // reading move an argument across parentheses — the reordering a group
        // exists to prevent.
        //
        // What holds it together here is that the section parser renders a group
        // as one argument (`render_nested_group`), not the barrier split: this
        // case passes with either of those reverted individually. The barrier
        // split earns its place by keeping the runs *around* a group sorting
        // independently, which is what the formatter does.
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
    fn test_a_comment_may_move_within_its_command_but_not_out_of_it() {
        // The formatter re-places a comment inside a command — a property key's
        // trailing comment moves past the value it precedes — so comparing where
        // it sits refused files it had written correctly. Its text and count are
        // still compared, and a comment leaving its command still shows up.
        let config = FormatConfig::default();
        assert!(accepts(
            "set_target_properties(t PROPERTIES K # note\nv)\n",
            "set_target_properties(t PROPERTIES K v # note\n)\n",
            &config
        ));
        // Leaving the command is a difference
        assert!(!accepts(
            "set(A b # note\n)\nset(C d)\n",
            "set(A b)\nset(C d # note\n)\n",
            &config
        ));
        // And losing it outright
        assert!(!accepts("set(A b # note\n)\n", "set(A b)\n", &config));
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

    /// The property that would have caught four separate false positives at
    /// once: comparing a text against itself can never report a difference.
    ///
    /// Every one of them came from normalising the input to what a setting
    /// *would* emit, in a place where the formatter does not apply it — inside a
    /// `# cmake-fmt: off` region, on a line `--line-ranges` did not select, on a
    /// command the formatter's block lists do not carry, and where the opener
    /// held a comment the formatter drops. Each refused a file the formatter had
    /// written correctly.
    #[test]
    fn test_comparing_a_text_against_itself_never_fires() {
        let sources = [
            // a closer with arguments, which closing_style has opinions about
            "if(A)\n\tmessage(hi)\nendif(A)\n",
            "foreach(f a b)\nendforeach(f)\n",
            "while(A)\nendwhile(A)\n",
            "function(f a)\nendfunction(f)\n",
            "macro(m)\nendmacro(m)\n",
            "block(PROPAGATE x)\nset(x 1)\nendblock()\n",
            "block()\nendblock(foo)\n",
            "if(A)\nelse(A)\nendif(A)\n",
            "if(A)\nelseif(B)\nendif(A)\n",
            // an opener holding a comment, which the formatter drops from a
            // rebuilt closer
            "if(A # why\n)\nmessage(hi)\nendif()\n",
            "foreach(f a b # trailing\n)\nendforeach()\n",
            // an unmatched closer
            "endif(A)\n",
            "else(A)\n",
            // suppressed regions, which are emitted verbatim
            "# cmake-fmt: off\nif(A)\nendif(A)\n# cmake-fmt: on\n",
            "set(A b) # cmake-fmt: no-sort\n",
            // ordinary content
            "set(SOURCES z.cpp a.cpp)\n",
            "target_sources(t PRIVATE a.cpp a.h)\n",
            "install(FILES a.h DESTINATION inc)\n",
            "# a comment\nset(A b) # trailing\n",
            "set(V [[bracket\nargument]])\n",
            "f((a b) c)\n",
            "",
            "\n\n",
        ];

        for source in sources {
            for closing_style in [
                ClosingStyle::Preserve,
                ClosingStyle::Remove,
                ClosingStyle::Force,
            ] {
                for (sort_sources, source_grouping) in [
                    (SortSources::None, SourceGrouping::None),
                    (SortSources::Alphabetical, SourceGrouping::HeadersFirst),
                ] {
                    let config = FormatConfig {
                        closing_style,
                        sort_sources,
                        source_grouping,
                        ..Default::default()
                    };
                    // `compare`, not `check`: `check` returns early when the
                    // two texts are equal, so asserting this through it
                    // asserted a `return None` — the property passed for any
                    // implementation of the comparison it is about.
                    assert!(
                        compare(source, source, &config, &UserGrammars::new()).is_none(),
                        "comparing {:?} against itself fired under {:?}/{:?}/{:?}",
                        source,
                        closing_style,
                        sort_sources,
                        source_grouping
                    );
                    // And through `check` as well, so the short-circuit itself
                    // stays covered.
                    assert!(accepts(source, source, &config));
                }
            }
        }
    }

    #[test]
    fn test_handing_over_a_parse_gives_the_same_answer_as_parsing_again() {
        // The formatter passes the CST it already built, which saves the third
        // parse of a changed file. The two entry points have to agree, and the
        // handover carries an invariant the compiler cannot check: the text and
        // the tree must be the same `\r`-stripped text.
        let cases = [
            ("set(A b)\n", "set(A b)\n"),
            ("set(  A   b)\n", "set(A b)\n"),
            ("set(A b)\n", "set(A c)\n"),
            ("set(A b)\n", "unset(A b)\n"),
            ("if(A)\nendif(A)\n", "if(A)\nendif()\n"),
            ("set(SRCS b.cpp a.cpp)\n", "set(SRCS a.cpp b.cpp)\n"),
            (
                "# cmake-fmt: off\nset(  A b)\n",
                "# cmake-fmt: off\nset(A b)\n",
            ),
            ("", "set(A b)\n"),
            ("set(A b)\n", ""),
        ];
        for closing_style in [ClosingStyle::Preserve, ClosingStyle::Remove] {
            for sort_sources in [SortSources::None, SortSources::Alphabetical] {
                let config = FormatConfig {
                    closing_style,
                    sort_sources,
                    ..Default::default()
                };
                for (input, output) in cases {
                    let grammars = UserGrammars::new();
                    let parsed =
                        check_parsed(&parse_text(input), input, output, &config, &grammars)
                            .map(|d| d.summary);
                    let fresh = check(input, output, &config, &grammars).map(|d| d.summary);
                    assert_eq!(
                        parsed, fresh,
                        "the two entry points disagree about {:?} -> {:?}",
                        input, output
                    );
                }
            }
        }
        // A CRLF output against the stripped input the formatter parsed: the
        // caller's side of the invariant, which is what the main path does.
        let config = FormatConfig::default();
        assert!(
            check_parsed(
                &parse_text("set(A b)\n"),
                "set(A b)\n",
                "set(A b)\r\n",
                &config,
                &UserGrammars::new()
            )
            .is_none(),
            "applying CRLF is not a difference"
        );
    }

    #[test]
    fn test_a_lone_carriage_return_is_not_a_difference() {
        // The formatter strips every `\r` before parsing; handling only `\r\n`
        // here meant a lone `\r` reached the lexer for the first time through
        // this function, and the lexer loops on one — `set(A b)\rset(C d)`
        // allocated until the process was killed.
        let config = FormatConfig::default();
        assert!(accepts(
            "set(A b)\rset(C d)\r",
            "set(A b)\nset(C d)\n",
            &config
        ));
        assert!(accepts("set(A b)\r\n", "set(A b)\n", &config));
    }

    #[test]
    fn test_crlf_is_not_a_difference() {
        // Between lines this is not what the replacement is for — newline tokens
        // carry no content and are dropped anyway, so deleting the replacement
        // left this half green. What it is for is a newline *inside* a bracket
        // argument, where the CRLF pass rewrites part of an argument's value.
        let config = FormatConfig {
            line_ending: LineEnding::CrLf,
            ..Default::default()
        };
        assert!(accepts("set(A b)\n", "set(A b)\r\n", &config));
        assert!(accepts(
            "set(V [[a\nb]])\n",
            "set(V [[a\r\nb]])\r\n",
            &config
        ));
        assert!(accepts("#[[a\nb]]\n", "#[[a\r\nb]]\r\n", &config));
    }
}
