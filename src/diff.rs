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

                    if inserts.is_empty() {
                        // Pure deletes, no inserts to compare against
                        for del in deletes {
                            print_plain_line("-", &del.1, true);
                        }
                    } else {
                        // Join all deletes and inserts, do char-level diff on joined text
                        let del_text: String = deletes.iter().map(|d| d.1.as_str()).collect();
                        let ins_text: String = inserts.iter().map(|i| i.1.as_str()).collect();
                        print_inline_side(&del_text, &ins_text, true);
                        print_inline_side(&del_text, &ins_text, false);
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

/// Print one side (delete or insert) of an inline-highlighted diff block.
///
/// Joins all delete/insert lines, does char-level diff on joined text,
/// and renders with proper `-`/`+` prefixes at newline boundaries.
fn print_inline_side(del_text: &str, ins_text: &str, is_delete: bool) {
    let inline_diff = TextDiff::from_chars(del_text, ins_text);
    let prefix = if is_delete { "-" } else { "+" };
    let mut need_prefix = true;

    for change in inline_diff.iter_all_changes() {
        let tag = change.tag();

        // For delete side: render Equal + Delete, skip Insert
        // For insert side: render Equal + Insert, skip Delete
        let dominated = if is_delete { ChangeTag::Insert } else { ChangeTag::Delete };
        if tag == dominated {
            continue;
        }

        let emphasized = tag != ChangeTag::Equal;

        for ch in change.value().chars() {
            if need_prefix {
                print!("{}", prefix);
                need_prefix = false;
            }
            if ch == '\n' {
                println!();
                need_prefix = true;
            } else {
                // Replace tabs with spaces in emphasized portions - terminals don't
                // render background color for tab characters
                let s = if ch == '\t' && emphasized { "    ".to_string() } else { ch.to_string() };
                if emphasized {
                    if is_delete {
                        print!("{}", s.if_supports_color(Stream::Stdout, |t| t.bright_white().on_red()));
                    } else {
                        print!("{}", s.if_supports_color(Stream::Stdout, |t| t.bright_white().on_green()));
                    }
                } else if is_delete {
                    print!("{}", s.if_supports_color(Stream::Stdout, |t| t.red()));
                } else {
                    print!("{}", s.if_supports_color(Stream::Stdout, |t| t.green()));
                }
            }
        }
    }
    // If we didn't end on a newline, add one
    if !need_prefix {
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
