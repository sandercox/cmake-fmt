use owo_colors::{OwoColorize, Stream};
use similar::TextDiff;

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

/// Print a unified diff with color highlighting.
/// Lines are colored based on their prefix:
/// - "---" and "+++": dimmed
/// - "@@": cyan
/// - "-": red (except "---")
/// - "+": green (except "+++")
pub fn print_colored_diff(diff_text: &str) {
    for line in diff_text.lines() {
        if line.starts_with("---") || line.starts_with("+++") {
            println!("{}", line.if_supports_color(Stream::Stdout, |t| t.dimmed()));
        } else if line.starts_with("@@") {
            println!("{}", line.if_supports_color(Stream::Stdout, |t| t.cyan()));
        } else if line.starts_with('-') {
            println!("{}", line.if_supports_color(Stream::Stdout, |t| t.red()));
        } else if line.starts_with('+') {
            println!("{}", line.if_supports_color(Stream::Stdout, |t| t.green()));
        } else {
            println!("{}", line);
        }
    }
}
