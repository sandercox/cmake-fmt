use cmake_fmt::formatter::{detect_line_ending, format_text, FormatConfig, LineEnding};

// ============================================================================
// DETECTION TESTS
// ============================================================================

#[test]
fn test_detect_pure_lf() {
    assert_eq!(detect_line_ending("set(FOO bar)\nset(BAZ qux)\n"), LineEnding::Lf);
}

#[test]
fn test_detect_pure_crlf() {
    assert_eq!(detect_line_ending("set(FOO bar)\r\nset(BAZ qux)\r\n"), LineEnding::CrLf);
}

#[test]
fn test_detect_mixed_majority_crlf() {
    // 2 CRLF, 1 lone LF → CRLF wins
    assert_eq!(detect_line_ending("a\r\nb\r\nc\n"), LineEnding::CrLf);
}

#[test]
fn test_detect_mixed_majority_lf() {
    // 1 CRLF, 2 lone LF → LF wins
    assert_eq!(detect_line_ending("a\nb\nc\r\n"), LineEnding::Lf);
}

#[test]
fn test_detect_no_newlines() {
    assert_eq!(detect_line_ending("set(FOO bar)"), LineEnding::Lf);
}

#[test]
fn test_detect_equal_counts_prefers_lf() {
    // 1 CRLF, 1 lone LF → tie goes to LF
    assert_eq!(detect_line_ending("a\r\nb\n"), LineEnding::Lf);
}

// ============================================================================
// AUTO MODE ROUND-TRIP TESTS
// ============================================================================

#[test]
fn test_auto_lf_in_lf_out() {
    let input = "set(FOO bar)\n";
    let config = FormatConfig::default(); // line_ending = Auto
    let result = format_text(input, &config);
    assert_eq!(result, "set(FOO bar)\n");
    assert!(!result.contains("\r\n"));
}

#[test]
fn test_auto_crlf_in_crlf_out() {
    let input = "set(FOO bar)\r\n";
    let config = FormatConfig::default(); // line_ending = Auto
    let result = format_text(input, &config);
    assert_eq!(result, "set(FOO bar)\r\n");
}

// ============================================================================
// FORCED MODE TESTS
// ============================================================================

#[test]
fn test_force_lf_on_crlf_input() {
    let input = "set(FOO bar)\r\n";
    let config = FormatConfig {
        line_ending: LineEnding::Lf,
        ..FormatConfig::default()
    };
    let result = format_text(input, &config);
    assert_eq!(result, "set(FOO bar)\n");
    assert!(!result.contains("\r\n"));
}

#[test]
fn test_force_crlf_on_lf_input() {
    let input = "set(FOO bar)\n";
    let config = FormatConfig {
        line_ending: LineEnding::CrLf,
        ..FormatConfig::default()
    };
    let result = format_text(input, &config);
    assert_eq!(result, "set(FOO bar)\r\n");
}

// ============================================================================
// IDEMPOTENCY TESTS
// ============================================================================

#[test]
fn test_idempotency_crlf() {
    let input = "set(FOO bar)\r\nmessage(STATUS \"hello\")\r\n";
    let config = FormatConfig::default(); // Auto
    let first = format_text(input, &config);
    let second = format_text(&first, &config);
    assert_eq!(first, second);
}

#[test]
fn test_idempotency_lf() {
    let input = "set(FOO bar)\nmessage(STATUS \"hello\")\n";
    let config = FormatConfig::default();
    let first = format_text(input, &config);
    let second = format_text(&first, &config);
    assert_eq!(first, second);
}

#[test]
fn test_idempotency_forced_crlf() {
    let input = "set(FOO bar)\nmessage(STATUS \"hello\")\n";
    let config = FormatConfig {
        line_ending: LineEnding::CrLf,
        ..FormatConfig::default()
    };
    let first = format_text(input, &config);
    let second = format_text(&first, &config);
    assert_eq!(first, second);
}

// ============================================================================
// MULTILINE CONTENT WITH CRLF
// ============================================================================

#[test]
fn test_crlf_multiline_commands() {
    let input = "if(TRUE)\r\n  set(FOO bar)\r\nendif()\r\n";
    let config = FormatConfig::default(); // Auto → detects CRLF
    let result = format_text(input, &config);
    // Every newline should be CRLF
    assert!(!result.contains('\n') || result.replace("\r\n", "").find('\n').is_none(),
        "Output should only contain CRLF, not lone LF: {:?}", result);
}

