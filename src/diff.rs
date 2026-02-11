use owo_colors::{OwoColorize, Stream};
use similar::{ChangeTag, TextDiff};

/// Generate a unified diff between original and formatted text.
/// Returns None if the texts are identical, Some(diff_string) otherwise.
pub fn generate_diff(original: &str, formatted: &str, path: &str) -> Option<String> {
    if original == formatted {
        return None;
    }

    // Convert Windows backslashes to forward slashes for cross-platform consistency
    let normalized_path = path.replace('\\', "/");

    let diff = TextDiff::from_lines(original, formatted);
    let diff_output = diff
        .unified_diff()
        .context_radius(3)
        .header(&format!("a/{}", normalized_path), &format!("b/{}", normalized_path))
        .missing_newline_hint(true)
        .to_string();

    Some(diff_output)
}

/// Print a unified diff with color highlighting and inline change emphasis.
/// Changed portions within lines are highlighted with background colors.
///
/// # Arguments
/// * `original` - The original text
/// * `formatted` - The formatted text
/// * `path` - The file path (used in diff headers)
pub fn print_colored_diff(original: &str, formatted: &str, path: &str) {
    if original == formatted {
        return;
    }

    // Convert Windows backslashes to forward slashes for cross-platform consistency
    let normalized_path = path.replace('\\', "/");

    let diff = TextDiff::from_lines(original, formatted);

    // Print file headers
    println!("{}", format!("--- a/{}", normalized_path).if_supports_color(Stream::Stdout, |t| t.dimmed()));
    println!("{}", format!("+++ b/{}", normalized_path).if_supports_color(Stream::Stdout, |t| t.dimmed()));

    // Process each hunk
    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        // Print hunk header
        println!("{}", format!("{}", hunk.header()).if_supports_color(Stream::Stdout, |t| t.cyan()));

        // Process each change using inline diff
        for (idx, change) in hunk.iter_changes().enumerate() {
            let tag = change.tag();
            let value = change.value();

            match tag {
                ChangeTag::Equal => {
                    // Context line - no color
                    print!(" {}", value);
                    if !value.ends_with('\n') {
                        println!();
                    }
                }
                ChangeTag::Delete => {
                    // Check if there's a corresponding insert for inline diff
                    let has_inline = hunk.iter_changes().skip(idx + 1).next()
                        .map(|next| next.tag() == ChangeTag::Insert)
                        .unwrap_or(false);

                    if has_inline {
                        // Get the corresponding insert line
                        let insert_value = hunk.iter_changes().skip(idx + 1).next().unwrap().value();

                        // Perform character-level diff for inline highlighting
                        let inline_diff = TextDiff::from_chars(value, insert_value);

                        print!("-");
                        for inline_change in inline_diff.iter_all_changes() {
                            let inline_tag = inline_change.tag();
                            let inline_value = inline_change.value();

                            match inline_tag {
                                ChangeTag::Equal => {
                                    // Unchanged portion - red foreground only
                                    print!("{}", inline_value.if_supports_color(Stream::Stdout, |t| t.red()));
                                }
                                ChangeTag::Delete => {
                                    // Changed portion - red foreground + dark red background
                                    print!("{}", inline_value.if_supports_color(Stream::Stdout, |t| t.red().on_red()));
                                }
                                ChangeTag::Insert => {
                                    // This shouldn't appear in the delete line
                                }
                            }
                        }
                        if !value.ends_with('\n') {
                            println!();
                        }
                    } else {
                        // No corresponding insert - just red foreground
                        print!("-{}", value.if_supports_color(Stream::Stdout, |t| t.red()));
                        if !value.ends_with('\n') {
                            println!();
                        }
                    }
                }
                ChangeTag::Insert => {
                    // Check if the previous change was a delete (for inline diff)
                    let prev_is_delete = if idx > 0 {
                        hunk.iter_changes().nth(idx - 1)
                            .map(|prev| prev.tag() == ChangeTag::Delete)
                            .unwrap_or(false)
                    } else {
                        false
                    };

                    if prev_is_delete {
                        // Get the corresponding delete line
                        let delete_value = hunk.iter_changes().nth(idx - 1).unwrap().value();

                        // Perform character-level diff for inline highlighting
                        let inline_diff = TextDiff::from_chars(delete_value, value);

                        print!("+");
                        for inline_change in inline_diff.iter_all_changes() {
                            let inline_tag = inline_change.tag();
                            let inline_value = inline_change.value();

                            match inline_tag {
                                ChangeTag::Equal => {
                                    // Unchanged portion - green foreground only
                                    print!("{}", inline_value.if_supports_color(Stream::Stdout, |t| t.green()));
                                }
                                ChangeTag::Insert => {
                                    // Changed portion - green foreground + dark green background
                                    print!("{}", inline_value.if_supports_color(Stream::Stdout, |t| t.green().on_green()));
                                }
                                ChangeTag::Delete => {
                                    // This shouldn't appear in the insert line
                                }
                            }
                        }
                        if !value.ends_with('\n') {
                            println!();
                        }
                    } else {
                        // No corresponding delete - just green foreground
                        print!("+{}", value.if_supports_color(Stream::Stdout, |t| t.green()));
                        if !value.ends_with('\n') {
                            println!();
                        }
                    }
                }
            }
        }
    }
}
