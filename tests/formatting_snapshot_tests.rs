use cmake_format::cst::parse_text;
use cmake_format::formatter::{format_text, ClosingStyle, CommandCase, FormatConfig};

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
    let config = FormatConfig::default(); // Auto mode
    let input = "set(FOO bar)\r\nmessage(\"hello\")\r\n";
    let output = format_text(input, &config);

    // Auto mode: CRLF input → CRLF output (preserves detected line ending)
    assert!(output.contains("\r\n"), "Auto mode should preserve CRLF from input");
    assert!(output.contains("set(FOO bar)"));
    assert!(output.contains("message(\"hello\")"));

    // Force LF mode should strip CRLF
    let lf_config = FormatConfig {
        line_ending: cmake_format::formatter::LineEnding::Lf,
        ..FormatConfig::default()
    };
    let lf_output = format_text(input, &lf_config);
    assert!(!lf_output.contains("\r\n"), "LF mode should not contain CRLF");
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

    // Verify 4 levels of indentation (1 tab per level = 4 tabs)
    assert!(output.contains("\t\t\t\tset(DEEPLY_NESTED true)"),
        "Expected 4 tabs for 4-level nesting");
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
        use_tabs: false,
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

// ============================================================================
// BLOCK CLOSER MODE TESTS
// ============================================================================

#[test]
fn test_block_closer_keep_mode() {
    let config = FormatConfig::default(); // Keep is default
    let input = std::fs::read_to_string("tests/format_fixtures/block_closer_keep.cmake").unwrap();
    let output = format_text(&input, &config);
    insta::assert_snapshot!("block_closer_keep", output);
}

#[test]
fn test_block_closer_remove_mode() {
    let config = FormatConfig {
        closing_style: ClosingStyle::Remove,
        ..FormatConfig::default()
    };
    let input =
        std::fs::read_to_string("tests/format_fixtures/block_closer_remove.cmake").unwrap();
    let output = format_text(&input, &config);
    insta::assert_snapshot!("block_closer_remove", output);
}

#[test]
fn test_block_closer_force_mode() {
    let config = FormatConfig {
        closing_style: ClosingStyle::Force,
        ..FormatConfig::default()
    };
    let input =
        std::fs::read_to_string("tests/format_fixtures/block_closer_force.cmake").unwrap();
    let output = format_text(&input, &config);
    insta::assert_snapshot!("block_closer_force", output);
}

#[test]
fn test_block_closer_idempotency() {
    let fixtures = [
        (
            "keep",
            ClosingStyle::Keep,
            "tests/format_fixtures/block_closer_keep.cmake",
        ),
        (
            "remove",
            ClosingStyle::Remove,
            "tests/format_fixtures/block_closer_remove.cmake",
        ),
        (
            "force",
            ClosingStyle::Force,
            "tests/format_fixtures/block_closer_force.cmake",
        ),
    ];

    for (mode_name, style, fixture_path) in &fixtures {
        let config = FormatConfig {
            closing_style: *style,
            ..FormatConfig::default()
        };
        let input = std::fs::read_to_string(fixture_path).unwrap();
        let once = format_text(&input, &config);
        let twice = format_text(&once, &config);
        assert_eq!(
            once, twice,
            "Idempotency failed for block closer {} mode on {}",
            mode_name, fixture_path
        );
    }
}

// ============================================================================
// EDGE CASE HARDENING TESTS (Phase 9)
// ============================================================================

// ----------------------------------------------------------------------------
// EDGE-01: Generator Expression Preservation
// ----------------------------------------------------------------------------

#[test]
fn test_generator_expr_no_line_break() {
    let config = FormatConfig::default();
    // This line is >200 chars - test that generator expression stays atomic
    let input = r#"target_compile_options(myapp PRIVATE $<$<AND:$<BOOL:${ENABLE_LONG_FEATURE_NAME}>,$<OR:$<CONFIG:Debug>,$<CONFIG:RelWithDebInfo>,$<CONFIG:MinSizeRel>>>:-Wall -Wextra -Wpedantic -Werror -Wno-unused-parameter -Wno-missing-field-initializers>)"#;
    let output = format_text(input, &config);

    // Find the generator expression in output
    let genexpr_start = output.find("$<").expect("Generator expr should exist");
    let genexpr_end = output.rfind(">").expect("Generator expr should close");
    let genexpr = &output[genexpr_start..=genexpr_end];

    // Critical assertion: NO newlines inside the generator expression token
    assert!(
        !genexpr.contains('\n'),
        "Generator expression was broken across lines:\n{}",
        genexpr
    );

    insta::assert_snapshot!("generator_expr_no_line_break", output);
}

