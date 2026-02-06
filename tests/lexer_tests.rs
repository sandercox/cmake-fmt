use cmake_formatter::lexer::{lex, Token};
use cmake_formatter::SyntaxKind;
use pretty_assertions::assert_eq;
use rstest::rstest;

// Helper function to check token properties
fn assert_token(token: &Token, kind: SyntaxKind, text: &str) {
    assert_eq!(token.kind, kind, "Token kind mismatch");
    assert_eq!(token.text, text, "Token text mismatch");
    assert!(!token.text.is_empty(), "Token text should not be empty");
    assert!(token.span.end > token.span.start, "Token span should be non-empty");
}

// Unit tests for each token type

#[test]
fn test_lex_whitespace() {
    let tokens = lex("   \t  ");
    assert_eq!(tokens.len(), 1);
    assert_token(&tokens[0], SyntaxKind::WHITESPACE, "   \t  ");
}

#[test]
fn test_lex_newline_lf() {
    let tokens = lex("\n");
    assert_eq!(tokens.len(), 1);
    assert_token(&tokens[0], SyntaxKind::NEWLINE, "\n");
}

#[test]
fn test_lex_newline_crlf() {
    let tokens = lex("\r\n");
    assert_eq!(tokens.len(), 1);
    assert_token(&tokens[0], SyntaxKind::NEWLINE, "\r\n");
}

#[test]
fn test_lex_line_comment() {
    let tokens = lex("# This is a comment\n");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], SyntaxKind::COMMENT, "# This is a comment");
    assert_token(&tokens[1], SyntaxKind::NEWLINE, "\n");
}

#[test]
fn test_lex_bracket_comment_0_equals() {
    let tokens = lex("#[[This is a bracket comment]]");
    assert_eq!(tokens.len(), 1);
    assert_token(&tokens[0], SyntaxKind::BRACKET_COMMENT, "#[[This is a bracket comment]]");
}

#[test]
fn test_lex_bracket_comment_1_equals() {
    let tokens = lex("#[=[comment]=]");
    assert_eq!(tokens.len(), 1);
    assert_token(&tokens[0], SyntaxKind::BRACKET_COMMENT, "#[=[comment]=]");
}

#[test]
fn test_lex_bracket_comment_2_equals() {
    let tokens = lex("#[==[comment]==]");
    assert_eq!(tokens.len(), 1);
    assert_token(&tokens[0], SyntaxKind::BRACKET_COMMENT, "#[==[comment]==]");
}

#[test]
fn test_lex_bracket_argument_0_equals() {
    let tokens = lex("message([[text]])");
    assert_eq!(tokens.len(), 4);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
    assert_token(&tokens[1], SyntaxKind::LPAREN, "(");
    assert_token(&tokens[2], SyntaxKind::BRACKET_ARGUMENT, "[[text]]");
    assert_token(&tokens[3], SyntaxKind::RPAREN, ")");
}

#[test]
fn test_lex_bracket_argument_1_equals() {
    let tokens = lex("message([=[text]=])");
    assert_eq!(tokens.len(), 4);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
    assert_token(&tokens[1], SyntaxKind::LPAREN, "(");
    assert_token(&tokens[2], SyntaxKind::BRACKET_ARGUMENT, "[=[text]=]");
    assert_token(&tokens[3], SyntaxKind::RPAREN, ")");
}

#[test]
fn test_lex_bracket_argument_2_equals() {
    let tokens = lex("message([==[text]==])");
    assert_eq!(tokens.len(), 4);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
    assert_token(&tokens[1], SyntaxKind::LPAREN, "(");
    assert_token(&tokens[2], SyntaxKind::BRACKET_ARGUMENT, "[==[text]==]");
    assert_token(&tokens[3], SyntaxKind::RPAREN, ")");
}

#[test]
fn test_lex_bracket_argument_with_nested_close() {
    let tokens = lex("message([=[contains ]] inside]=])");
    assert_eq!(tokens.len(), 4);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
    assert_token(&tokens[1], SyntaxKind::LPAREN, "(");
    assert_token(&tokens[2], SyntaxKind::BRACKET_ARGUMENT, "[=[contains ]] inside]=]");
    assert_token(&tokens[3], SyntaxKind::RPAREN, ")");
}

