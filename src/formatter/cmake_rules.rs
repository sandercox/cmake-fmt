use crate::cst::ArgumentList;
use crate::syntax_kind::SyntaxKind;
use pretty::RcDoc;
use rowan::NodeOrToken;
use std::collections::{HashMap, HashSet};

use super::config::FormatConfig;
use super::cst_to_doc::detect_argument_formatting_signals;
use super::grammar::{CommandGrammar, KeywordType};

/// Recognized header extensions
const HEADER_EXTS: &[&str] = &["h", "hh", "hpp", "hxx", "ipp", "h++", "H", "vert"];
/// Recognized source extensions
const SOURCE_EXTS: &[&str] = &["c", "cc", "cpp", "cxx", "c++", "C", "m", "mm", "frag"];

/// Check if a filename has a header extension
fn is_header_file(name: &str) -> bool {
    name.rsplit('.')
        .next()
        .is_some_and(|ext| HEADER_EXTS.contains(&ext))
}

/// Check if a filename has a source extension
fn is_source_file(name: &str) -> bool {
    name.rsplit('.')
        .next()
        .is_some_and(|ext| SOURCE_EXTS.contains(&ext))
}

/// Normalize whitespace in line comments according to the specified style.
/// Only affects line comments (not bracket comments).
/// Multi-hash comments (##, ###, etc.) are always preserved as-is.
/// Examples:
///   - Preserve: "#\t\tfoo" -> "#\t\tfoo" (unchanged)
///   - HashSpace: "#\t\tfoo" -> "# foo", "#no-space" -> "# no-space", "#" -> "#"
///   - HashNoSpace: "#  foo" -> "#foo", "#" -> "#"
///   - Any style: "## heading" -> "## heading" (multi-hash preserved)
pub fn normalize_comment_whitespace(comment: &str, style: super::config::CommentStyle) -> String {
    use super::config::CommentStyle;

    // Multi-hash comments (##, ###, etc.) are preserved as-is
    if comment.starts_with("##") {
        return comment.to_string();
    }

    match style {
        CommentStyle::Preserve => comment.to_string(),
        CommentStyle::HashSpace => {
            if let Some(content) = comment.strip_prefix('#') {
                let trimmed = content.trim_start();
                if trimmed.is_empty() {
                    "#".to_string()
                } else {
                    format!("# {}", trimmed)
                }
            } else {
                comment.to_string()
            }
        }
        CommentStyle::HashNoSpace => {
            if let Some(content) = comment.strip_prefix('#') {
                let trimmed = content.trim_start();
                if trimmed.is_empty() {
                    "#".to_string()
                } else {
                    format!("#{}", trimmed)
                }
            } else {
                comment.to_string()
            }
        }
    }
}

/// Extract the base name (without extension) from a file path
fn base_name(name: &str) -> Option<&str> {
    // Handle paths: take the last component, then strip extension
    let filename = name.rsplit('/').next().unwrap_or(name);
    let filename = filename.rsplit('\\').next().unwrap_or(filename);
    filename.rfind('.').map(|pos| &filename[..pos])
}

/// Get extension priority for HeadersFirst ordering.
/// Lower numbers come first. Header extensions get lower priorities than source extensions.
fn ext_priority(name: &str) -> usize {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        // Header extensions (lower priorities = earlier in order)
        "h" => 0,
        "hh" => 1,
        "hpp" => 2,
        "hxx" => 3,
        "ipp" => 4,
        "h++" => 5,
        "H" => 6,
        "vert" => 7,
        // Source extensions (higher priorities = later in order)
        "c" => 8,
        "cc" => 9,
        "cpp" => 10,
        "cxx" => 11,
        "c++" => 12,
        "C" => 13,
        "m" => 14,
        "mm" => 15,
        "frag" => 16,
        // Unknown extensions go at the end
        _ => 99,
    }
}

/// Group source files into N-way groups based on matching base names
///
/// Takes a list of file arguments and returns a new list where files with
/// matching base names are placed adjacent to each other and joined as a
/// single string (space-separated) so they render on the same line.
///
/// Files are ordered within each group by extension priority (see ext_priority).
/// Arguments that are not source/header files pass through unchanged.
/// Files without a matching pair pass through unchanged.
///
/// Returns: (grouped_args, old_to_new_index_mapping)
/// The index mapping maps each original index to its new position in the result.
/// When multiple files are grouped, they all map to the same result index.
pub fn group_source_pairs(
    args: &[String],
    grouping: super::config::SourceGrouping,
) -> (Vec<String>, Vec<usize>) {
    use super::config::SourceGrouping;

    if grouping == SourceGrouping::None {
        let identity_map: Vec<usize> = (0..args.len()).collect();
        return (args.to_vec(), identity_map);
    }

    // Build index: base_name -> all indices with that base name
    let mut base_map: HashMap<String, Vec<usize>> = HashMap::new();

    for (i, arg) in args.iter().enumerate() {
        if let Some(base) = base_name(arg) {
            let base_lower = base.to_lowercase();
            // Only include files that are recognized headers or sources
            if is_header_file(arg) || is_source_file(arg) {
                base_map.entry(base_lower).or_default().push(i);
            }
        }
    }

    // Identify groups: any base name with 2+ files forms a group
    let mut is_grouped = vec![false; args.len()];
    let mut groups: Vec<Vec<usize>> = Vec::new();

    for indices in base_map.values() {
        if indices.len() >= 2 {
            // Mark all indices as grouped
            for &idx in indices {
                is_grouped[idx] = true;
            }
            groups.push(indices.clone());
        }
    }

    // Build result: emit groups at the position of their earliest index
    let mut emitted = vec![false; args.len()];
    let mut result = Vec::new();
    let mut old_to_new = vec![0; args.len()];

    // Create a lookup: index -> group containing that index
    let mut index_to_group: HashMap<usize, usize> = HashMap::new();
    for (group_id, group) in groups.iter().enumerate() {
        for &idx in group {
            index_to_group.insert(idx, group_id);
        }
    }

    for (i, arg) in args.iter().enumerate() {
        if emitted[i] {
            continue;
        }

        if is_grouped[i] {
            // This file is part of a group
            let group_id = index_to_group[&i];
            let group = &groups[group_id];

            // Emit the entire group (only once, at the earliest index position)
            let earliest = *group.iter().min().unwrap();
            if i == earliest {
                // Collect all files in the group
                let mut group_files: Vec<(usize, &String)> =
                    group.iter().map(|&idx| (idx, &args[idx])).collect();

                // Sort by extension priority
                match grouping {
                    SourceGrouping::HeadersFirst => {
                        // Lower priority numbers first (headers before sources)
                        group_files.sort_by_key(|(_, file)| ext_priority(file));
                    }
                    SourceGrouping::SourcesFirst => {
                        // Higher priority numbers first (sources before headers)
                        group_files.sort_by_key(|(_, file)| std::cmp::Reverse(ext_priority(file)));
                    }
                    SourceGrouping::None => unreachable!(),
                }

                // Join the group as a space-separated string
                let group_str = group_files
                    .iter()
                    .map(|(_, file)| file.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");

                let result_idx = result.len();
                result.push(group_str);

                // Map all original indices in this group to the same result index
                for &idx in group {
                    old_to_new[idx] = result_idx;
                    emitted[idx] = true;
                }
            }
        } else {
            // Not grouped, pass through unchanged
            let result_idx = result.len();
            result.push(arg.clone());
            old_to_new[i] = result_idx;
            emitted[i] = true;
        }
    }

    (result, old_to_new)
}

/// A section's arguments as they should be emitted, grouped when
/// `source_grouping` is on and the section's own allowlist entry permits it.
///
/// Every place that renders a section's arguments goes through this. Five copies
/// of the decision meant the `Flag` arm — which carries the positional run after
/// a flag, as in `add_library(l STATIC a.cpp a.h)` — never grouped at all, so
/// the two reordering passes disagreed about a list the allowlist owns; and
/// three of the five copies had no test.
fn grouped_section(
    section: &KeywordSection,
    grouping: super::config::SourceGrouping,
) -> GroupedSection {
    if grouping != super::config::SourceGrouping::None && section.trailing_comments.is_empty() {
        group_source_pairs_preserving_blanks(section, grouping)
    } else {
        GroupedSection {
            args: section.args.clone(),
            annotations: section.annotations.clone(),
        }
    }
}

/// One thing the author wrote between a section's arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Annotation {
    /// A comment on a line of its own.
    Comment(String),
    /// A blank line.
    Blank,
}

/// Everything written between a section's arguments, in source order.
///
/// One ordered list, each item keyed by the argument position it precedes. This
/// replaces a position-keyed *set* of blank lines plus two further arrays whose
/// only job was to re-encode the ordering that set had thrown away:
/// `post_comment_blanks` ("the blank at this position comes after the comments,
/// not before") and `comment_blank_indices` ("a blank precedes comment number
/// k"). Both were unrepresentable states waiting to be reached, and both were
/// reached — a blank written twice because two rules matched one entry, and an
/// index naming a comment that sorting had since moved, or that never arrived.
///
/// Here the order *is* the order, so there is nothing to keep in step.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Annotations {
    items: Vec<(usize, Annotation)>,
}

