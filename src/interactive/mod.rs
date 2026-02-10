mod diff;
mod prompt;
mod apply;

pub use diff::{DiffHunk, generate_hunks};
pub use prompt::{UserChoice, display_hunk, prompt_user_choice};
pub use apply::apply_decisions;

use anyhow::{Context, Result};
use console::Term;
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;
use std::io::Write;

use crate::formatter::{format_text_with_diagnostics, FormatConfig};

/// Result of interactive formatting session
#[derive(Debug, Clone)]
pub struct InteractiveResult {
    /// Number of hunks accepted
    pub accepted: usize,
    /// Number of hunks rejected
    pub rejected: usize,
    /// Number of hunks suppressed
    pub suppressed: usize,
}

/// Run interactive formatting for a file
///
/// # Arguments
/// * `file_path` - Path to the file to format
/// * `config` - Formatting configuration
///
/// # Returns
/// Statistics about the interactive session
pub fn run_interactive(file_path: &Path, config: &FormatConfig) -> Result<InteractiveResult> {
    // Read the file
    let original = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    // Format the text
    let (formatted, _warnings) = format_text_with_diagnostics(&original, config);

    // Check if already formatted
    if original == formatted {
        let term = Term::stderr();
        term.write_line("File is already formatted.")?;
        return Ok(InteractiveResult {
            accepted: 0,
            rejected: 0,
            suppressed: 0,
        });
    }

    // Generate diff hunks
    let hunks = diff::generate_hunks(&original, &formatted, 3);

    // Interactive loop
    let term = Term::stderr();
    let mut decisions = Vec::new();
    let total_hunks = hunks.len();

    for (idx, hunk) in hunks.iter().enumerate() {
        // Display the hunk
        prompt::display_hunk(&term, hunk, idx + 1, total_hunks)?;

        // Get user choice
        let choice = loop {
            let choice = prompt::prompt_user_choice(&term)?;
            if choice == UserChoice::Help {
                // Help was already displayed in prompt_user_choice, ask again
                continue;
            }
            break choice;
        };

        // Store decision
        decisions.push((hunk.clone(), choice));

        // If user quits, treat remaining hunks as rejected
        if choice == UserChoice::Quit {
            // Add remaining hunks as rejected
            for remaining_hunk in hunks.iter().skip(idx + 1) {
                decisions.push((remaining_hunk.clone(), UserChoice::Reject));
            }
            break;
        }
    }

    // Apply decisions to produce final text
    let final_text = apply::apply_decisions(&original, &decisions);

    // Write to file atomically (same pattern as cli.rs)
    let parent = file_path.parent().unwrap_or_else(|| Path::new("."));
    let mut temp = NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to create temp file in {}", parent.display()))?;
    temp.write_all(final_text.as_bytes())
        .context("Failed to write to temp file")?;
    temp.persist(file_path)
        .with_context(|| format!("Failed to write file: {}", file_path.display()))?;

    // Calculate statistics
    let mut accepted = 0;
    let mut rejected = 0;
    let mut suppressed = 0;

    for (_hunk, choice) in &decisions {
        match choice {
            UserChoice::Accept => accepted += 1,
            UserChoice::Reject | UserChoice::Quit => rejected += 1,
            UserChoice::Suppress => suppressed += 1,
            UserChoice::Help => {}, // Should not happen
        }
    }

    // Print summary
    term.write_line("")?;
    term.write_line("Interactive review complete:")?;
    term.write_line(&format!("  {} hunk(s) accepted", accepted))?;
    term.write_line(&format!("  {} hunk(s) rejected", rejected))?;
    term.write_line(&format!("  {} hunk(s) suppressed", suppressed))?;

    // If there are suppressed hunks, show approximate line numbers
    if suppressed > 0 {
        let mut suppressed_lines = Vec::new();
        for (hunk, choice) in &decisions {
            if *choice == UserChoice::Suppress {
                suppressed_lines.push(hunk.old_start + 1); // Convert to 1-indexed
            }
        }
        term.write_line(&format!("  Suppression markers added near line(s): {}",
            suppressed_lines.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")))?;
    }

    Ok(InteractiveResult {
        accepted,
        rejected,
        suppressed,
    })
}
