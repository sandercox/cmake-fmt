use cmake_fmt::formatter::{format_text, format_text_with_diagnostics, FormatConfig, SuppressionWarning};

// ============================================================================
// BLOCK SUPPRESSION TESTS (SUP-01, SUP-03)
// ============================================================================

#[test]
fn test_block_suppression_preserves_formatting() {
    let config = FormatConfig::default();
    let input = r#"set(FORMATTED value)

# cmake-fmt: off
set(  UGLY_VAR    value1   value2  )
message(   "unformatted"   )
# cmake-fmt: on

set(BACK_TO_NORMAL value)
"#;
    let output = format_text(input, &config);

    // Commands outside the suppressed region should be formatted
    assert!(output.contains("set(FORMATTED value)"), "Command before block should be formatted");
    assert!(output.contains("set(BACK_TO_NORMAL value)"), "Command after block should be formatted");

    // Commands inside the suppressed region should preserve exact whitespace
    assert!(output.contains("set(  UGLY_VAR    value1   value2  )"),
        "Suppressed command should preserve original formatting");
    assert!(output.contains("message(   \"unformatted\"   )"),
        "Suppressed command should preserve original formatting");

    // Directive comments should be preserved
    assert!(output.contains("# cmake-fmt: off"));
    assert!(output.contains("# cmake-fmt: on"));
}

#[test]
fn test_block_suppression_multiple_regions() {
    let config = FormatConfig::default();
    let input = r#"set(NORMAL1 value)

# cmake-fmt: off
set(  UGLY1   a   b  )
# cmake-fmt: on

set(NORMAL2 value)

# cmake-fmt: off
set(  UGLY2   c   d  )
# cmake-fmt: on

set(NORMAL3 value)
"#;
    let output = format_text(input, &config);

    // Normal commands should be formatted
    assert!(output.contains("set(NORMAL1 value)"));
    assert!(output.contains("set(NORMAL2 value)"));
    assert!(output.contains("set(NORMAL3 value)"));

    // Both suppressed regions should preserve ugly formatting
    assert!(output.contains("set(  UGLY1   a   b  )"));
    assert!(output.contains("set(  UGLY2   c   d  )"));
}

#[test]
fn test_block_suppression_preserves_indentation() {
    let config = FormatConfig::default();
    let input = r#"if(CONDITION)
	set(NORMAL value)

	# cmake-fmt: off
	set(  UGLY_INDENTED    value1   value2  )
	# cmake-fmt: on

	set(BACK_TO_NORMAL value)
endif()
"#;
    let output = format_text(input, &config);

    // Suppressed command should keep its original indentation and spacing
    assert!(output.contains("set(  UGLY_INDENTED    value1   value2  )"),
        "Indented suppressed command should preserve original formatting");
}

// ============================================================================
// SKIP SUPPRESSION TESTS (SUP-02, SUP-03)
// ============================================================================

#[test]
fn test_skip_preserves_next_command() {
    let config = FormatConfig::default();
    let input = r#"set(BEFORE value)

# cmake-fmt: skip
set(  SKIPPED_VAR    value1   value2  )

set(AFTER value)
"#;
    let output = format_text(input, &config);

    // Commands before and after should be formatted
    assert!(output.contains("set(BEFORE value)"));
    assert!(output.contains("set(AFTER value)"));

    // Skipped command should preserve exact formatting
    assert!(output.contains("set(  SKIPPED_VAR    value1   value2  )"),
        "Skipped command should preserve original formatting");

    // Skip directive should be preserved
    assert!(output.contains("# cmake-fmt: skip"));
}

#[test]
fn test_skip_only_affects_one_command() {
    let config = FormatConfig::default();
    let input = r#"# cmake-fmt: skip
set(  SKIPPED    a   b  )
set(  NOT_SKIPPED    c   d  )
"#;
    let output = format_text(input, &config);

    // First command should be skipped
    assert!(output.contains("set(  SKIPPED    a   b  )"),
        "First command should be skipped");

    // Second command should be formatted normally
    assert!(output.contains("set(NOT_SKIPPED c d)"),
        "Second command should be formatted normally");
}

#[test]
fn test_skip_with_keyword_command() {
    let config = FormatConfig::default();
    let input = r#"# cmake-fmt: skip
target_link_libraries(  mylib    PUBLIC   foo   bar  )

target_link_libraries(  formatted    PUBLIC   baz  )
"#;
    let output = format_text(input, &config);

    // Skipped keyword command should preserve ugly formatting
    assert!(output.contains("target_link_libraries(  mylib    PUBLIC   foo   bar  )"),
        "Skipped keyword command should preserve original formatting");

    // Next keyword command should be formatted (normalized whitespace, even if not multiline)
    // Just verify it's different from the skipped one and doesn't have excessive spaces
    assert!(!output.contains("target_link_libraries(  formatted    PUBLIC   baz  )"),
        "Non-skipped keyword command should not preserve ugly formatting");
    assert!(output.contains("target_link_libraries(formatted PUBLIC baz)"),
        "Non-skipped keyword command should be formatted normally");
}

// ============================================================================
// DIRECTIVE PRESERVATION TESTS (SUP-05)
// ============================================================================