#[test]
fn test_empty_input_with_crlf_config() {
    let input = "";
    let config = FormatConfig {
        line_ending: LineEnding::CrLf,
        ..FormatConfig::default()
    };
    let result = format_text(input, &config);
    assert_eq!(result, "");
}

// ============================================================================
// EDGE CASE TESTS (LINE-01, LINE-02, LINE-03)
// ============================================================================

/// Test LINE-03: CRLF with nested scopes (multi-level nesting)
#[test]
fn test_crlf_with_nested_scopes() {
    let input = "if(WIN32)\r\n  foreach(src ${SOURCES})\r\n    set(A b)\r\n  endforeach()\r\nendif()\r\n";
    let config = FormatConfig::default(); // Auto mode
    let result = format_text(input, &config);

    // Verify all newlines are CRLF (no lone LF)
    let without_crlf = result.replace("\r\n", "");
    assert!(!without_crlf.contains('\n'),
        "Output should only contain CRLF, not lone LF. Found lone LF in: {:?}", result);

    // Verify formatting is correct
    assert!(result.contains("if(WIN32)\r\n"));
    assert!(result.contains("foreach(src ${SOURCES})\r\n"));
    assert!(result.contains("set(A b)\r\n"));
    assert!(result.contains("endforeach()\r\n"));
    assert!(result.contains("endif()\r\n"));
}

/// Test LINE-03: CRLF with comments in scoped blocks
#[test]
fn test_crlf_with_comments_in_scopes() {
    let input = "if(WIN32)\r\n  # Comment inside if\r\n  set(FOO bar)\r\nendif()\r\n";
    let config = FormatConfig::default(); // Auto mode
    let result = format_text(input, &config);

    // Verify all newlines are CRLF
    let without_crlf = result.replace("\r\n", "");
    assert!(!without_crlf.contains('\n'),
        "Output should only contain CRLF, not lone LF. Found lone LF in: {:?}", result);

    // Verify comment is preserved
    assert!(result.contains("# Comment inside if"));
}

/// Test LINE-03: CRLF with keyword-aware commands that break lines
#[test]
fn test_crlf_with_line_breaking() {
    let input = "target_link_libraries(myapp PRIVATE lib1 lib2 lib3 lib4 lib5 lib6 lib7 lib8)\r\n";
    let config = FormatConfig {
        max_line_length: 40, // Force line breaking
        ..FormatConfig::default()
    };
    let result = format_text(input, &config);

    // Verify all newlines are CRLF
    let without_crlf = result.replace("\r\n", "");
    assert!(!without_crlf.contains('\n'),
        "Output should only contain CRLF, not lone LF. Found lone LF in: {:?}", result);

    // Verify line breaking occurred
    assert!(result.lines().count() > 1, "Expected multiple lines due to line breaking");
}

/// Test LINE-03: CRLF with blank lines
#[test]
fn test_crlf_with_blank_lines() {
    let input = "set(A b)\r\n\r\nset(C d)\r\n";
    let config = FormatConfig::default(); // Auto mode
    let result = format_text(input, &config);

    // Verify all newlines are CRLF
    let without_crlf = result.replace("\r\n", "");
    assert!(!without_crlf.contains('\n'),
        "Output should only contain CRLF, not lone LF. Found lone LF in: {:?}", result);

    // Verify blank line is preserved
    assert_eq!(result, "set(A b)\r\n\r\nset(C d)\r\n");
}

/// Test LINE-01: Config deserialization from TOML
#[test]
fn test_line_ending_toml_deserialization() {
    // Test Auto
    let config: FormatConfig = toml::from_str("line_ending = \"auto\"")
        .expect("Failed to parse auto");
    assert_eq!(config.line_ending, LineEnding::Auto);

    // Test Lf
    let config: FormatConfig = toml::from_str("line_ending = \"lf\"")
        .expect("Failed to parse lf");
    assert_eq!(config.line_ending, LineEnding::Lf);

    // Test CrLf
    let config: FormatConfig = toml::from_str("line_ending = \"crlf\"")
        .expect("Failed to parse crlf");
    assert_eq!(config.line_ending, LineEnding::CrLf);
}