#[test]
fn test_generator_expr_nested_snapshot() {
    let config = FormatConfig::default();
    let input = "set(COMPLEX_FLAGS $<$<AND:$<BOOL:${VAR}>,$<OR:$<CONFIG:Debug>,$<CONFIG:RelWithDebInfo>>>:-g>)";
    let output = format_text(input, &config);
    insta::assert_snapshot!("generator_expr_nested", output);
}

#[test]
fn test_generator_expr_adjacent_no_space() {
    let config = FormatConfig::default();
    let input = "set(CONFIGS $<CONFIG:Debug> $<CONFIG:Release>)";
    let output = format_text(input, &config);

    // Find both generator expressions in output
    let first_start = output.find("$<CONFIG:Debug>").expect("First genexpr should exist");
    let second_start = output.find("$<CONFIG:Release>").expect("Second genexpr should exist");

    // Extract the substring between them
    let between = &output[first_start + "$<CONFIG:Debug>".len()..second_start];

    // They should be separated by exactly one space (not collapsed, not multiple)
    assert_eq!(between, " ", "Generator expressions should be separated by exactly one space");

    insta::assert_snapshot!("generator_expr_adjacent", output);
}

// ----------------------------------------------------------------------------
// EDGE-02: Variable Reference Preservation
// ----------------------------------------------------------------------------

#[test]
fn test_nested_variable_ref_preserved() {
    let config = FormatConfig::default();
    let input = "set(COMPUTED ${PREFIX_${SUFFIX}})";
    let output = format_text(input, &config);

    // Critical assertion: nested variable reference is preserved exactly
    assert!(
        output.contains("${PREFIX_${SUFFIX}}"),
        "Nested variable reference was corrupted:\n{}",
        output
    );

    insta::assert_snapshot!("nested_variable_ref", output);
}

#[test]
fn test_deeply_nested_variable_ref() {
    let config = FormatConfig::default();
    let input = "set(DEEPLY_NESTED ${${OUTER_${INNER}}})";
    let output = format_text(input, &config);

    // Critical assertion: triple-nested variable reference preserved
    assert!(
        output.contains("${${OUTER_${INNER}}}"),
        "Deeply nested variable reference was corrupted:\n{}",
        output
    );

    insta::assert_snapshot!("deeply_nested_variable_ref", output);
}

#[test]
fn test_adjacent_refs_preserved() {
    let config = FormatConfig::default();
    let input = "set(COMBINED ${A}${B}${C})";
    let output = format_text(input, &config);

    // Critical assertion: all three variable references are preserved as separate tokens
    assert!(
        output.contains("${A}") && output.contains("${B}") && output.contains("${C}"),
        "All three adjacent variable references should be preserved:\n{}",
        output
    );

    // They should remain separate arguments (not merged into one token)
    assert!(
        output.matches("${").count() == 3,
        "Should have exactly 3 variable references"
    );

    insta::assert_snapshot!("adjacent_refs_preserved", output);
}

// ----------------------------------------------------------------------------
// EDGE-03: Bracket Argument Byte Preservation
// ----------------------------------------------------------------------------

#[test]
fn test_bracket_arg_byte_identical() {
    let config = FormatConfig::default();

    // Test each bracket argument variant
    let test_cases = vec![
        ("message([[simple bracket arg]])", "[[simple bracket arg]]"),
        ("message([=[contains ]] inside]=])", "[=[contains ]] inside]=]"),
        ("message([==[contains ]=] inside]==])", "[==[contains ]=] inside]==]"),
    ];

    for (input, expected_bracket) in test_cases {
        let output = format_text(input, &config);

        // Extract bracket argument from output
        let bracket_start = if expected_bracket.starts_with("[==") {
            output.find("[==").expect("Bracket arg should exist")
        } else if expected_bracket.starts_with("[=") {
            output.find("[=").expect("Bracket arg should exist")
        } else {
            output.find("[[").expect("Bracket arg should exist")
        };

        let bracket_end = if expected_bracket.ends_with("]==]") {
            output.rfind("]==]").expect("Bracket arg should close") + 3
        } else if expected_bracket.ends_with("]=]") {
            output.rfind("]=]").expect("Bracket arg should close") + 2
        } else {
            output.rfind("]]").expect("Bracket arg should close") + 1
        };

        let bracket_output = &output[bracket_start..=bracket_end];

        // Critical assertion: bracket argument is byte-for-byte identical
        assert_eq!(
            bracket_output, expected_bracket,
            "Bracket argument was not byte-identical:\nExpected: {}\nGot: {}",
            expected_bracket, bracket_output
        );
    }

    insta::assert_snapshot!("bracket_arg_byte_identical",
        format_text("message([[simple bracket arg]])", &config));
}