#[test]
fn test_directives_preserved_in_output() {
    let config = FormatConfig::default();
    let input = r#"set(A value)

# cmake-fmt: off
set(  B   value  )
# cmake-fmt: on

# cmake-fmt: skip
set(  C   value  )

set(D value)
"#;
    let output = format_text(input, &config);

    // All three directive types should be preserved
    assert!(output.contains("# cmake-fmt: off"), "off directive should be preserved");
    assert!(output.contains("# cmake-fmt: on"), "on directive should be preserved");
    assert!(output.contains("# cmake-fmt: skip"), "skip directive should be preserved");
}

#[test]
fn test_directive_comment_not_duplicated() {
    let config = FormatConfig::default();
    let input = r#"# cmake-fmt: off
set(  UGLY   value  )
# cmake-fmt: on
"#;
    let output = format_text(input, &config);

    // Each directive should appear exactly once
    assert_eq!(output.matches("# cmake-fmt: off").count(), 1,
        "off directive should appear exactly once");
    assert_eq!(output.matches("# cmake-fmt: on").count(), 1,
        "on directive should appear exactly once");
}

// ============================================================================
// WARNING TESTS (SUP-04)
// ============================================================================

#[test]
fn test_unclosed_region_warning() {
    let config = FormatConfig::default();
    let input = r#"set(NORMAL value)

# cmake-fmt: off
set(  UGLY   value  )

set(ALSO_UGLY value)
"#;
    let (_output, warnings) = format_text_with_diagnostics(input, &config);

    // Should have exactly one UnclosedRegion warning
    assert_eq!(warnings.len(), 1, "Should have exactly one warning");

    match &warnings[0] {
        SuppressionWarning::UnclosedRegion { start_line } => {
            assert_eq!(*start_line, 3, "Warning should point to line 3 where 'off' appears");
        }
        _ => panic!("Expected UnclosedRegion warning, got {:?}", warnings[0]),
    }
}

#[test]
fn test_unmatched_on_warning() {
    let config = FormatConfig::default();
    let input = r#"set(NORMAL value)

# cmake-fmt: on
set(AFTER value)
"#;
    let (_output, warnings) = format_text_with_diagnostics(input, &config);

    // Should have exactly one UnmatchedOn warning
    assert_eq!(warnings.len(), 1, "Should have exactly one warning");

    match &warnings[0] {
        SuppressionWarning::UnmatchedOn { line } => {
            assert_eq!(*line, 3, "Warning should point to line 3 where 'on' appears");
        }
        _ => panic!("Expected UnmatchedOn warning, got {:?}", warnings[0]),
    }
}

#[test]
fn test_nested_off_warning() {
    let config = FormatConfig::default();
    let input = r#"# cmake-fmt: off
set(  UGLY1   value  )

# cmake-fmt: off
set(  UGLY2   value  )
"#;
    let (_output, warnings) = format_text_with_diagnostics(input, &config);

    // Should have NestedOff warning + UnclosedRegion warning
    assert_eq!(warnings.len(), 2, "Should have two warnings");

    // Check for NestedOff warning
    let has_nested_off = warnings.iter().any(|w| matches!(w, SuppressionWarning::NestedOff { line: 4 }));
    assert!(has_nested_off, "Should have NestedOff warning on line 4");

    // Check for UnclosedRegion warning
    let has_unclosed = warnings.iter().any(|w| matches!(w, SuppressionWarning::UnclosedRegion { start_line: 1 }));
    assert!(has_unclosed, "Should have UnclosedRegion warning starting at line 1");
}

#[test]
fn test_no_warnings_for_valid_usage() {
    let config = FormatConfig::default();
    let input = r#"set(NORMAL value)

# cmake-fmt: off
set(  UGLY   value  )
# cmake-fmt: on

# cmake-fmt: skip
set(  ALSO_UGLY   value  )

set(BACK_TO_NORMAL value)
"#;
    let (_output, warnings) = format_text_with_diagnostics(input, &config);

    // Should have no warnings
    assert!(warnings.is_empty(), "Valid suppression usage should produce no warnings");
}

// ============================================================================
// IDEMPOTENCY TESTS
// ============================================================================

#[test]
fn test_suppression_idempotent_block() {
    let config = FormatConfig::default();
    let input = std::fs::read_to_string("tests/format_fixtures/suppression_block.cmake").unwrap();

    let once = format_text(&input, &config);
    let twice = format_text(&once, &config);

    assert_eq!(once, twice, "Block suppression should be idempotent");
}

#[test]
fn test_suppression_idempotent_skip() {
    let config = FormatConfig::default();
    let input = std::fs::read_to_string("tests/format_fixtures/suppression_skip.cmake").unwrap();

    let once = format_text(&input, &config);
    let twice = format_text(&once, &config);

    assert_eq!(once, twice, "Skip suppression should be idempotent");
}

#[test]
fn test_suppression_idempotent_mixed() {
    let config = FormatConfig::default();
    let input = std::fs::read_to_string("tests/format_fixtures/suppression_mixed.cmake").unwrap();

    let once = format_text(&input, &config);
    let twice = format_text(&once, &config);

    assert_eq!(once, twice, "Mixed suppression should be idempotent");
}

// ============================================================================
// SNAPSHOT TESTS
// ============================================================================
// Note: The three new fixtures (suppression_block.cmake, suppression_skip.cmake,
// suppression_mixed.cmake) are automatically picked up by the glob pattern in
// formatting_snapshot_tests.rs. We run cargo insta test to generate snapshots.
