use cmake_fmt::cst::parse_text;
use cmake_fmt::parser::ParseError;
use rstest::rstest;
use std::fs;

// ============================================================================
// Round-trip tests - All fixture files must reconstruct byte-for-byte
// ============================================================================

#[rstest]
#[case("simple_set.cmake")]
#[case("bracket_arguments.cmake")]
#[case("generator_expressions.cmake")]
#[case("comments.cmake")]
#[case("variable_references.cmake")]
#[case("error_recovery.cmake")]
#[case("nested_commands.cmake")]
#[case("real_world_target.cmake")]
#[case("complex_expressions.cmake")]
#[case("edge_cases.cmake")]
fn test_fixture_roundtrip(#[case] filename: &str) {
    let path = format!("tests/fixtures/{}", filename);
    let input =
        fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read fixture: {}", path));

    let cst = parse_text(&input);
    let reconstructed = cst.text();

    assert_eq!(
        reconstructed, input,
        "Round-trip failed for {}: reconstructed text does not match input",
        filename
    );
}

// ============================================================================
// Error location tests - line:column computation
// ============================================================================

#[test]
fn test_error_line_column_first_line() {
    let source = "set(";
    let cst = parse_text(source);

    assert!(cst.has_errors(), "Expected parse error");
    let error = &cst.errors[0];
    let (line, _col) = error.line_col(source);

    // Error should be on line 1
    assert_eq!(line, 1, "Error should be on line 1");
}

#[test]
fn test_error_line_column_later_line() {
    let source = "set(A B)\nset(C D)\nset(E F";
    let cst = parse_text(source);

    assert!(cst.has_errors(), "Expected parse error on line 3");

    // Find the error (should be at the unclosed paren on line 3)
    let errors = cst.errors;
    assert!(!errors.is_empty(), "Should have at least one error");

    // Check that at least one error is on line 3
    let has_line_3_error = errors.iter().any(|e| {
        let (line, _col) = e.line_col(source);
        line == 3
    });

    assert!(has_line_3_error, "Should have an error on line 3");
}

#[test]
fn test_error_line_column_with_crlf() {
    let source = "set(A B)\r\nset(C D)\r\nset(E F";
    let cst = parse_text(source);

    assert!(cst.has_errors(), "Expected parse error");

    // Verify CRLF is handled correctly (should have 3 lines)
    let errors = cst.errors;
    let has_line_3_error = errors.iter().any(|e| {
        let (line, _col) = e.line_col(source);
        line == 3
    });

    assert!(
        has_line_3_error,
        "CRLF line counting should result in error on line 3"
    );
}

#[test]
fn test_format_errors_output() {
    let source = "set(A B)\nset(C D)\nset(E F";
    let cst = parse_text(source);

    let formatted = cst.format_errors(source);
    assert!(!formatted.is_empty(), "Should have formatted errors");

    // Each formatted error should match pattern "line:col: error: message"
    for error_str in &formatted {
        assert!(
            error_str.contains(':'),
            "Formatted error should contain colons: {}",
            error_str
        );
        assert!(
            error_str.contains("error:"),
            "Formatted error should contain 'error:': {}",
            error_str
        );
    }
}

#[test]
fn test_line_col_hello_world() {
    let source = "hello\nworld\n";
    let error = ParseError {
        message: "test".to_string(),
        offset: 6, // Start of "world"
    };

    let (line, col) = error.line_col(source);
    assert_eq!(line, 2, "Should be line 2");
    assert_eq!(col, 1, "Should be column 1 (start of 'world')");
}

#[test]
fn test_line_col_first_char() {
    let source = "hello\nworld\n";
    let error = ParseError {
        message: "test".to_string(),
        offset: 0, // First character
    };

    let (line, col) = error.line_col(source);
    assert_eq!(line, 1, "Should be line 1");
    assert_eq!(col, 1, "Should be column 1");
}

// ============================================================================
// Complex construct tests
// ============================================================================

#[test]
fn test_generator_expr_3_levels() {
    let input = r#"target_compile_definitions(mylib PRIVATE
  $<$<AND:$<BOOL:${VAR}>,$<CONFIG:Debug>>:DEBUG_ENABLED>
)"#;

    let cst = parse_text(input);
    assert_eq!(
        cst.text(),
        input,
        "3-level nested generator expression should round-trip"
    );
    assert!(!cst.has_errors(), "Should parse without errors");
}

#[test]
fn test_mixed_refs_in_quoted() {
    let input = r#"set(RESULT "${VAR}_$ENV{HOME}_${A_${B}}")"#;

    let cst = parse_text(input);
    assert_eq!(
        cst.text(),
        input,
        "Mixed variable references should round-trip"
    );
    assert!(!cst.has_errors(), "Should parse without errors");
}

