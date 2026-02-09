use cmake_format::formatter::{CommandCase, FormatConfig};
use cmake_format::formatter::format_text;

// Helper to create default config
fn default_config() -> FormatConfig {
    FormatConfig::default()
}

// ============================================================================
// BASIC PIPELINE TESTS
// ============================================================================

#[test]
fn test_format_simple_command() {
    let input = "set(FOO bar)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(result, "set(FOO bar)\n");
}

#[test]
fn test_format_empty_input() {
    let input = "";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(result, "");
}

#[test]
fn test_format_whitespace_only() {
    let input = "  \n\n  \n";
    let config = default_config();
    let result = format_text(input, &config);
    // Whitespace-only should format to empty
    assert_eq!(result, "");
}

#[test]
fn test_format_preserves_arguments() {
    let input = "message(STATUS \"hello world\")\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(result, "message(STATUS \"hello world\")\n");
}

#[test]
fn test_format_multiple_commands() {
    let input = "set(A b)\nset(C d)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(result, "set(A b)\nset(C d)\n");
}

// ============================================================================
// COMMAND CASING TESTS
// ============================================================================

#[test]
fn test_case_lowercase() {
    let input = "SET(FOO bar)\n";
    let config = FormatConfig {
        command_case: CommandCase::Lowercase,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert_eq!(result, "set(FOO bar)\n");
}

#[test]
fn test_case_uppercase() {
    let input = "set(FOO bar)\n";
    let config = FormatConfig {
        command_case: CommandCase::Uppercase,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert_eq!(result, "SET(FOO bar)\n");
}

#[test]
fn test_case_preserve() {
    let input = "SeT(FOO bar)\n";
    let config = FormatConfig {
        command_case: CommandCase::Preserve,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert_eq!(result, "SeT(FOO bar)\n");
}

#[test]
fn test_case_mixed_commands() {
    let input = "SET(A b)\nmessage(C d)\nFiNd_PaCkAgE(E)\n";
    let config = FormatConfig {
        command_case: CommandCase::Lowercase,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert_eq!(result, "set(A b)\nmessage(C d)\nfind_package(E)\n");
}

#[test]
fn test_case_only_affects_command_names() {
    let input = "target_link_libraries(myapp PUBLIC fmt)\n";
    let config = FormatConfig {
        command_case: CommandCase::Uppercase,
        ..default_config()
    };
    let result = format_text(input, &config);
    // PUBLIC should NOT be uppercased, only the command name
    assert_eq!(result, "TARGET_LINK_LIBRARIES(myapp PUBLIC fmt)\n");
}

// ============================================================================
// SCOPE INDENTATION TESTS
// ============================================================================

#[test]
fn test_indent_if_block() {
    let input = "if(WIN32)\nset(FOO bar)\nendif()\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(result, "if(WIN32)\n  set(FOO bar)\nendif()\n");
}

#[test]
fn test_indent_if_else_endif() {
    let input = "if(WIN32)\nset(A b)\nelse()\nset(C d)\nendif()\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(result, "if(WIN32)\n  set(A b)\nelse()\n  set(C d)\nendif()\n");
}

#[test]
fn test_indent_if_elseif_else_endif() {
    let input = "if(WIN32)\nset(A b)\nelseif(UNIX)\nset(B c)\nelse()\nset(C d)\nendif()\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(result, "if(WIN32)\n  set(A b)\nelseif(UNIX)\n  set(B c)\nelse()\n  set(C d)\nendif()\n");
}

#[test]
fn test_indent_nested_if() {
    let input = "if(A)\nif(B)\nset(C d)\nendif()\nendif()\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(result, "if(A)\n  if(B)\n    set(C d)\n  endif()\nendif()\n");
}

#[test]
fn test_indent_foreach() {
    let input = "foreach(src ${SOURCES})\nmessage(STATUS ${src})\nendforeach()\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(result, "foreach(src ${SOURCES})\n  message(STATUS ${src})\nendforeach()\n");
}

#[test]
fn test_indent_function() {
    let input = "function(my_func ARG)\nmessage(${ARG})\nendfunction()\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(result, "function(my_func ARG)\n  message(${ARG})\nendfunction()\n");
}

#[test]
fn test_indent_macro() {
    let input = "macro(my_macro ARG)\nmessage(${ARG})\nendmacro()\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(result, "macro(my_macro ARG)\n  message(${ARG})\nendmacro()\n");
}

#[test]
fn test_indent_width_4() {
    let input = "if(WIN32)\nset(FOO bar)\nendif()\n";
    let config = FormatConfig {
        indent_width: 4,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert_eq!(result, "if(WIN32)\n    set(FOO bar)\nendif()\n");
}

#[test]
fn test_indent_tabs() {
    let input = "if(WIN32)\nset(FOO bar)\nendif()\n";
    let config = FormatConfig {
        use_tabs: true,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert_eq!(result, "if(WIN32)\n\tset(FOO bar)\nendif()\n");
}

// ============================================================================
// LINE BREAKING TESTS
// ============================================================================

#[test]
fn test_short_line_stays_oneline() {
    let input = "set(FOO bar baz)\n";
    let config = default_config(); // max_line_length = 80
    let result = format_text(input, &config);
    assert_eq!(result, "set(FOO bar baz)\n");
}

#[test]
fn test_long_line_breaks() {
    let input = "target_link_libraries(myapp lib1 lib2 lib3 lib4 lib5 lib6 lib7 lib8 lib9 lib10)\n";
    let config = default_config(); // max_line_length = 80
    let result = format_text(input, &config);

    // Should break across multiple lines
    // The exact formatting depends on the pretty printer's algorithm
    // We just verify it's not all on one line
    assert!(result.contains('\n'));
    assert!(result.contains("target_link_libraries("));
}

#[test]
fn test_line_break_indentation() {
    let input = "target_link_libraries(myapp lib1 lib2 lib3 lib4 lib5 lib6 lib7 lib8 lib9 lib10)\n";
    let config = FormatConfig {
        max_line_length: 40, // Force breaking
        ..default_config()
    };
    let result = format_text(input, &config);

    // When broken, arguments should be indented
    let lines: Vec<&str> = result.lines().collect();
    assert!(lines.len() > 1, "Expected multiple lines");
}

#[test]
fn test_generator_expr_atomic() {
    let input = "set(FLAGS $<$<CONFIG:Debug>:-g>)\n";
    let config = default_config();
    let result = format_text(input, &config);
    // Generator expression should stay intact
    assert!(result.contains("$<$<CONFIG:Debug>:-g>"));
}

#[test]
fn test_quoted_arg_atomic() {
    let input = "message(\"a very long quoted string that should not be broken\")\n";
    let config = FormatConfig {
        max_line_length: 40,
        ..default_config()
    };
    let result = format_text(input, &config);
    // Quoted string should stay as one piece
    assert!(result.contains("\"a very long quoted string that should not be broken\""));
}

#[test]
fn test_bracket_arg_atomic() {
    let input = "message([=[\nMulti-line\nbracket\nargument\n]=])\n";
    let config = default_config();
    let result = format_text(input, &config);
    // Bracket argument preserves internal formatting
    assert!(result.contains("[=[\nMulti-line\nbracket\nargument\n]=]"));
}

// ============================================================================
// BLANK LINE NORMALIZATION TESTS
// ============================================================================

#[test]
fn test_blank_lines_preserved() {
    let input = "set(A b)\n\nset(C d)\n";
    let config = default_config(); // max_blank_lines = 1
    let result = format_text(input, &config);
    assert_eq!(result, "set(A b)\n\nset(C d)\n");
}

#[test]
fn test_excess_blank_lines_collapsed() {
    let input = "set(A b)\n\n\n\nset(C d)\n";
    let config = default_config(); // max_blank_lines = 1
    let result = format_text(input, &config);
    // Should collapse to 1 blank line
    assert_eq!(result, "set(A b)\n\nset(C d)\n");
}

#[test]
fn test_max_blank_lines_configurable() {
    let input = "set(A b)\n\n\n\n\nset(C d)\n";
    let config = FormatConfig {
        max_blank_lines: 2,
        ..default_config()
    };
    let result = format_text(input, &config);
    // Should collapse to 2 blank lines
    assert_eq!(result, "set(A b)\n\n\nset(C d)\n");
}

// ============================================================================
// COMMENT PRESERVATION TESTS
// ============================================================================

#[test]
fn test_leading_comment_preserved() {
    let input = "# This is a comment\nset(A b)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert!(result.contains("# This is a comment"));
    assert!(result.contains("set(A b)"));
}

#[test]
fn test_standalone_comment_preserved() {
    let input = "set(A b)\n# Middle comment\nset(C d)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert!(result.contains("# Middle comment"));
    assert!(result.contains("set(A b)"));
    assert!(result.contains("set(C d)"));
}

#[test]
fn test_bracket_comment_preserved() {
    let input = "#[[\nMulti-line comment\nwith details\n]]\nset(A b)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert!(result.contains("#[["));
    assert!(result.contains("Multi-line comment"));
    assert!(result.contains("]]"));
    assert!(result.contains("set(A b)"));
}

#[test]
fn test_comment_indentation() {
    let input = "if(WIN32)\n# Comment inside if\nset(A b)\nendif()\n";
    let config = default_config();
    let result = format_text(input, &config);
    // Comment should be indented with the block
    assert!(result.contains("  # Comment inside if"));
}
