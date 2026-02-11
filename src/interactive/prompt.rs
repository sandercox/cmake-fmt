use super::diff::{Change, DiffHunk};
use anyhow::Result;
use console::{Key, Term};
use owo_colors::{OwoColorize, Stream};
use similar::{ChangeTag, TextDiff};

/// User's choice for handling a diff hunk
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserChoice {
    /// Accept the hunk (apply the formatted version)
    Accept,
    /// Reject the hunk (keep the original)
    Reject,
    /// Suppress the hunk (keep original + add cmake-fmt off/on markers)
    Suppress,
    /// Quit interactive mode (reject remaining hunks)
    Quit,
    /// Show help
    Help,
}

/// Format a line pair with inline character-level highlighting
///
/// Returns (formatted_old, formatted_new) with ANSI color codes for inline emphasis
fn format_inline_pair(old_line: &str, new_line: &str) -> (String, String) {
    let inline_diff = TextDiff::from_chars(old_line, new_line);

    let mut formatted_old = String::new();
    let mut formatted_new = String::new();

    for change in inline_diff.iter_all_changes() {
        let tag = change.tag();
        let value = change.value();

        match tag {
            ChangeTag::Equal => {
                // Unchanged portions - red foreground only for old, green for new
                formatted_old.push_str(&value.to_string().if_supports_color(Stream::Stderr, |t| t.red()).to_string());
                formatted_new.push_str(&value.to_string().if_supports_color(Stream::Stderr, |t| t.green()).to_string());
            }
            ChangeTag::Delete => {
                // Changed portion in old line - white on red background
                formatted_old.push_str(&value.to_string().if_supports_color(Stream::Stderr, |t| t.bright_white().on_red()).to_string());
            }
            ChangeTag::Insert => {
                // Changed portion in new line - white on green background
                formatted_new.push_str(&value.to_string().if_supports_color(Stream::Stderr, |t| t.bright_white().on_green()).to_string());
            }
        }
    }

    (formatted_old, formatted_new)
}

/// Display a diff hunk to the terminal with inline change highlighting
///
/// # Arguments
/// * `term` - The terminal to write to
/// * `hunk` - The diff hunk to display
/// * `hunk_num` - Current hunk number (1-indexed)
/// * `total` - Total number of hunks
pub fn display_hunk(term: &Term, hunk: &DiffHunk, hunk_num: usize, total: usize) -> Result<()> {
    // Print header
    let header = format!("@@ Hunk {}/{} @@", hunk_num, total);
    term.write_line(&header.if_supports_color(Stream::Stderr, |text| text.cyan()).to_string())?;

    // Process changes to detect adjacent delete/insert pairs for inline highlighting
    let mut i = 0;
    while i < hunk.changes.len() {
        match &hunk.changes[i] {
            Change::Delete(old_line) => {
                // Check if the next change is an insert (for inline highlighting)
                if i + 1 < hunk.changes.len() {
                    if let Change::Insert(new_line) = &hunk.changes[i + 1] {
                        // Adjacent delete+insert pair - use inline highlighting
                        let (formatted_old, formatted_new) = format_inline_pair(old_line, new_line);
                        term.write_line(&format!("-{}", formatted_old))?;
                        term.write_line(&format!("+{}", formatted_new))?;
                        i += 2; // Skip both changes
                        continue;
                    }
                }

                // No corresponding insert - just show red foreground
                let text = format!("-{}", old_line);
                term.write_line(&text.if_supports_color(Stream::Stderr, |t| t.red()).to_string())?;
                i += 1;
            }
            Change::Insert(new_line) => {
                // Insert without preceding delete - just show green foreground
                let text = format!("+{}", new_line);
                term.write_line(&text.if_supports_color(Stream::Stderr, |t| t.green()).to_string())?;
                i += 1;
            }
            Change::Equal(line) => {
                // Context line - no color
                term.write_line(&format!(" {}", line))?;
                i += 1;
            }
        }
    }

    Ok(())
}

/// Display help message
pub fn display_help(term: &Term) -> Result<()> {
    term.write_line("")?;
    term.write_line("Interactive mode commands:")?;
    term.write_line("  y - Accept this hunk (apply formatting)")?;
    term.write_line("  n - Reject this hunk (keep original)")?;
    term.write_line("  s - Suppress this hunk (keep original + add suppression markers)")?;
    term.write_line("  q - Quit (reject all remaining hunks)")?;
    term.write_line("  ? - Show this help")?;
    term.write_line("")?;
    Ok(())
}

/// Prompt the user for their choice
///
/// # Arguments
/// * `term` - The terminal to read from
///
/// # Returns
/// The user's choice
pub fn prompt_user_choice(term: &Term) -> Result<UserChoice> {
    loop {
        term.write_str("Apply this hunk? [y,n,s,q,?] ")?;
        term.flush()?;

        let key = term.read_key()?;
        term.write_line("")?; // Move to next line after input

        match key {
            Key::Char('y') | Key::Char('Y') => return Ok(UserChoice::Accept),
            Key::Char('n') | Key::Char('N') => return Ok(UserChoice::Reject),
            Key::Char('s') | Key::Char('S') => return Ok(UserChoice::Suppress),
            Key::Char('q') | Key::Char('Q') => return Ok(UserChoice::Quit),
            Key::Char('?') => {
                display_help(term)?;
                return Ok(UserChoice::Help);
            }
            _ => {
                term.write_line("Invalid choice. Press ? for help.")?;
                continue;
            }
        }
    }
}