#[test]
fn test_lex_quoted_argument() {
    let tokens = lex("message(\"hello world\")");
    assert_eq!(tokens.len(), 4);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
    assert_token(&tokens[1], SyntaxKind::LPAREN, "(");
    assert_token(&tokens[2], SyntaxKind::QUOTED_ARGUMENT, "\"hello world\"");
    assert_token(&tokens[3], SyntaxKind::RPAREN, ")");
}

#[test]
fn test_lex_quoted_argument_with_escapes() {
    let tokens = lex("message(\"hello\\nworld\")");
    assert_eq!(tokens.len(), 4);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
    assert_token(&tokens[1], SyntaxKind::LPAREN, "(");
    assert_token(&tokens[2], SyntaxKind::QUOTED_ARGUMENT, "\"hello\\nworld\"");
    assert_token(&tokens[3], SyntaxKind::RPAREN, ")");
}

#[test]
fn test_lex_quoted_argument_with_embedded_var() {
    let tokens = lex("message(\"prefix ${VAR} suffix\")");
    assert_eq!(tokens.len(), 4);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
    assert_token(&tokens[1], SyntaxKind::LPAREN, "(");
    assert_token(&tokens[2], SyntaxKind::QUOTED_ARGUMENT, "\"prefix ${VAR} suffix\"");
    assert_token(&tokens[3], SyntaxKind::RPAREN, ")");
}

#[test]
fn test_lex_variable_ref() {
    let tokens = lex("message(${MY_VAR})");
    assert_eq!(tokens.len(), 4);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
    assert_token(&tokens[1], SyntaxKind::LPAREN, "(");
    assert_token(&tokens[2], SyntaxKind::VARIABLE_REF, "${MY_VAR}");
    assert_token(&tokens[3], SyntaxKind::RPAREN, ")");
}

#[test]
fn test_lex_env_var_ref() {
    let tokens = lex("message($ENV{PATH})");
    assert_eq!(tokens.len(), 4);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
    assert_token(&tokens[1], SyntaxKind::LPAREN, "(");
    assert_token(&tokens[2], SyntaxKind::ENV_VAR_REF, "$ENV{PATH}");
    assert_token(&tokens[3], SyntaxKind::RPAREN, ")");
}

#[test]
fn test_lex_cache_var_ref() {
    let tokens = lex("message($CACHE{VAR})");
    assert_eq!(tokens.len(), 4);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
    assert_token(&tokens[1], SyntaxKind::LPAREN, "(");
    assert_token(&tokens[2], SyntaxKind::CACHE_VAR_REF, "$CACHE{VAR}");
    assert_token(&tokens[3], SyntaxKind::RPAREN, ")");
}

#[test]
fn test_lex_nested_variable_ref() {
    let tokens = lex("set(X ${PREFIX_${SUFFIX}})");
    // Expect: COMMAND_NAME LPAREN UNQUOTED_ARGUMENT WHITESPACE VARIABLE_REF RPAREN
    assert_eq!(tokens.len(), 6);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "set");
    assert_token(&tokens[4], SyntaxKind::VARIABLE_REF, "${PREFIX_${SUFFIX}}");
}

#[test]
fn test_lex_generator_expr_simple() {
    let tokens = lex("set(X $<CONFIG:Debug>)");
    assert_eq!(tokens.len(), 6);
    assert_token(&tokens[4], SyntaxKind::GENERATOR_EXPR, "$<CONFIG:Debug>");
}

#[test]
fn test_lex_generator_expr_nested() {
    let tokens = lex("set(X $<$<BOOL:${VAR}>:value>)");
    assert_eq!(tokens.len(), 6);
    assert_token(&tokens[4], SyntaxKind::GENERATOR_EXPR, "$<$<BOOL:${VAR}>:value>");
}

#[test]
fn test_lex_generator_expr_deep() {
    let tokens = lex("set(X $<$<AND:$<BOOL:${A}>,$<CONFIG:Debug>>:-g>)");
    assert_eq!(tokens.len(), 6);
    assert_token(&tokens[4], SyntaxKind::GENERATOR_EXPR, "$<$<AND:$<BOOL:${A}>,$<CONFIG:Debug>>:-g>");
}

#[test]
fn test_lex_parens() {
    let tokens = lex("()");
    assert_eq!(tokens.len(), 2);
    assert_token(&tokens[0], SyntaxKind::LPAREN, "(");
    assert_token(&tokens[1], SyntaxKind::RPAREN, ")");
}