impl Annotations {
    // The two questions the arms ask before they open a comment block at all —
    // `use_per_line`, `has_annotations`. They still ask them of `comments` and
    // `blank_lines`, which is the last thing left to move before those four
    // fields can go.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn push_comment(&mut self, position: usize, text: String) {
        self.items.push((position, Annotation::Comment(text)));
    }

    pub fn push_blank(&mut self, position: usize) {
        self.items.push((position, Annotation::Blank));
    }

    /// The own-line comments, as `(position, text)` in source order.
    pub fn comments(&self) -> impl Iterator<Item = (usize, &str)> + '_ {
        self.items.iter().filter_map(|(position, item)| match item {
            Annotation::Comment(text) => Some((*position, text.as_str())),
            Annotation::Blank => None,
        })
    }

    #[allow(dead_code)]
    pub fn has_comments(&self) -> bool {
        self.comments().next().is_some()
    }

    /// Everything the author wrote before argument `index`, in the order they
    /// wrote it.
    ///
    /// This is the whole of what a render arm needs. The five rules it used to
    /// apply — a blank before the comments here, unless this position is in
    /// `post_comment_blanks`, in which case after them; a blank before comment
    /// number `k`; the comment itself; the argument — were five ways of asking
    /// what order the author wrote things in, from three arrays that had thrown
    /// that order away.
    pub fn at(&self, index: usize) -> impl Iterator<Item = &Annotation> + '_ {
        self.items
            .iter()
            .filter(move |(position, _)| *position == index)
            .map(|(_, item)| item)
    }

    /// The positions that carry at least one blank line, deduplicated and in
    /// order — the old `blank_lines`.
    ///
    /// This and the two views below say what the four arrays say. They began as
    /// the faithfulness assertion's instruments; `sort_source_args` now derives
    /// the arrays through them rather than maintaining a second copy by hand,
    /// which is what stopped the two encodings drifting apart under a
    /// permutation. They go when the arrays do.
    pub fn blank_positions(&self) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        for (position, item) in &self.items {
            if matches!(item, Annotation::Blank) && !out.contains(position) {
                out.push(*position);
            }
        }
        out
    }

    /// Whether a blank line was written before argument `position`.
    pub fn has_blank_at(&self, position: usize) -> bool {
        self.items
            .iter()
            .any(|(at, item)| *at == position && matches!(item, Annotation::Blank))
    }

    /// Whether the blank at `position` was written *after* the comments there
    /// rather than before them — the old `post_comment_blanks`.
    pub fn blank_follows_comments_at(&self, position: usize) -> bool {
        let mut seen_comment = false;
        for (item_position, item) in &self.items {
            if *item_position != position {
                continue;
            }
            match item {
                Annotation::Comment(_) => seen_comment = true,
                Annotation::Blank => return seen_comment,
            }
        }
        false
    }

    /// Move every item to a new position, keeping source order.
    ///
    /// Source order, not position order: grouping's mapping is not monotonic —
    /// `[a.h, b.cpp, a.cpp]` pairs indices 0 and 2 into one line, so index 2
    /// maps *behind* index 1. [`Annotations::settle_crossed`] puts them back
    /// afterwards, and rebuilds rather than sorts, because a comment that
    /// crossed another has to lose the blank line that used to sit between
    /// them.
    pub fn remap(&mut self, mut position_of: impl FnMut(usize) -> usize) {
        for (position, _) in &mut self.items {
            *position = position_of(*position);
        }
    }

    /// The items written before argument `index` up to and including the last
    /// blank line there — what stays put when sorting moves the argument.
    ///
    /// A blank line is a segment boundary: sorting permutes arguments *within*
    /// the runs it delimits, so it has to stay at the index it is at or the next
    /// pass segments the list differently. Anything the author wrote above that
    /// blank is held there by it, so it stays too.
    pub fn pinned_prefix(&self, index: usize) -> impl Iterator<Item = &Annotation> + '_ {
        let items: Vec<&Annotation> = self.at(index).collect();
        let last_blank = items
            .iter()
            .rposition(|item| matches!(item, Annotation::Blank));
        let keep = last_blank.map_or(0, |at| at + 1);
        items.into_iter().take(keep)
    }

    /// The comments written after the last blank line before argument `index` —
    /// what travels with the argument when sorting moves it.
    ///
    /// These are the ones written directly above the argument, with nothing
    /// between, so they are about it and follow it.
    pub fn comments_after_last_blank(&self, index: usize) -> impl Iterator<Item = &str> + '_ {
        let items: Vec<&Annotation> = self.at(index).collect();
        let last_blank = items
            .iter()
            .rposition(|item| matches!(item, Annotation::Blank));
        let from = last_blank.map_or(0, |at| at + 1);
        items.into_iter().skip(from).filter_map(|item| match item {
            Annotation::Comment(text) => Some(text.as_str()),
            Annotation::Blank => None,
        })
    }

    /// Put a comment back in front of the argument it was written for.
    ///
    /// Grouping moves an argument backwards when it pairs with an earlier one, and
    /// the comment written in front of it moves with it — past any comment in
    /// between, so that comment's position now runs backwards. Every render arm
    /// walks the comment list forwards, matching each position as it reaches it, so
    /// a position that runs backwards is never matched again: the comment fell out
    /// at the end of the section, in front of the closing paren, describing nothing.
    ///
    /// ```text
    /// target_sources(t PRIVATE      # with source_grouping=headers_first
    ///   a.h                             a.h a.cpp
    ///   # about b               ->      # about b
    ///   b.cpp                           b.cpp
    ///   # about a.cpp                   # about a.cpp
    ///   a.cpp                       )
    /// )
    /// ```
    ///
    /// A stable fixed point, and no corpus file or generated shape reaches it, which
    /// is why twelve rounds of review did not either. Found by asking what the
    /// ordered list would render for a crossed position, and answering that the
    /// arrays render this.
    ///
    /// Sorting by position restores the pairing. It also renames every
    /// `comment_blank_indices` entry, so the rule `sort_source_args` applies when it
    /// permutes the same list applies here: an entry means "the author left a blank
    /// between these two comment groups", and reordering has dissolved the groups.
    pub fn settle_crossed(&mut self) {
        let mut previous = None;
        let mut crossed = false;
        for (position, _) in self.comments() {
            if previous.is_some_and(|before| before > position) {
                crossed = true;
                break;
            }
            previous = Some(position);
        }
        if !crossed {
            return;
        }

        let mut comments: Vec<(usize, String)> = self
            .comments()
            .map(|(position, text)| (position, text.to_string()))
            .collect();
        comments.sort_by_key(|(position, _)| *position);
        let blanks = self.blank_positions();
        let after_comments: Vec<usize> = blanks
            .iter()
            .copied()
            .filter(|position| self.blank_follows_comments_at(*position))
            .collect();

        *self = Self::rebuilt_for_permuted_comments(&comments, &blanks, |position| {
            after_comments.contains(&position)
        });
    }

    /// Rebuild from a permutation that has already moved the comments.
    ///
    /// Sorting reorders the arguments within each blank-line segment, so it
    /// cannot be expressed as a position rewrite: two comments can swap places
    /// in the list. The blanks are the segment boundaries and so are fixed
    /// points; `comments` arrives already remapped — each entry is the comment
    /// and the argument position it now sits in front of, in its new order.
    fn rebuilt_for_permuted_comments(
        comments: &[(usize, String)],
        blank_positions: &[usize],
        blank_follows_comments: impl Fn(usize) -> bool,
    ) -> Self {
        let mut positions: Vec<usize> = comments.iter().map(|(position, _)| *position).collect();
        positions.extend_from_slice(blank_positions);
        positions.sort_unstable();
        positions.dedup();

        let mut rebuilt = Self::default();
        for position in positions {
            let carries_blank = blank_positions.contains(&position);
            let blank_last = carries_blank && blank_follows_comments(position);
            if carries_blank && !blank_last {
                rebuilt.push_blank(position);
            }
            for (_, text) in comments.iter().filter(|(at, _)| *at == position) {
                rebuilt.push_comment(position, text.clone());
            }
            if blank_last {
                rebuilt.push_blank(position);
            }
        }
        rebuilt
    }
}

/// Whether the section before `index` has already put a comment on its line.
///
/// A line comment runs to the end of its line, so nothing may follow that
/// section on the same line. Every separator that collapses a keyword onto the
/// previous line has to ask this first: collapsing regardless put the keyword
/// *inside* the comment — `# note QUIET` — and the keyword was gone from the
/// file, permanently, at a stable fixed point and exit 0.
///
/// Three arms have made that same mistake in three consecutive rounds, each time
/// one arm over from the last fix, so the question lives in one place and every
/// separator consults it.
fn previous_section_ended_in_a_comment(sections: &[KeywordSection], index: usize) -> bool {
    index
        .checked_sub(1)
        .and_then(|previous| sections.get(previous))
        .is_some_and(|previous| {
            previous.annotations.has_comments() || !previous.trailing_comments.is_empty()
        })
}

/// Emit the comments of a keyword section that has no values.
///
/// Every arm that renders a section used to guard its comment machinery on the
/// section having arguments, so a comment attached to a keyword with no values
/// had nowhere to go and was deleted outright — `target_sources(t\n\tPRIVATE # note)`
/// lost `# note`, and so did the `PairValue`, `BinPack` and inline equivalents.
///
/// They are always own-line comments: the section parser only records a trailing
/// comment for a section that already holds an argument, so a comment written on
/// the same line as a valueless keyword arrives at position 0 of `comments`.
///
/// At `keyword_indent`, not `value_indent`: there are no values for the comment
/// to sit under, and the comment belongs to the keyword. Three of the five arms
/// used the deeper level, so `find_package(Foo REQUIRED # n)` put its comment one
/// tab in and `target_sources(t PRIVATE # n)` put an identical construct two.
/// Whether this section's keyword is emitted on the opening line, beside the
/// command name — `list(APPEND …)`, `define_property(TEST …)`.
///
/// Only a multi-mode command's first section qualifies, and not when a comment
/// section precedes it: a line comment runs to the end of its line, so inlining
/// put the mode keyword *inside* the comment.
///
/// Asked here rather than in each arm because "one arm over from the last fix"
/// has been the shape of four consecutive defects in this function, and the two
/// questions already hoisted — `previous_section_ended_in_a_comment` and the
/// comment emission — are the two that stopped recurring. Note four of the six
/// arms still never ask this one, which is why `file(CHMOD v)` breaks to its own
/// line; making them ask is a behaviour change rather than a fix, so it is not
/// done here.
fn keyword_stays_on_the_opening_line(
    sections: &[KeywordSection],
    index: usize,
    first_keyword_inline: bool,
) -> bool {
    first_keyword_inline && !previous_section_ended_in_a_comment(sections, index)
}

/// Whether this `SingleValue` section is grouped onto the line of the valueless
/// `Flag` that opened a multi-mode command — the `PROPERTY name` of
/// `define_property(TEST PROPERTY name)`.
///
/// Section index 1 only: anywhere later this is an ordinary keyword.
fn groups_with_the_leading_flag(
    sections: &[KeywordSection],
    index: usize,
    first_keyword_inline: bool,
) -> bool {
    first_keyword_inline
        && index == 1
        && matches!(
            sections.first(),
            Some(first) if first.keyword_type == Some(KeywordType::Flag) && first.args.is_empty()
        )
        && !previous_section_ended_in_a_comment(sections, index)
}

fn push_valueless_section_comments(
    docs: &mut Vec<RcDoc<'static, ()>>,
    annotations: &Annotations,
    indent: &str,
) {
    for (_, comment) in annotations.comments() {
        docs.push(RcDoc::hardline());
        docs.push(RcDoc::text(indent.to_string()));
        docs.push(RcDoc::text(comment.to_string()));
    }
}

/// Whether a blank line the author left at the end of the previous section has
/// still to be written here.
///
/// A blank recorded at a section's own `args.len()` sits between that section
/// and this one, and no arm emits it: the arms walk `annotations.at(arg_idx)`
/// for each argument they render, and there is no argument at that index. So
/// it is written here, before the next section opens.
///
/// The exception is a section whose own-line comments live at that same
/// position. Every arm emits those, and the blank goes with them, so writing it
/// again turned one blank line in the source into two.
///
/// Both render paths ask this. Only the general one used to, and the
/// `inline_single_keyword` twin paid for it with its first-pass fixed point on
/// `list(APPEND V)` with a blank before the elements: the blank demoted the
/// mode keyword off the opening line and was then dropped, so the next pass,
/// seeing no blank, put the keyword back.
fn blank_line_between_sections(sections: &[KeywordSection], index: usize) -> bool {
    if index == 0 {
        return false;
    }
    let previous = &sections[index - 1];
    let previous_already_wrote_it = previous
        .annotations
        .comments()
        .any(|(position, _)| position == previous.args.len());
    !previous_already_wrote_it && previous.annotations.has_blank_at(previous.args.len())
}

/// Emit what the author wrote before an argument: their own-line comments and
/// the blank lines around them, in the order they wrote them.
///
/// Seven arms had a copy of the five rules this replaces, and the rules could
/// not express every order the author can write. Enumerating the thirteen ways
/// to arrange up to two comments and two blank lines in one gap, four came back
/// changed:
///
/// ```text
///   as written    was written back as
///   ----------    -------------------
///   BCB           BC     the blank after the comment was dropped outright
///   CBC           CCB    the blank between the comments slid below both
///   BCCB          BCC    dropped
///   CBCB          CCB    both
/// ```
///
/// `post_comment_blanks` says "the blank at this position comes after the
/// comments" and `comment_blank_indices` says "a blank precedes comment number
/// k" — between them they can place a blank before the comments, after them, or
/// between two groups, but only ever one of the three, and only when a *second*
/// blank at that position is there to carry the third case. An ordered walk has
/// no such gap: it emits what is there.
///
/// One rule of its own lives here, now that there is one place to put it: a gap
/// never ends with a blank line when it already holds one — see below.
///
/// Returns whether anything was emitted, which is what tells a caller to stop
/// treating the next thing as the section's first argument.
fn push_annotations_before_argument(
    docs: &mut Vec<RcDoc<'static, ()>>,
    annotations: &Annotations,
    index: usize,
    indent: &str,
    force_multiline: bool,
) -> bool {
    let items: Vec<&Annotation> = annotations.at(index).collect();

    // Air on both sides of the same note is one blank line too many. A note
    // describes what comes after it, so the blank above is the one doing the
    // separating and the one below only pushes the note away from what it is
    // about:
    //
    //     compositor.cpp            compositor.cpp
    //                           ->
    //     # goes with the 3.0 ABI   # goes with the 3.0 ABI
    //                               legacy_shim.cpp
    //     legacy_shim.cpp
    //
    // Stated as "a gap never ends with a blank line when it already holds one",
    // which also covers the note-blank-note-blank case, and leaves a gap whose
    // only blank is the trailing one alone — there the author put the blank
    // between the note and the closing paren deliberately, and nothing above it
    // is separating anything.
    //
    // Two blanks in a row do reach here, so "already holds one" is meant
    // literally and not as "holds one with a note under it". A run of blank
    // lines in the source collapses to a single one whatever `max_blank_lines`
    // says, but an argument the parser drops between two of them — `()`, say —
    // leaves the pair adjacent. Dropping the second is right there too.
    let surplus_trailing_blank = matches!(items.last(), Some(Annotation::Blank))
        && items[..items.len() - 1]
            .iter()
            .any(|item| matches!(item, Annotation::Blank));
    let items = if surplus_trailing_blank {
        &items[..items.len() - 1]
    } else {
        &items[..]
    };

    let mut emitted = false;

    for item in items {
        match item {
            Annotation::Blank => {
                // Nothing to separate when the whole command is going on one
                // line, and a hardline there would force it apart.
                if force_multiline {
                    docs.push(RcDoc::hardline());
                    emitted = true;
                }
            }
            Annotation::Comment(text) => {
                if force_multiline {
                    docs.push(RcDoc::hardline());
                    docs.push(RcDoc::text(indent.to_string()));
                } else {
                    docs.push(RcDoc::flat_alt(
                        RcDoc::hardline().append(RcDoc::text(indent.to_string())),
                        RcDoc::space(),
                    ));
                }
                docs.push(RcDoc::text(text.clone()));
                emitted = true;
            }
        }
    }

    emitted
}

/// Emit the comments written after a section's last argument.
///
/// Four arms had a copy of this and none of them consulted
/// `post_comment_blanks`, so a blank line the author left *after* the last
/// comment came back out *before* it. The comment let go of the argument it
/// followed and attached itself to the closing paren instead:
///
/// ```text
/// target_sources(t              target_sources(t
///   PRIVATE                         PRIVATE
///     a.cpp              ->             a.cpp
///     # note                            <blank>
///     <blank>                           # note
/// )                             )
/// ```
///
/// One place now, for the reason the other two hoisted questions live in one
/// place: four consecutive defects in this function were each one arm over from
/// the last fix, and the questions that got hoisted are the ones that stopped
/// recurring.
///
/// A blank with no comment after it stays dropped. Nothing follows it in the
/// section, so emitting it only put a blank line in front of the closing paren —
/// which is the one thing this cannot delegate to
/// [`push_annotations_before_argument`], since there is no argument coming.
///
/// Returns whether anything was emitted, which is all one caller needs in order
/// to stop treating the next thing as the section's first argument.
fn push_end_of_section_comments(
    docs: &mut Vec<RcDoc<'static, ()>>,
    annotations: &Annotations,
    args_len: usize,
    indent: &str,
    force_multiline: bool,
) -> bool {
    if !annotations
        .at(args_len)
        .any(|item| matches!(item, Annotation::Comment(_)))
    {
        return false;
    }

    push_annotations_before_argument(docs, annotations, args_len, indent, force_multiline)
}

