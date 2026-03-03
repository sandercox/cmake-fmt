use similar::{ChangeTag, TextDiff};

/// Represents a single change in a diff hunk
#[derive(Debug, Clone)]
pub enum Change {
    /// A line that exists in both original and formatted versions (context)
    Equal(String),
    /// A line that was removed from the original
    Delete(String),
    /// A line that was added to the formatted version
    Insert(String),
}

/// Represents a single diff hunk with context lines
#[derive(Debug, Clone)]
pub struct DiffHunk {
    /// 0-indexed line number in original where the change region begins (NOT context)
    pub old_start: usize,
    /// Number of original lines in the change region only
    pub old_count: usize,
    /// 0-indexed line number in formatted where the change region begins
    pub new_start: usize,
    /// Number of formatted lines in the change region only
    pub new_count: usize,
    /// All lines including context, for display
    pub changes: Vec<Change>,
}

/// Generate diff hunks between original and formatted text
///
/// # Arguments
/// * `original` - The original text
/// * `formatted` - The formatted text
/// * `context_lines` - Number of context lines to show around changes
///
/// # Returns
/// A list of diff hunks
pub fn generate_hunks(original: &str, formatted: &str, context_lines: usize) -> Vec<DiffHunk> {
    let diff = TextDiff::from_lines(original, formatted);
    let mut hunks = Vec::new();

    for ops in diff.grouped_ops(context_lines) {
        let mut hunk_changes = Vec::new();
        let mut old_start = None;
        let mut new_start = None;
        let mut old_count = 0;
        let mut new_count = 0;

        for op in &ops {
            for change in diff.iter_changes(op) {
                let line = change
                    .value()
                    .trim_end_matches(&['\r', '\n'][..])
                    .to_string();

                match change.tag() {
                    ChangeTag::Equal => {
                        hunk_changes.push(Change::Equal(line));
                    }
                    ChangeTag::Delete => {
                        // Track the first delete/insert as the start position
                        if old_start.is_none() {
                            old_start = change.old_index();
                        }
                        old_count += 1;
                        hunk_changes.push(Change::Delete(line));
                    }
                    ChangeTag::Insert => {
                        // Track the first delete/insert as the start position
                        if new_start.is_none() {
                            new_start = change.new_index();
                        }
                        new_count += 1;
                        hunk_changes.push(Change::Insert(line));
                    }
                }
            }
        }

        // If we have changes, create a hunk
        if !hunk_changes.is_empty() {
            hunks.push(DiffHunk {
                old_start: old_start.unwrap_or(0),
                old_count,
                new_start: new_start.unwrap_or(0),
                new_count,
                changes: hunk_changes,
            });
        }
    }

    hunks
}
