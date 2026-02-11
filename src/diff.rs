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

        // Collect all changes so we can process runs of deletes/inserts together
        let changes: Vec<_> = hunk.iter_changes().map(|c| (c.tag(), c.value().to_string())).collect();
        let mut i = 0;

        while i < changes.len() {
            match changes[i].0 {
                ChangeTag::Equal => {
                    print!(" {}", changes[i].1);
                    if !changes[i].1.ends_with('\n') {
                        println!();
                    }
                    i += 1;
                }
                ChangeTag::Delete => {
                    // Collect consecutive deletes
                    let del_start = i;
                    while i < changes.len() && changes[i].0 == ChangeTag::Delete {
                        i += 1;
                    }
                    let deletes = &changes[del_start..i];

                    // Collect consecutive inserts that follow
                    let ins_start = i;
                    while i < changes.len() && changes[i].0 == ChangeTag::Insert {
                        i += 1;
                    }
                    let inserts = &changes[ins_start..i];

                    // Only do inline highlighting when delete/insert counts match
                    if !inserts.is_empty() && deletes.len() == inserts.len() {
                        for (del, ins) in deletes.iter().zip(inserts.iter()) {
                            print_inline_delete(&del.1, &ins.1);
                            print_inline_insert(&del.1, &ins.1);
                        }
                    } else {
                        // Unmatched counts - show plain colored lines
                        for del in deletes {
                            print_plain_line("-", &del.1, true);
                        }
                        for ins in inserts {
                            print_plain_line("+", &ins.1, false);
                        }
                    }
                }
                ChangeTag::Insert => {
                    // Orphan insert (no preceding delete) - plain green
                    print_plain_line("+", &changes[i].1, false);
                    i += 1;
                }
            }
        }
    }
}

/// Print a delete line with inline character-level highlighting
fn print_inline_delete(old: &str, new: &str) {
    let inline_diff = TextDiff::from_chars(old, new);
    print!("-");
    for change in inline_diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                print!("{}", change.value().if_supports_color(Stream::Stdout, |t| t.red()));
            }
            ChangeTag::Delete => {
                print!("{}", change.value().if_supports_color(Stream::Stdout, |t| t.bright_white().on_red()));
            }
            ChangeTag::Insert => {}
        }
    }
    if !old.ends_with('\n') {
        println!();
    }
}

/// Print an insert line with inline character-level highlighting
fn print_inline_insert(old: &str, new: &str) {
    let inline_diff = TextDiff::from_chars(old, new);
    print!("+");
    for change in inline_diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                print!("{}", change.value().if_supports_color(Stream::Stdout, |t| t.green()));
            }
            ChangeTag::Insert => {
                print!("{}", change.value().if_supports_color(Stream::Stdout, |t| t.bright_white().on_green()));
            }
            ChangeTag::Delete => {}
        }
    }
    if !new.ends_with('\n') {
        println!();
    }
}

/// Print a plain colored line (no inline highlighting)
fn print_plain_line(prefix: &str, value: &str, is_delete: bool) {
    if is_delete {
        print!("{}{}", prefix, value.if_supports_color(Stream::Stdout, |t| t.red()));
    } else {
        print!("{}{}", prefix, value.if_supports_color(Stream::Stdout, |t| t.green()));
    }
    if !value.ends_with('\n') {
        println!();
    }
}
