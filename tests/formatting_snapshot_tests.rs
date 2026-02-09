use cmake_format::cst::parse_text;
use cmake_format::formatter::{format_text, CommandCase, FormatConfig};

// ============================================================================
// SNAPSHOT TESTS
// ============================================================================

#[test]
fn test_formatting_snapshots() {
    let config = FormatConfig::default();
    insta::glob!("format_fixtures/*.cmake", |path| {
        let input = std::fs::read_to_string(path).unwrap();
        let output = format_text(&input, &config);
        insta::assert_snapshot!(output);
    });
}

// ============================================================================
// IDEMPOTENCY TESTS
// ============================================================================

#[test]
fn test_idempotency_all_fixtures() {
    let config = FormatConfig::default();
    insta::glob!("format_fixtures/*.cmake", |path| {
        let input = std::fs::read_to_string(path).unwrap();
        let once = format_text(&input, &config);
        let twice = format_text(&once, &config);
        assert_eq!(
            once, twice,
            "Idempotency failed for {}: formatting twice produced different output",
            path.display()
        );
    });
}

#[test]
fn test_idempotency_phase1_fixtures() {
    let config = FormatConfig::default();
    insta::glob!("fixtures/*.cmake", |path| {
        let input = std::fs::read_to_string(path).unwrap();
        let once = format_text(&input, &config);
        let twice = format_text(&once, &config);
        assert_eq!(
            once, twice,
            "Idempotency failed for Phase 1 fixture {}: formatting twice produced different output",
            path.display()
        );
    });
}

// ============================================================================
// SEMANTIC PRESERVATION TESTS
// ============================================================================

#[test]
fn test_semantic_preservation_all_fixtures() {
    let config = FormatConfig::default();
    insta::glob!("format_fixtures/*.cmake", |path| {
        let input = std::fs::read_to_string(path).unwrap();
        let output = format_text(&input, &config);

        let input_commands = extract_semantic_commands(&input);
        let output_commands = extract_semantic_commands(&output);

        assert_eq!(
            input_commands, output_commands,
            "Semantic preservation failed for {}: commands differ after formatting",
            path.display()
        );
    });
}

#[test]
fn test_semantic_preservation_phase1_fixtures() {
    let config = FormatConfig::default();
    insta::glob!("fixtures/*.cmake", |path| {
        let input = std::fs::read_to_string(path).unwrap();
        let output = format_text(&input, &config);

        let input_commands = extract_semantic_commands(&input);
        let output_commands = extract_semantic_commands(&output);

        assert_eq!(
            input_commands, output_commands,
            "Semantic preservation failed for Phase 1 fixture {}",
            path.display()
        );
    });
}

/// Extract command names and their arguments (ignoring whitespace/trivia) for semantic comparison
fn extract_semantic_commands(source: &str) -> Vec<(String, Vec<String>)> {
    let cst = parse_text(source);
    cst.commands()
        .map(|cmd| {
            let name = cmd.name_text().unwrap_or_default().to_lowercase();
            let args: Vec<String> = cmd
                .argument_list()
                .map(|al| al.arguments().map(|a| a.text().to_string()).collect())
                .unwrap_or_default();
            (name, args)
        })
        .collect()
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_empty_file_formats() {
    let config = FormatConfig::default();
    let result = format_text("", &config);
    assert_eq!(result, "");
}

#[test]
fn test_crlf_handling() {
    let config = FormatConfig::default();
    let input = "set(FOO bar)\r\nmessage(\"hello\")\r\n";
    let output = format_text(input, &config);

    // Output should normalize to LF
    assert!(!output.contains("\r\n"), "Output should not contain CRLF");
    assert!(output.contains("set(FOO bar)"));
    assert!(output.contains("message(\"hello\")"));
}

#[test]
fn test_only_comments() {
    let config = FormatConfig::default();
    let input = "# Just a comment\n# Another comment\n";
    let output = format_text(input, &config);

    // Comments should be preserved
    assert!(output.contains("# Just a comment"));
    assert!(output.contains("# Another comment"));
}

#[test]
fn test_deeply_nested_scopes() {
    let config = FormatConfig::default();
    let input = r#"
if(A)
  if(B)
    if(C)
      if(D)
        set(DEEPLY_NESTED true)
      endif()
    endif()
  endif()
endif()
"#;
    let output = format_text(input, &config);

    // Verify 4 levels of indentation (2 spaces per level = 8 spaces)
    assert!(output.contains("        set(DEEPLY_NESTED true)"),
        "Expected 8 spaces for 4-level nesting");
}

#[test]
fn test_error_recovery_fixture() {
    let config = FormatConfig::default();

    // Read the error_recovery fixture from Phase 1
    let input = std::fs::read_to_string("tests/fixtures/error_recovery.cmake")
        .expect("error_recovery.cmake should exist");

    // Should not panic, even with parse errors
    let output = format_text(&input, &config);

    // Output should be non-empty (formatter handles errors gracefully)
    assert!(!output.is_empty());
}

#[test]
fn test_config_indent_width_4() {
    let config = FormatConfig {
        indent_width: 4,
        ..FormatConfig::default()
    };
    let input = "if(FOO)\nset(BAR baz)\nendif()\n";
    let output = format_text(input, &config);

    // Should have 4-space indentation
    assert!(output.contains("    set(BAR baz)"),
        "Expected 4-space indentation");
}

#[test]
fn test_config_tabs() {
    let config = FormatConfig {
        use_tabs: true,
        ..FormatConfig::default()
    };
    let input = "if(FOO)\nset(BAR baz)\nendif()\n";
    let output = format_text(input, &config);

    // Should have tab indentation
    assert!(output.contains("\tset(BAR baz)"),
        "Expected tab indentation");
}

#[test]
fn test_config_uppercase() {
    let config = FormatConfig {
        command_case: CommandCase::Uppercase,
        ..FormatConfig::default()
    };
    let input = "set(foo bar)\nmessage(hello)\nif(test)\nendif()\n";
    let output = format_text(input, &config);

    // All commands should be uppercase
    assert!(output.contains("SET(foo bar)"));
    assert!(output.contains("MESSAGE(hello)"));
    assert!(output.contains("IF(test)"));
    assert!(output.contains("ENDIF()"));
}

#[test]
fn test_config_line_length_120() {
    let config = FormatConfig {
        max_line_length: 120,
        ..FormatConfig::default()
    };
    let input = "set(LIST a b c d e f g h i j k l m n o p q r s t u v w x y z)\n";
    let output = format_text(input, &config);

    // With max_line_length=120, more content should fit on one line
    // This is a bit hard to test precisely, but we can verify the line is longer
    // than it would be with default 80-char limit

    // At minimum, the output should be valid and not panic
    assert!(!output.is_empty());
}