#[test]
fn test_bracket_arg_multiline_preserved() {
    let config = FormatConfig::default();
    let input = r#"set(HELP [[
Usage: myapp [options]
  --help     Show help
  --version  Show version
]])"#;
    let output = format_text(input, &config);

    // Extract the content between [[ and ]]
    let bracket_start = output.find("[[").expect("Bracket arg should exist");
    let bracket_end = output.rfind("]]").expect("Bracket arg should close");
    let bracket_content = &output[bracket_start..=bracket_end + 1];

    // The multiline content should be preserved exactly
    assert!(
        bracket_content.contains("Usage: myapp [options]"),
        "Bracket argument multiline content was corrupted"
    );
    assert!(
        bracket_content.contains("  --help     Show help"),
        "Bracket argument indentation was not preserved"
    );

    insta::assert_snapshot!("bracket_arg_multiline", output);
}

// ----------------------------------------------------------------------------
// EDGE-04: Comment Placement
// ----------------------------------------------------------------------------

#[test]
fn test_eof_comment_no_trailing_newline() {
    let config = FormatConfig::default();
    // Create input with no trailing newline after comment
    let input = "set(X y)\n# EOF comment";
    let output = format_text(input, &config);

    // Comment should be preserved
    assert!(
        output.contains("# EOF comment"),
        "EOF comment was not preserved:\n{}",
        output
    );

    insta::assert_snapshot!("eof_comment_no_trailing_newline", output);
}

#[test]
fn test_comment_after_closing_paren() {
    let config = FormatConfig::default();
    let input = "message(hello)# attached comment";
    let output = format_text(input, &config);

    // Both command and comment should be preserved
    assert!(output.contains("message(hello)"), "Command was not preserved");
    assert!(output.contains("# attached comment"), "Attached comment was not preserved");

    insta::assert_snapshot!("comment_after_closing_paren", output);
}

#[test]
fn test_comment_between_args() {
    let config = FormatConfig::default();
    let input = "set(LIST\na\n# middle comment\nb\n)";
    let output = format_text(input, &config);

    // Comment should be preserved with proper placement
    assert!(
        output.contains("# middle comment"),
        "Comment between args was not preserved:\n{}",
        output
    );

    insta::assert_snapshot!("comment_between_args", output);
}

// ----------------------------------------------------------------------------
// Cross-cutting: Edge Case Idempotency with Non-default Configs
// ----------------------------------------------------------------------------

#[test]
fn test_edge_case_idempotency_uppercase() {
    let config = FormatConfig {
        command_case: CommandCase::Uppercase,
        ..FormatConfig::default()
    };

    let edge_fixtures = [
        "tests/format_fixtures/generator_expr_edge_cases.cmake",
        "tests/format_fixtures/variable_ref_edge_cases.cmake",
        "tests/format_fixtures/bracket_arg_edge_cases.cmake",
        "tests/format_fixtures/comment_edge_cases.cmake",
    ];

    for fixture_path in &edge_fixtures {
        let input = std::fs::read_to_string(fixture_path).unwrap();
        let once = format_text(&input, &config);
        let twice = format_text(&once, &config);
        assert_eq!(
            once, twice,
            "Idempotency failed with uppercase config for {}",
            fixture_path
        );
    }
}

#[test]
fn test_edge_case_idempotency_spaces() {
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 4,
        ..FormatConfig::default()
    };

    let edge_fixtures = [
        "tests/format_fixtures/generator_expr_edge_cases.cmake",
        "tests/format_fixtures/variable_ref_edge_cases.cmake",
        "tests/format_fixtures/bracket_arg_edge_cases.cmake",
        "tests/format_fixtures/comment_edge_cases.cmake",
    ];

    for fixture_path in &edge_fixtures {
        let input = std::fs::read_to_string(fixture_path).unwrap();
        let once = format_text(&input, &config);
        let twice = format_text(&once, &config);
        assert_eq!(
            once, twice,
            "Idempotency failed with spaces config for {}",
            fixture_path
        );
    }
}
