use cmake_fmt::formatter::{
    CommentStyle, FormatConfig, FormatWarning, format_text, format_text_with_diagnostics,
};

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
    assert!(
        output.contains("set(FORMATTED value)"),
        "Command before block should be formatted"
    );
    assert!(
        output.contains("set(BACK_TO_NORMAL value)"),
        "Command after block should be formatted"
    );

    // Commands inside the suppressed region should preserve exact whitespace
    assert!(
        output.contains("set(  UGLY_VAR    value1   value2  )"),
        "Suppressed command should preserve original formatting"
    );
    assert!(
        output.contains("message(   \"unformatted\"   )"),
        "Suppressed command should preserve original formatting"
    );

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
    assert!(
        output.contains("set(  UGLY_INDENTED    value1   value2  )"),
        "Indented suppressed command should preserve original formatting"
    );
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
    assert!(
        output.contains("set(  SKIPPED_VAR    value1   value2  )"),
        "Skipped command should preserve original formatting"
    );

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
    assert!(
        output.contains("set(  SKIPPED    a   b  )"),
        "First command should be skipped"
    );

    // Second command should be formatted normally
    assert!(
        output.contains("set(NOT_SKIPPED c d)"),
        "Second command should be formatted normally"
    );
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
    assert!(
        output.contains("target_link_libraries(  mylib    PUBLIC   foo   bar  )"),
        "Skipped keyword command should preserve original formatting"
    );

    // Next keyword command should be formatted (short command fits on one line)
    assert!(
        !output.contains("target_link_libraries(  formatted    PUBLIC   baz  )"),
        "Non-skipped keyword command should not preserve ugly formatting"
    );
    assert!(
        output.contains("target_link_libraries(formatted PUBLIC baz)"),
        "Non-skipped keyword command should be formatted normally"
    );
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
    assert!(
        output.contains("# cmake-fmt: off"),
        "off directive should be preserved"
    );
    assert!(
        output.contains("# cmake-fmt: on"),
        "on directive should be preserved"
    );
    assert!(
        output.contains("# cmake-fmt: skip"),
        "skip directive should be preserved"
    );
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
    assert_eq!(
        output.matches("# cmake-fmt: off").count(),
        1,
        "off directive should appear exactly once"
    );
    assert_eq!(
        output.matches("# cmake-fmt: on").count(),
        1,
        "on directive should appear exactly once"
    );
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

    // Unclosed regions no longer produce warnings - it's valid to leave cmake-fmt: off open at EOF
    assert_eq!(
        warnings.len(),
        0,
        "Should have no warnings for unclosed region at EOF"
    );
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
        FormatWarning::UnmatchedOn { line } => {
            assert_eq!(
                *line, 3,
                "Warning should point to line 3 where 'on' appears"
            );
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

    // Should have NestedOff warning only (unclosed regions no longer warn)
    assert_eq!(warnings.len(), 1, "Should have one warning");

    // Check for NestedOff warning
    let has_nested_off = warnings
        .iter()
        .any(|w| matches!(w, FormatWarning::NestedOff { line: 4 }));
    assert!(has_nested_off, "Should have NestedOff warning on line 4");
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
    assert!(
        warnings.is_empty(),
        "Valid suppression usage should produce no warnings"
    );
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

// ============================================================================
// INLINE STYLE OVERRIDE TESTS
// ============================================================================

#[test]
fn test_inline_style_override_indent_width() {
    let config = FormatConfig {
        use_tabs: false, // Use spaces so we can test indent_width
        indent_width: 4, // Default
        ..Default::default()
    };

    let input = r#"# cmake-fmt: indent_width=2
if(CONDITION)
    set(MY_VAR value)
endif()
"#;
    let output = format_text(input, &config);

    // Should use 2-space indentation (not 4)
    assert!(
        output.contains("  set(MY_VAR value)"),
        "Should use 2-space indentation after override"
    );
    assert!(
        !output.contains("    set(MY_VAR value)"),
        "Should not use 4-space indentation"
    );

    // Directive should be preserved
    assert!(output.contains("# cmake-fmt: indent_width=2"));
}

#[test]
fn test_inline_style_override_max_line_length() {
    let config = FormatConfig {
        max_line_length: 80, // Default
        ..Default::default()
    };

    let input = r#"# cmake-fmt: max_line_length=40
target_link_libraries(mylib PUBLIC foo bar baz qux)
"#;
    let output = format_text(input, &config);

    // Should break to multiline due to 40-char limit
    assert!(
        output.contains("target_link_libraries(mylib\n")
            || output.contains("target_link_libraries(mylib\t"),
        "Should break to multiline with 40-char limit"
    );
}

#[test]
fn test_inline_style_override_midfile() {
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 4,
        ..Default::default()
    };

    let input = r#"if(FIRST)
    set(VAR1 value)
endif()

# cmake-fmt: indent_width=2
if(SECOND)
    set(VAR2 value)
endif()
"#;
    let output = format_text(input, &config);

    // First block should use 4 spaces (original config)
    assert!(
        output.contains("    set(VAR1 value)"),
        "First block should use 4-space indentation"
    );

    // Second block should use 2 spaces (after override)
    assert!(
        output.contains("  set(VAR2 value)"),
        "Second block should use 2-space indentation after override"
    );
}

