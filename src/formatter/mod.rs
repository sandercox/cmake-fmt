pub mod config;
mod cst_to_doc;
mod cmake_rules;
mod comments;

pub use config::{CommandCase, FormatConfig};

use crate::cst::parse_text;
use pretty::RcDoc;

/// Format CMake code with the given configuration
///
/// # Arguments
/// * `input` - The CMake source code to format
/// * `config` - Formatting configuration
///
/// # Returns
/// Formatted CMake code as a String, guaranteed to end with a single newline
pub fn format_text(input: &str, config: &FormatConfig) -> String {
    let cst = parse_text(input);
    let doc = cst_to_doc::format_cst(&cst, config);
    render_doc(doc, config)
}

/// Render a Doc to a String
fn render_doc(doc: RcDoc<'static, ()>, config: &FormatConfig) -> String {
    let mut output = Vec::new();
    doc.render(config.max_line_length, &mut output)
        .expect("rendering to Vec should not fail");
    let result = String::from_utf8(output)
        .expect("formatted output should be valid UTF-8");

    // Strip trailing whitespace from each line (the pretty crate can produce
    // indentation on otherwise-blank lines when nest() wraps line() breaks)
    let result: String = result
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");

    // Trim the result and ensure proper ending
    let trimmed = result.trim();

    // If nothing meaningful, return empty
    if trimmed.is_empty() {
        String::new()
    } else if !trimmed.ends_with('\n') {
        format!("{}\n", trimmed)
    } else {
        trimmed.to_string()
    }
}
