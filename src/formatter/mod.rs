pub mod config;
mod cst_to_doc;

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

    // Ensure file ends with single newline
    if result.is_empty() {
        String::new()
    } else if !result.ends_with('\n') {
        format!("{}\n", result)
    } else {
        result
    }
}
