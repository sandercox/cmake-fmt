use cmake_fmt::cst::parse_text;
use cmake_fmt::syntax_kind::SyntaxKind;
use pretty_assertions::assert_eq;
use rstest::rstest;
use std::fs;
use std::path::PathBuf;

// ============================================================================
// Round-trip tests
// ============================================================================

#[rstest]
fn test_roundtrip_all_fixtures(
    #[values(
        "simple_set.cmake",
        "bracket_arguments.cmake",
        "generator_expressions.cmake",
        "comments.cmake",
        "variable_references.cmake",
        "error_recovery.cmake",
        "nested_commands.cmake"
    )]
    fixture: &str,
) {
    let path = PathBuf::from("tests/fixtures").join(fixture);
    let input =
        fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read fixture: {:?}", path));

    let cst = parse_text(&input);
    let output = cst.text();

    assert_eq!(
        input, output,
        "Round-trip failed for {}: input != output",
        fixture
    );
}

// ============================================================================
// CST structure tests
// ============================================================================

#[test]
fn test_parse_simple_command() {
    let input = "set(MY_VAR value)\n";
    let cst = parse_text(input);

    // Should have FILE node at root
    assert_eq!(cst.root.kind(), SyntaxKind::FILE);

    // Should have one COMMAND_INVOCATION child
    let commands: Vec<_> = cst.commands().collect();
    assert_eq!(commands.len(), 1);

    let cmd = &commands[0];
    assert_eq!(cmd.name_text(), Some("set".to_string()));

    // Should have ARGUMENT_LIST with arguments
    let arg_list = cmd.argument_list().expect("command has argument list");
    let args: Vec<_> = arg_list.arguments().collect();
    assert_eq!(args.len(), 2);
    assert_eq!(args[0].text(), "MY_VAR");
    assert_eq!(args[1].text(), "value");
}

#[test]
fn test_parse_multiple_commands() {
    let input = "project(MyProject)\nset(VAR value)\n";
    let cst = parse_text(input);

    let commands: Vec<_> = cst.commands().collect();
    assert_eq!(commands.len(), 2);

    assert_eq!(commands[0].name_text(), Some("project".to_string()));
    assert_eq!(commands[1].name_text(), Some("set".to_string()));
}

#[test]
fn test_parse_empty_args() {
    let input = "message()\n";
    let cst = parse_text(input);

    let commands: Vec<_> = cst.commands().collect();
    assert_eq!(commands.len(), 1);

    let arg_list = commands[0].argument_list().expect("has argument list");
    let args: Vec<_> = arg_list.arguments().collect();
    assert_eq!(args.len(), 0, "empty argument list");
}

#[test]
fn test_parse_nested_parens() {
    let input = "command(a (b c) d)\n";
    let cst = parse_text(input);

    let commands: Vec<_> = cst.commands().collect();
    assert_eq!(commands.len(), 1);

    let arg_list = commands[0].argument_list().expect("has argument list");

    // Should have arguments a and d at top level
    let args: Vec<_> = arg_list.arguments().collect();
    assert_eq!(args.len(), 2);
    assert_eq!(args[0].text(), "a");
    assert_eq!(args[1].text(), "d");

    // Should have nested argument list
    let nested_lists: Vec<_> = arg_list.nested_lists().collect();
    assert_eq!(nested_lists.len(), 1);

    let nested_args: Vec<_> = nested_lists[0].arguments().collect();
    assert_eq!(nested_args.len(), 2);
    assert_eq!(nested_args[0].text(), "b");
    assert_eq!(nested_args[1].text(), "c");
}

#[test]
fn test_parse_command_name_accessor() {
    let input = "message(hello)\n";
    let cst = parse_text(input);

    let commands: Vec<_> = cst.commands().collect();
    let cmd = &commands[0];

    let name_token = cmd.command_name().expect("has command name");
    assert_eq!(name_token.kind(), SyntaxKind::COMMAND_NAME);
    assert_eq!(name_token.text(), "message");

    assert_eq!(cmd.name_text(), Some("message".to_string()));
}

#[test]
fn test_parse_argument_iteration() {
    let input = "set(A ${VAR} \"quoted\" [[bracket]])\n";
    let cst = parse_text(input);

    let commands: Vec<_> = cst.commands().collect();
    let arg_list = commands[0].argument_list().expect("has argument list");
    let args: Vec<_> = arg_list.arguments().collect();

    assert_eq!(args.len(), 4);
    assert_eq!(args[0].kind(), SyntaxKind::UNQUOTED_ARGUMENT);
    assert_eq!(args[1].kind(), SyntaxKind::VARIABLE_REF);
    assert_eq!(args[2].kind(), SyntaxKind::QUOTED_ARGUMENT);
    assert_eq!(args[3].kind(), SyntaxKind::BRACKET_ARGUMENT);
}

// ============================================================================
// Error recovery tests
// ============================================================================

#[test]
fn test_error_missing_rparen() {
    let input = "set(VAR value\nproject(Test)\n";
    let cst = parse_text(input);

    // Should have errors
    assert!(cst.has_errors());
    assert!(!cst.errors.is_empty());

    // Note: When there's a missing ), the lexer remains at paren_depth > 0,
    // so subsequent text is treated as arguments until a ) is found.
    // This is expected error recovery behavior.
    let commands: Vec<_> = cst.commands().collect();
    assert!(
        !commands.is_empty(),
        "should parse at least the first command"
    );

    // Round-trip should still preserve all text
    assert_eq!(cst.text(), input);
}

