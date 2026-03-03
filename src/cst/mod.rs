pub mod nodes;

pub use nodes::*;

use crate::SyntaxNode;
use crate::parser::ParseError;

/// Root of the Concrete Syntax Tree
pub struct CSTRoot {
    pub root: SyntaxNode,
    pub errors: Vec<ParseError>,
}

impl CSTRoot {
    /// Get the full text of the CST (round-trip reconstruction)
    pub fn text(&self) -> String {
        self.root.text().to_string()
    }

    /// Check if parsing produced any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Iterate over all top-level commands (convenience wrapper)
    pub fn commands(&self) -> impl Iterator<Item = CommandInvocation> + '_ {
        self.root.children().filter_map(CommandInvocation::cast)
    }

    /// Format all errors as human-readable strings with line:column info
    ///
    /// # Arguments
    /// * `source` - The original source text
    ///
    /// # Returns
    /// Vec of error strings in format "line:col: error: message"
    pub fn format_errors(&self, source: &str) -> Vec<String> {
        self.errors
            .iter()
            .map(|e| {
                let (line, col) = e.line_col(source);
                format!("{}:{}: error: {}", line, col, e.message)
            })
            .collect()
    }

    /// Convert errors into miette Reports for rich display
    ///
    /// Returns None if there are no errors
    pub fn into_report(self, source: impl Into<String>) -> Option<miette::Report> {
        if self.errors.is_empty() {
            return None;
        }

        let source_text: String = source.into();

        // Create a multi-error report
        let mut report: Option<miette::Report> = None;

        for error in self.errors {
            let span = miette::SourceSpan::from(error.offset..error.offset.saturating_add(1));
            let err = miette::miette! {
                labels = vec![
                    miette::LabeledSpan::at(span, "here"),
                ],
                help = "Check the CMake syntax at this location",
                "{}",
                error.message
            }
            .with_source_code(source_text.clone());

            report = Some(match report {
                None => err,
                Some(existing) => {
                    // Chain errors together
                    existing.wrap_err(err)
                }
            });
        }

        report
    }
}

/// Convenience function: lex + parse in one call
pub fn parse_text(input: &str) -> CSTRoot {
    let tokens = crate::lexer::lex(input);
    let result = crate::parser::parse(tokens);
    let root = SyntaxNode::new_root(result.green);
    CSTRoot {
        root,
        errors: result.errors,
    }
}
