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

/// Format one side (delete or insert) of an inline-highlighted diff block as lines.
///
/// Joins all text, does char-level diff, and returns Vec of formatted lines with prefixes.
/// Tab width for expanding tabs in diff output (terminal/POSIX standard)
const TAB_WIDTH: usize = 8;

fn format_inline_side(del_text: &str, ins_text: &str, is_delete: bool) -> Vec<String> {
    let inline_diff = TextDiff::from_chars(del_text, ins_text);
    let prefix = if is_delete { "-" } else { "+" };
    let mut lines = Vec::new();
    let mut current_line = String::from(prefix);
    let mut buf = String::new();
    let mut buf_emphasized = false;
    let mut col: usize = 1; // prefix takes 1 column

    for change in inline_diff.iter_all_changes() {
        let tag = change.tag();
        let dominated = if is_delete { ChangeTag::Insert } else { ChangeTag::Delete };
        if tag == dominated {
            continue;
        }

        let emphasized = tag != ChangeTag::Equal;

        for ch in change.value().chars() {
            if ch == '\n' {
                flush_inline_buf_to_string(&mut current_line, &buf, buf_emphasized, is_delete);
                buf.clear();
                lines.push(current_line);
                current_line = String::from(prefix);
                col = 1;
            } else {
                // Flush if style changed
                if !buf.is_empty() && buf_emphasized != emphasized {
                    flush_inline_buf_to_string(&mut current_line, &buf, buf_emphasized, is_delete);
                    buf.clear();
                }
                buf_emphasized = emphasized;
                if ch == '\t' {
                    let spaces = TAB_WIDTH - (col % TAB_WIDTH);
                    for _ in 0..spaces {
                        buf.push(' ');
                    }
                    col += spaces;
                } else {
                    buf.push(ch);
                    col += 1;
                }
            }
        }
    }
    flush_inline_buf_to_string(&mut current_line, &buf, buf_emphasized, is_delete);
    if current_line != prefix {
        lines.push(current_line);
    }
    lines
}

/// Flush a buffered inline segment with the appropriate ANSI color into a string
fn flush_inline_buf_to_string(out: &mut String, buf: &str, emphasized: bool, is_delete: bool) {
    if buf.is_empty() {
        return;
    }
    if emphasized {
        if is_delete {
            out.push_str(&buf.if_supports_color(Stream::Stderr, |t| t.bright_white().on_red()).to_string());
        } else {
            out.push_str(&buf.if_supports_color(Stream::Stderr, |t| t.bright_white().on_green()).to_string());
        }
    } else if is_delete {
        out.push_str(&buf.if_supports_color(Stream::Stderr, |t| t.red()).to_string());
    } else {
        out.push_str(&buf.if_supports_color(Stream::Stderr, |t| t.green()).to_string());
    }
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

    // Process changes, collecting delete/insert runs for proper positional pairing
    let mut i = 0;
    while i < hunk.changes.len() {
        match &hunk.changes[i] {
            Change::Delete(_) => {
                // Collect consecutive deletes
                let del_start = i;
                while i < hunk.changes.len() && matches!(&hunk.changes[i], Change::Delete(_)) {
                    i += 1;
                }
                let del_end = i;

                // Collect consecutive inserts that follow
                let ins_start = i;
                while i < hunk.changes.len() && matches!(&hunk.changes[i], Change::Insert(_)) {
                    i += 1;
                }
                let ins_end = i;

                let ins_count = ins_end - ins_start;

                if ins_count == 0 {
                    // Pure deletes, no inserts to compare against
                    for j in del_start..del_end {
                        if let Change::Delete(line) = &hunk.changes[j] {
                            let text = format!("-{}", expand_tabs_interactive(line, 1));
                            term.write_line(&text.if_supports_color(Stream::Stderr, |t| t.red()).to_string())?;
                        }
                    }
                } else {
                    // Join all deletes and inserts, char-level diff on joined text
                    let del_text: String = (del_start..del_end)
                        .filter_map(|j| if let Change::Delete(l) = &hunk.changes[j] { Some(format!("{}\n", l)) } else { None })
                        .collect();
                    let ins_text: String = (ins_start..ins_end)
                        .filter_map(|j| if let Change::Insert(l) = &hunk.changes[j] { Some(format!("{}\n", l)) } else { None })
                        .collect();

                    for line in format_inline_side(&del_text, &ins_text, true) {
                        term.write_line(&line)?;
                    }
                    for line in format_inline_side(&del_text, &ins_text, false) {
                        term.write_line(&line)?;
                    }
                }
            }
            Change::Insert(new_line) => {
                let text = format!("+{}", expand_tabs_interactive(new_line, 1));
                term.write_line(&text.if_supports_color(Stream::Stderr, |t| t.green()).to_string())?;
                i += 1;
            }
            Change::Equal(line) => {
                term.write_line(&format!(" {}", expand_tabs_interactive(line, 1)))?;
                i += 1;
            }
        }
    }

    Ok(())
}

/// Expand tab characters to spaces based on column position
fn expand_tabs_interactive(s: &str, start_col: usize) -> String {
    let mut result = String::with_capacity(s.len());
    let mut col = start_col;
    for ch in s.chars() {
        if ch == '\t' {
            let spaces = TAB_WIDTH - (col % TAB_WIDTH);
            for _ in 0..spaces {
                result.push(' ');
            }
            col += spaces;
        } else {
            result.push(ch);
            col += 1;
        }
    }
    result
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
