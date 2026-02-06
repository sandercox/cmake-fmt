mod cursor;

use cursor::Cursor;
use crate::SyntaxKind;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: SyntaxKind,
    pub text: String,
    pub span: Range<usize>,
}

pub fn lex(input: &str) -> Vec<Token> {
    Lexer::new(input).tokenize()
}

struct Lexer<'a> {
    cursor: Cursor<'a>,
    /// Track whether we're expecting a command name (after newline/start) or in arguments (after LPAREN)
    expecting_command: bool,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            cursor: Cursor::new(input),
            expecting_command: true, // Start of file expects command
        }
    }

    fn tokenize(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        while !self.cursor.at_end() {
            if let Some(token) = self.next_token() {
                tokens.push(token);
            }
        }

        tokens
    }

    fn next_token(&mut self) -> Option<Token> {
        let start = self.cursor.pos();
        let ch = self.cursor.peek()?;

        let kind = match ch {
            // Newlines (separate from whitespace for line tracking)
            '\n' => {
                self.cursor.advance();
                self.expecting_command = true; // After newline, expect command
                SyntaxKind::NEWLINE
            }
            '\r' if self.cursor.peek_nth(1) == Some('\n') => {
                self.cursor.advance(); // \r
                self.cursor.advance(); // \n
                self.expecting_command = true;
                SyntaxKind::NEWLINE
            }

            // Whitespace (spaces and tabs, NOT newlines)
            ' ' | '\t' => {
                self.cursor.advance();
                while matches!(self.cursor.peek(), Some(' ' | '\t')) {
                    self.cursor.advance();
                }
                SyntaxKind::WHITESPACE
            }

            // Comments and bracket comments
            '#' => {
                self.cursor.advance();
                if self.cursor.peek() == Some('[') {
                    // Might be bracket comment
                    if let Some(kind) = self.try_lex_bracket_comment(start) {
                        kind
                    } else {
                        // Not a bracket comment, it's a line comment starting with #[
                        self.lex_line_comment()
                    }
                } else {
                    // Line comment
                    self.lex_line_comment()
                }
            }

            // Parentheses
            '(' => {
                self.cursor.advance();
                self.expecting_command = false; // Inside arguments now
                SyntaxKind::LPAREN
            }
            ')' => {
                self.cursor.advance();
                self.expecting_command = true; // After closing paren, expect command
                SyntaxKind::RPAREN
            }

            // Quoted arguments
            '"' => self.lex_quoted_argument(),

            // Bracket arguments or just regular text with '['
            '[' => {
                if let Some(kind) = self.try_lex_bracket_argument(start) {
                    kind
                } else {
                    // Not a bracket argument, treat as unquoted argument character
                    self.lex_unquoted_argument_or_command()
                }
            }

            // Variable references and generator expressions
            '$' => self.lex_dollar_construct(),

            // Everything else: command name or unquoted argument
            _ => self.lex_unquoted_argument_or_command(),
        };

        let text = self.cursor.text_from(start).to_string();
        let span = self.cursor.span_from(start);

        Some(Token { kind, text, span })
    }

    fn lex_line_comment(&mut self) -> SyntaxKind {
        // Consume until newline (but don't consume the newline itself)
        while let Some(ch) = self.cursor.peek() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            self.cursor.advance();
        }
        SyntaxKind::COMMENT
    }

    fn try_lex_bracket_comment(&mut self, _start: usize) -> Option<SyntaxKind> {
        // We've already consumed '#', now check for bracket pattern: [=*[
        let after_hash = self.cursor.pos();

        if self.cursor.peek() != Some('[') {
            self.cursor.pos = after_hash;
            return None;
        }
        self.cursor.advance(); // consume '['

        // Count equals signs
        let mut equals_count = 0u8;
        while self.cursor.peek() == Some('=') {
            equals_count = equals_count.saturating_add(1);
            self.cursor.advance();
        }

        // Must have opening '['
        if self.cursor.peek() != Some('[') {
            // Not a bracket comment pattern, backtrack
            self.cursor.pos = after_hash;
            return None;
        }
        self.cursor.advance(); // consume second '['

        // Now find matching closing bracket: ]=*] with same number of equals
        while !self.cursor.at_end() {
            if self.cursor.peek() == Some(']') {
                let before_close = self.cursor.pos();
                self.cursor.advance(); // consume ']'

                // Count equals
                let mut close_equals = 0u8;
                while self.cursor.peek() == Some('=') && close_equals < equals_count {
                    close_equals += 1;
                    self.cursor.advance();
                }

                // Check if we have the final ']'
                if close_equals == equals_count && self.cursor.peek() == Some(']') {
                    self.cursor.advance(); // consume final ']'
                    return Some(SyntaxKind::BRACKET_COMMENT);
                } else {
                    // Not a match, continue searching from after the first ']'
                    self.cursor.pos = before_close + 1;
                }
            } else {
                self.cursor.advance();
            }
        }

        // No closing delimiter found, consume to end (error recovery)
        Some(SyntaxKind::BRACKET_COMMENT)
    }

    fn try_lex_bracket_argument(&mut self, _start: usize) -> Option<SyntaxKind> {
        // Check for bracket pattern: [=*[
        let before_bracket = self.cursor.pos();

        if self.cursor.peek() != Some('[') {
            return None;
        }
        self.cursor.advance(); // consume '['

        // Count equals signs
        let mut equals_count = 0u8;
        while self.cursor.peek() == Some('=') {
            equals_count = equals_count.saturating_add(1);
            self.cursor.advance();
        }

        // Must have opening '['
        if self.cursor.peek() != Some('[') {
            // Not a bracket argument pattern, backtrack
            self.cursor.pos = before_bracket;
            return None;
        }
        self.cursor.advance(); // consume second '['

        // Now find matching closing bracket: ]=*] with same number of equals
        while !self.cursor.at_end() {
            if self.cursor.peek() == Some(']') {
                let before_close = self.cursor.pos();
                self.cursor.advance(); // consume ']'

                // Count equals
                let mut close_equals = 0u8;
                while self.cursor.peek() == Some('=') && close_equals < equals_count {
                    close_equals += 1;
                    self.cursor.advance();
                }

                // Check if we have the final ']'
                if close_equals == equals_count && self.cursor.peek() == Some(']') {
                    self.cursor.advance(); // consume final ']'
                    return Some(SyntaxKind::BRACKET_ARGUMENT);
                } else {
                    // Not a match, continue searching from after the first ']'
                    self.cursor.pos = before_close + 1;
                }
            } else {
                self.cursor.advance();
            }
        }

        // No closing delimiter found, consume to end (error recovery)
        Some(SyntaxKind::BRACKET_ARGUMENT)
    }

    fn lex_quoted_argument(&mut self) -> SyntaxKind {
        // Consume opening quote
        self.cursor.advance();

        while let Some(ch) = self.cursor.peek() {
            match ch {
                '"' => {
                    // Closing quote
                    self.cursor.advance();
                    return SyntaxKind::QUOTED_ARGUMENT;
                }
                '\\' => {
                    // Escape sequence - consume backslash and next character
                    self.cursor.advance();
                    self.cursor.advance();
                }
                _ => {
                    self.cursor.advance();
                }
            }
        }

        // No closing quote found (error recovery)
        SyntaxKind::QUOTED_ARGUMENT
    }

    fn lex_dollar_construct(&mut self) -> SyntaxKind {
        // We're at '$', check what follows
        self.cursor.advance(); // consume '$'

        match self.cursor.peek() {
            Some('<') => {
                // Generator expression: $<...>
                self.lex_generator_expression()
            }
            Some('{') => {
                // Variable reference: ${...}
                self.lex_variable_reference(SyntaxKind::VARIABLE_REF)
            }
            Some('E') if self.cursor.starts_with("ENV{") => {
                // Environment variable: $ENV{...}
                self.cursor.advance_by(3); // "ENV"
                self.lex_variable_reference(SyntaxKind::ENV_VAR_REF)
            }
            Some('C') if self.cursor.starts_with("CACHE{") => {
                // Cache variable: $CACHE{...}
                self.cursor.advance_by(5); // "CACHE"
                self.lex_variable_reference(SyntaxKind::CACHE_VAR_REF)
            }
            _ => {
                // Just a dollar sign as part of unquoted argument
                // Continue lexing as unquoted argument
                self.lex_unquoted_argument_or_command()
            }
        }
    }

    fn lex_generator_expression(&mut self) -> SyntaxKind {
        // We've consumed "$<", now find balanced ">"
        self.cursor.advance(); // consume '<'

        let mut depth = 1;
        while !self.cursor.at_end() && depth > 0 {
            match self.cursor.peek() {
                Some('$') if self.cursor.peek_nth(1) == Some('<') => {
                    // Nested generator expression
                    self.cursor.advance(); // $
                    self.cursor.advance(); // <
                    depth += 1;
                }
                Some('>') => {
                    self.cursor.advance();
                    depth -= 1;
                }
                Some('\\') => {
                    // Escape sequence
                    self.cursor.advance();
                    self.cursor.advance();
                }
                _ => {
                    self.cursor.advance();
                }
            }
        }

        SyntaxKind::GENERATOR_EXPR
    }

    fn lex_variable_reference(&mut self, kind: SyntaxKind) -> SyntaxKind {
        // We've consumed "${" or "ENV{" or "CACHE{", now find balanced "}"
        self.cursor.advance(); // consume '{'

        let mut depth = 1;
        while !self.cursor.at_end() && depth > 0 {
            match self.cursor.peek() {
                Some('$') if self.cursor.peek_nth(1) == Some('{') => {
                    // Nested variable reference
                    self.cursor.advance(); // $
                    self.cursor.advance(); // {
                    depth += 1;
                }
                Some('{') => {
                    self.cursor.advance();
                    depth += 1;
                }
                Some('}') => {
                    self.cursor.advance();
                    depth -= 1;
                }
                Some('\\') => {
                    // Escape sequence
                    self.cursor.advance();
                    self.cursor.advance();
                }
                _ => {
                    self.cursor.advance();
                }
            }
        }

        kind
    }

    fn lex_unquoted_argument_or_command(&mut self) -> SyntaxKind {
        // Consume characters that are not whitespace, parens, quotes, or special characters
        while let Some(ch) = self.cursor.peek() {
            match ch {
                ' ' | '\t' | '\n' | '\r' | '(' | ')' | '"' | '#' => break,
                _ => {
                    self.cursor.advance();
                }
            }
        }

        if self.expecting_command {
            SyntaxKind::COMMAND_NAME
        } else {
            SyntaxKind::UNQUOTED_ARGUMENT
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_simple_command() {
        let tokens = lex("message(hello)");
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].kind, SyntaxKind::COMMAND_NAME);
        assert_eq!(tokens[0].text, "message");
        assert_eq!(tokens[1].kind, SyntaxKind::LPAREN);
        assert_eq!(tokens[2].kind, SyntaxKind::UNQUOTED_ARGUMENT);
        assert_eq!(tokens[2].text, "hello");
        assert_eq!(tokens[3].kind, SyntaxKind::RPAREN);
    }

    #[test]
    fn test_roundtrip() {
        let input = "message(STATUS \"hello world\")\n";
        let tokens = lex(input);
        let reconstructed: String = tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(reconstructed, input);
    }
}