#[test]
fn test_error_missing_lparen() {
    let input = "message \"no paren\"\n";
    let cst = parse_text(input);

    // Should have error
    assert!(cst.has_errors());

    // Round-trip preserved
    assert_eq!(cst.text(), input);
}

#[test]
fn test_error_unexpected_token() {
    let input = "!!!invalid\n";
    let cst = parse_text(input);

    // Lexer treats !!!invalid as COMMAND_NAME (since paren_depth = 0)
    // Parser expects LPAREN after command name, doesn't find it, records error
    assert!(cst.has_errors(), "should have parse error");

    // Round-trip preserved
    assert_eq!(cst.text(), input);
}

#[test]
fn test_error_recovery_continues() {
    let input = "message(hello)\n!!!bad\nproject(Test)\n";
    let cst = parse_text(input);

    // Should have errors
    assert!(cst.has_errors());

    // But should parse valid commands
    let commands: Vec<_> = cst.commands().collect();
    assert!(
        commands.len() >= 2,
        "should parse valid commands around error"
    );

    // First and last commands should be valid
    assert_eq!(commands[0].name_text(), Some("message".to_string()));
    let last_valid = commands.iter().rfind(|cmd| cmd.name_text().is_some());
    assert_eq!(last_valid.unwrap().name_text(), Some("project".to_string()));
}

#[test]
fn test_error_count() {
    let path = PathBuf::from("tests/fixtures/error_recovery.cmake");
    let input = fs::read_to_string(&path).expect("fixture exists");
    let cst = parse_text(&input);

    // error_recovery.cmake has:
    // - missing closing paren (causes lexer to stay at depth > 0)
    // - "!!!invalid" becomes UNQUOTED_ARGUMENT due to depth > 0
    // - "add_executable" becomes UNQUOTED_ARGUMENT due to depth > 0
    // So we expect at least 1 error (missing closing paren)
    assert!(
        !cst.errors.is_empty(),
        "expected at least 1 error, got {}",
        cst.errors.len()
    );
}

#[test]
fn test_error_has_offset() {
    let input = "set(VAR value\n"; // missing )
    let cst = parse_text(input);

    assert!(!cst.errors.is_empty());

    // Error should have a byte offset
    for error in &cst.errors {
        // Offset should be somewhere in the input
        assert!(error.offset <= input.len(), "offset out of bounds");
    }
}

// ============================================================================
// Integration tests
// ============================================================================

#[test]
fn test_parse_text_convenience() {
    let input = "message(hello)\n";
    let cst = parse_text(input);

    // Should work end-to-end
    assert_eq!(cst.text(), input);
    assert!(!cst.has_errors());

    let commands: Vec<_> = cst.commands().collect();
    assert_eq!(commands.len(), 1);
}

#[test]
fn test_parse_text_empty() {
    let input = "";
    let cst = parse_text(input);

    // Empty file is valid
    assert!(!cst.has_errors());
    assert_eq!(cst.text(), input);

    let commands: Vec<_> = cst.commands().collect();
    assert_eq!(commands.len(), 0);
}

#[test]
fn test_parse_text_whitespace_only() {
    let input = "  \n  \n";
    let cst = parse_text(input);

    // Whitespace-only file is valid
    assert!(!cst.has_errors());
    assert_eq!(cst.text(), input);

    let commands: Vec<_> = cst.commands().collect();
    assert_eq!(commands.len(), 0);
}

// ============================================================================
// Snapshot tests
// ============================================================================

#[test]
fn test_snapshot_simple_set() {
    let input = fs::read_to_string("tests/fixtures/simple_set.cmake").expect("fixture exists");
    let cst = parse_text(&input);

    insta::assert_debug_snapshot!("simple_set_cst", cst.root);
}

#[test]
fn test_snapshot_bracket_arguments() {
    let input =
        fs::read_to_string("tests/fixtures/bracket_arguments.cmake").expect("fixture exists");
    let cst = parse_text(&input);

    insta::assert_debug_snapshot!("bracket_arguments_cst", cst.root);
}

#[test]
fn test_snapshot_error_recovery() {
    let input = fs::read_to_string("tests/fixtures/error_recovery.cmake").expect("fixture exists");
    let cst = parse_text(&input);

    insta::assert_debug_snapshot!("error_recovery_cst", cst.root);
}

#[test]
fn test_nested_group_node_owns_its_parens() {
    // The formatter renders a group from its node text, so the node has to
    // span the parens themselves, not just what sits between them.
    let cst = parse_text("if((TRUE) AND (FALSE))\n");
    let commands: Vec<_> = cst.commands().collect();
    let arg_list = commands[0].argument_list().expect("has argument list");

    let groups: Vec<String> = arg_list
        .nested_lists()
        .map(|g| g.syntax().text().to_string())
        .collect();

    assert_eq!(groups, vec!["(TRUE)".to_string(), "(FALSE)".to_string()]);
}

#[test]
fn test_unterminated_nested_group_roundtrips() {
    // Error recovery must still reproduce the input byte for byte
    for input in [
        "if((A)\n",
        "if((\n",
        "if((((((\n",
        "if(()\n",
        "if((A))extra\n",
    ] {
        let cst = parse_text(input);
        assert_eq!(
            cst.root.text().to_string(),
            input,
            "roundtrip failed for {:?}",
            input
        );
    }
}
