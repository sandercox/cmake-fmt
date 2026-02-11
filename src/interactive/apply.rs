use super::diff::{Change, DiffHunk};
use super::prompt::UserChoice;

/// Apply user decisions to produce final text
///
/// # Arguments
/// * `original` - The original text
/// * `decisions` - List of (hunk, choice) pairs
///
/// # Returns
/// The final text with accepted hunks applied and suppression markers inserted
pub fn apply_decisions(original: &str, decisions: &[(DiffHunk, UserChoice)]) -> String {
    // If no decisions, return original unchanged
    if decisions.is_empty() {
        return if original.ends_with('\n') {
            original.to_string()
        } else {
            format!("{}\n", original)
        };
    }

    let orig_lines: Vec<&str> = original.lines().collect();
    let mut result: Vec<String> = Vec::new();
    let mut orig_idx: usize = 0;

    for (hunk, choice) in decisions {
        // Compute where this hunk starts in the original, including leading context.
        // old_start is the index of the first Delete, but the hunk's changes may
        // begin with Equal (context) lines before that.
        let leading_context = hunk.changes.iter()
            .take_while(|c| matches!(c, Change::Equal(_)))
            .count();
        let span_start = hunk.old_start.saturating_sub(leading_context);

        // Copy original lines before this hunk
        while orig_idx < span_start {
            result.push(orig_lines[orig_idx].to_string());
            orig_idx += 1;
        }

        match choice {
            UserChoice::Accept => {
                // Walk through changes: keep Equal + Insert, skip Delete
                for change in &hunk.changes {
                    match change {
                        Change::Equal(_) => {
                            if orig_idx < orig_lines.len() {
                                result.push(orig_lines[orig_idx].to_string());
                                orig_idx += 1;
                            }
                        }
                        Change::Delete(_) => {
                            // Skip this original line
                            orig_idx += 1;
                        }
                        Change::Insert(line) => {
                            result.push(line.clone());
                        }
                    }
                }
            }
            UserChoice::Reject | UserChoice::Quit | UserChoice::Help => {
                // Keep all original lines, skip inserts
                for change in &hunk.changes {
                    match change {
                        Change::Equal(_) | Change::Delete(_) => {
                            if orig_idx < orig_lines.len() {
                                result.push(orig_lines[orig_idx].to_string());
                                orig_idx += 1;
                            }
                        }
                        Change::Insert(_) => {
                            // Skip inserted lines
                        }
                    }
                }
            }
            UserChoice::Suppress => {
                // Keep original lines but wrap changed region with suppression markers.
                // First, emit leading context normally.
                // Then find the change region (first Delete to last Delete) and wrap it.
                let changes = &hunk.changes;

                // Find index of first and last Delete in the changes list
                let first_delete_idx = changes.iter()
                    .position(|c| matches!(c, Change::Delete(_)));
                let last_delete_idx = changes.iter()
                    .rposition(|c| matches!(c, Change::Delete(_)));

                if let (Some(first_del), Some(last_del)) = (first_delete_idx, last_delete_idx) {
                    // Emit leading context (Equal lines before first Delete)
                    for change in &changes[..first_del] {
                        if let Change::Equal(_) = change {
                            if orig_idx < orig_lines.len() {
                                result.push(orig_lines[orig_idx].to_string());
                                orig_idx += 1;
                            }
                        }
                    }

                    // Count original lines in the change region (Delete + Equal between)
                    let mut delete_orig_count = 0;
                    for change in &changes[first_del..=last_del] {
                        if matches!(change, Change::Delete(_) | Change::Equal(_)) {
                            delete_orig_count += 1;
                        }
                    }

                    // Get indentation from the first line in the change region
                    let indent = if orig_idx < orig_lines.len() {
                        let line = orig_lines[orig_idx];
                        let trimmed = line.trim_start();
                        line[..line.len() - trimmed.len()].to_string()
                    } else {
                        String::new()
                    };

                    if delete_orig_count == 1 {
                        // Single line: use skip directive
                        result.push(format!("{}# cmake-fmt: skip", indent));
                        if orig_idx < orig_lines.len() {
                            result.push(orig_lines[orig_idx].to_string());
                            orig_idx += 1;
                        }
                    } else {
                        // Multiple lines: use off/on pair, keep all original lines
                        result.push(format!("{}# cmake-fmt: off", indent));
                        for change in &changes[first_del..=last_del] {
                            if matches!(change, Change::Delete(_) | Change::Equal(_)) {
                                if orig_idx < orig_lines.len() {
                                    result.push(orig_lines[orig_idx].to_string());
                                    orig_idx += 1;
                                }
                            }
                        }
                        result.push(format!("{}# cmake-fmt: on", indent));
                    }

                    // Emit trailing context (Equal lines after last Delete)
                    for change in &changes[last_del + 1..] {
                        if let Change::Equal(_) = change {
                            if orig_idx < orig_lines.len() {
                                result.push(orig_lines[orig_idx].to_string());
                                orig_idx += 1;
                            }
                        }
                    }
                } else {
                    // No Delete changes in hunk (shouldn't happen), treat as reject
                    for change in changes {
                        if matches!(change, Change::Equal(_) | Change::Delete(_)) {
                            if orig_idx < orig_lines.len() {
                                result.push(orig_lines[orig_idx].to_string());
                                orig_idx += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    // Copy remaining original lines after the last hunk
    while orig_idx < orig_lines.len() {
        result.push(orig_lines[orig_idx].to_string());
        orig_idx += 1;
    }

    // Join lines and ensure trailing newline
    let mut text = result.join("\n");
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}