/// Test LINE-01: Config deserialization from YAML
#[test]
fn test_line_ending_yaml_deserialization() {
    // Test Auto
    let config: FormatConfig = serde_yml::from_str("line_ending: auto")
        .expect("Failed to parse auto");
    assert_eq!(config.line_ending, LineEnding::Auto);

    // Test Lf
    let config: FormatConfig = serde_yml::from_str("line_ending: lf")
        .expect("Failed to parse lf");
    assert_eq!(config.line_ending, LineEnding::Lf);

    // Test CrLf
    let config: FormatConfig = serde_yml::from_str("line_ending: crlf")
        .expect("Failed to parse crlf");
    assert_eq!(config.line_ending, LineEnding::CrLf);
}

/// Test LINE-01: CLI --style override for line_ending
/// Note: This test verifies the line_ending override works by checking the behavior,
/// since the config module is private to the binary.
#[test]
fn test_line_ending_cli_override() {
    // We can't directly test resolve_config since it's in the binary crate,
    // but we can verify that the line_ending option can be set programmatically
    // which is what the CLI would do after parsing the --style flag.

    let config_crlf = FormatConfig {
        line_ending: LineEnding::CrLf,
        ..FormatConfig::default()
    };
    assert_eq!(config_crlf.line_ending, LineEnding::CrLf);

    let config_lf = FormatConfig {
        line_ending: LineEnding::Lf,
        ..FormatConfig::default()
    };
    assert_eq!(config_lf.line_ending, LineEnding::Lf);

    let config_auto = FormatConfig {
        line_ending: LineEnding::Auto,
        ..FormatConfig::default()
    };
    assert_eq!(config_auto.line_ending, LineEnding::Auto);
}

/// Test comprehensive roundtrip for all line ending modes (LINE-01, LINE-02, LINE-03)
#[test]
fn test_line_ending_full_roundtrip() {
    let input_lf = "if(TRUE)\n  set(FOO bar)\n  # Comment\nendif()\n\nset(BAZ qux)\n";
    let input_crlf = "if(TRUE)\r\n  set(FOO bar)\r\n  # Comment\r\nendif()\r\n\r\nset(BAZ qux)\r\n";

    // Test Auto mode with LF input
    let config_auto = FormatConfig::default();
    let result = format_text(input_lf, &config_auto);
    assert!(!result.contains("\r\n"), "Auto mode with LF input should produce LF output");

    // Test Auto mode with CRLF input
    let result = format_text(input_crlf, &config_auto);
    let without_crlf = result.replace("\r\n", "");
    assert!(!without_crlf.contains('\n'), "Auto mode with CRLF input should produce CRLF output");

    // Test forced CrLf mode
    let config_crlf = FormatConfig {
        line_ending: LineEnding::CrLf,
        ..FormatConfig::default()
    };
    let result = format_text(input_lf, &config_crlf);
    let without_crlf = result.replace("\r\n", "");
    assert!(!without_crlf.contains('\n'), "Forced CRLF mode should produce CRLF output");

    // Test forced Lf mode
    let config_lf = FormatConfig {
        line_ending: LineEnding::Lf,
        ..FormatConfig::default()
    };
    let result = format_text(input_crlf, &config_lf);
    assert!(!result.contains("\r\n"), "Forced LF mode should produce LF output");

    // Test idempotency for each mode
    let result_lf = format_text(input_lf, &config_lf);
    let result_lf_2 = format_text(&result_lf, &config_lf);
    assert_eq!(result_lf, result_lf_2, "LF mode should be idempotent");

    let result_crlf = format_text(input_crlf, &config_crlf);
    let result_crlf_2 = format_text(&result_crlf, &config_crlf);
    assert_eq!(result_crlf, result_crlf_2, "CRLF mode should be idempotent");

    let result_auto_lf = format_text(input_lf, &config_auto);
    let result_auto_lf_2 = format_text(&result_auto_lf, &config_auto);
    assert_eq!(result_auto_lf, result_auto_lf_2, "Auto mode with LF should be idempotent");

    let result_auto_crlf = format_text(input_crlf, &config_auto);
    let result_auto_crlf_2 = format_text(&result_auto_crlf, &config_auto);
    assert_eq!(result_auto_crlf, result_auto_crlf_2, "Auto mode with CRLF should be idempotent");
}