/// Group only the runs the sorting pass is allowed to permute: from `sort_from`
/// onward, and never across a barrier.
///
/// `source_grouping` reorders on its own — it hoists a later header to its
/// pair's index whether or not `sort_sources` is on — so without this it moved a
/// file across a `${...}` that `sort_source_args` refuses to cross, and past the
/// target name of an `add_library`. `set(SRCS b.cpp ${GENERATED} ${OTHER} b.h)`
/// became `set(SRCS b.h b.cpp ${GENERATED} ${OTHER})`, which is a reorder of a
/// list whose contents nobody can read.
fn group_sortable_runs(
    args: &[String],
    sort_from: usize,
    grouping: super::config::SourceGrouping,
) -> (Vec<String>, Vec<usize>) {
    // A section with no sortable range pins everything, so `min` here is what
    // makes `sort_from = usize::MAX` mean "group nothing"
    let pinned = sort_from.min(args.len());
    let mut out: Vec<String> = args[..pinned].to_vec();
    let mut old_to_new: Vec<usize> = (0..pinned).collect();

    for run in split_at_barriers(args, pinned..args.len()) {
        let base = out.len();
        let (grouped, local) = group_source_pairs(&args[run.clone()], grouping);
        out.extend(grouped);
        old_to_new.extend(local.into_iter().map(|new| base + new));
    }

    (out, old_to_new)
}

/// A section after grouping: the arguments, and everything the author wrote
/// between them, at the positions grouping has moved them to.
struct GroupedSection {
    args: Vec<String>,
    annotations: Annotations,
}

/// Group within each blank-line segment independently, moving what the author
/// wrote between the arguments to the positions the shorter segments give them.
fn group_source_pairs_preserving_blanks(
    section: &KeywordSection,
    grouping: super::config::SourceGrouping,
) -> GroupedSection {
    let args = &section.args;
    let blank_lines = section.annotations.blank_positions();

    // A section with no sortable run groups nothing, so `None` pins everything
    let sort_from = section.sort_from.unwrap_or(usize::MAX);

    if blank_lines.is_empty() {
        let (grouped_args, old_to_new) = group_sortable_runs(args, sort_from, grouping);
        let mut annotations = section.annotations.clone();
        annotations.remap(|position| {
            old_to_new
                .get(position)
                .copied()
                .unwrap_or(grouped_args.len())
        });
        annotations.settle_crossed();
        return GroupedSection {
            args: grouped_args,
            annotations,
        };
    }

    // Split args into segments at blank line boundaries
    let mut segments: Vec<&[String]> = Vec::new();
    let mut start = 0;

    for &bl_pos in &blank_lines {
        let end = bl_pos.min(args.len());
        segments.push(&args[start..end]);
        start = end;
    }
    // Final segment, always — even when empty.
    //
    // Blanks are re-emitted at segment boundaries, so `segments.len()` has to be
    // one more than the number of blanks or a blank at the end has no boundary
    // to be written at and is dropped. The next parse then read the survivor as
    // a different kind of entry and laid the section out differently — no
    // first-pass fixed point.
    segments.push(&args[start.min(args.len())..]);

    // Group each segment independently, tracking where each argument ends up
    let mut result = Vec::new();
    let mut global_old_to_new = vec![0; args.len()];

    let mut segment_start = 0;
    for segment in &segments {
        let (grouped, segment_old_to_new) =
            group_sortable_runs(segment, sort_from.saturating_sub(segment_start), grouping);

        for (local_index, &local_new) in segment_old_to_new.iter().enumerate() {
            let global_index = segment_start + local_index;
            if global_index < global_old_to_new.len() {
                global_old_to_new[global_index] = result.len() + local_new;
            }
        }

        result.extend(grouped);
        segment_start += segment.len();
    }

    // One mapping for comments and blanks alike. A blank sits at a segment
    // boundary, and `group_sortable_runs` always maps a segment's first argument
    // to the start of that segment's grouped output — `group_source_pairs` emits
    // each group at its earliest member's index, and index 0 is its own
    // earliest — so the boundary and that argument still share a position.
    let mut annotations = section.annotations.clone();
    annotations.remap(|position| {
        global_old_to_new
            .get(position)
            .copied()
            .unwrap_or(result.len())
    });
    annotations.settle_crossed();

    GroupedSection {
        args: result,
        annotations,
    }
}

/// Check if a command name requires keyword-aware formatting
pub fn is_keyword_aware_command(name: &str) -> bool {
    use super::grammar::GrammarRegistry;
    GrammarRegistry::global()
        .get(&name.to_lowercase())
        .is_some()
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
    /// Trailing inline comments: (arg_index, comment_text) - comment on same line after arg
    pub trailing_comments: Vec<(usize, String)>,
    /// Everything the author wrote between this section's arguments — their
    /// own-line comments and the blank lines around them — in the order they
    /// wrote it.
    pub annotations: Annotations,
    /// The type of the keyword (if known from grammar)
    pub keyword_type: Option<KeywordType>,
    /// `Some(n)` when this section's arguments are an unordered list that
    /// `sort_sources` and `source_grouping` may reorder, starting at index `n`.
    /// `None` means the order is meaningful and must be left alone.
    ///
    /// Decided by the grammar while sections are parsed — see
    /// [`CommandGrammar::sortable_keywords`] and
    /// [`CommandGrammar::sortable_positional`].
    pub sort_from: Option<usize>,
    /// Whether a newline appeared between the keyword and its first value
    /// (i.e., values were written on separate lines from the keyword)
    pub values_on_new_line: bool,
}

