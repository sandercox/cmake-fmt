use super::diff::{Change, DiffHunk};
use anyhow::Result;
use console::{Key, Term};
use owo_colors::{OwoColorize, Stream};

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

/// Display a diff hunk to the terminal
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

    // Print each change
    for change in &hunk.changes {
        match change {
            Change::Delete(line) => {
                let text = format!("-{}", line);
                term.write_line(&text.if_supports_color(Stream::Stderr, |t| t.red()).to_string())?;
            }
            Change::Insert(line) => {
                let text = format!("+{}", line);
                term.write_line(&text.if_supports_color(Stream::Stderr, |t| t.green()).to_string())?;
            }
            Change::Equal(line) => {
                term.write_line(&format!(" {}", line))?;
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
