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

    // Split original into lines
    let mut lines: Vec<String> = original.lines().map(String::from).collect();

    // Track cumulative line offset
    let mut offset: isize = 0;

    for (hunk, choice) in decisions {
        match choice {
            UserChoice::Accept => {
                // Replace old lines with new lines
                let start = (hunk.old_start as isize + offset) as usize;

                // Extract new lines (only Insert changes from the hunk)
                let new_lines: Vec<String> = hunk.changes.iter()
                    .filter_map(|change| {
                        if let Change::Insert(line) = change {
                            Some(line.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                // Remove old lines
                if start < lines.len() && hunk.old_count > 0 {
                    let end = (start + hunk.old_count).min(lines.len());
                    lines.drain(start..end);
                }

                // Insert new lines
                for (i, line) in new_lines.iter().enumerate() {
                    lines.insert(start + i, line.clone());
                }

                // Update offset
                offset += new_lines.len() as isize - hunk.old_count as isize;
            }
            UserChoice::Suppress => {
                // Insert suppression markers around original lines
                let start = (hunk.old_start as isize + offset) as usize;

                // Determine indentation from the line at start position
                let indent = if start < lines.len() {
                    let line = &lines[start];
                    let trimmed = line.trim_start();
                    line[..line.len() - trimmed.len()].to_string()
                } else {
                    String::new()
                };

                if hunk.old_count == 1 {
                    // Single line: use skip directive
                    lines.insert(start, format!("{}# cmake-fmt: skip", indent));
                    offset += 1;
                } else {
                    // Multiple lines: use off/on pair
                    lines.insert(start, format!("{}# cmake-fmt: off", indent));
                    offset += 1;

                    let end_pos = start + 1 + hunk.old_count;
                    if end_pos <= lines.len() {
                        lines.insert(end_pos, format!("{}# cmake-fmt: on", indent));
                        offset += 1;
                    }
                }
            }
            UserChoice::Reject | UserChoice::Quit | UserChoice::Help => {
                // Do nothing - keep original lines
            }
        }
    }

    // Join lines and ensure trailing newline
    let mut result = lines.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}