/// Parse an argument list into keyword sections with optional grammar guidance
pub fn parse_keyword_sections_with_grammar(
    arg_list: &ArgumentList,
    grammar: Option<&CommandGrammar>,
    comment_style: super::config::CommentStyle,
) -> Vec<KeywordSection> {
    let mut sections = Vec::new();
    let mut current_section = KeywordSection {
        keyword: None,
        args: Vec::new(),
        trailing_comments: Vec::new(),
        annotations: Annotations::default(),
        keyword_type: None,
        // Leading positional run: index 0 is the variable or target name
        sort_from: grammar.is_some_and(|g| g.sortable_positional).then_some(1),
        values_on_new_line: false,
    };

    let mut consecutive_newlines = 0;
    let mut saw_separator = true; // tracks whitespace for adjacent token merging
    // tracks newlines between a keyword and its first value
    let mut saw_newline_since_keyword = false;

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
                    let is_kw = if let Some(g) = grammar {
                        // Use grammar to determine if this is a keyword
                        g.keyword_type(&text).is_some()
                    } else {
                        // Fall back to hardcoded keyword check
                        is_cmake_keyword(&text)
                    };

                    // Check if this keyword should be consumed by the current BinPack or MultiValue section
                    // (e.g., DESTINATION inside LIBRARY BinPack, or PATTERN inside FILES_MATCHING MultiValue)
                    let consumed_as_sub_keyword = is_kw
                        && grammar.is_some_and(|g| g.sub_keywords.contains(&text))
                        && match current_section.keyword_type {
                            Some(KeywordType::BinPack) => true,
                            Some(KeywordType::MultiValue) => {
                                // Only consume sub_keywords in MultiValue sections that are
                                // explicitly marked as collection keywords (e.g., FILES_MATCHING)
                                current_section.keyword.as_ref().is_some_and(|kw| {
                                    grammar.is_some_and(|g| {
                                        g.collection_keywords.contains(kw.as_str())
                                    })
                                })
                            }
                            _ => false,
                        };

                    if is_kw && !consumed_as_sub_keyword {
                        // Get the keyword type from grammar if available
                        let kw_type = grammar.and_then(|g| g.keyword_type(&text));
                        // Whether this keyword's values are an unordered list
                        let kw_sort_from = grammar
                            .is_some_and(|g| g.is_sortable_keyword(&text))
                            .then_some(0);

                        // Start a new section.
                        //
                        // A leading section holding *only* comments is pushed
                        // too. Dropping it deleted every comment written before
                        // a command's first argument when that argument is a
                        // keyword — `install(\n\t# ship the headers\n\tFILES a.h\n\tDESTINATION inc)`
                        // lost the comment, at a stable fixed point and exit 0.
                        // The condition was "has anything worth emitting", and
                        // comments are worth emitting.
                        // `trailing_comments` is deliberately not asked: the
                        // parser only records one for a section that already
                        // holds an argument, so the first disjunct covers it.
                        if !current_section.args.is_empty()
                            || current_section.keyword.is_some()
                            || current_section.annotations.has_comments()
                        {
                            sections.push(current_section);
                        }
                        current_section = KeywordSection {
                            keyword: Some(text),
                            args: Vec::new(),
                            trailing_comments: Vec::new(),
                            annotations: Annotations::default(),
                            keyword_type: kw_type,
                            sort_from: kw_sort_from,
                            values_on_new_line: false,
                        };
                        saw_separator = true;
                        saw_newline_since_keyword = false;
                    } else if !saw_separator && !current_section.args.is_empty() {
                        // Adjacent to previous token (no whitespace) — merge
                        // e.g. ${VAR}/suffix is two tokens but one logical argument
                        current_section.args.last_mut().unwrap().push_str(&text);
                        saw_separator = false;
                    } else {
                        // Enforce SingleValue consumption: if the current keyword is
                        // SingleValue and already has its one value, overflow to a new
                        // positional section. This mirrors cmake_parse_arguments behavior
                        // where one_value_keywords consume exactly one argument.
                        let at_capacity =
                            matches!(current_section.keyword_type, Some(KeywordType::SingleValue))
                                && !current_section.args.is_empty();

                        if at_capacity {
                            // A leading mode keyword that consumed the list
                            // variable opens an unordered run, as in
                            // `list(APPEND var a b)`. Only the command's first
                            // section qualifies: anywhere later, this is a stray
                            // positional argument rather than the command's
                            // argument list.
                            //
                            // "Nothing but comments so far", not "nothing so
                            // far": a leading section holding only comments is
                            // pushed too, so `sections.is_empty()` stopped
                            // meaning what it said the moment that changed — and
                            // a comment before the mode keyword switched this
                            // branch's own feature off, silently, at exit 0.
                            // `list(# note / APPEND V z.cpp a.cpp)` stopped
                            // sorting, and so did every `sortable_positional`
                            // grammar.
                            let overflow_sortable = sections
                                .iter()
                                .all(|s| s.keyword.is_none() && s.args.is_empty())
                                && grammar.is_some_and(|g| g.sortable_positional);
                            sections.push(current_section);
                            current_section = KeywordSection {
                                keyword: None,
                                args: vec![text],
                                trailing_comments: Vec::new(),
                                annotations: Annotations::default(),
                                keyword_type: None,
                                sort_from: overflow_sortable.then_some(0),
                                values_on_new_line: false,
                            };
                        } else {
                            // Track if first value is on a new line from its keyword
                            if current_section.args.is_empty()
                                && current_section.keyword.is_some()
                                && saw_newline_since_keyword
                            {
                                current_section.values_on_new_line = true;
                            }
                            // Add as argument to current section
                            current_section.args.push(text);
                        }
                        saw_separator = false;
                    }
                }
                // Track comments
                SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
                    saw_separator = true;
                    // Normalize whitespace in line comments (not bracket comments)
                    let text = if token.kind() == SyntaxKind::COMMENT {
                        normalize_comment_whitespace(&text, comment_style)
                    } else {
                        text
                    };
                    if consecutive_newlines == 0 && !current_section.args.is_empty() {
                        // Same line as previous arg (trailing inline comment)
                        let arg_index = current_section.args.len() - 1;
                        current_section.trailing_comments.push((arg_index, text));
                    } else {
                        // Own line (leading comment before next arg)
                        let position = current_section.args.len();
                        current_section.annotations.push_comment(position, text);
                    }
                    consecutive_newlines = 0;
                }
                // Track newlines for blank line detection
                SyntaxKind::NEWLINE => {
                    saw_separator = true;
                    saw_newline_since_keyword = true;
                    consecutive_newlines += 1;
                    // Exactly 2, not 2-or-more. The third newline of one
                    // blank run took the `else` branch below and recorded "a
                    // blank line before comment index 0" for a comment that sits
                    // at a later argument position — so the next pass wrote a
                    // blank in front of that comment, and the output had no
                    // first-pass fixed point. A blank line *between comment
                    // groups* still reaches the `else`: the counter resets at
                    // the comment and climbs back to exactly 2.
                    if consecutive_newlines == 2 {
                        // Blank line detected - record position after last arg
                        let position = current_section.args.len();
                        current_section.annotations.push_blank(position);
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

    // Push the last section if it has content.
    //
    // Deliberately *not* the same question the mid-loop push asks, and the
    // asymmetry is not an oversight. Adding `|| !comments.is_empty()` here would
    // keep the comments of an argument list that holds nothing else — but a line
    // comment runs to the end of its line, so in `source_group(# why)` the
    // comment token already contains the command's closing paren. Emitting it on
    // its own line puts that paren inside the comment and the renderer writes a
    // second one, inventing a character. Guarding on "a newline followed the
    // comment" fixes the plain `f(# why)` spelling and not the grammar-aware
    // one, so the whole shape stays as the previous release left it: the
    // comments of an argument list holding nothing else are dropped.
    //
    // That is a real bug, it is older than this branch, and it is not reached by
    // any keyword this branch adds — an unknown command loses such a comment
    // too. Fixing it means teaching the emitter that a comment can own the
    // closing paren, which is where `render_nested_group` had to go for the same
    // reason.
    if !current_section.args.is_empty() || current_section.keyword.is_some() {
        sections.push(current_section);
    }

    // `add_library`/`add_executable` name a target in their first positional,
    // not the variable that governs the list
    let first_positional_governs = owning_command_name(arg_list)
        .map(|name| !matches!(name.as_str(), "add_library" | "add_executable"))
        .unwrap_or(true);
    unmark_unsortable_positional_runs(&mut sections, first_positional_governs);

    sections
}

/// Variables whose element order is a search precedence, not a set.
///
/// `list(APPEND CMAKE_MODULE_PATH cmake/overrides cmake/defaults)` resolves
/// first match wins, so sorting it silently picks a different module. Only ever
/// disables reordering, so a false positive costs a little tidiness and a false
/// negative costs correctness.
fn is_search_path_variable(name: &str) -> bool {
    // Compared case-insensitively: CMake variable names are case-sensitive, and
    // project-local lists are conventionally lowercase (`warning_flags`), so a
    // byte comparison would miss most of them. Unquoted first, as every sibling
    // predicate does — `"CMAKE_MODULE_PATH"` is as ordinary a spelling as the
    // bare one, and only the `FLAGS` substring test survived the quotes.
    let name = name.trim_matches('"').to_ascii_uppercase();

    const EXACT: &[&str] = &[
        "CMAKE_MODULE_PATH",
        "CMAKE_PREFIX_PATH",
        "CMAKE_FIND_ROOT_PATH",
        "CMAKE_INCLUDE_PATH",
        "CMAKE_LIBRARY_PATH",
        "CMAKE_PROGRAM_PATH",
    ];
    const SUFFIXES: &[&str] = &[
        "_PATH",
        "_PATHS",
        "_DIRS",
        "_DIRECTORIES",
        "_OPTIONS",
        // Argument lists feeding a COMMAND: `list(APPEND ARGS -o out.png)` then
        // `add_custom_command(COMMAND tool ${ARGS})` is an argv one level of
        // indirection away
        "_ARGS",
        "_ARGUMENTS",
        // Static archive link order is significant
        "_LIBS",
        "_LIBRARIES",
        // Glob/regex lists are evaluated in order, and that order decides the
        // order of what they match — see OCV_GLOB_PATTERNS in the OpenCV corpus
        "_PATTERNS",
    ];

    if EXACT.contains(&name.as_str()) || SUFFIXES.iter().any(|suffix| name.ends_with(suffix)) {
        return true;
    }

    // Compiler and linker flag lists, where the last flag usually wins.
    // `contains` rather than `ends_with`, because the common spelling is
    // `CMAKE_CXX_FLAGS_RELEASE` / `CMAKE_EXE_LINKER_FLAGS_DEBUG`, and because
    // pkg-config names are `GTK_CFLAGS` / `MY_LDFLAGS` with no underscore.
    name.contains("FLAGS")
}

/// True for a value that may be reordered inside a positional run.
///
/// A keyword names what its values are, so a keyword section trusts the grammar
/// alone. A positional run has no such label — `set(VAR …)` holds sources in one
/// project and compiler flags in the next — so its values have to look like
/// source files before anything moves. This sits behind the grammar allowlist
/// and only ever narrows it, so it cannot reopen the `CACHE`, `FILES_MATCHING`
/// or `file(RENAME)` cases: the first two are keyword sections, and `file` is a
/// recognised multi-mode command whose `RENAME` mode opts nothing in.
fn is_sortable_positional_value(arg: &str) -> bool {
    // Unquote first: a quoted flag is still a flag, and `"-I/usr/inc/a.h"`
    // would otherwise pass the prefix test and then look like a header
    let name = arg.trim_matches('"');

    // Flags and options: -Wall, --input, /O2, /wd4100, -I/usr/include. This
    // also rejects absolute POSIX paths, deliberately: protecting an MSVC flag
    // list is worth more than sorting a list of literal /usr/src/… paths.
    if name.starts_with('-') || name.starts_with('/') {
        return false;
    }
    // Definitions and assignments: -DVERSION=1.0, A=1
    if name.contains('=') {
        return false;
    }

    let Some(extension) = name.rsplit('.').next().filter(|ext| *ext != name) else {
        // No extension: a bare word like README or a target name. Sorting those
        // is fine where a keyword vouches for them, but not here.
        return false;
    };

    // A version number is not a file: `set(PYTHON_VERSIONS 3.9 3.12)` reads as
    // extension "9", and version lists are usually precedence lists.
    if extension.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    // Libraries, where link order decides symbol resolution
    const LINKABLE_EXTS: &[&str] = &["a", "so", "dylib", "lib", "dll", "o", "obj"];
    !LINKABLE_EXTS.contains(&extension.to_ascii_lowercase().as_str())
}

/// The command this argument list belongs to, when the tree above it says so.
///
/// Whether the leading positional is a variable or a target name is a property
/// of the command, and the section parser is otherwise given only the grammar,
/// which does not carry it.
fn owning_command_name(arg_list: &ArgumentList) -> Option<String> {
    let invocation = arg_list.syntax().parent()?;
    if invocation.kind() != SyntaxKind::COMMAND_INVOCATION {
        return None;
    }
    invocation
        .children_with_tokens()
        .filter_map(|child| child.into_token())
        .find(|token| token.kind() == SyntaxKind::COMMAND_NAME)
        .map(|token| token.text().to_lowercase())
}

/// Clear `sort_from` on positional runs that must not be reordered after all.
///
/// A positional run is opted in by the command's grammar, which cannot know what
/// the list actually holds. Two things disqualify one:
///
/// - its governing variable names a search path, flag list or argv — the name is
///   the run's own first argument (`set(VAR …)`) or the value of the preceding
///   single-value mode keyword (`list(APPEND VAR …)`);
/// - any of its values does not look like a source file.
///
/// `first_positional_governs` says whether the leading run's own first argument
/// is that variable. It is not for `add_library`/`add_executable`, where it
/// names a target: reading a target name as a variable name stopped
/// `add_library(my_libs z.cpp a.cpp)` sorting, because the name ends in `_LIBS`.
fn unmark_unsortable_positional_runs(
    sections: &mut [KeywordSection],
    first_positional_governs: bool,
) {
    for idx in 0..sections.len() {
        let Some(sort_from) = sections[idx].sort_from else {
            continue;
        };
        if sections[idx].keyword.is_some() {
            continue;
        }

        // At index 0 the first argument names the thing being defined — a
        // variable for `set`, but a *target* for add_library/add_executable.
        //
        // Which of the two is still decided by builtin command name, inside a
        // path that is otherwise grammar-driven, so a user wrapper cannot say
        // "my first positional is a target" and gets the conservative answer:
        // `my_add_library(my_LIBS z.cpp a.cpp)` holds where `add_library` sorts.
        // Saying it needs a third grammar field; until there is one, a wrapper
        // declines to sort where the builtin would, which is the safe direction.
        // A later run is opened by a mode keyword that already consumed the
        // list variable, so there the name provably governs the list.
        let (governing, governs_the_list) = if idx == 0 {
            (sections[idx].args.first(), first_positional_governs)
        } else {
            let previous = &sections[idx - 1];
            let name = previous
                .keyword
                .is_some()
                .then(|| previous.args.first())
                .flatten();
            (name, true)
        };

        // A list variable we cannot read is one we cannot vet — but only when
        // the whole token is a reference. `${PREFIX}_SOURCES` has a readable
        // suffix, and a dynamic *target* name says nothing about its sources.
        let blocked_by_name = governing.is_some_and(|name| {
            governs_the_list && (is_whole_variable_reference(name) || is_search_path_variable(name))
        });

        let blocked_by_value = sections[idx]
            .args
            .iter()
            .skip(sort_from)
            .any(|arg| !is_variable_like(arg) && !is_sortable_positional_value(arg));

        if blocked_by_name || blocked_by_value {
            sections[idx].sort_from = None;
        }
    }
}

/// Parse an argument list into keyword sections (backward compatibility wrapper)
#[allow(dead_code)]
pub fn parse_keyword_sections(arg_list: &ArgumentList) -> Vec<KeywordSection> {
    parse_keyword_sections_with_grammar(arg_list, None, super::config::CommentStyle::HashSpace)
}

/// Split `seg` into runs of adjacent sortable args, with each variable-like arg
/// becoming its own single-element run — a barrier that can neither move nor let
/// its neighbours move across it.
fn split_at_barriers(args: &[String], seg: std::ops::Range<usize>) -> Vec<std::ops::Range<usize>> {
    let mut runs: Vec<std::ops::Range<usize>> = Vec::new();
    let mut run_start = seg.start;

    for (offset, arg) in args[seg.start..seg.end].iter().enumerate() {
        let idx = seg.start + offset;
        if is_variable_like(arg) {
            if run_start < idx {
                runs.push(run_start..idx);
            }
            runs.push(idx..idx + 1);
            run_start = idx + 1;
        }
    }

    if run_start < seg.end {
        runs.push(run_start..seg.end);
    }

    runs
}

/// True for arguments that expand to something unknown at format time, so their
/// position may be meaningful even inside a list that is otherwise unordered.
///
/// A leading quote is stripped first: `"${GENERATED}"` is as common as the bare
/// spelling, and it would otherwise sort ahead of everything because `"` (0x22)
/// precedes every letter.
fn is_variable_like(s: &str) -> bool {
    let s = s.trim_start_matches('"');
    s.starts_with("${") || s.starts_with("$<") || s.starts_with("$ENV{") || s.starts_with("$CACHE{")
}

/// True when the whole token is a single variable reference, so nothing about
/// the name can be read. `${PREFIX}_SOURCES` is not one: its suffix is readable
/// and can be vetted against the search-path list.
fn is_whole_variable_reference(s: &str) -> bool {
    let s = s.trim_matches('"');
    ((s.starts_with("${") || s.starts_with("$ENV{") || s.starts_with("$CACHE{"))
        && s.ends_with('}'))
        || (s.starts_with("$<") && s.ends_with('>'))
}

/// Sort the arguments of a section the grammar marked as an unordered list,
/// respecting blank line boundaries and keeping comments with their arguments.
///
/// Rules:
/// - Only sections with `sort_from` set are touched; whether a list may be
///   reordered is the grammar's decision, not a guess from the argument text
/// - Arguments before `sort_from` are pinned (the variable or target name)
/// - Variable references and generator expressions hold their index and keep
///   arguments from moving across them
/// - Blank lines create separate sortable segments
/// - Comments at position N are associated with the argument at position N
///   (i.e., comment before a filename moves with that filename)
/// - Paired entries from source_grouping (e.g., "foo.h foo.cpp") sort as a unit
///   using their first component as sort key
/// - Case-insensitive sort
pub fn sort_source_args(section: &mut KeywordSection) {
    let Some(sort_start) = section.sort_from else {
        return;
    };

    if section.args.len() <= sort_start + 1 {
        // Nothing to reorder
        return;
    }

    // Split into segments by blank lines, but only for the sortable range
    let mut segments: Vec<std::ops::Range<usize>> = Vec::new();
    let mut seg_start = sort_start;
    for bl in section.annotations.blank_positions() {
        if bl > seg_start && bl <= section.args.len() {
            segments.push(seg_start..bl);
            seg_start = bl;
        }
    }
    if seg_start < section.args.len() {
        segments.push(seg_start..section.args.len());
    }

    // Split each segment further at variable-like arguments, which hold their
    // index and act as a barrier for the arguments around them
    let segments: Vec<std::ops::Range<usize>> = segments
        .into_iter()
        .flat_map(|seg| split_at_barriers(&section.args, seg))
        .collect();

    // For each segment, build sortable entries (arg + associated comments)
    // then sort and reassemble
    let mut new_args: Vec<String> = Vec::with_capacity(section.args.len());
    let mut new_trailing_comments: Vec<(usize, String)> = Vec::new();

    // Which argument ends up at each position. The comments written directly
    // above an argument follow it; everything else in the gap stays where the
    // author put it, so this is all the rebuild below needs.
    let mut origin_of: Vec<usize> = Vec::with_capacity(section.args.len());

    // First, preserve args before sort_start (e.g., target name in add_executable)
    for idx in 0..sort_start {
        new_args.push(section.args[idx].clone());
        origin_of.push(idx);
        // Preserve trailing comments for this arg
        for (tc_idx, tc_text) in &section.trailing_comments {
            if *tc_idx == idx {
                new_trailing_comments.push((new_args.len() - 1, tc_text.clone()));
            }
        }
    }

    // Now process sortable segments
    for seg in &segments {
        // Collect entries: each entry is (sort_key, arg, comments_before, trailing_comment)
        struct SortEntry {
            sort_key: String,
            arg: String,
            origin: usize, // where this argument was, so its comments can follow it
            trailing_comment: Option<String>, // trailing comment on same line as this arg
        }

        let mut entries: Vec<SortEntry> = Vec::new();

        for idx in seg.clone() {
            // Collect trailing comment at this arg index
            let trailing_comment = section
                .trailing_comments
                .iter()
                .find(|(tc_idx, _)| *tc_idx == idx)
                .map(|(_, text)| text.clone());

            let arg = &section.args[idx];

            // Check if this is a commented-out filename in the comments
            // (handled by keeping comments associated with args)

            // Sort key: case-insensitive, use first component for paired entries
            let sort_key = if arg.contains(' ') {
                // Paired entry (e.g., "foo.h foo.cpp") - sort by first component
                arg.split_whitespace().next().unwrap_or(arg).to_lowercase()
            } else {
                arg.to_lowercase()
            };

            entries.push(SortEntry {
                sort_key,
                arg: arg.clone(),
                origin: idx,
                trailing_comment,
            });
        }

        // Also collect comments that are at position == seg.end (trailing comments of segment)
        // These go after the last entry

        // Sort entries by sort key (stable sort preserves order of equal keys)
        entries.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));

        // Reassemble into new_args and new_trailing_comments
        for entry in &entries {
            new_args.push(entry.arg.clone());
            origin_of.push(entry.origin);
            if let Some(trailing) = &entry.trailing_comment {
                new_trailing_comments.push((new_args.len() - 1, trailing.clone()));
            }
        }
    }

    // Rebuild what is written between the arguments, in one pass over the new
    // order. Each gap is what the author left at that index and still holds —
    // the blank line that bounds the segment, and anything above it — followed
    // by the comments belonging to whichever argument landed there.
    //
    // The previous rebuild moved *every* comment with its argument and then
    // asked `post_comment_blanks` where the blank went. When sorting carried the
    // last comment off a position, that left the array claiming a blank was
    // "after the comments" at a position with no comments: unobservable at the
    // time, so the assertion skipped it, and live again as soon as grouping
    // moved a comment back. The list and the arrays then disagreed about which
    // came first, and the section rendered differently on the second pass.
    let mut annotations = Annotations::default();
    let push = |annotations: &mut Annotations, position: usize, item: &Annotation| match item {
        Annotation::Comment(text) => annotations.push_comment(position, text.clone()),
        Annotation::Blank => annotations.push_blank(position),
    };

    for (position, &origin) in origin_of.iter().enumerate() {
        for item in section.annotations.pinned_prefix(position) {
            push(&mut annotations, position, item);
        }
        for text in section.annotations.comments_after_last_blank(origin) {
            annotations.push_comment(position, text.to_string());
        }
    }

    // Nothing lands after the last argument, so that gap is untouched.
    for item in section.annotations.at(new_args.len()) {
        push(&mut annotations, new_args.len(), item);
    }

    section.annotations = annotations;
    section.args = new_args;
    section.trailing_comments = new_trailing_comments;
}

