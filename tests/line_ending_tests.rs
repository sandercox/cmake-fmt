use cmake_format::formatter::{detect_line_ending, format_text, FormatConfig, LineEnding};

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