#[test]
fn test_lex_command_name() {
    let tokens = lex("message(hello)");
    assert_eq!(tokens.len(), 4);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
}

#[test]
fn test_lex_command_after_newline() {
    let tokens = lex("message(a)\nmessage(b)");
    // COMMAND_NAME LPAREN UNQUOTED_ARGUMENT RPAREN NEWLINE COMMAND_NAME LPAREN UNQUOTED_ARGUMENT RPAREN
    assert_eq!(tokens.len(), 9);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
    assert_token(&tokens[5], SyntaxKind::COMMAND_NAME, "message");
}

// Edge case tests

#[test]
fn test_lex_empty_input() {
    let tokens = lex("");
    assert_eq!(tokens.len(), 0);
}

#[test]
fn test_lex_only_whitespace() {
    let tokens = lex("   ");
    assert_eq!(tokens.len(), 1);
    assert_token(&tokens[0], SyntaxKind::WHITESPACE, "   ");
}

#[test]
fn test_lex_unclosed_bracket() {
    let tokens = lex("message([[unclosed)");
    // Should lex as: COMMAND_NAME LPAREN BRACKET_ARGUMENT (no RPAREN - consumed by bracket)
    // Error recovery: unclosed bracket consumes to end of input including the )
    assert_eq!(tokens.len(), 3);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
    assert_token(&tokens[1], SyntaxKind::LPAREN, "(");
    assert_token(&tokens[2], SyntaxKind::BRACKET_ARGUMENT, "[[unclosed)");
}

#[test]
fn test_lex_unclosed_quote() {
    let tokens = lex("message(\"unclosed)");
    // Should lex as: COMMAND_NAME LPAREN QUOTED_ARGUMENT (no RPAREN - consumed by quote)
    // Error recovery: unclosed quote consumes to end of input including the )
    assert_eq!(tokens.len(), 3);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
    assert_token(&tokens[1], SyntaxKind::LPAREN, "(");
    assert_token(&tokens[2], SyntaxKind::QUOTED_ARGUMENT, "\"unclosed)");
}

#[test]
fn test_lex_command_with_no_args() {
    let tokens = lex("message()");
    assert_eq!(tokens.len(), 3);
    assert_token(&tokens[0], SyntaxKind::COMMAND_NAME, "message");
    assert_token(&tokens[1], SyntaxKind::LPAREN, "(");
    assert_token(&tokens[2], SyntaxKind::RPAREN, ")");
}

// Round-trip tests using rstest fixtures

#[rstest]
#[case("simple_set.cmake")]
#[case("bracket_arguments.cmake")]
#[case("generator_expressions.cmake")]
#[case("comments.cmake")]
#[case("variable_references.cmake")]
fn test_fixture_roundtrip(#[case] filename: &str) {
    let path = format!("tests/fixtures/{}", filename);
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Failed to read fixture: {}", path));

    let tokens = lex(&input);

    // Verify no token has empty text
    for token in &tokens {
        assert!(!token.text.is_empty(), "Token should not have empty text: {:?}", token);
        assert!(token.span.end > token.span.start, "Token span should be non-empty: {:?}", token);
    }

    // Concatenate all token texts
    let reconstructed: String = tokens.iter().map(|t| t.text.as_str()).collect();

    // Verify byte-for-byte reconstruction
    assert_eq!(
        reconstructed, input,
        "Round-trip failed for {}\nExpected: {:?}\nGot: {:?}",
        filename, input, reconstructed
    );
}

// Snapshot tests with insta

#[rstest]
#[case("simple_set.cmake")]
#[case("bracket_arguments.cmake")]
#[case("generator_expressions.cmake")]
#[case("comments.cmake")]
#[case("variable_references.cmake")]
fn test_fixture_snapshot(#[case] filename: &str) {
    let path = format!("tests/fixtures/{}", filename);
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Failed to read fixture: {}", path));

    let tokens = lex(&input);

    // Create a simplified representation for snapshot
    let snapshot: Vec<(String, String)> = tokens
        .iter()
        .map(|t| (format!("{:?}", t.kind), t.text.clone()))
        .collect();

    insta::assert_yaml_snapshot!(filename.replace(".cmake", ""), snapshot);
}