#[test]
fn test_bracket_arg_0_through_5_equals() {
    let test_cases = vec![
        r#"message([[bracket]])"#,
        r#"message([=[bracket]=])"#,
        r#"message([==[bracket]==])"#,
        r#"message([===[bracket]===])"#,
        r#"message([====[bracket]====])"#,
        r#"message([=====[bracket]=====])"#,
    ];

    for input in test_cases {
        let cst = parse_text(input);
        assert_eq!(
            cst.text(),
            input,
            "Bracket argument should round-trip: {}",
            input
        );
        assert!(!cst.has_errors(), "Should parse without errors: {}", input);
    }
}

#[test]
fn test_adjacent_specials() {
    let input = r#"message(${A}${B}$<CONFIG:Debug>"quoted")"#;

    let cst = parse_text(input);
    assert_eq!(
        cst.text(),
        input,
        "Adjacent special constructs should round-trip"
    );
    assert!(!cst.has_errors(), "Should parse without errors");
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_empty_file() {
    let input = "";
    let cst = parse_text(input);

    assert_eq!(cst.text(), input, "Empty file should round-trip");
    assert!(!cst.has_errors(), "Empty file should parse without errors");
}

#[test]
fn test_whitespace_only_file() {
    let input = "  \n\n  \n";
    let cst = parse_text(input);

    assert_eq!(cst.text(), input, "Whitespace-only file should round-trip");
    assert!(
        !cst.has_errors(),
        "Whitespace-only file should parse without errors"
    );
}

#[test]
fn test_crlf_line_endings() {
    let input = "set(A B)\r\nset(C D)\r\n";
    let cst = parse_text(input);

    assert_eq!(cst.text(), input, "CRLF line endings should be preserved");
    assert!(!cst.has_errors(), "Should parse without errors");
}

#[test]
fn test_tab_indentation() {
    let input = "\tmessage(\ttabbed\t)\n";
    let cst = parse_text(input);

    assert_eq!(cst.text(), input, "Tab characters should be preserved");
    assert!(!cst.has_errors(), "Should parse without errors");
}

#[test]
fn test_comment_after_paren() {
    let input = "message(hello)# comment\n";
    let cst = parse_text(input);

    assert_eq!(
        cst.text(),
        input,
        "Comment immediately after paren should round-trip"
    );
    assert!(!cst.has_errors(), "Should parse without errors");
}

#[test]
fn test_semicolons_in_quoted() {
    let input = r#"set(LIST_VAR "a;b;c")"#;
    let cst = parse_text(input);

    assert_eq!(
        cst.text(),
        input,
        "Semicolons in quoted strings should be preserved"
    );
    assert!(!cst.has_errors(), "Should parse without errors");
}

// ============================================================================
// Stress tests
// ============================================================================

#[test]
fn test_many_commands() {
    // Generate 100 simple commands
    let mut input = String::new();
    for i in 0..100 {
        input.push_str(&format!("set(VAR_{} value_{})\n", i, i));
    }

    let cst = parse_text(&input);
    assert_eq!(cst.text(), input, "100 commands should round-trip");
    assert!(!cst.has_errors(), "Should parse without errors");
}

#[test]
fn test_large_argument_list() {
    // Command with 50 arguments
    let mut input = String::from("target_sources(myapp PRIVATE");
    for i in 0..50 {
        input.push_str(&format!(" src/file_{}.cpp", i));
    }
    input.push_str(")\n");

    let cst = parse_text(&input);
    assert_eq!(cst.text(), input, "Large argument list should round-trip");
    assert!(!cst.has_errors(), "Should parse without errors");
}

// ============================================================================
// Real-world validation - Check that realistic CMake parses cleanly
// ============================================================================

#[test]
fn test_real_world_target_no_errors() {
    let input = fs::read_to_string("tests/fixtures/real_world_target.cmake")
        .expect("Failed to read real_world_target.cmake");

    let cst = parse_text(&input);

    if cst.has_errors() {
        let formatted_errors = cst.format_errors(&input);
        panic!(
            "real_world_target.cmake should parse without errors, but got:\n{}",
            formatted_errors.join("\n")
        );
    }
}

#[test]
fn test_complex_expressions_no_errors() {
    let input = fs::read_to_string("tests/fixtures/complex_expressions.cmake")
        .expect("Failed to read complex_expressions.cmake");

    let cst = parse_text(&input);

    if cst.has_errors() {
        let formatted_errors = cst.format_errors(&input);
        panic!(
            "complex_expressions.cmake should parse without errors, but got:\n{}",
            formatted_errors.join("\n")
        );
    }
}

#[test]
fn test_edge_cases_no_errors() {
    let input = fs::read_to_string("tests/fixtures/edge_cases.cmake")
        .expect("Failed to read edge_cases.cmake");

    let cst = parse_text(&input);

    if cst.has_errors() {
        let formatted_errors = cst.format_errors(&input);
        panic!(
            "edge_cases.cmake should parse without errors, but got:\n{}",
            formatted_errors.join("\n")
        );
    }
}
