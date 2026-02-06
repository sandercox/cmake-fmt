use std::fmt;
use miette::Diagnostic;

/// Error encountered during parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Error message describing what went wrong
    pub message: String,
    /// Byte offset in the source where the error occurred
    pub offset: usize,
}

impl ParseError {
    /// Compute 1-based line and column number from byte offset
    ///
    /// # Arguments
    /// * `source` - The original source text
    ///
    /// # Returns
    /// Tuple of (line, column) where both are 1-based
    ///
    /// # Example
    /// ```
    /// use cmake_formatter::parser::ParseError;
    /// let source = "hello\nworld\n";
    /// let error = ParseError { message: "test".to_string(), offset: 6 };
    /// assert_eq!(error.line_col(source), (2, 1)); // "world" starts at line 2, col 1
    /// ```
    pub fn line_col(&self, source: &str) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;

        for (i, ch) in source.char_indices() {
            if i >= self.offset {
                break;
            }

            if ch == '\n' {
                line += 1;
                col = 1;
            } else if ch == '\r' {
                // Handle CRLF: \r\n counts as one newline
                // Check if next char is \n
                if source.as_bytes().get(i + 1) == Some(&b'\n') {
                    // Skip the \r, let the \n increment line on next iteration
                } else {
                    // Standalone \r (rare, but treat as newline)
                    line += 1;
                    col = 1;
                }
            } else {
                col += 1;
            }
        }

        (line, col)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error: {}", self.message)
    }
}

impl std::error::Error for ParseError {}

impl Diagnostic for ParseError {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new("parse_error"))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        None
    }
}
