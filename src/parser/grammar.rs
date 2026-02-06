use crate::lexer::Token;
use crate::syntax_kind::{SyntaxKind, CMakeLang};
use crate::parser::error::ParseError;
use rowan::{GreenNodeBuilder, Language};

/// Recursive descent parser that builds a green tree from tokens
pub(crate) struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<ParseError>,
    /// Running byte offset for error reporting
    byte_offset: usize,
}

impl Parser {
    pub(crate) fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
            byte_offset: 0,
        }
    }

    /// Check if current token is of given kind
    fn at(&self, kind: SyntaxKind) -> bool {
        self.current_kind() == kind
    }

    /// Get current token kind (EOF if past end)
    fn current_kind(&self) -> SyntaxKind {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].kind
        } else {
            SyntaxKind::EOF
        }
    }

    /// Check if at end of token stream
    fn eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// Consume current token and add it to the green tree
    fn bump(&mut self) {
        if self.pos < self.tokens.len() {
            let token = &self.tokens[self.pos];
            self.builder.token(CMakeLang::kind_to_raw(token.kind), &token.text);
            self.byte_offset += token.text.len();
            self.pos += 1;
        }
    }

    /// Expect a specific token kind, emit error if not found
    fn expect(&mut self, kind: SyntaxKind) {
        if self.at(kind) {
            self.bump();
        } else {
            let message = format!(
                "Expected {:?}, found {:?}",
                kind,
                self.current_kind()
            );
            self.errors.push(ParseError {
                message,
                offset: self.byte_offset,
            });
        }
    }

    /// Skip trivia tokens (WHITESPACE, NEWLINE, COMMENT, BRACKET_COMMENT)
    /// IMPORTANT: This "skips" by bumping them into the tree - trivia is preserved!
    fn skip_trivia(&mut self) {
        while matches!(
            self.current_kind(),
            SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE | SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT
        ) {
            self.bump();
        }
    }

    /// Parse the entire file
    pub(crate) fn parse_file(&mut self) {
        self.builder.start_node(CMakeLang::kind_to_raw(SyntaxKind::FILE));

        while !self.eof() {
            self.skip_trivia();

            if self.eof() {
                break;
            }

            if self.at(SyntaxKind::COMMAND_NAME) {
                self.parse_command();
            } else {
                // Error recovery: unexpected token, wrap in ERROR node
                self.builder.start_node(CMakeLang::kind_to_raw(SyntaxKind::ERROR));
                let message = format!("Unexpected token: {:?}", self.current_kind());
                self.errors.push(ParseError {
                    message,
                    offset: self.byte_offset,
                });
                self.bump();
                self.builder.finish_node();
            }
        }

        self.builder.finish_node();
    }

    /// Parse a command invocation: COMMAND_NAME LPAREN argument_list RPAREN
    fn parse_command(&mut self) {
        self.builder.start_node(CMakeLang::kind_to_raw(SyntaxKind::COMMAND_INVOCATION));

        // Expect command name
        self.expect(SyntaxKind::COMMAND_NAME);

        // Skip whitespace between command name and paren
        self.skip_trivia();

        // Expect opening paren
        if !self.at(SyntaxKind::LPAREN) {
            let message = "Expected '(' after command name".to_string();
            self.errors.push(ParseError {
                message,
                offset: self.byte_offset,
            });
            self.builder.finish_node();
            return;
        }
        self.bump(); // consume LPAREN

        // Parse argument list
        self.parse_argument_list();

        // Expect closing paren
        if !self.at(SyntaxKind::RPAREN) {
            let message = "Expected ')' to close command invocation".to_string();
            self.errors.push(ParseError {
                message,
                offset: self.byte_offset,
            });
        } else {
            self.bump(); // consume RPAREN
        }

        self.builder.finish_node();
    }

    /// Parse argument list inside parens
    fn parse_argument_list(&mut self) {
        self.builder.start_node(CMakeLang::kind_to_raw(SyntaxKind::ARGUMENT_LIST));

        loop {
            self.skip_trivia();

            // Stop at closing paren or EOF
            if self.at(SyntaxKind::RPAREN) || self.eof() {
                break;
            }

            // Handle nested parens: CMake allows (a (b c) d)
            if self.at(SyntaxKind::LPAREN) {
                self.bump(); // consume LPAREN
                self.parse_argument_list(); // recursive parse
                if self.at(SyntaxKind::RPAREN) {
                    self.bump(); // consume RPAREN
                } else {
                    let message = "Expected ')' to close nested argument list".to_string();
                    self.errors.push(ParseError {
                        message,
                        offset: self.byte_offset,
                    });
                }
            }
            // Check for argument tokens
            else if self.is_argument_token() {
                self.bump();
            }
            // Error recovery: unexpected token
            else {
                self.builder.start_node(CMakeLang::kind_to_raw(SyntaxKind::ERROR));
                let message = format!("Unexpected token in argument list: {:?}", self.current_kind());
                self.errors.push(ParseError {
                    message,
                    offset: self.byte_offset,
                });
                self.bump();
                self.builder.finish_node();
            }
        }

        self.builder.finish_node();
    }

    /// Check if current token is a valid argument token
    fn is_argument_token(&self) -> bool {
        matches!(
            self.current_kind(),
            SyntaxKind::UNQUOTED_ARGUMENT
                | SyntaxKind::QUOTED_ARGUMENT
                | SyntaxKind::BRACKET_ARGUMENT
                | SyntaxKind::VARIABLE_REF
                | SyntaxKind::ENV_VAR_REF
                | SyntaxKind::CACHE_VAR_REF
                | SyntaxKind::GENERATOR_EXPR
        )
    }

    /// Consume all tokens and return the green tree and errors
    pub(crate) fn finish(self) -> (rowan::GreenNode, Vec<ParseError>) {
        (self.builder.finish(), self.errors)
    }
}
