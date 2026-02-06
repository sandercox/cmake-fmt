pub mod nodes;

pub use nodes::*;

use crate::parser::ParseError;
use crate::SyntaxNode;

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
        self.root
            .children()
            .filter_map(CommandInvocation::cast)
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