/// Group args that start with sub-keywords into logical groups.
///
/// Each sub_keyword and its following non-sub-keyword args form one logical group.
/// For example, ["PATTERN", "*.h", "EXCLUDE", "PATTERN", "internal/*"] with
/// sub_keywords {"PATTERN", "EXCLUDE"} yields ["PATTERN *.h", "EXCLUDE", "PATTERN internal/*"].
///
/// Used by FILES_MATCHING (MultiValue) to render each PATTERN/REGEX/EXCLUDE rule
/// as a single "logical line" in the output.
pub fn group_sub_keyword_args(args: &[String], sub_keywords: &HashSet<String>) -> Vec<String> {
    let mut groups: Vec<String> = Vec::new();
    let mut current_group = String::new();
    // Tracks whether the current group has any non-sub-keyword args yet.
    // When false, the group is still a "keyword-only prefix" and a following
    // sub_keyword should be appended (e.g., EXCLUDE followed by PATTERN).
    let mut current_has_value = false;

    for arg in args {
        if sub_keywords.contains(arg.as_str()) {
            if !current_group.is_empty() && current_has_value {
                // Current group has a value: flush and start a new group
                groups.push(std::mem::replace(&mut current_group, arg.clone()));
                current_has_value = false;
            } else if !current_group.is_empty() {
                // Current group is keyword-only prefix (e.g., EXCLUDE): append the next sub_keyword
                current_group.push(' ');
                current_group.push_str(arg);
                // Still no value appended
            } else {
                // No current group: start one
                current_group = arg.clone();
                current_has_value = false;
            }
        } else if current_group.is_empty() {
            // Arg before any sub_keyword: treat as its own group
            groups.push(arg.clone());
        } else {
            // Append value to current group
            current_group.push(' ');
            current_group.push_str(arg);
            current_has_value = true;
        }
    }
    if !current_group.is_empty() {
        groups.push(current_group);
    }
    groups
}

