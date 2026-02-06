use std::fmt;

/// Error encountered during parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Error message describing what went wrong
    pub message: String,
    /// Byte offset in the source where the error occurred
    pub offset: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parse error at offset {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ParseError {}