#[test]
fn test_inline_style_override_multiple() {
    let config = FormatConfig {
        use_tabs: true,
        indent_width: 4,
        ..Default::default()
    };

    let input = r#"# cmake-fmt: indent_width=2
# cmake-fmt: use_tabs=false
if(CONDITION)
    set(MY_VAR value)
endif()
"#;
    let output = format_text(input, &config);

    // Should use 2 spaces (not tabs, not 4 spaces)
    assert!(
        output.contains("  set(MY_VAR value)"),
        "Should use 2-space indentation after both overrides"
    );
    assert!(
        !output.contains("\tset(MY_VAR value)"),
        "Should not use tabs after use_tabs=false override"
    );
}

#[test]
fn test_inline_style_override_idempotent() {
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 4,
        ..Default::default()
    };

    let input = r#"# cmake-fmt: indent_width=2
if(CONDITION)
    set(MY_VAR value)
endif()
"#;
    let once = format_text(input, &config);
    let twice = format_text(&once, &config);

    assert_eq!(
        once, twice,
        "Style override formatting should be idempotent"
    );
}

#[test]
fn test_inline_style_override_with_suppression() {
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 4,
        ..Default::default()
    };

    let input = r#"# cmake-fmt: indent_width=2
if(CONDITION)
    set(FORMATTED value)
endif()

# cmake-fmt: off
if(  UGLY  )
    set(  UGLY_VAR    value  )
endif()
# cmake-fmt: on

if(BACK_TO_FORMATTED)
    set(ALSO_FORMATTED value)
endif()
"#;
    let output = format_text(input, &config);

    // First block should use 2-space indentation (style override)
    assert!(
        output.contains("  set(FORMATTED value)"),
        "First block should use overridden indent_width"
    );

    // Suppressed block should preserve ugly formatting
    assert!(
        output.contains("set(  UGLY_VAR    value  )"),
        "Suppressed block should preserve original formatting"
    );

    // Last block should also use 2-space indentation (style override persists)
    assert!(
        output.contains("  set(ALSO_FORMATTED value)"),
        "Last block should still use overridden indent_width"
    );
}

#[test]
fn test_inline_style_override_invalid_key_no_crash() {
    let config = FormatConfig::default();
    let input = r#"# cmake-fmt: nonexistent_key=value
set(MY_VAR value)
"#;
    // Should not crash, just warn
    let output = format_text(input, &config);

    // Comment should be preserved
    assert!(output.contains("# cmake-fmt: nonexistent_key=value"));

    // Command should be formatted normally
    assert!(output.contains("set(MY_VAR value)"));
}

#[test]
fn test_inline_style_override_preserved_in_output() {
    let config = FormatConfig {
        use_tabs: false,
        ..Default::default()
    };

    let input = r#"# cmake-fmt: indent_width=2
if(CONDITION)
    set(MY_VAR value)
endif()
"#;
    let output = format_text(input, &config);

    // The style override comment itself should appear in output
    assert!(
        output.contains("# cmake-fmt: indent_width=2"),
        "Style override comment should be preserved in output"
    );
}

// ============================================================================
// COMMENT NORMALIZATION SUPPRESSION TESTS
// ============================================================================

#[test]
fn test_suppression_off_prevents_comment_normalization() {
    let config = FormatConfig {
        comment_style: CommentStyle::HashSpace,
        ..Default::default()
    };

    let input = r#"# cmake-fmt: off
#if (WIN32 OR APPLE)
#    target_link_libraries(Alley PUBLIC Sparkle BugSplat)
#endif()
# cmake-fmt: on
"#;
    let output = format_text(input, &config);

    // With comment_style=HashSpace, comments would normally be normalized to have space after #
    // But inside cmake-fmt:off region, they should be preserved exactly as-is
    assert!(
        output.contains("#if (WIN32 OR APPLE)"),
        "Suppressed comment should preserve no space after # in #if"
    );
    assert!(
        output.contains("#    target_link_libraries(Alley PUBLIC Sparkle BugSplat)"),
        "Suppressed comment should preserve exact whitespace (4 spaces after #)"
    );
    assert!(
        output.contains("#endif()"),
        "Suppressed comment should preserve no space after # in #endif"
    );
}

#[test]
fn test_suppression_off_at_end_of_file() {
    let config = FormatConfig {
        comment_style: CommentStyle::HashSpace,
        ..Default::default()
    };

    let input = r#"some_command()
# cmake-fmt: off
#if(FOO)
#    bar()
#endif()
"#;
    let (output, warnings) = format_text_with_diagnostics(input, &config);

    // Verify: no formatting of the comment lines, no warnings about unclosed region
    assert!(
        output.contains("#if(FOO)"),
        "Suppressed comment should preserve no space after # in #if"
    );
    assert!(
        output.contains("#    bar()"),
        "Suppressed comment should preserve exact whitespace"
    );
    assert!(
        output.contains("#endif()"),
        "Suppressed comment should preserve no space after # in #endif"
    );

    // No warning about unclosed cmake-fmt:off at EOF
    assert_eq!(
        warnings.len(),
        0,
        "Should have no warnings for unclosed cmake-fmt:off at EOF"
    );
}

#[test]
fn test_suppression_off_leading_comments_raw() {
    let config = FormatConfig {
        comment_style: CommentStyle::HashSpace,
        ..Default::default()
    };

    let input = r#"# cmake-fmt: off
#  indented comment
some_command(ARG1 ARG2)
# cmake-fmt: on
"#;
    let output = format_text(input, &config);

    // Verify the leading comment preserves its double space after #
    assert!(
        output.contains("#  indented comment"),
        "Suppressed leading comment should preserve double space after #"
    );
}