/// Format arguments for a keyword-aware command
///
/// `first_keyword_inline`: when true, the first keyword (SingleValue or Flag) stays on the
/// same line as the command name (used for multi-mode commands like `list(APPEND var)`
/// or `define_property(TEST PROPERTY name)`).
/// `builtin_grammar`: when true, Flag keywords after positional args stay inline
/// (e.g., `add_library(mylib STATIC ...)`). When false (user/auto-detected grammars),
/// flags break to new lines like other keywords.
/// `sub_keywords`: optional set of keywords that should be grouped within collection keywords
/// like FILES_MATCHING. When present, MultiValue sections that contain sub_keywords will
/// render logical groups (each sub_keyword + its values) per line.
#[allow(clippy::too_many_arguments)]
pub fn format_keyword_aware_args(
    arg_list: &ArgumentList,
    sections: Vec<KeywordSection>,
    config: &FormatConfig,
    indent_level: usize,
    first_keyword_inline: bool,
    builtin_grammar: bool,
    force_args_on_new_line: bool,
    sub_keywords: Option<&HashSet<String>>,
    command_name_len: usize,
) -> RcDoc<'static, ()> {
    if sections.is_empty() {
        return RcDoc::nil();
    }

    // Detect formatting signals from the input (same as non-grammar path).
    // Single-line input → force_multiline=false → flat_alt + group() tries flat first
    // Already-multiline input (has newlines/comments/blank lines) → force_multiline=true → preserves multiline
    let mut signals = detect_argument_formatting_signals(arg_list);

    // Config override: force_break_keywords always forces multiline
    if config.force_break_keywords {
        signals.force_multiline = true;
    }

    // Apply sorting if enabled (do this BEFORE source grouping)
    let mut sections = sections; // make mutable
    if config.sort_sources != super::config::SortSources::None {
        for section in &mut sections {
            sort_source_args(section);
        }
    }

    // Check if we have any actual keywords (not just pre-keyword args)
    let has_keywords = sections.iter().any(|s| s.keyword.is_some());

    if !has_keywords {
        // No keywords found, fall back to simple formatting
        return format_simple_args(
            &sections,
            config,
            signals.force_multiline,
            indent_level,
            force_args_on_new_line,
        );
    }

    // Explicit indentation strings for correct tab/space handling at any nesting depth
    let base_indent = super::cst_to_doc::indent_string(indent_level, config);
    let keyword_indent = super::cst_to_doc::indent_string(indent_level + 1, config);
    let value_indent = super::cst_to_doc::indent_string(indent_level + 2, config);

    // inline_single_keyword: when enabled and there is exactly one keyword section,
    // emit the keyword inline with the pre-keyword args and use single indentation for values.
    // Guard: only inline if the pre-keyword args + keyword fit on the command's opening line.
    // If pre-keyword args are multiline (don't fit), fall through to normal keyword formatting.
    let keyword_section_count = sections.iter().filter(|s| s.keyword.is_some()).count();
    if config.inline_single_keyword && keyword_section_count == 1 {
        // Compute whether pre-keyword args + keyword fit on the command's opening line.
        // If they don't fit, the pre-keyword args will break to multiple lines and the
        // keyword should NOT be inlined — fall through to normal formatting.
        let pre_kw_section = sections.iter().find(|s| s.keyword.is_none());
        let pre_kw_fits = if let Some(pre_section) = pre_kw_section {
            // Check: would "indent + cmd_name( + pre_args_joined_by_space + space + keyword" fit?
            let indent_width = config.indent_width; // used for both tab and space modes
            let paren_extra = if config.space_between_command_parens {
                2
            } else {
                1
            }; // '(' plus optional space
            let prefix_len = indent_level * indent_width + command_name_len + paren_extra;
            let pre_args_len: usize = pre_section.args.iter().map(|a| a.len()).sum::<usize>()
                + pre_section.args.len().saturating_sub(1); // spaces between args
            let keyword_section = sections.iter().find(|s| s.keyword.is_some()).unwrap();
            let keyword_len = keyword_section.keyword.as_ref().unwrap().len();
            let total = prefix_len + pre_args_len + 1 + keyword_len; // +1 for space before keyword
            // Also reject if pre-keyword section has comments (they force line breaks)
            !pre_section.annotations.has_comments() && total <= config.max_line_length
        } else {
            true // No pre-keyword args — keyword is first thing, always OK to inline
        };

        if pre_kw_fits {
            return format_keyword_aware_args_inline_single(
                &sections,
                config,
                &signals,
                &base_indent,
                &keyword_indent,
            );
        }
        // Otherwise fall through to normal keyword formatting
    }

    // Build keyword-aware Doc structure
    // ARGL-03: first arg should stay on same line as command (no separator before it)
    let mut docs = Vec::new();
    let mut is_first_arg = true;

    for (i, section) in sections.iter().enumerate() {
        if signals.force_multiline && blank_line_between_sections(&sections, i) {
            docs.push(RcDoc::hardline());
        }

        if let Some(keyword) = &section.keyword {
            // Handle different keyword types
            match section.keyword_type {
                // Flag keywords: group consecutive flags together
                Some(KeywordType::Flag) => {
                    // Grouping applies here too. A `Flag` section carries the
                    // positional run that follows the flag —
                    // `add_library(l STATIC a.cpp a.h)` — and `sort_sources`
                    // already reorders that run, so skipping `source_grouping`
                    // made the two passes disagree about a list the allowlist
                    // owns.
                    let GroupedSection {
                        args: flag_args,
                        annotations: flag_annotations,
                    } = grouped_section(section, config.source_grouping);
                    // Flags typically have no values, but flag_args may contain
                    // non-keyword arguments that follow before the next keyword
                    // Add separator before the flag keyword
                    if is_first_arg {
                        is_first_arg = false;
                        if keyword_stays_on_the_opening_line(&sections, i, first_keyword_inline) {
                            // No separator: the keyword belongs on the opening line
                        } else if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.clone()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::nil(),
                            ));
                        }
                    } else {
                        // Consecutive flags group with space; builtin flags after positional args stay inline
                        let prev_is_flag = matches!(
                            sections.get(i.saturating_sub(1)),
                            Some(prev) if prev.keyword_type == Some(KeywordType::Flag)
                        );
                        let prev_is_pre_keyword = builtin_grammar
                            && matches!(
                                sections.get(i.saturating_sub(1)),
                                Some(prev) if prev.keyword.is_none()
                            );
                        let flag_has_trailing_args = !flag_args.is_empty();
                        if !previous_section_ended_in_a_comment(&sections, i)
                            && ((prev_is_pre_keyword && flag_has_trailing_args)
                                || ((prev_is_flag || prev_is_pre_keyword)
                                    && config.collapse_empty_flags))
                        {
                            docs.push(RcDoc::space());
                        } else if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.clone()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::space(),
                            ));
                        }
                    }
                    docs.push(RcDoc::text(keyword.clone()));

                    // A flag with no values still carries its comments — the
                    // whole comment machinery below lives inside the
                    // `!flag_args.is_empty()` arm, so `find_package(Foo REQUIRED
                    // # note\n COMPONENTS ...)` lost `# note` outright.
                    //
                    // They are all own-line comments here whatever the author
                    // wrote: the section parser only fills `trailing_comments`
                    // for a section that already has an argument, so a comment on
                    // the same line as a valueless flag arrives at position 0 of
                    // `comments`. Each therefore takes its own line, and the
                    // separator for the *next* section refuses to collapse after
                    // one — a line comment runs to end of line, and collapsing
                    // put the next keyword inside it.
                    if flag_args.is_empty() {
                        push_valueless_section_comments(
                            &mut docs,
                            &flag_annotations,
                            &keyword_indent,
                        );
                    }

                    // Output any trailing non-keyword arguments in this section
                    if !flag_args.is_empty() {
                        // Use per-line when values were explicitly on new lines,
                        // or when there are comments/blank lines that can't go inline
                        let use_per_line = section.values_on_new_line
                            || !flag_annotations.is_empty()
                            || !section.trailing_comments.is_empty();

                        if use_per_line {
                            for (arg_idx, arg) in flag_args.iter().enumerate() {
                                // What the author wrote before this argument, in their order
                                push_annotations_before_argument(
                                    &mut docs,
                                    &flag_annotations,
                                    arg_idx,
                                    &keyword_indent,
                                    signals.force_multiline,
                                );
                                if signals.force_multiline {
                                    docs.push(RcDoc::hardline());
                                    docs.push(RcDoc::text(keyword_indent.clone()));
                                } else {
                                    docs.push(RcDoc::flat_alt(
                                        RcDoc::hardline()
                                            .append(RcDoc::text(keyword_indent.clone())),
                                        RcDoc::space(),
                                    ));
                                }
                                docs.push(RcDoc::text(arg.clone()));
                                // Emit trailing comment if present
                                for (tc_idx, tc_text) in &section.trailing_comments {
                                    if *tc_idx == arg_idx {
                                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                                    }
                                }
                            }
                            // The comments after the last argument, and the blank line the author
                            // left around them.
                            push_end_of_section_comments(
                                &mut docs,
                                &flag_annotations,
                                flag_args.len(),
                                &keyword_indent,
                                signals.force_multiline,
                            );
                        } else {
                            // Values on same line as keyword: flat_alt inherits from outer group
                            for (arg_idx, arg) in flag_args.iter().enumerate() {
                                docs.push(RcDoc::flat_alt(
                                    RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                    RcDoc::space(),
                                ));
                                docs.push(RcDoc::text(arg.clone()));
                                // Emit trailing comment if present
                                for (tc_idx, tc_text) in &section.trailing_comments {
                                    if *tc_idx == arg_idx {
                                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                                    }
                                }
                            }
                        }
                    }
                }

                // SingleValue keywords: keep value inline (ignore force_multiline for
                // idempotency).
                //
                // A comment used to demote the whole section into the catch-all
                // arm, which puts the keyword on its own line and its value one
                // level deeper. That cost the fixed point as well as the layout:
                // the parser gives a comment written after this keyword's one
                // value to *this* section, while a `list(APPEND V …)` run's
                // elements live in the following keyword-less section — so
                // sorting the run to put the commented element first moved the
                // comment into this slot, and the next pass laid the command out
                // differently. `--check` then rejected freshly formatted output.
                //
                // The comments are emitted after the value instead, so the arm
                // renders the same shape with them as without.
                Some(KeywordType::SingleValue) if section.args.len() == 1 => {
                    // Add separator before the keyword
                    if is_first_arg
                        && keyword_stays_on_the_opening_line(&sections, i, first_keyword_inline)
                    {
                        is_first_arg = false;
                        // No separator: the keyword belongs on the opening line
                    } else if is_first_arg {
                        is_first_arg = false;
                        // Regular command: first keyword drops to next line when multiline
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.clone()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::nil(),
                            ));
                        }
                    } else {
                        if groups_with_the_leading_flag(&sections, i, first_keyword_inline) {
                            docs.push(RcDoc::space());
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::space(),
                            ));
                        }
                    }
                    docs.push(RcDoc::text(keyword.clone()));
                    // Add the single value inline
                    docs.push(RcDoc::space());
                    docs.push(RcDoc::text(section.args[0].clone()));
                    // Then anything written about it, on its own line — a
                    // comment runs to end of line, so it cannot precede the
                    // value, and it must not be dropped
                    for (_, comment) in &section.trailing_comments {
                        docs.push(RcDoc::text(format!(" {}", comment)));
                    }
                    push_valueless_section_comments(
                        &mut docs,
                        &section.annotations,
                        &keyword_indent,
                    );
                }

                // PairValue keywords: format as key-value pairs
                Some(KeywordType::PairValue) => {
                    // Add separator before the keyword
                    if is_first_arg {
                        is_first_arg = false;
                        // First keyword in command: drop to next line when multiline
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.clone()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::nil(),
                            ));
                        }
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

                    // Format values as key-value pairs
                    if section.args.is_empty() {
                        push_valueless_section_comments(
                            &mut docs,
                            &section.annotations,
                            &keyword_indent,
                        );
                    }
                    if !section.args.is_empty() {
                        let pairs: Vec<_> = section.args.chunks(2).collect();
                        let use_per_line = section.values_on_new_line
                            || !section.annotations.is_empty()
                            || !section.trailing_comments.is_empty();

                        if pairs.len() == 1
                            && !section.annotations.has_comments()
                            && section.trailing_comments.is_empty()
                        {
                            // Single pair: keep inline with keyword (e.g., PROPERTIES KEY VALUE).
                            // Guarded on comments like every other shortcut that
                            // emits the keyword and its values and nothing else —
                            // and this one is tested *before* `use_per_line`, so
                            // the comments that set that flag never reached it.
                            docs.push(RcDoc::space());
                            docs.push(RcDoc::text(pairs[0][0].clone()));
                            if pairs[0].len() > 1 {
                                docs.push(RcDoc::space());
                                docs.push(RcDoc::text(pairs[0][1].clone()));
                            }
                        } else if use_per_line || signals.force_multiline {
                            // Per-line pairs. The comments are what put us on
                            // this branch — `use_per_line` is set by them — and
                            // they were then never emitted, so a `PROPERTIES`
                            // run with comments lost every one of them.
                            // Still the array here: this arm keys its comments
                            // to a *pair*, not to an argument, and it has never
                            // emitted a blank line at all. Moving it onto the
                            // ordered list would give it blank lines it does not
                            // have today, which is a behaviour change and not
                            // this commit's.
                            let mut comment_iter = section.annotations.comments().peekable();
                            for (pair_idx, chunk) in pairs.iter().enumerate() {
                                let key_index = pair_idx * 2;

                                // Own-line comments written before this pair
                                while let Some((position, comment)) = comment_iter.peek() {
                                    if *position > key_index {
                                        break;
                                    }
                                    docs.push(RcDoc::hardline());
                                    docs.push(RcDoc::text(value_indent.clone()));
                                    docs.push(RcDoc::text(comment.to_string()));
                                    comment_iter.next();
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
                                // key
                                docs.push(RcDoc::text(chunk[0].clone()));
                                // value (if present — odd number of args means last key has no value)
                                if chunk.len() > 1 {
                                    docs.push(RcDoc::space());
                                    docs.push(RcDoc::text(chunk[1].clone()));
                                }
                                // Both trailing comments come after the value. A comment runs to
                                // the end of its line, so emitting the key's before the value put
                                // the value *inside* the comment — `CXX_STANDARD # note 17` — and
                                // the next pass swallowed the following key too, losing a token per
                                // run. The inline_single_keyword twin has always emitted them in
                                // this order.
                                for (tc_index, tc_text) in &section.trailing_comments {
                                    if *tc_index == key_index
                                        || (chunk.len() > 1 && *tc_index == key_index + 1)
                                    {
                                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                                    }
                                }
                            }

                            // Anything written after the last pair
                            for (_, comment) in comment_iter {
                                docs.push(RcDoc::hardline());
                                docs.push(RcDoc::text(value_indent.clone()));
                                docs.push(RcDoc::text(comment.to_string()));
                            }
                        } else {
                            // Auto-layout: flat_alt pairs inherit from outer group
                            for chunk in pairs {
                                docs.push(RcDoc::flat_alt(
                                    RcDoc::hardline().append(RcDoc::text(value_indent.clone())),
                                    RcDoc::space(),
                                ));
                                docs.push(RcDoc::text(chunk[0].clone()));
                                if chunk.len() > 1 {
                                    docs.push(RcDoc::space());
                                    docs.push(RcDoc::text(chunk[1].clone()));
                                }
                            }
                        }
                    }
                }

                // MultiValue with exactly 1 arg: keep inline like SingleValue
                // A single value goes inline with its keyword — but only when
                // there is nothing else to place. This arm emits the keyword and
                // the value and nothing else, so a comment written on its own
                // line inside the section was silently deleted:
                // `target_sources(t PRIVATE\n\t# impl\n\tb.cpp)` lost `# impl`
                // entirely. The `inline_single_keyword` path already guards its
                // equivalent shortcut this way; the general path did not.
                Some(KeywordType::MultiValue)
                    if section.args.len() == 1
                        && !section.annotations.has_comments()
                        && section.trailing_comments.is_empty() =>
                {
                    // Add separator before the keyword
                    if is_first_arg {
                        is_first_arg = false;
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.clone()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::nil(),
                            ));
                        }
                    } else {
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                            RcDoc::space(),
                        ));
                    }
                    docs.push(RcDoc::text(keyword.clone()));
                    // Single value inline with keyword
                    docs.push(RcDoc::space());
                    docs.push(RcDoc::text(section.args[0].clone()));
                }

                // BinPack keywords (e.g., COMMAND): pack values to fill lines
                Some(KeywordType::BinPack) => {
                    // Add separator before the keyword (same pattern as other keywords)
                    if is_first_arg {
                        is_first_arg = false;
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.clone()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::nil(),
                            ));
                        }
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

                    // Values: bin-pack using per-value groups
                    if section.args.is_empty() {
                        push_valueless_section_comments(
                            &mut docs,
                            &section.annotations,
                            &keyword_indent,
                        );
                    }
                    if !section.args.is_empty() {
                        let has_annotations = !section.annotations.is_empty()
                            || !section.trailing_comments.is_empty();

                        if has_annotations {
                            // Path A: per-line with full comment support (same pattern as MultiValue use_per_line)

                            for (arg_idx, arg) in section.args.iter().enumerate() {
                                // What the author wrote before this argument, in their order
                                push_annotations_before_argument(
                                    &mut docs,
                                    &section.annotations,
                                    arg_idx,
                                    &value_indent,
                                    signals.force_multiline,
                                );

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
                                // Emit trailing comment if present
                                for (tc_idx, tc_text) in &section.trailing_comments {
                                    if *tc_idx == arg_idx {
                                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                                    }
                                }
                            }

                            // The comments after the last argument, and the blank line the author
                            // left around them.
                            push_end_of_section_comments(
                                &mut docs,
                                &section.annotations,
                                section.args.len(),
                                &value_indent,
                                signals.force_multiline,
                            );
                        } else if signals.force_multiline {
                            // Path B: manual bin-packing with width tracking
                            // The pretty printer's group(flat_alt(...)) doesn't break inner groups
                            // when the parent is already broken, so we manually track line width.
                            let mut current_line_width = keyword_indent.len() + keyword.len();
                            for arg in &section.args {
                                let needed = 1 + arg.len();
                                if current_line_width + needed <= config.max_line_length {
                                    docs.push(RcDoc::space());
                                    docs.push(RcDoc::text(arg.clone()));
                                    current_line_width += needed;
                                } else {
                                    docs.push(RcDoc::hardline());
                                    docs.push(RcDoc::text(value_indent.clone()));
                                    docs.push(RcDoc::text(arg.clone()));
                                    current_line_width = value_indent.len() + arg.len();
                                }
                            }
                        } else {
                            // Path C: group(flat_alt(...)) for non-multiline input
                            // The pretty printer checks each inner group independently:
                            // if flat form (space + arg) fits remaining line width, stays flat;
                            // otherwise breaks to new line with value_indent
                            for arg in &section.args {
                                docs.push(RcDoc::group(
                                    RcDoc::flat_alt(
                                        RcDoc::hardline().append(RcDoc::text(value_indent.clone())),
                                        RcDoc::space(),
                                    )
                                    .append(RcDoc::text(arg.clone())),
                                ));
                            }
                        }
                    }
                }

                // MultiValue or SingleValue with >1 arg in force_multiline mode: vertical layout
                _ => {
                    // Standard vertical keyword formatting
                    if is_first_arg {
                        is_first_arg = false;
                        // First keyword in command: drop to next line when multiline
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.clone()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                                RcDoc::nil(),
                            ));
                        }
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
                    if section.args.is_empty() {
                        push_valueless_section_comments(
                            &mut docs,
                            &section.annotations,
                            &keyword_indent,
                        );
                    }
                    if !section.args.is_empty() {
                        // Check if this is a collection keyword with sub_keyword grouping
                        // (e.g., FILES_MATCHING with PATTERN/REGEX/EXCLUDE sub-items)
                        // Only apply when: section has sub_keywords, no interleaved comments/blank lines
                        let grouped_collection = if let Some(sub_kws) = sub_keywords {
                            if !sub_kws.is_empty()
                                && section.annotations.is_empty()
                                && section.trailing_comments.is_empty()
                                && section.args.iter().any(|a| sub_kws.contains(a.as_str()))
                            {
                                Some(group_sub_keyword_args(&section.args, sub_kws))
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(groups) = grouped_collection {
                            // Collection keyword: render logical groups
                            if groups.len() == 1 {
                                // Single logical group: keep inline with the keyword
                                docs.push(RcDoc::space());
                                docs.push(RcDoc::text(groups[0].clone()));
                            } else {
                                // Multiple groups: each on its own indented line below the keyword
                                for group in &groups {
                                    if signals.force_multiline {
                                        docs.push(RcDoc::hardline());
                                        docs.push(RcDoc::text(value_indent.clone()));
                                    } else {
                                        docs.push(RcDoc::flat_alt(
                                            RcDoc::hardline()
                                                .append(RcDoc::text(value_indent.clone())),
                                            RcDoc::space(),
                                        ));
                                    }
                                    docs.push(RcDoc::text(group.clone()));
                                }
                            }
                            continue; // Section fully rendered, skip to next section
                        }

                        // Apply source grouping if enabled
                        // Disable grouping only when trailing comments are present (can't merge inline comments)
                        // Leading comments can be remapped to their new positions
                        // Blank lines are preserved as segment boundaries
                        let GroupedSection {
                            args: effective_args,
                            annotations: effective_annotations,
                        } = grouped_section(section, config.source_grouping);

                        // Use per-line when values were explicitly on new lines,
                        // or when there are comments/blank lines that can't go inline
                        let use_per_line =
                            section.values_on_new_line || !effective_annotations.is_empty();

                        if use_per_line {
                            // Values on separate lines or has comments: keep per-line behavior

                            for (arg_idx, arg) in effective_args.iter().enumerate() {
                                // What the author wrote before this argument, in their order
                                push_annotations_before_argument(
                                    &mut docs,
                                    &effective_annotations,
                                    arg_idx,
                                    &value_indent,
                                    signals.force_multiline,
                                );

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
                                // Emit trailing comment if present
                                for (tc_idx, tc_text) in &section.trailing_comments {
                                    if *tc_idx == arg_idx {
                                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                                    }
                                }
                            }

                            // The comments after the last argument, and the blank line the author
                            // left around them.
                            push_end_of_section_comments(
                                &mut docs,
                                &effective_annotations,
                                effective_args.len(),
                                &value_indent,
                                signals.force_multiline,
                            );
                        } else {
                            // Values on same line as keyword: flat_alt inherits from outer group
                            for (arg_idx, arg) in effective_args.iter().enumerate() {
                                docs.push(RcDoc::flat_alt(
                                    RcDoc::hardline().append(RcDoc::text(value_indent.clone())),
                                    RcDoc::space(),
                                ));
                                docs.push(RcDoc::text(arg.clone()));
                                // Emit trailing comment if present (should be rare since use_per_line checks trailing_comments)
                                for (tc_idx, tc_text) in &section.trailing_comments {
                                    if *tc_idx == arg_idx {
                                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Pre-keyword arguments (e.g., target name, or file list)
            // ARGL-03 refined: first arg stays inline only when it's a single
            // pre-keyword arg (e.g., target name). When there are multiple
            // pre-keyword args (a file list), all go on separate lines.

            // Apply source grouping if enabled
            // Disable grouping only when trailing comments are present (can't merge inline comments)
            // Leading comments can be remapped to their new positions
            // Blank lines are preserved as segment boundaries
            let GroupedSection {
                args: effective_args,
                annotations: effective_annotations,
            } = grouped_section(section, config.source_grouping);

            let is_list = effective_args.len() > 1 || force_args_on_new_line;

            for (arg_idx, arg) in effective_args.iter().enumerate() {
                // What the author wrote before this argument, in their order
                if push_annotations_before_argument(
                    &mut docs,
                    &effective_annotations,
                    arg_idx,
                    &keyword_indent,
                    signals.force_multiline,
                ) {
                    is_first_arg = false;
                }

                if is_first_arg && !is_list {
                    // Single pre-keyword arg: keep inline with command
                    is_first_arg = false;
                } else if is_first_arg {
                    // First arg of a list: treat like subsequent args
                    is_first_arg = false;
                    if signals.force_multiline {
                        docs.push(RcDoc::hardline());
                        docs.push(RcDoc::text(keyword_indent.clone()));
                    } else {
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(keyword_indent.clone())),
                            RcDoc::nil(),
                        ));
                    }
                } else {
                    // Subsequent args: add separator with explicit indentation
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
                // Emit trailing comment if present
                for (tc_idx, tc_text) in &section.trailing_comments {
                    if *tc_idx == arg_idx {
                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                    }
                }
            }

            // The comments after the last argument, and the blank line the author
            // left around them.
            push_end_of_section_comments(
                &mut docs,
                &effective_annotations,
                effective_args.len(),
                &keyword_indent,
                signals.force_multiline,
            );
        }
    }

    // Closing paren position
    docs.push(super::cst_to_doc::closing_paren_position(
        config,
        indent_level,
        signals.force_multiline,
    ));

    let combined = RcDoc::concat(docs);

    if signals.force_multiline {
        combined
    } else {
        combined.group()
    }
}

/// Format keyword-aware args with a single keyword section using inline layout.
///
/// When `inline_single_keyword = true` and there is exactly one keyword section:
/// - The keyword is emitted on the SAME line as the pre-keyword args (with a space separator).
/// - Values under the keyword are indented at `keyword_indent` level (one level, not two).
/// - The closing paren is at `base_indent` level as usual.
///
/// When the command fits on one line, flat rendering keeps everything inline (unchanged).
fn format_keyword_aware_args_inline_single(
    sections: &[KeywordSection],
    config: &FormatConfig,
    signals: &super::cst_to_doc::ArgumentFormatSignals,
    base_indent: &str,
    keyword_indent: &str,
) -> RcDoc<'static, ()> {
    let mut docs = Vec::new();
    let mut is_first_arg = true;

    for (i, section) in sections.iter().enumerate() {
        if signals.force_multiline && blank_line_between_sections(sections, i) {
            docs.push(RcDoc::hardline());
        }

        if let Some(keyword) = &section.keyword {
            // There is exactly one keyword section — emit keyword INLINE with preceding args.
            // Separator: space (flat and broken both use space here)
            if is_first_arg {
                // Keyword is the very first thing (no pre-keyword args).
                // In inline_single_keyword mode the keyword appears directly after '(' — no separator.
                // This is correct for both flat and broken (force_multiline) rendering since the
                // keyword's position on the opening line is the whole point of this function.
                is_first_arg = false;
                if !signals.force_multiline {
                    docs.push(RcDoc::flat_alt(
                        RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                        RcDoc::nil(),
                    ));
                }
                // force_multiline=true: emit nothing — keyword stays on the opening line after '('
            } else if previous_section_ended_in_a_comment(sections, i) {
                // The pre-keyword args ended in a comment, so this keyword cannot
                // share their line — it would become part of the comment.
                is_first_arg = false;
                docs.push(RcDoc::hardline());
                docs.push(RcDoc::text(keyword_indent.to_string()));
            } else {
                // Keyword follows pre-keyword args — stays on same line as the last pre-keyword arg.
                // In flat mode this is already inline; in broken mode we want a space (not a newline).
                is_first_arg = false;
                docs.push(RcDoc::space());
            }
            docs.push(RcDoc::text(keyword.clone()));

            // For SingleValue keywords with exactly one arg, render the value inline (same line as
            // the keyword) rather than indented below. This keeps "APPEND SOURCES" together on the
            // opening line when the overflow positional args follow in the next section.
            //
            // Comments do not disqualify it — they are emitted after the value
            // below, exactly as the general path does. Requiring the section to
            // carry none put the mode keyword on its own line as soon as anyone
            // wrote one, and cost the first-pass fixed point under sorting: the
            // parser gives a comment written after this keyword's value to *this*
            // section while the run's elements live in the next, so sorting the
            // commented element to the front moved the comment into that slot and
            // the following pass laid the command out differently.
            //
            // A blank line does not disqualify it either, now that
            // `blank_line_between_sections` writes one left at the end of a
            // section. While nothing wrote it, the blank demoted the keyword
            // here and was then dropped, so the next pass put the keyword back
            // and `--check` rejected what `-i` had written.
            let is_single_value_with_one_arg =
                section.keyword_type == Some(KeywordType::SingleValue) && section.args.len() == 1;

            let is_pair_value = section.keyword_type == Some(KeywordType::PairValue);

            if is_single_value_with_one_arg {
                // Always emit inline with a space — even when force_multiline is true.
                docs.push(RcDoc::space());
                docs.push(RcDoc::text(section.args[0].clone()));
                // Then anything written about it, on its own line — a comment
                // runs to end of line, so it cannot precede the value
                for (_, comment) in &section.trailing_comments {
                    docs.push(RcDoc::text(format!(" {}", comment)));
                }
                push_valueless_section_comments(&mut docs, &section.annotations, keyword_indent);
            } else if is_pair_value && !section.args.is_empty() {
                // PairValue keywords (e.g., PROPERTIES): format as key-value pairs
                // Use keyword_indent (single indent) since the keyword is inlined on the command line
                let pairs: Vec<_> = section.args.chunks(2).collect();
                let use_per_line = section.values_on_new_line
                    || !section.annotations.is_empty()
                    || !section.trailing_comments.is_empty();

                if pairs.len() == 1
                    && !section.annotations.has_comments()
                    && section.trailing_comments.is_empty()
                {
                    // Single pair: keep inline with keyword. Same guard as the
                    // general path's twin, for the same reason.
                    docs.push(RcDoc::space());
                    docs.push(RcDoc::text(pairs[0][0].clone()));
                    if pairs[0].len() > 1 {
                        docs.push(RcDoc::space());
                        docs.push(RcDoc::text(pairs[0][1].clone()));
                    }
                } else if use_per_line || signals.force_multiline {
                    let mut pair_comments = section.annotations.comments().peekable();
                    for (pair_idx, chunk) in pairs.iter().enumerate() {
                        if section.annotations.has_blank_at(pair_idx * 2) && signals.force_multiline
                        {
                            docs.push(RcDoc::hardline());
                        }
                        // Own-line comments written before this pair. This loop
                        // was missing entirely, so the twin emitted only trailing
                        // comments and deleted every own-line one.
                        while let Some((position, comment)) = pair_comments.peek() {
                            if *position > pair_idx * 2 {
                                break;
                            }
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.to_string()));
                            docs.push(RcDoc::text(comment.to_string()));
                            pair_comments.next();
                        }
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.to_string()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                                RcDoc::space(),
                            ));
                        }
                        docs.push(RcDoc::text(chunk[0].clone()));
                        if chunk.len() > 1 {
                            docs.push(RcDoc::space());
                            docs.push(RcDoc::text(chunk[1].clone()));
                        }
                        // Trailing comments for key (even index) and value (odd index)
                        for (tc_idx, tc_text) in &section.trailing_comments {
                            let key_idx = pair_idx * 2;
                            if *tc_idx == key_idx || (chunk.len() > 1 && *tc_idx == key_idx + 1) {
                                docs.push(RcDoc::text(format!(" {}", tc_text)));
                            }
                        }
                    }
                    // Anything written after the last pair
                    for (_, comment) in pair_comments {
                        docs.push(RcDoc::hardline());
                        docs.push(RcDoc::text(keyword_indent.to_string()));
                        docs.push(RcDoc::text(comment.to_string()));
                    }
                } else {
                    for chunk in pairs {
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                            RcDoc::space(),
                        ));
                        docs.push(RcDoc::text(chunk[0].clone()));
                        if chunk.len() > 1 {
                            docs.push(RcDoc::space());
                            docs.push(RcDoc::text(chunk[1].clone()));
                        }
                    }
                }
            } else if section.args.is_empty() {
                push_valueless_section_comments(&mut docs, &section.annotations, keyword_indent);
            } else {
                // Apply source grouping to keyword section args (e.g., source files after PUBLIC)
                let GroupedSection {
                    args: effective_args,
                    annotations: effective_annotations,
                } = grouped_section(section, config.source_grouping);

                // Values are indented at keyword_indent level (single indent, not double)
                let use_per_line = section.values_on_new_line
                    || !effective_annotations.is_empty()
                    || !section.trailing_comments.is_empty();

                if use_per_line {
                    for (arg_idx, arg) in effective_args.iter().enumerate() {
                        // What the author wrote before this argument, in their order
                        push_annotations_before_argument(
                            &mut docs,
                            &effective_annotations,
                            arg_idx,
                            keyword_indent,
                            signals.force_multiline,
                        );
                        if signals.force_multiline {
                            docs.push(RcDoc::hardline());
                            docs.push(RcDoc::text(keyword_indent.to_string()));
                        } else {
                            docs.push(RcDoc::flat_alt(
                                RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                                RcDoc::space(),
                            ));
                        }
                        docs.push(RcDoc::text(arg.clone()));
                        for (tc_idx, tc_text) in &section.trailing_comments {
                            if *tc_idx == arg_idx {
                                docs.push(RcDoc::text(format!(" {}", tc_text)));
                            }
                        }
                    }
                    // The comments after the last argument, and the blank line the author
                    // left around them.
                    push_end_of_section_comments(
                        &mut docs,
                        &effective_annotations,
                        effective_args.len(),
                        keyword_indent,
                        signals.force_multiline,
                    );
                } else {
                    // Flat layout: values go on new lines below keyword (single indent) when broken
                    for (arg_idx, arg) in effective_args.iter().enumerate() {
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                            RcDoc::space(),
                        ));
                        docs.push(RcDoc::text(arg.clone()));
                        for (tc_idx, tc_text) in &section.trailing_comments {
                            if *tc_idx == arg_idx {
                                docs.push(RcDoc::text(format!(" {}", tc_text)));
                            }
                        }
                    }
                }
            }
        } else {
            // Pre-keyword section: same as the main loop's pre-keyword handling
            let GroupedSection {
                args: effective_args,
                annotations: effective_annotations,
            } = grouped_section(section, config.source_grouping);

            let is_list = effective_args.len() > 1;

            for (arg_idx, arg) in effective_args.iter().enumerate() {
                // What the author wrote before this argument, in their order
                if push_annotations_before_argument(
                    &mut docs,
                    &effective_annotations,
                    arg_idx,
                    keyword_indent,
                    signals.force_multiline,
                ) {
                    is_first_arg = false;
                }

                if is_first_arg && !is_list {
                    is_first_arg = false;
                } else if is_first_arg {
                    is_first_arg = false;
                    if signals.force_multiline {
                        docs.push(RcDoc::hardline());
                        docs.push(RcDoc::text(keyword_indent.to_string()));
                    } else {
                        docs.push(RcDoc::flat_alt(
                            RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                            RcDoc::nil(),
                        ));
                    }
                } else if signals.force_multiline {
                    docs.push(RcDoc::hardline());
                    docs.push(RcDoc::text(keyword_indent.to_string()));
                } else {
                    docs.push(RcDoc::flat_alt(
                        RcDoc::hardline().append(RcDoc::text(keyword_indent.to_string())),
                        RcDoc::space(),
                    ));
                }
                docs.push(RcDoc::text(arg.clone()));
                for (tc_idx, tc_text) in &section.trailing_comments {
                    if *tc_idx == arg_idx {
                        docs.push(RcDoc::text(format!(" {}", tc_text)));
                    }
                }
            }

            // The comments after the last argument, and the blank line the author
            // left around them.
            push_end_of_section_comments(
                &mut docs,
                &effective_annotations,
                effective_args.len(),
                keyword_indent,
                signals.force_multiline,
            );
        }
    }

    // Closing paren position
    // Need indent_level to compute closing indent — derive from base_indent length
    // The base_indent is already computed for us; use closing_paren_position with a reconstructed level.
    // Since format_keyword_aware_args_inline_single doesn't take indent_level directly,
    // we push the closing paren directly using the same logic as closing_paren_position.
    let closing_indent = if config.indent_closing_paren {
        // We need to add one extra indent level. base_indent is the current level string.
        // Compute by checking config to get a single indent unit.
        let one_indent = if config.use_tabs {
            "\t".to_string()
        } else {
            " ".repeat(config.indent_width)
        };
        format!("{}{}", base_indent, one_indent)
    } else {
        base_indent.to_string()
    };
    let flat_text = if config.space_between_command_parens {
        RcDoc::text(" ")
    } else {
        RcDoc::nil()
    };
    if signals.force_multiline {
        docs.push(RcDoc::hardline());
        docs.push(RcDoc::text(closing_indent));
    } else {
        docs.push(RcDoc::flat_alt(
            RcDoc::hardline().append(RcDoc::text(closing_indent)),
            flat_text,
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
fn format_simple_args(
    sections: &[KeywordSection],
    config: &FormatConfig,
    force_multiline: bool,
    indent_level: usize,
    force_args_on_new_line: bool,
) -> RcDoc<'static, ()> {
    let inner_indent = super::cst_to_doc::indent_string(indent_level + 1, config);

    let mut docs = Vec::new();
    let mut is_first_arg = true;

    // In flat mode (auto-layout), start with nothing before first arg
    if !force_multiline {
        docs.push(RcDoc::flat_alt(RcDoc::nil(), RcDoc::nil()));
    }

    // Apply sorting if enabled (do this BEFORE source grouping)
    let mut sections_owned: Vec<KeywordSection> = sections.to_vec();
    if config.sort_sources != super::config::SortSources::None {
        for section in &mut sections_owned {
            sort_source_args(section);
        }
    }
    let sections = &sections_owned;

    // Collect all args and comments from all sections
    for section in sections {
        // Apply source grouping if enabled
        // Disable grouping only when trailing comments are present (can't merge inline comments)
        // Leading comments can be remapped to their new positions
        // Blank lines are preserved as segment boundaries
        let GroupedSection {
            args: effective_args,
            annotations: effective_annotations,
        } = grouped_section(section, config.source_grouping);

        for (arg_idx, arg) in effective_args.iter().enumerate() {
            // What the author wrote before this argument, in their order
            if push_annotations_before_argument(
                &mut docs,
                &effective_annotations,
                arg_idx,
                &inner_indent,
                force_multiline,
            ) {
                is_first_arg = false;
            }

            // Add separator before arg (except for the very first arg, unless force_args_on_new_line)
            if !is_first_arg || force_args_on_new_line {
                if force_multiline {
                    docs.push(RcDoc::hardline());
                    docs.push(RcDoc::text(inner_indent.clone()));
                } else {
                    // For the first arg with force_args_on_new_line, use nil in flat mode
                    // (no space before first arg when everything fits on one line)
                    let flat = if is_first_arg {
                        RcDoc::nil()
                    } else {
                        RcDoc::space()
                    };
                    docs.push(RcDoc::flat_alt(
                        RcDoc::hardline().append(RcDoc::text(inner_indent.clone())),
                        flat,
                    ));
                }
            }
            docs.push(RcDoc::text(arg.clone()));
            // Emit trailing comment if present
            for (tc_idx, tc_text) in &section.trailing_comments {
                if *tc_idx == arg_idx {
                    docs.push(RcDoc::text(format!(" {}", tc_text)));
                }
            }
            is_first_arg = false;
        }

        // The comments after the last argument, and the blank line the author
        // left around them.
        if push_end_of_section_comments(
            &mut docs,
            &effective_annotations,
            effective_args.len(),
            &inner_indent,
            force_multiline,
        ) {
            is_first_arg = false;
        }
    }

    // Closing paren position
    docs.push(super::cst_to_doc::closing_paren_position(
        config,
        indent_level,
        force_multiline,
    ));

    let combined = RcDoc::concat(docs);

    if force_multiline {
        combined
    } else {
        combined.group()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatter::config::SourceGrouping;

    /// A section holding nothing but what the author wrote between its
    /// arguments, which is now the whole of it.
    fn section(
        args: &[&str],
        annotations: Annotations,
        sort_from: Option<usize>,
    ) -> KeywordSection {
        KeywordSection {
            keyword: None,
            args: args.iter().map(|arg| arg.to_string()).collect(),
            trailing_comments: Vec::new(),
            annotations,
            keyword_type: None,
            sort_from,
            values_on_new_line: false,
        }
    }

    /// The comments a section holds, as `(position, text)`.
    fn comments_of(annotations: &Annotations) -> Vec<(usize, String)> {
        annotations
            .comments()
            .map(|(position, text)| (position, text.to_string()))
            .collect()
    }

    fn comment_at(position: usize, text: &str) -> (usize, Annotation) {
        (position, Annotation::Comment(text.to_string()))
    }

    fn list(items: &[(usize, Annotation)]) -> Annotations {
        let mut annotations = Annotations::default();
        for (position, item) in items {
            match item {
                Annotation::Comment(text) => annotations.push_comment(*position, text.clone()),
                Annotation::Blank => annotations.push_blank(*position),
            }
        }
        annotations
    }

    /// Sorting carries a comment along with the argument it sits in front of, so
    /// the ordered list has to be rebuilt rather than repositioned — two
    /// comments can swap places, which no position rewrite expresses.
    #[test]
    fn sorting_moves_the_ordered_list_with_the_comments() {
        let mut section = section(
            &["c.cpp", "b.cpp", "a.cpp"],
            list(&[comment_at(0, "# about c")]),
            Some(0),
        );

        sort_source_args(&mut section);

        assert_eq!(section.args, ["a.cpp", "b.cpp", "c.cpp"]);
        assert_eq!(
            comments_of(&section.annotations),
            [(2, "# about c".to_string())]
        );
    }

    /// A blank line holds everything above it in place when sorting moves the
    /// arguments. Only the note written directly above an argument, with nothing
    /// between, is about that argument and travels with it.
    #[test]
    fn sorting_leaves_what_a_blank_line_holds_where_it_is() {
        let mut section = section(
            &["c.cpp", "b.cpp", "a.cpp"],
            list(&[
                (0, Annotation::Blank),
                comment_at(0, "# one"),
                (0, Annotation::Blank),
                comment_at(0, "# two"),
            ]),
            Some(0),
        );

        sort_source_args(&mut section);

        assert_eq!(section.args, ["a.cpp", "b.cpp", "c.cpp"]);
        assert_eq!(
            comments_of(&section.annotations),
            [(0, "# one".to_string()), (2, "# two".to_string())],
            "`# two` was written directly above c.cpp, so it follows it; `# one` \
             is held above the blank line and stays"
        );
        assert_eq!(
            section.annotations.blank_positions(),
            [0],
            "the segment boundary holds"
        );
    }

    /// Grouping's mapping runs backwards: `a.h` and `a.cpp` pair onto one line
    /// at the earlier index, so `a.cpp` lands *before* the `b.cpp` between them,
    /// and the comment it carries crosses the comment about `b.cpp`. Both
    /// encodings come out of grouping back in position order, so every arm — all
    /// of which walk forwards — still meets each comment at its argument.
    #[test]
    fn grouping_puts_a_crossed_comment_back_in_front_of_its_argument() {
        let section = section(
            &["a.h", "b.cpp", "a.cpp"],
            list(&[comment_at(1, "# about b"), comment_at(2, "# about a.cpp")]),
            Some(0),
        );

        let grouped = grouped_section(&section, SourceGrouping::HeadersFirst);

        assert_eq!(grouped.args, ["a.h a.cpp", "b.cpp"]);
        assert_eq!(
            comments_of(&grouped.annotations),
            [
                (0, "# about a.cpp".to_string()),
                (1, "# about b".to_string()),
            ],
            "the comment follows the argument grouping moved"
        );
    }

    /// A blank line is a segment boundary, and grouping shortens the segments it
    /// bounds. The boundary and the argument that opens the segment still share
    /// a position afterwards — which is why one mapping serves both — and a
    /// blank the author wrote *after* the comments there stays after them.
    #[test]
    fn grouping_keeps_a_blank_with_the_argument_that_opens_its_segment() {
        let section = section(
            &["x.cpp", "a.h", "b.cpp", "a.cpp"],
            list(&[comment_at(1, "# next group"), (1, Annotation::Blank)]),
            Some(0),
        );
        assert!(section.annotations.blank_follows_comments_at(1));

        let grouped = grouped_section(&section, SourceGrouping::HeadersFirst);

        assert_eq!(grouped.args, ["x.cpp", "a.h a.cpp", "b.cpp"]);
        assert_eq!(grouped.annotations.blank_positions(), [1]);
        assert_eq!(
            comments_of(&grouped.annotations),
            [(1, "# next group".to_string())]
        );
        assert!(grouped.annotations.blank_follows_comments_at(1));
    }

    /// A comment written after the last argument has no argument to be mapped
    /// through, and grouping has just made the list shorter, so it has to be
    /// pulled back to the new end.
    #[test]
    fn grouping_pulls_a_trailing_comment_back_to_the_shortened_end() {
        let section = section(
            &["a.h", "a.cpp"],
            list(&[comment_at(2, "# after them all")]),
            Some(0),
        );

        let grouped = grouped_section(&section, SourceGrouping::HeadersFirst);

        assert_eq!(grouped.args, ["a.h a.cpp"]);
        assert_eq!(
            comments_of(&grouped.annotations),
            [(1, "# after them all".to_string())]
        );
    }

    /// The same pull-back, on the segmented path — a section with a blank line
    /// takes a different route through grouping, and the two routes each carry
    /// their own end-of-list fallback.
    #[test]
    fn grouping_pulls_a_trailing_comment_back_across_segments_too() {
        let section = section(
            &["x.cpp", "a.h", "b.cpp", "a.cpp"],
            list(&[(1, Annotation::Blank), comment_at(4, "# after them all")]),
            Some(0),
        );

        let grouped = grouped_section(&section, SourceGrouping::HeadersFirst);

        assert_eq!(grouped.args, ["x.cpp", "a.h a.cpp", "b.cpp"]);
        assert_eq!(grouped.annotations.blank_positions(), [1]);
        assert_eq!(
            comments_of(&grouped.annotations),
            [(3, "# after them all".to_string())]
        );
    }

    #[test]
    fn a_rebuilt_blank_sits_where_it_was_written() {
        let comments = [(1, "# note".to_string())];

        let before = Annotations::rebuilt_for_permuted_comments(&comments, &[1], |_| false);
        assert!(!before.blank_follows_comments_at(1));

        let after = Annotations::rebuilt_for_permuted_comments(&comments, &[1], |_| true);
        assert!(after.blank_follows_comments_at(1));

        for rebuilt in [before, after] {
            assert_eq!(rebuilt.blank_positions(), [1]);
            assert_eq!(
                rebuilt
                    .at(1)
                    .filter(|item| matches!(item, Annotation::Blank))
                    .count(),
                1
            );
        }
    }
}
