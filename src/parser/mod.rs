pub mod error;
pub mod grammar;

pub use error::ParseError;

use crate::lexer::Token;
use grammar::Parser;

/// Result of parsing
pub struct ParseResult {
    pub green: rowan::GreenNode,
    pub errors: Vec<ParseError>,
}

/// Parse a token stream into a rowan green tree
pub fn parse(tokens: Vec<Token>) -> ParseResult {
    let mut parser = Parser::new(tokens);
    parser.parse_file();
    let (green, errors) = parser.finish();
    ParseResult { green, errors }
}
