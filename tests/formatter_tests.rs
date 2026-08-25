use cmake_fmt::formatter::format_text;
use cmake_fmt::formatter::{
    ClosingStyle, CommandCase, CommentStyle, FinalNewline, FormatConfig, UserCommandCase,
};

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
    // find_package with single arg fits on one line
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
    // Short command fits on one line
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
    assert_eq!(result, "if(WIN32)\n\tset(FOO bar)\nendif()\n");
}

#[test]
fn test_indent_if_else_endif() {
    let input = "if(WIN32)\nset(A b)\nelse()\nset(C d)\nendif()\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(
        result,
        "if(WIN32)\n\tset(A b)\nelse()\n\tset(C d)\nendif()\n"
    );
}

#[test]
fn test_indent_if_elseif_else_endif() {
    let input = "if(WIN32)\nset(A b)\nelseif(UNIX)\nset(B c)\nelse()\nset(C d)\nendif()\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(
        result,
        "if(WIN32)\n\tset(A b)\nelseif(UNIX)\n\tset(B c)\nelse()\n\tset(C d)\nendif()\n"
    );
}

#[test]
fn test_indent_nested_if() {
    let input = "if(A)\nif(B)\nset(C d)\nendif()\nendif()\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(result, "if(A)\n\tif(B)\n\t\tset(C d)\n\tendif()\nendif()\n");
}

#[test]
fn test_indent_foreach() {
    let input = "foreach(src ${SOURCES})\nmessage(STATUS ${src})\nendforeach()\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(
        result,
        "foreach(src ${SOURCES})\n\tmessage(STATUS ${src})\nendforeach()\n"
    );
}

#[test]
fn test_indent_function() {
    let input = "function(my_func ARG)\nmessage(${ARG})\nendfunction()\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(
        result,
        "function(my_func ARG)\n\tmessage(${ARG})\nendfunction()\n"
    );
}

#[test]
fn test_indent_macro() {
    let input = "macro(my_macro ARG)\nmessage(${ARG})\nendmacro()\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(
        result,
        "macro(my_macro ARG)\n\tmessage(${ARG})\nendmacro()\n"
    );
}

#[test]
fn test_indent_width_4() {
    let input = "if(WIN32)\nset(FOO bar)\nendif()\n";
    let config = FormatConfig {
        indent_width: 4,
        use_tabs: false,
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
    assert!(result.contains("\t# Comment inside if"));
}

// ============================================================================
// COMMENT INDENTATION WITH TABS VS SPACES (CMNT-02)
// ============================================================================

/// Test CMNT-02: Comment indentation respects use_tabs=false with indent_width=2
#[test]
fn test_comment_indentation_with_spaces() {
    let input = "if(WIN32)\n# Comment inside if\nset(A b)\nendif()\n";
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 2,
        ..default_config()
    };
    let result = format_text(input, &config);
    // Comment should be indented with 2 spaces (not tab)
    assert!(
        result.contains("  # Comment inside if"),
        "Expected '  # Comment inside if' but got: {:?}",
        result
    );
    assert!(
        !result.contains("\t# Comment inside if"),
        "Should not contain tab before comment"
    );
}

/// Test CMNT-02: Comment indentation respects use_tabs=false with indent_width=4
#[test]
fn test_comment_indentation_with_spaces_4() {
    let input = "if(WIN32)\n# Comment inside if\nset(A b)\nendif()\n";
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 4,
        ..default_config()
    };
    let result = format_text(input, &config);
    // Comment should be indented with 4 spaces (not tab)
    assert!(
        result.contains("    # Comment inside if"),
        "Expected '    # Comment inside if' but got: {:?}",
        result
    );
    assert!(
        !result.contains("\t# Comment inside if"),
        "Should not contain tab before comment"
    );
}

/// Test CMNT-02: Nested comment indentation with tabs (2 levels)
#[test]
fn test_nested_comment_indentation_tabs() {
    let input = "if(A)\nif(B)\n# Deep comment\nset(X y)\nendif()\nendif()\n";
    let config = FormatConfig {
        use_tabs: true,
        ..default_config()
    };
    let result = format_text(input, &config);
    // Comment should be indented with 2 tabs (2 levels deep)
    assert!(
        result.contains("\t\t# Deep comment"),
        "Expected '\\t\\t# Deep comment' but got: {:?}",
        result
    );
}

/// Test CMNT-02: Nested comment indentation with spaces (2 levels)
#[test]
fn test_nested_comment_indentation_spaces() {
    let input = "if(A)\nif(B)\n# Deep comment\nset(X y)\nendif()\nendif()\n";
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 2,
        ..default_config()
    };
    let result = format_text(input, &config);
    // Comment should be indented with 4 spaces (2 levels * 2 width)
    assert!(
        result.contains("    # Deep comment"),
        "Expected '    # Deep comment' (4 spaces) but got: {:?}",
        result
    );
    assert!(
        !result.contains("\t\t# Deep comment"),
        "Should not contain tabs before comment"
    );
}

/// Test CMNT-02: Standalone comment (not leading a command) inside if block with spaces
#[test]
fn test_standalone_comment_indentation_spaces() {
    let input = "if(WIN32)\n# Standalone comment\nendif()\n";
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 4,
        ..default_config()
    };
    let result = format_text(input, &config);
    // Standalone comment should be indented with 4 spaces
    assert!(
        result.contains("    # Standalone comment"),
        "Expected '    # Standalone comment' but got: {:?}",
        result
    );
    assert!(
        !result.contains("\t# Standalone comment"),
        "Should not contain tab before standalone comment"
    );
}

// ============================================================================
// ARGUMENT LIST ENHANCEMENT TESTS (Phase 7)
// ============================================================================

/// Test CMNT-01: Comments inside argument lists are preserved
#[test]
fn test_arglist_comment_inside_set() {
    let input = "set(MY_LIST\n  item1\n  # Group A\n  item2\n  item3\n)\n";
    let config = default_config();
    let result = format_text(input, &config);

    // Comment should be present
    assert!(
        result.contains("# Group A"),
        "Comment should be preserved, got: {}",
        result
    );
    // All items should be present
    assert!(result.contains("item1"), "item1 should be present");
    assert!(result.contains("item2"), "item2 should be present");
    assert!(result.contains("item3"), "item3 should be present");

    // Comment should appear between item1 and item2
    let item1_pos = result.find("item1").unwrap();
    let comment_pos = result.find("# Group A").unwrap();
    let item2_pos = result.find("item2").unwrap();
    assert!(
        item1_pos < comment_pos && comment_pos < item2_pos,
        "Comment should appear between item1 and item2"
    );
}

/// Test CMNT-01: Comments are not duplicated
#[test]
fn test_arglist_comment_not_duplicated() {
    let input = "set(MY_LIST\n  item1\n  # Comment inside\n  item2\n)\n";
    let config = default_config();
    let result = format_text(input, &config);

    // Count occurrences of comment
    let count = result.matches("# Comment inside").count();
    assert_eq!(
        count, 1,
        "Comment should appear exactly once, got {} occurrences in: {}",
        count, result
    );
}

/// Test ARGL-01: Blank lines are preserved in argument lists
#[test]
fn test_arglist_blank_line_preserved() {
    let input = "set(SOURCES\n  src/a.cpp\n  src/b.cpp\n\n  src/c.cpp\n  src/d.cpp\n)\n";
    let config = default_config();
    let result = format_text(input, &config);

    // Should have a blank line between src/b.cpp and src/c.cpp
    // Look for pattern: b.cpp\n\n (two newlines)
    assert!(
        result.contains("src/b.cpp\n\n"),
        "Should have blank line after src/b.cpp, got: {}",
        result
    );
}

/// Test ARGL-01: Blank lines respect max_blank_lines config
#[test]
fn test_arglist_blank_line_respects_max() {
    let input = "set(SOURCES\n  src/a.cpp\n\n\n\n  src/b.cpp\n)\n";
    let config = FormatConfig {
        max_blank_lines: 1,
        ..default_config()
    };
    let result = format_text(input, &config);

    // Should have at most 1 blank line (2 consecutive newlines)
    assert!(
        !result.contains("\n\n\n"),
        "Should not have more than 1 blank line (max_blank_lines=1), got: {}",
        result
    );
    // Should still have 1 blank line
    assert!(
        result.contains("src/a.cpp\n\n"),
        "Should have 1 blank line preserved, got: {}",
        result
    );
}

/// Test ARGL-02: Multiline argument lists stay multiline
#[test]
fn test_arglist_multiline_stays_multiline() {
    let input = "set(SHORT_LIST\n  a\n  b\n)\n";
    let config = default_config();
    let result = format_text(input, &config);

    // Should stay multiline (not collapse to one line)
    assert!(
        result.contains("a\n"),
        "Should have newline after 'a', got: {}",
        result
    );
    assert!(
        result.contains("b\n"),
        "Should have newline after 'b', got: {}",
        result
    );
    assert!(
        !result.eq("set(SHORT_LIST a b)\n"),
        "Should NOT collapse to single line, got: {}",
        result
    );
}

/// Test ARGL-02: One-line argument lists stay one line
#[test]
fn test_arglist_oneline_stays_oneline() {
    let input = "set(SHORT_LIST a b)\n";
    let config = default_config();
    let result = format_text(input, &config);

    // Should stay on one line
    assert_eq!(result, "set(SHORT_LIST a b)\n", "Should stay on one line");
}

/// Test ARGL-03: First argument appears on same line as command name
#[test]
fn test_arglist_first_arg_same_line() {
    let input = "set(MY_LIST\n  item1\n  item2\n  item3\n)\n";
    let config = default_config();
    let result = format_text(input, &config);

    // First line should start with "set(MY_LIST"
    let first_line = result.lines().next().unwrap();
    assert!(
        first_line.starts_with("set(MY_LIST"),
        "First line should start with 'set(MY_LIST', got: {}",
        first_line
    );

    // Should NOT contain "set(\n" pattern
    assert!(
        !result.contains("set(\n"),
        "Should not have newline immediately after opening paren"
    );
}

/// Test ARGL-03: First argument same line even with comment
#[test]
fn test_arglist_first_arg_same_line_with_comment() {
    let input = "set(MY_LIST\n  item1\n  # comment\n  item2\n)\n";
    let config = default_config();
    let result = format_text(input, &config);

    // First line should start with "set(MY_LIST"
    let first_line = result.lines().next().unwrap();
    assert!(
        first_line.starts_with("set(MY_LIST"),
        "First line should start with 'set(MY_LIST' even with comment, got: {}",
        first_line
    );
}

/// Test combined: comment and blank line in argument list
#[test]
fn test_arglist_comment_and_blank_line() {
    let input = "set(SOURCES\n  src/a.cpp\n  # Group separator\n\n  src/b.cpp\n)\n";
    let config = default_config();
    let result = format_text(input, &config);

    // Comment should be preserved
    assert!(
        result.contains("# Group separator"),
        "Comment should be preserved"
    );

    // Blank line should be preserved
    assert!(result.contains("\n\n"), "Blank line should be preserved");

    // First arg on same line
    let first_line = result.lines().next().unwrap();
    assert!(
        first_line.starts_with("set(SOURCES"),
        "First arg should be on same line as command"
    );
}

/// Test idempotency: formatting twice produces same result
#[test]
fn test_arglist_idempotency() {
    let inputs = vec![
        "set(MY_LIST\n  item1\n  # Group A\n  item2\n)\n",
        "set(SOURCES\n  src/a.cpp\n\n  src/b.cpp\n)\n",
        "set(SHORT_LIST\n  a\n  b\n)\n",
        "set(MY_LIST\n  item1\n  item2\n)\n",
    ];

    let config = default_config();

    for input in inputs {
        let once = format_text(input, &config);
        let twice = format_text(&once, &config);
        assert_eq!(
            once, twice,
            "Formatting should be idempotent for input: {}",
            input
        );
    }
}

// ============================================================================
// USER COMMAND CASING TESTS
// ============================================================================

#[test]
fn test_builtin_lowercased_user_left_alone_default() {
    // Default config: command_case=Lowercase, user_command_case=Infer
    let input = "SET(X y)\nMyCmd(z)\n";
    let config = default_config();
    let result = format_text(input, &config);
    // SET is builtin -> lowercased; MyCmd is unknown user -> left as-is (infer, no def found)
    assert_eq!(result, "set(X y)\nMyCmd(z)\n");
}

#[test]
fn test_infer_with_function_definition() {
    let input = "function(MyHelper arg)\nmessage(${arg})\nendfunction()\nmyhelper(foo)\n";
    let config = default_config();
    let result = format_text(input, &config);
    // myhelper() call should be inferred as MyHelper from the definition
    assert!(
        result.contains("MyHelper(foo)"),
        "Should infer MyHelper casing from definition, got: {}",
        result
    );
}

#[test]
fn test_infer_with_macro_definition() {
    let input =
        "macro(GenerateCI target)\nadd_test(NAME ${target})\nendmacro()\ngenerateci(mytest)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert!(
        result.contains("GenerateCI(mytest)"),
        "Should infer GenerateCI casing from macro definition, got: {}",
        result
    );
}

#[test]
fn test_infer_without_definition_leaves_as_is() {
    let input = "SomeExternalCommand(arg1 arg2)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(result, "SomeExternalCommand(arg1 arg2)\n");
}

#[test]
fn test_user_command_case_explicit_lowercase() {
    let input = "MyCmd(z)\n";
    let config = FormatConfig {
        user_command_case: UserCommandCase::Lowercase,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert_eq!(result, "mycmd(z)\n");
}

#[test]
fn test_user_command_case_explicit_uppercase() {
    let input = "MyCmd(z)\n";
    let config = FormatConfig {
        user_command_case: UserCommandCase::Uppercase,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert_eq!(result, "MYCMD(z)\n");
}

#[test]
fn test_user_command_case_leave() {
    let input = "function(MyHelper)\nendfunction()\nmyhelper(foo)\n";
    let config = FormatConfig {
        user_command_case: UserCommandCase::Preserve,
        ..default_config()
    };
    let result = format_text(input, &config);
    // Even though definition says MyHelper, Preserve mode keeps original casing
    assert!(
        result.contains("myhelper(foo)"),
        "Preserve mode should keep original casing, got: {}",
        result
    );
}

#[test]
fn test_builtin_uppercase_user_infer() {
    let input = "function(MyHelper)\nendfunction()\nset(X y)\nmyhelper(foo)\n";
    let config = FormatConfig {
        command_case: CommandCase::Uppercase,
        user_command_case: UserCommandCase::Infer,
        ..default_config()
    };
    let result = format_text(input, &config);
    // Builtins uppercased, user command inferred
    assert!(
        result.contains("SET(X y)"),
        "Builtins should be uppercase, got: {}",
        result
    );
    assert!(
        result.contains("MyHelper(foo)"),
        "User commands should be inferred, got: {}",
        result
    );
}

#[test]
fn test_user_command_casing_idempotency() {
    let inputs = vec![
        "SET(X y)\nMyCmd(z)\n",
        "function(MyHelper arg)\nendfunction()\nmyhelper(foo)\n",
        "macro(GenCI t)\nendmacro()\ngenci(x)\n",
    ];

    let config = default_config();
    for input in inputs {
        let once = format_text(input, &config);
        let twice = format_text(&once, &config);
        assert_eq!(
            once, twice,
            "User command casing should be idempotent for input: {}",
            input
        );
    }
}

// ============================================================================
// LARGE FILE REGRESSION TESTS (Stack Overflow Fix)
// ============================================================================

#[test]
fn test_large_file_no_stack_overflow() {
    // Generate a file with 2000 commands
    let mut commands = vec!["cmake_minimum_required(VERSION 3.10)".to_string()];
    for i in 0..2000 {
        commands.push(format!("set(VAR_{} value_{})", i, i));
    }
    let input = commands.join("\n") + "\n";

    let config = default_config();
    let result = format_text(&input, &config);

    // Verify result is not empty
    assert!(!result.is_empty(), "Result should not be empty");

    // Verify first and last commands are present
    assert!(
        result.contains("cmake_minimum_required(VERSION 3.10)"),
        "First command should be present"
    );
    assert!(
        result.contains("set(VAR_1999 value_1999)"),
        "Last command should be present"
    );

    // Verify idempotency
    let result2 = format_text(&result, &config);
    assert_eq!(
        result, result2,
        "Formatting should be idempotent for large files"
    );
}

#[test]
fn test_very_large_file_no_stack_overflow() {
    // Generate a file with 5000 commands (mix of types)
    let mut commands = vec!["cmake_minimum_required(VERSION 3.10)".to_string()];
    for i in 0..5000 {
        match i % 3 {
            0 => commands.push(format!("set(VAR_{} value_{})", i, i)),
            1 => commands.push(format!("message(STATUS \"Message {}\")", i)),
            _ => {
                commands.push(format!("if(VAR_{})", i));
                commands.push(format!("    set(INNER_{} val)", i));
                commands.push("endif()".to_string());
            }
        }
    }
    let input = commands.join("\n") + "\n";

    let config = default_config();
    let result = format_text(&input, &config);

    // Verify it completes without panic
    assert!(!result.is_empty(), "Result should not be empty");

    // Verify output ends with newline
    assert!(result.ends_with('\n'), "Output should end with newline");
}

/// Regression test: blank lines between commands/comments must be preserved
/// across batch rendering boundaries. The formatter renders docs in batches
/// of 500 to prevent stack overflow. Each command produces 2 docs (command +
/// hardline), so the first batch flush occurs after 250 commands. Previously,
/// blank lines were dropped at batch boundaries because `!docs.is_empty()`
/// was used to detect "not at start of file", but after a batch flush, docs
/// is empty even though we're mid-file.
#[test]
fn test_blank_line_preserved_across_batch_boundary() {
    // Generate 260 simple commands (520 docs) to ensure at least one batch flush.
    // Batch size is 500 docs, so flush occurs after command 250.
    let mut lines = Vec::new();
    for i in 0..260 {
        lines.push(format!("set(VAR_{} value_{})", i, i));
    }

    // After the batch boundary, add a blank line followed by a comment and command.
    // This is the exact pattern that was broken: blank line + comment after batch flush.
    lines.push(String::new()); // blank line
    lines.push("# Section after batch boundary".to_string());
    lines.push("set(FINAL_VAR final_value)".to_string());

    let input = lines.join("\n") + "\n";
    let config = default_config();
    let result = format_text(&input, &config);

    // The blank line before the comment must be preserved
    assert!(
        result.contains("set(VAR_259 value_259)\n\n# Section after batch boundary"),
        "Blank line before comment must be preserved across batch boundary.\n\
         Expected to find: set(VAR_259 value_259)\\n\\n# Section after batch boundary\n\
         Got around boundary:\n{}",
        // Show context around the boundary for debugging
        result
            .lines()
            .collect::<Vec<_>>()
            .windows(5)
            .find(|w| w.iter().any(|l| l.contains("VAR_259")))
            .map(|w| w.join("\n"))
            .unwrap_or_else(|| "VAR_259 not found".to_string())
    );

    // The comment and final command must also be present
    assert!(
        result.contains("# Section after batch boundary"),
        "Comment after batch boundary should be present"
    );
    assert!(
        result.contains("set(FINAL_VAR final_value)"),
        "Command after comment should be present"
    );

    // Verify idempotency
    let result2 = format_text(&result, &config);
    assert_eq!(
        result, result2,
        "Formatting should be idempotent across batch boundaries with blank lines"
    );
}

/// Regression test: blank lines between two commands (no comments) must also
/// be preserved across batch boundaries.
#[test]
fn test_blank_line_between_commands_across_batch_boundary() {
    let mut lines = Vec::new();
    for i in 0..260 {
        lines.push(format!("set(VAR_{} value_{})", i, i));
    }

    // Blank line between two commands right after batch boundary
    lines.push(String::new()); // blank line
    lines.push("set(AFTER_BOUNDARY after_value)".to_string());

    let input = lines.join("\n") + "\n";
    let config = default_config();
    let result = format_text(&input, &config);

    assert!(
        result.contains("set(VAR_259 value_259)\n\nset(AFTER_BOUNDARY after_value)"),
        "Blank line between commands must be preserved across batch boundary.\n\
         Got around boundary:\n{}",
        result
            .lines()
            .collect::<Vec<_>>()
            .windows(5)
            .find(|w| w.iter().any(|l| l.contains("VAR_259")))
            .map(|w| w.join("\n"))
            .unwrap_or_else(|| "VAR_259 not found".to_string())
    );
}

// ============================================================================
// FINAL_NEWLINE TESTS
// ============================================================================

#[test]
fn test_final_newline_leave_default() {
    // Default is Preserve — preserves input's trailing newline state
    let input_no_nl = "set(FOO bar)";
    let config = default_config();
    let result = format_text(input_no_nl, &config);
    assert_eq!(
        result, "set(FOO bar)",
        "Default Preserve should not add trailing newline when input lacks one"
    );

    let input_with_nl = "set(FOO bar)\n";
    let result = format_text(input_with_nl, &config);
    assert_eq!(
        result, "set(FOO bar)\n",
        "Default Preserve should preserve trailing newline when input has one"
    );
}

#[test]
fn test_final_newline_false_no_trailing_newline() {
    let input = "set(FOO bar)\n";
    let mut config = default_config();
    config.final_newline = FinalNewline::Remove;
    let result = format_text(input, &config);
    assert_eq!(
        result, "set(FOO bar)",
        "final_newline=Remove should not add trailing newline"
    );
}

#[test]
fn test_final_newline_false_empty_input() {
    let input = "";
    let mut config = default_config();
    config.final_newline = FinalNewline::Remove;
    let result = format_text(input, &config);
    assert_eq!(
        result, "",
        "Empty input should remain empty regardless of final_newline"
    );
}

#[test]
fn test_final_newline_false_multiline() {
    let input = "set(A b)\nset(C d)\n";
    let mut config = default_config();
    config.final_newline = FinalNewline::Remove;
    let result = format_text(input, &config);
    assert_eq!(
        result, "set(A b)\nset(C d)",
        "final_newline=Remove should not add trailing newline on multiline"
    );
}

#[test]
fn test_final_newline_true_preserves_existing_behavior() {
    let input = "set(FOO bar)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(
        result, "set(FOO bar)\n",
        "final_newline=true (default) should preserve single trailing newline"
    );
}

// Force mode tests

#[test]
fn test_final_newline_force_adds_newline() {
    let input = "set(FOO bar)";
    let mut config = default_config();
    config.final_newline = FinalNewline::Force;
    let result = format_text(input, &config);
    assert_eq!(
        result, "set(FOO bar)\n",
        "FinalNewline::Force should add trailing newline when missing"
    );
}

// Preserve mode tests

#[test]
fn test_final_newline_leave_with_trailing_newline() {
    let input = "set(FOO bar)\n";
    let mut config = default_config();
    config.final_newline = FinalNewline::Preserve;
    let result = format_text(input, &config);
    assert_eq!(
        result, "set(FOO bar)\n",
        "Preserve mode should preserve trailing newline when present"
    );
}

#[test]
fn test_final_newline_leave_without_trailing_newline() {
    let input = "set(FOO bar)";
    let mut config = default_config();
    config.final_newline = FinalNewline::Preserve;
    let result = format_text(input, &config);
    assert_eq!(
        result, "set(FOO bar)",
        "Preserve mode should not add trailing newline when input had none"
    );
}

#[test]
fn test_final_newline_leave_multiline_with_newline() {
    let input = "set(A b)\nset(C d)\n";
    let mut config = default_config();
    config.final_newline = FinalNewline::Preserve;
    let result = format_text(input, &config);
    assert_eq!(
        result, "set(A b)\nset(C d)\n",
        "Preserve mode should preserve trailing newline in multiline input"
    );
}

#[test]
fn test_final_newline_leave_multiline_without_newline() {
    let input = "set(A b)\nset(C d)";
    let mut config = default_config();
    config.final_newline = FinalNewline::Preserve;
    let result = format_text(input, &config);
    assert_eq!(
        result, "set(A b)\nset(C d)",
        "Preserve mode should not add trailing newline to multiline input without one"
    );
}

#[test]
fn test_final_newline_leave_empty_input() {
    let input = "";
    let mut config = default_config();
    config.final_newline = FinalNewline::Preserve;
    let result = format_text(input, &config);
    assert_eq!(
        result, "",
        "Preserve mode should return empty for empty input"
    );
}

// Backward-compat deserialization tests

#[test]
fn test_final_newline_deserialize_bool_true() {
    let toml = "final_newline = true\n";
    let config: FormatConfig = toml::from_str(toml).expect("Should parse final_newline = true");
    assert_eq!(
        config.final_newline,
        FinalNewline::Force,
        "bool true should map to Force"
    );
}

#[test]
fn test_final_newline_deserialize_bool_false() {
    let toml = "final_newline = false\n";
    let config: FormatConfig = toml::from_str(toml).expect("Should parse final_newline = false");
    assert_eq!(
        config.final_newline,
        FinalNewline::Remove,
        "bool false should map to Remove"
    );
}

#[test]
fn test_final_newline_deserialize_string_leave() {
    let toml = "final_newline = \"leave\"\n";
    let config: FormatConfig =
        toml::from_str(toml).expect("Should parse final_newline = \"leave\"");
    assert_eq!(
        config.final_newline,
        FinalNewline::Preserve,
        "string \"leave\" should map to Preserve"
    );
}

#[test]
fn test_final_newline_deserialize_string_remove() {
    let toml = "final_newline = \"remove\"\n";
    let config: FormatConfig =
        toml::from_str(toml).expect("Should parse final_newline = \"remove\"");
    assert_eq!(
        config.final_newline,
        FinalNewline::Remove,
        "string \"remove\" should map to Remove"
    );
}

#[test]
fn test_final_newline_deserialize_string_force() {
    let toml = "final_newline = \"force\"\n";
    let config: FormatConfig =
        toml::from_str(toml).expect("Should parse final_newline = \"force\"");
    assert_eq!(
        config.final_newline,
        FinalNewline::Force,
        "string \"force\" should map to Force"
    );
}

#[test]
fn test_final_newline_deserialize_string_preserve() {
    let toml = "final_newline = \"preserve\"\n";
    let config: FormatConfig =
        toml::from_str(toml).expect("Should parse final_newline = \"preserve\"");
    assert_eq!(
        config.final_newline,
        FinalNewline::Preserve,
        "string \"preserve\" should map to Preserve"
    );
}

// Backward-compatibility: "leave" still accepted as alias for "preserve"
#[test]
fn test_command_case_leave_backward_compat() {
    let toml = "command_case = \"leave\"\n";
    let config: FormatConfig = toml::from_str(toml).expect("Should parse command_case = \"leave\"");
    assert_eq!(
        config.command_case,
        CommandCase::Preserve,
        "\"leave\" should be accepted as alias for CommandCase::Preserve"
    );
}

#[test]
fn test_closing_style_leave_backward_compat() {
    let toml = "closing_style = \"leave\"\n";
    let config: FormatConfig =
        toml::from_str(toml).expect("Should parse closing_style = \"leave\"");
    assert_eq!(
        config.closing_style,
        ClosingStyle::Preserve,
        "\"leave\" should be accepted as alias for ClosingStyle::Preserve"
    );
}

// ============================================================================
// COMMENT WHITESPACE NORMALIZATION TESTS
// ============================================================================

#[test]
fn test_comment_tabs_normalized_to_single_space() {
    let input = "set(SOURCES\n\t#\t\tfilename.hpp\n\tvalue\n)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert!(
        result.contains("# filename.hpp"),
        "Tab whitespace in comment should be normalized. Got: {}",
        result
    );
    assert!(
        !result.contains("#\t"),
        "No tabs should remain in comment. Got: {}",
        result
    );
}

#[test]
fn test_comment_already_normalized_unchanged() {
    let input = "set(SOURCES\n\t# already normal\n\tvalue\n)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert!(
        result.contains("# already normal"),
        "Already-normalized comment should stay the same. Got: {}",
        result
    );
}

#[test]
fn test_comment_no_space_after_hash() {
    let input = "set(SOURCES\n\t#no-space\n\tvalue\n)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert!(
        result.contains("# no-space"),
        "Comment without space after hash should get one. Got: {}",
        result
    );
}

#[test]
fn test_comment_hash_only_stays_bare() {
    let input = "set(SOURCES\n\t#\n\tvalue\n)\n";
    let config = default_config();
    let result = format_text(input, &config);
    // The hash-only comment should remain as "#" (no trailing space)
    // Check it does NOT become "# " (hash + space)
    for line in result.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') && !trimmed.starts_with("#[") && trimmed.len() <= 1 {
            assert_eq!(
                trimmed, "#",
                "Hash-only comment should stay as '#'. Got: '{}'",
                trimmed
            );
        }
    }
}

#[test]
fn test_trailing_comment_whitespace_normalized() {
    let input = "set(FLAGS\n\t-Wall #   extra spaces comment\n)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert!(
        result.contains("# extra spaces comment"),
        "Trailing comment whitespace should be normalized. Got: {}",
        result
    );
}

#[test]
fn test_bracket_comment_inside_arglist_unchanged() {
    let input = "set(LIST\n\tvalue1\n\t#[=[\n  Keep   this   spacing\n  ]=]\n\tvalue2\n)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert!(
        result.contains("Keep   this   spacing"),
        "Bracket comment content should be preserved. Got: {}",
        result
    );
}

#[test]
fn test_comment_multiple_spaces_normalized() {
    let input = "set(SOURCES\n\t#     lots   of   spaces\n\tvalue\n)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert!(
        result.contains("# lots   of   spaces"),
        "Only leading whitespace after # should be normalized, internal spacing preserved. Got: {}",
        result
    );
}

// Test comment_style=leave preserves original whitespace
#[test]
fn test_comment_style_leave_preserves_original() {
    let input = "set(SOURCES\n\t#no-space\n\tvalue\n)\n";
    let config = FormatConfig {
        comment_style: CommentStyle::Preserve,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert!(
        result.contains("#no-space"),
        "comment_style=leave should preserve original '#no-space'. Got: {}",
        result
    );
    assert!(
        !result.contains("# no-space"),
        "comment_style=leave should NOT normalize to '# no-space'. Got: {}",
        result
    );
}

// Test comment_style=hash_no_space removes space after hash
#[test]
fn test_comment_style_hash_no_space() {
    let input = "set(SOURCES\n\t# has space\n\tvalue\n)\n";
    let config = FormatConfig {
        comment_style: CommentStyle::HashNoSpace,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert!(
        result.contains("#has space"),
        "comment_style=hash_no_space should strip space after hash. Got: {}",
        result
    );
    assert!(
        !result.contains("# has space"),
        "comment_style=hash_no_space should NOT keep space after hash. Got: {}",
        result
    );
}

// Test comment_style=hash_space is default
#[test]
fn test_comment_style_hash_space_is_default() {
    let input = "set(SOURCES\n\t#no-space\n\tvalue\n)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert!(
        result.contains("# no-space"),
        "default config should normalize to '# no-space'. Got: {}",
        result
    );
}

// Test comment_style=leave preserves tabs
#[test]
fn test_comment_style_leave_preserves_tabs() {
    let input = "set(SOURCES\n\t#\t\ttabbed\n\tvalue\n)\n";
    let config = FormatConfig {
        comment_style: CommentStyle::Preserve,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert!(
        result.contains("#\t\ttabbed"),
        "comment_style=leave should preserve tabs. Got: {}",
        result
    );
}

// Test comment_style=hash_no_space handles hash-only comments
#[test]
fn test_comment_style_hash_no_space_hash_only() {
    let input = "set(SOURCES\n\t#\n\tvalue\n)\n";
    let config = FormatConfig {
        comment_style: CommentStyle::HashNoSpace,
        ..default_config()
    };
    let result = format_text(input, &config);
    // Hash-only comments should remain as just "#"
    let lines: Vec<&str> = result.lines().collect();
    let comment_line = lines.iter().find(|l| l.trim() == "#");
    assert!(
        comment_line.is_some(),
        "Hash-only comment should remain as '#'. Got: {}",
        result
    );
}

// Test standalone comments respect comment_style
#[test]
fn test_standalone_comment_respects_comment_style() {
    let input = "#no-space standalone\nset(X value)\n";
    let config = FormatConfig {
        comment_style: CommentStyle::Preserve,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert!(
        result.starts_with("#no-space standalone"),
        "Standalone comment should preserve original with comment_style=leave. Got: {}",
        result
    );
}

// Test trailing comments respect comment_style
#[test]
fn test_trailing_comment_respects_comment_style() {
    let input = "set(X value) #no-space trailing\n";
    let config = FormatConfig {
        comment_style: CommentStyle::Preserve,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert!(
        result.contains("#no-space trailing"),
        "Trailing comment should preserve original with comment_style=leave. Got: {}",
        result
    );
}

// ============================================================================
// MULTI-HASH COMMENT PRESERVATION TESTS
// ============================================================================

// Test double-hash comments preserved with HashSpace
#[test]
fn test_double_hash_preserved_with_hash_space() {
    let input = "set(SOURCES\n\t## DXV4 After Effects exporter\n\tvalue\n)\n";
    let config = FormatConfig {
        comment_style: CommentStyle::HashSpace,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert!(
        result.contains("## DXV4 After Effects exporter"),
        "Double-hash comment should be preserved exactly with HashSpace. Got: {}",
        result
    );
}

// Test double-hash without space preserved
#[test]
fn test_double_hash_no_space_preserved() {
    let input = "set(SOURCES\n\t##foo\n\tvalue\n)\n";
    let config = FormatConfig {
        comment_style: CommentStyle::HashSpace,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert!(
        result.contains("##foo"),
        "Double-hash comment without space should stay as-is (no space added). Got: {}",
        result
    );
    assert!(
        !result.contains("## foo"),
        "HashSpace should NOT normalize ##foo. Got: {}",
        result
    );
}

// Test triple-hash comments preserved
#[test]
fn test_triple_hash_preserved() {
    let input = "set(SOURCES\n\t### Section\n\tvalue\n)\n";
    let config = FormatConfig {
        comment_style: CommentStyle::HashSpace,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert!(
        result.contains("### Section"),
        "Triple-hash comment should be preserved exactly. Got: {}",
        result
    );
}

// Test double-hash preserved with HashNoSpace
#[test]
fn test_double_hash_preserved_with_hash_no_space() {
    let input = "set(SOURCES\n\t## foo\n\tvalue\n)\n";
    let config = FormatConfig {
        comment_style: CommentStyle::HashNoSpace,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert!(
        result.contains("## foo"),
        "Double-hash comment should be preserved (space NOT removed) with HashNoSpace. Got: {}",
        result
    );
    assert!(
        !result.contains("##foo"),
        "HashNoSpace should NOT strip space from ## foo. Got: {}",
        result
    );
}

// Test double-hash in full format context
#[test]
fn test_double_hash_in_full_format() {
    let input = "## Main heading\nset(X value)\n## Another section\nset(Y value)\n";
    let config = FormatConfig {
        comment_style: CommentStyle::HashSpace,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert!(
        result.contains("## Main heading"),
        "Double-hash standalone comment should be preserved. Got: {}",
        result
    );
    assert!(
        result.contains("## Another section"),
        "Multiple double-hash comments should all be preserved. Got: {}",
        result
    );
}

// ============================================================================
// DISABLE_FORMAT TESTS
// ============================================================================

#[test]
fn test_disable_format_returns_input_unchanged() {
    let input = "set(  FOO   BAR  )\n";
    let config = FormatConfig {
        disable_format: true,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert_eq!(
        result, input,
        "disable_format=true should return input unchanged"
    );
}

#[test]
fn test_disable_format_false_still_formats() {
    let input = "set(  FOO   BAR  )\n";
    let config = FormatConfig {
        disable_format: false,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert_ne!(
        result, input,
        "disable_format=false (default) should still format input"
    );
    assert_eq!(
        result, "set(FOO BAR)\n",
        "Expected properly formatted output"
    );
}

// ============================================================================
// BLANK LINE AFTER LEADING COMMENT TESTS
// ============================================================================

/// Blank line between a leading comment and its command should be preserved
#[test]
fn test_blank_line_after_leading_comment_preserved() {
    let input = "# Leading comment\n\nset(FOO bar)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(
        result, "# Leading comment\n\nset(FOO bar)\n",
        "Blank line between comment and command should be preserved"
    );
}

/// Blank line before AND after a leading comment should both be preserved
#[test]
fn test_blank_lines_before_and_after_leading_comment() {
    let input = "set(A a)\n\n# Comment\n\nset(B b)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(
        result, "set(A a)\n\n# Comment\n\nset(B b)\n",
        "Both blank lines (before and after comment) should be preserved"
    );
}

/// No blank line between comment and command: should stay tight
#[test]
fn test_no_blank_line_after_leading_comment_unchanged() {
    let input = "# Leading comment\nset(FOO bar)\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(
        result, "# Leading comment\nset(FOO bar)\n",
        "No blank line should be inserted when none existed"
    );
}

/// Idempotency: formatting blank-line-after-comment twice yields same result
#[test]
fn test_blank_line_after_leading_comment_idempotent() {
    let inputs = vec![
        "# Comment\n\nset(FOO bar)\n",
        "set(A a)\n\n# Comment\n\nset(B b)\n",
        "# Comment\nset(FOO bar)\n",
    ];
    let config = default_config();
    for input in inputs {
        let once = format_text(input, &config);
        let twice = format_text(&once, &config);
        assert_eq!(
            once, twice,
            "Formatting should be idempotent for input: {:?}",
            input
        );
    }
}

// ============================================================================
// COMMENT BLOCK PRESERVATION TESTS
// ============================================================================

/// Copyright header block: all lines preserved exactly as-is (indentation, alignment, blank # separators)
#[test]
fn test_comment_block_preserved() {
    let input = "\
# Copyright 2011 Author. All rights reserved.
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions are met:
#
#  1. Redistributions of source code must retain the above copyright notice,
#     this list of conditions and the following disclaimer.
#
#  2. Redistributions in binary form must reproduce the above copyright
#     notice, this list of conditions and the following disclaimer in the
#     documentation and/or other materials provided with the distribution.
#
cmake_minimum_required(VERSION 3.10)
";
    let config = FormatConfig {
        comment_style: CommentStyle::HashSpace,
        ..default_config()
    };
    let result = format_text(input, &config);
    // Every comment line must be preserved exactly (no normalization of #  1. or #     this)
    assert!(
        result.contains("#  1. Redistributions of source code"),
        "Double-space indentation in block should be preserved, got: {}",
        result
    );
    assert!(
        result.contains("#     this list of conditions"),
        "Five-space alignment in block should be preserved, got: {}",
        result
    );
    assert!(
        result.contains("#\n"),
        "Blank # separators in block should be preserved, got: {}",
        result
    );
    assert!(
        result.contains("#  2. Redistributions in binary form"),
        "Second item double-space indentation should be preserved, got: {}",
        result
    );
}

/// Isolated single-line comment is still normalized per comment_style
#[test]
fn test_isolated_comment_still_normalized() {
    let input = "\
#TODO fix this
cmake_minimum_required(VERSION 3.10)
";
    let config = FormatConfig {
        comment_style: CommentStyle::HashSpace,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert!(
        result.contains("# TODO fix this"),
        "Isolated comment should be normalized with HashSpace, got: {}",
        result
    );
    assert!(
        !result.contains("#TODO"),
        "Original un-normalized form should not appear, got: {}",
        result
    );
}

/// Formatting a copyright block twice yields identical output (idempotent)
#[test]
fn test_comment_block_idempotent() {
    let input = "\
# Copyright 2011 Author. All rights reserved.
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions are met:
#
#  1. Redistributions of source code must retain the above copyright notice,
#     this list of conditions and the following disclaimer.
#
#  2. Redistributions in binary form must reproduce the above copyright
#     notice, this list of conditions and the following disclaimer in the
#     documentation and/or other materials provided with the distribution.
#
cmake_minimum_required(VERSION 3.10)
";
    let config = FormatConfig {
        comment_style: CommentStyle::HashSpace,
        ..default_config()
    };
    let once = format_text(input, &config);
    let twice = format_text(&once, &config);
    assert_eq!(once, twice, "Comment block formatting should be idempotent");
}

// ============================================================================
// BLANK LINE BEFORE TRAILING COMMENTS TESTS
// ============================================================================

/// Blank line before trailing comment block in set() (grammar pre-keyword section)
#[test]
fn test_blank_line_before_trailing_comments_in_set() {
    let input = "\
set(RESAPI_SOURCE_MAC
    \"source/network/RANetworkAdapterOSX.mm\"

    # \"source/network/gcd/GCDAsyncSocket.h\"
    # \"source/network/gcd/GCDAsyncSocket.m\"
    # \"source/network/gcd/GCDAsyncUdpSocket.h\"
    # \"source/network/gcd/GCDAsyncUdpSocket.m\"
)
";
    let config = FormatConfig {
        comment_style: CommentStyle::Preserve,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert!(
        result.contains("RANetworkAdapterOSX.mm\"\n\n"),
        "Blank line before trailing comments should be preserved, got:\n{}",
        result
    );
}

/// Blank line before trailing comment block in keyword section (target_link_libraries)
#[test]
fn test_blank_line_before_trailing_comments_in_keyword_section() {
    let input = "\
target_link_libraries(myapp
    PUBLIC
        lib1
        lib2

        # lib3_disabled
        # lib4_disabled
)
";
    let config = FormatConfig {
        comment_style: CommentStyle::Preserve,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert!(
        result.contains("lib2\n\n"),
        "Blank line before trailing comments in keyword section should be preserved, got:\n{}",
        result
    );
}

/// Blank line before trailing comment block in non-grammar command (format_simple_args path)
#[test]
fn test_blank_line_before_trailing_comments_no_grammar() {
    let input = "\
some_custom_command(
    arg1
    arg2

    # disabled_arg
)
";
    let config = FormatConfig {
        comment_style: CommentStyle::Preserve,
        ..default_config()
    };
    let result = format_text(input, &config);
    assert!(
        result.contains("arg2\n\n"),
        "Blank line before trailing comments in simple args should be preserved, got:\n{}",
        result
    );
}

/// Formatting is idempotent for blank line before trailing comments
#[test]
fn test_blank_line_before_trailing_comments_idempotent() {
    let input = "\
set(RESAPI_SOURCE_MAC
    \"source/network/RANetworkAdapterOSX.mm\"

    # \"source/network/gcd/GCDAsyncSocket.h\"
    # \"source/network/gcd/GCDAsyncSocket.m\"
    # \"source/network/gcd/GCDAsyncUdpSocket.h\"
    # \"source/network/gcd/GCDAsyncUdpSocket.m\"
)
";
    let config = FormatConfig {
        comment_style: CommentStyle::Preserve,
        ..default_config()
    };
    let once = format_text(input, &config);
    let twice = format_text(&once, &config);
    assert_eq!(
        once, twice,
        "Blank line before trailing comments should be idempotent"
    );
}

// ============================================================================
// CONDITION WRAPPING (if / elseif / while)
// ============================================================================

#[test]
fn test_long_condition_breaks_at_boolean_operators() {
    // Regression: https://github.com/sandercox/cmake-fmt/issues/2
    // A wrapped condition used to put every word on its own line.
    let input = concat!(
        "if(NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\"\n",
        "  AND NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"GROUP\")\n",
        "endif()\n"
    );
    let config = default_config();
    let result = format_text(input, &config);

    assert_eq!(
        result,
        concat!(
            "if(NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\"\n",
            "\tAND NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"GROUP\"\n",
            ")\n",
            "endif()\n"
        )
    );
}

#[test]
fn test_long_condition_breaks_before_or() {
    let input = concat!(
        "while(SOME_LONG_CONDITION_VARIABLE AND ANOTHER_LONG_CONDITION_VARIABLE ",
        "OR YET_MORE_STUFF)\n",
        "endwhile()\n"
    );
    let config = default_config();
    let result = format_text(input, &config);

    assert_eq!(
        result,
        concat!(
            "while(SOME_LONG_CONDITION_VARIABLE\n",
            "\tAND ANOTHER_LONG_CONDITION_VARIABLE\n",
            "\tOR YET_MORE_STUFF\n",
            ")\n",
            "endwhile()\n"
        )
    );
}

#[test]
fn test_long_elseif_condition_breaks_at_operators() {
    let input = concat!(
        "if(FOO)\n",
        "elseif(SOME_LONG_CONDITION_VARIABLE AND ANOTHER_LONG_CONDITION_VARIABLE ",
        "OR MORE_STUFF)\n",
        "endif()\n"
    );
    let config = default_config();
    let result = format_text(input, &config);

    assert!(
        result.contains("elseif(SOME_LONG_CONDITION_VARIABLE\n\tAND ANOTHER_LONG_CONDITION_VARIABLE\n\tOR MORE_STUFF\n)"),
        "elseif condition was not laid out by clause:\n{}",
        result
    );
}

#[test]
fn test_short_condition_stays_on_one_line() {
    let input = "if(A AND B)\nendif()\n";
    let config = default_config();
    let result = format_text(input, &config);
    assert_eq!(result, input);
}

#[test]
fn test_over_long_clause_fills_continuation_lines() {
    // A clause that can't fit on one line is filled across continuation lines
    // indented one level deeper, so it still reads as one clause.
    let input = concat!(
        "if(SUPER_DUPER_EXTREMELY_LONG_SINGLE_CLAUSE_VARIABLE_NAME_THAT_WONT_FIT_AT_ALL_OK ",
        "STREQUAL \"value\" AND B)\n",
        "endif()\n"
    );
    let config = default_config();
    let result = format_text(input, &config);

    assert_eq!(
        result,
        concat!(
            "if(SUPER_DUPER_EXTREMELY_LONG_SINGLE_CLAUSE_VARIABLE_NAME_THAT_WONT_FIT_AT_ALL_OK\n",
            "\t\tSTREQUAL \"value\"\n",
            "\tAND B\n",
            ")\n",
            "endif()\n"
        )
    );
}

#[test]
fn test_long_condition_respects_indent_and_paren_style() {
    let input = concat!(
        "if(NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\" ",
        "AND NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"GROUP\")\n",
        "endif()\n"
    );
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 2,
        control_flow_space_before_paren: true,
        space_between_command_parens: true,
        indent_closing_paren: true,
        ..Default::default()
    };
    let result = format_text(input, &config);

    assert_eq!(
        result,
        concat!(
            "if ( NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\"\n",
            "  AND NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"GROUP\"\n",
            "  )\n",
            "endif ()\n"
        )
    );
}

#[test]
fn test_long_condition_nested_indentation() {
    let input = concat!(
        "if(OUTER)\n",
        "\tif(NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\" ",
        "AND NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"GROUP\")\n",
        "\tendif()\n",
        "endif()\n"
    );
    let config = default_config();
    let result = format_text(input, &config);

    assert_eq!(
        result,
        concat!(
            "if(OUTER)\n",
            "\tif(NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\"\n",
            "\t\tAND NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"GROUP\"\n",
            "\t)\n",
            "\tendif()\n",
            "endif()\n"
        )
    );
}

#[test]
fn test_long_condition_is_idempotent() {
    let input = concat!(
        "if(NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\" ",
        "AND NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"GROUP\" OR OVERRIDE_MODE)\n",
        "endif()\n"
    );
    let config = default_config();
    let once = format_text(input, &config);
    let twice = format_text(&once, &config);
    assert_eq!(once, twice, "condition layout is not idempotent");
    // Idempotency alone said nothing here — the generic one-argument-per-line
    // layout is a fixed point too, so this passed before the clause layout
    // existed. Assert the layout as well.
    assert_eq!(
        once,
        concat!(
            "if(NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\"\n",
            "\tAND NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"GROUP\"\n",
            "\tOR OVERRIDE_MODE\n",
            ")\n",
            "endif()\n"
        )
    );
}

#[test]
fn test_long_condition_with_comment_keeps_generic_layout() {
    // A comment inside the condition needs its own line, so the clause layout
    // steps aside rather than folding the comment into a clause.
    let input = concat!(
        "if(NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\" # only two modes\n",
        "\tAND NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"GROUP\")\n",
        "endif()\n"
    );
    let config = default_config();
    let result = format_text(input, &config);

    assert!(
        result.contains("# only two modes"),
        "comment was lost:\n{}",
        result
    );
    let twice = format_text(&result, &config);
    assert_eq!(result, twice, "commented condition is not idempotent");
}

#[test]
fn test_foreach_is_not_treated_as_condition() {
    // foreach carries a list, not a boolean expression: AND is just a value
    let input = concat!(
        "foreach(item IN LISTS SOME_LONG_LIST_VARIABLE ANOTHER_LONG_LIST_VARIABLE ",
        "AND_MORE_HERE)\n",
        "endforeach()\n"
    );
    let config = default_config();
    let result = format_text(input, &config);

    assert!(
        result.contains("\tLISTS\n"),
        "foreach should keep the generic layout:\n{}",
        result
    );
}

#[test]
fn test_hand_wrapped_short_condition_uses_clause_layout() {
    // The condition fits on one line, but the author broke it, so the generic
    // layout would honour that by putting every word on its own line.
    let input = concat!(
        "if(WITH_CUDA AND NOT OPENCV_PLATFORM_IOS\n",
        "   AND NOT OPENCV_PLATFORM_ANDROID)\n",
        "endif()\n"
    );
    let config = default_config();
    let result = format_text(input, &config);

    assert_eq!(
        result,
        concat!(
            "if(WITH_CUDA\n",
            "\tAND NOT OPENCV_PLATFORM_IOS\n",
            "\tAND NOT OPENCV_PLATFORM_ANDROID\n",
            ")\n",
            "endif()\n"
        )
    );
}

#[test]
fn test_lowercase_and_or_are_values_not_operators() {
    // CMake rejects `if(A and B)` outright, so a bare `and`/`or` can only be a
    // value — breaking there would tear a comparison off its operand.
    let input = concat!(
        "if(MY_VAR STREQUAL and OR MY_OTHER_VARIABLE_NAME_IS_LONG STREQUAL Or ",
        "AND SOMETHING_ELSE)\n",
        "endif()\n"
    );
    let config = default_config();
    let result = format_text(input, &config);

    assert!(
        result.contains("if(MY_VAR STREQUAL and\n"),
        "lowercase `and` was treated as an operator:\n{}",
        result
    );
    assert!(
        result.contains("STREQUAL Or\n"),
        "`Or` was treated as an operator:\n{}",
        result
    );
}

#[test]
fn test_condition_with_unlimited_line_length() {
    // max_line_length = 0 means unlimited: clauses still get a line each
    // because the author broke the condition, but nothing wraps on width.
    let input = "if(A\n AND B)\nendif()\n";
    let config = FormatConfig {
        max_line_length: 0,
        ..Default::default()
    };
    let result = format_text(input, &config);

    assert_eq!(result, "if(A\n\tAND B\n)\nendif()\n");
}

#[test]
fn test_single_clause_continuation_indents_one_level() {
    // With only one clause there are no clause lines to distinguish the
    // continuation from, so it sits where every other wrapped command puts it.
    let input = concat!(
        "if(NOT SOMETHING_EXTREMELY_LONG_THAT_EXCEEDS_EIGHTY_CHARACTERS_BY_ITSELF_YES_INDEED)\n",
        "endif()\n"
    );
    let config = default_config();
    let result = format_text(input, &config);

    assert_eq!(
        result,
        concat!(
            "if(NOT\n",
            "\tSOMETHING_EXTREMELY_LONG_THAT_EXCEEDS_EIGHTY_CHARACTERS_BY_ITSELF_YES_INDEED\n",
            ")\n",
            "endif()\n"
        )
    );
}

#[test]
fn test_preserved_closer_condition_matches_opener() {
    // endif echoes the opener's condition under closing_style = preserve; the
    // two must not be laid out differently.
    let cond = "NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\" AND NOT WICKHOPPER_MODE STREQUAL \"GROUP\"";
    let input = format!("if({})\nendif({})\n", cond, cond);
    let config = FormatConfig {
        closing_style: ClosingStyle::Preserve,
        ..Default::default()
    };
    let result = format_text(&input, &config);

    let laid_out = concat!(
        "(NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\"\n",
        "\tAND NOT WICKHOPPER_MODE STREQUAL \"GROUP\"\n",
        ")"
    );
    assert_eq!(
        result,
        format!("if{}\nendif{}\n", laid_out, laid_out),
        "opener and closer disagree"
    );
}

#[test]
fn test_very_long_condition_does_not_overflow_stack() {
    // The layout used to build one Doc node per line, whose left-nested Append
    // chain overflowed the stack on Drop for a condition this size.
    let clauses: Vec<String> = (0..20_000).map(|i| format!("VAR_{}", i)).collect();
    let input = format!("if({})\nendif()\n", clauses.join(" AND "));
    let config = default_config();

    let once = format_text(&input, &config);
    assert!(once.contains("\tAND VAR_19999\n"), "last clause missing");
    assert_eq!(once, format_text(&once, &config), "not idempotent");
}

#[test]
fn test_condition_width_uses_display_width_not_char_count() {
    // The layout has to measure width the way the renderer does. `pretty`
    // measures non-ASCII text with unicode-width, so counting chars
    // under-estimated every wide character: the layout decided the condition
    // fit, abstained, and the generic path — measuring correctly — exploded it
    // one argument per line. That also made the output non-idempotent.
    //
    // chars 58, bytes 80, display width 64, against a limit of 60.
    let input = concat!(
        "if(NAME STREQUAL \"éééééééééé\" AND OTHER STREQUAL \"你好你好你好\")\n",
        "endif()\n"
    );
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 2,
        max_line_length: 60,
        ..Default::default()
    };
    let result = format_text(input, &config);

    assert_eq!(
        result,
        concat!(
            "if(NAME STREQUAL \"éééééééééé\"\n",
            "  AND OTHER STREQUAL \"你好你好你好\"\n",
            ")\n",
            "endif()\n"
        )
    );
    assert_eq!(result, format_text(&result, &config), "not idempotent");
}

#[test]
fn test_forced_closer_condition_matches_opener() {
    // closing_style = force reconstructs the closer from the opener's
    // arguments, bypassing the argument list entirely. Without the condition
    // layout there it emitted one long line under a wrapped opener — the exact
    // inconsistency that including endif/endwhile is meant to prevent.
    let input = concat!(
        "if(NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\" ",
        "AND NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"GROUP\")\n",
        "\tmessage(hi)\n",
        "endif()\n"
    );
    let config = FormatConfig {
        closing_style: ClosingStyle::Force,
        ..Default::default()
    };
    let result = format_text(input, &config);

    let laid_out = concat!(
        "(NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\"\n",
        "\tAND NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"GROUP\"\n",
        ")"
    );
    assert_eq!(
        result,
        format!("if{}\n\tmessage(hi)\nendif{}\n", laid_out, laid_out)
    );
    assert_eq!(result, format_text(&result, &config), "not idempotent");
}

#[test]
fn test_else_gets_the_same_layout_as_its_opener() {
    // `else` is a mid-block command but not `elseif`, so it fell past both the
    // elseif special case and the condition check into the join fallback — a
    // 107-column line between a wrapped `if` and a wrapped `endif`.
    let cond = "NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\" AND NOT WICKHOPPER_MODE STREQUAL \"GROUP\"";
    let input = format!(
        "if({})\n\tmessage(hi)\nelse()\n\tmessage(bye)\nendif()\n",
        cond
    );
    let config = FormatConfig {
        closing_style: ClosingStyle::Force,
        ..Default::default()
    };
    let result = format_text(&input, &config);

    for line in result.lines() {
        assert!(
            line.len() <= 80,
            "line over the limit, so a closer did not get the clause layout:\n{}",
            result
        );
    }
    assert_eq!(result, format_text(&result, &config), "not idempotent");
}

#[test]
fn test_forced_closer_uses_merged_arguments() {
    // opener_args came from raw tokens while the opener used merged logical
    // arguments, so `${VAR}` and `/path` were one argument to the opener and
    // two to the closer. The injected space makes CMake itself warn that the
    // block opens and closes with mis-matching arguments.
    let input = concat!(
        "if(EXISTS ${CMAKE_CURRENT_SOURCE_DIR}/utils/googletest/include/gtest/gtest.h ",
        "AND SOMETHING_ELSE_LONG)\n",
        "\tmessage(hi)\n",
        "endif()\n"
    );
    let config = FormatConfig {
        closing_style: ClosingStyle::Force,
        ..Default::default()
    };
    let result = format_text(input, &config);

    assert!(
        !result.contains("${CMAKE_CURRENT_SOURCE_DIR} /utils"),
        "a space was injected into the path:\n{}",
        result
    );
    let opener = result
        .lines()
        .next()
        .expect("opener")
        .trim_start_matches("if");
    let closer_line = result
        .lines()
        .find(|l| l.starts_with("endif"))
        .expect("closer");
    assert_eq!(
        opener,
        closer_line.trim_start_matches("endif"),
        "opener and closer disagree:\n{}",
        result
    );
}

#[test]
fn test_forced_closer_paren_spacing_is_symmetric() {
    // has_args read the source `endif()`, which has no arguments, so the width
    // model assumed a space the renderer never emitted and `endif(X)` became
    // `endif( X)` on a second pass.
    let config = FormatConfig {
        closing_style: ClosingStyle::Force,
        space_between_command_parens: true,
        ..Default::default()
    };
    let result = format_text("if(A)\n\tmessage(x)\nendif()\n", &config);

    assert!(
        result.contains("endif( A )"),
        "closer spacing should mirror the opener:\n{}",
        result
    );
    assert_eq!(result, format_text(&result, &config), "not idempotent");
}

#[test]
fn test_paren_spacing_follows_the_arguments_actually_written() {
    // The same width-model bug in its two remaining shapes. `elseif` carries a
    // condition of its own, so `closing_style` never reconstructs it from the
    // opener — but the opener's arguments were consulted anyway, and an
    // argument-less `elseif` came out as `elseif( )`. Under `remove` a closer's
    // arguments are dropped, yet its own argument list still asked for a space,
    // so `endif(EXISTS x)` became `endif( )` and only reached `endif()` on a
    // second pass.
    let forced = FormatConfig {
        closing_style: ClosingStyle::Force,
        space_between_command_parens: true,
        ..Default::default()
    };
    let result = format_text(
        "if(A)\n\tmessage(x)\nelseif()\n\tmessage(y)\nendif()\n",
        &forced,
    );
    assert!(
        result.contains("elseif()\n"),
        "an argument-less elseif should stay empty:\n{}",
        result
    );
    assert!(
        result.contains("endif( A )"),
        "the closer should still echo its opener:\n{}",
        result
    );
    assert_eq!(result, format_text(&result, &forced), "not idempotent");

    let removed = FormatConfig {
        closing_style: ClosingStyle::Remove,
        space_between_command_parens: true,
        ..Default::default()
    };
    let result = format_text(
        "if(EXISTS foo)\n\tmessage(x)\nendif(EXISTS foo)\n",
        &removed,
    );
    assert!(
        result.contains("endif()\n"),
        "a removed closer should have no paren space:\n{}",
        result
    );
    assert_eq!(result, format_text(&result, &removed), "not idempotent");
}

#[test]
fn test_joined_condition_closes_on_the_same_line() {
    // A hand-wrapped condition that fits gets joined onto one line; a lone `)`
    // underneath it reads as a bug, and nothing else in the formatter puts a
    // hardline before `)` unless the arguments are themselves on separate lines.
    let config = default_config();
    let result = format_text(
        "if(CMAKE_CXX_COMPILER_ID STREQUAL\n   \"GNU\")\nendif()\n",
        &config,
    );

    assert_eq!(
        result,
        "if(CMAKE_CXX_COMPILER_ID STREQUAL \"GNU\")\nendif()\n"
    );
    assert_eq!(result, format_text(&result, &config), "not idempotent");
}

/// The largest condition that still fits on one line, and the first that does not.
///
/// Every term of the width model — the indent, the command name, the space
/// before `(`, the paren spaces, the inter-argument spaces and the closing `)` —
/// shifts this boundary by one column, so pinning the boundary pins all of them.
/// Six independent mutations of that arithmetic used to leave the whole suite
/// green.
fn assert_wrap_boundary(config: &FormatConfig, build: fn(&str) -> String, nesting: usize) {
    let limit = config.max_line_length;
    let mut last_flat: Option<(usize, usize)> = None;

    for n in 1..200 {
        let condition = build(&"A".repeat(n));
        let input = if nesting == 0 {
            format!("{}\n\tmessage(x)\nendif()\n", condition)
        } else {
            format!(
                "if(OUTER)\n\t{}\n\t\tmessage(x)\n\tendif()\nendif()\n",
                condition
            )
        };
        let result = format_text(&input, config);
        let line = result.lines().nth(nesting).expect("condition line");
        let width = line.chars().count();

        // The condition is on one line exactly when the `)` is still on it
        if line.trim_end().ends_with(')') {
            assert!(
                width <= limit,
                "a flat condition overflowed: {} columns > {} for n={}\n{}",
                width,
                limit,
                n,
                result
            );
            last_flat = Some((n, width));
            continue;
        }

        // It broke. The last one that fitted must have filled the line exactly:
        // one column short would mean the model reserves something the renderer
        // does not emit, one column over is caught above.
        let (flat_n, flat_width) = last_flat.expect("nothing fitted at all");
        assert_eq!(
            flat_width,
            limit,
            "the last flat condition (n={}) stopped {} columns short of the limit",
            flat_n,
            limit - flat_width
        );
        // And nothing in the broken form may overflow either
        for line in result.lines() {
            if line.split_whitespace().count() > 1 {
                assert!(
                    line.chars().count() <= limit,
                    "wrapped line overflows: {:?}\n{}",
                    line,
                    result
                );
            }
        }
        // A `)` alone on a line must follow arguments that are themselves split,
        // so the condition has to occupy at least two lines before it. The old
        // form of this compared the total line count against `nesting + 3`,
        // which the broken path always exceeds, so it could never fail.
        if let Some(closer) = result.lines().position(|l| l.trim() == ")") {
            assert!(
                closer >= nesting + 2,
                "the closing paren was stranded under a joined condition:\n{}",
                result
            );
        }
        return;
    }
    panic!("the condition never broke");
}

#[test]
fn test_wrap_boundary_accounts_for_every_column() {
    let single = |arg: &str| format!("if(FIRST STREQUAL \"{}\")", arg);
    let two_clause = |arg: &str| format!("if(FIRST AND SECOND STREQUAL \"{}\")", arg);

    for limit in [40, 60] {
        // Tabs, spaces, and a deeper indent — pins the indent term
        assert_wrap_boundary(
            &FormatConfig {
                max_line_length: limit,
                ..Default::default()
            },
            single,
            0,
        );
        assert_wrap_boundary(
            &FormatConfig {
                max_line_length: limit,
                use_tabs: false,
                indent_width: 4,
                ..Default::default()
            },
            single,
            1,
        );
        // Pins the paren-space term
        assert_wrap_boundary(
            &FormatConfig {
                max_line_length: limit,
                space_between_command_parens: true,
                ..Default::default()
            },
            single,
            0,
        );
        // Pins the space-before-paren term
        assert_wrap_boundary(
            &FormatConfig {
                max_line_length: limit,
                control_flow_space_before_paren: true,
                ..Default::default()
            },
            single,
            0,
        );
        // Pins the inter-argument spaces term
        assert_wrap_boundary(
            &FormatConfig {
                max_line_length: limit,
                ..Default::default()
            },
            two_clause,
            0,
        );
    }
}

#[test]
fn test_the_indent_counts_toward_the_conditions_width() {
    // A condition nested in another block starts further right, and the layout
    // only takes over when the condition will not fit. Leaving the indent out of
    // that decision made it decline on a condition that overflows *because* of
    // the indent, and the generic layout then put every argument on its own
    // line — which is the shape this whole feature exists to avoid. Taken from
    // llvm/AddLLVM.cmake, where it happens at a 40-column limit.
    let config = FormatConfig {
        max_line_length: 40,
        use_tabs: false,
        indent_width: 4,
        ..Default::default()
    };
    let result = format_text(
        "if(OUTER)\n\tif(NOT ARG_MODULE AND NOT ARG_OBJECT)\n\t\tmessage(x)\n\tendif()\nendif()\n",
        &config,
    );

    assert!(
        result.contains("if(NOT ARG_MODULE\n") && result.contains("AND NOT ARG_OBJECT\n"),
        "the condition was not laid out by clause:\n{}",
        result
    );
    assert_eq!(result, format_text(&result, &config), "not idempotent");
}

#[test]
fn test_a_clause_that_exactly_fills_the_line_is_not_broken() {
    // The fill test asks whether the next word still fits; off by one it breaks
    // the clause a word early and indents the tail for no reason. `AND SECOND
    // STREQUAL "AAAAAAA"` is exactly 30 columns with its indent.
    let config = FormatConfig {
        max_line_length: 30,
        ..Default::default()
    };
    let result = format_text(
        "if(FIRST AND SECOND STREQUAL \"AAAAAAA\" AND THIRD)\n\tmessage(x)\nendif()\n",
        &config,
    );

    assert!(
        result.contains("\tAND SECOND STREQUAL \"AAAAAAA\"\n"),
        "a clause that fits exactly was broken:\n{}",
        result
    );
    assert_eq!(result, format_text(&result, &config), "not idempotent");
}

/// Grow a two-clause condition to the wrapping boundary and assert that when it
/// breaks, the clause layout — not the generic one-argument-per-line layout —
/// is what produced the break.
///
/// This is the assertion `assert_wrap_boundary` cannot make. When the width
/// model under-counts by a column the layout *declines* at the boundary, the
/// generic path breaks the line correctly, and the flat width still comes out
/// equal to the limit — so the boundary assertion is satisfied by the wrong
/// layout. Three mutations of the arithmetic survived for exactly that reason.
fn assert_clause_layout_survives_the_boundary(config: &FormatConfig) {
    let limit = config.max_line_length;
    let mut broke = false;

    for n in 1..120 {
        let input = format!(
            "if(FIRST AND SECOND STREQUAL \"{}\")\n\tmessage(x)\nendif()\n",
            "A".repeat(n)
        );
        let result = format_text(&input, config);
        let first = result.lines().next().expect("a first line");
        if first.trim_end().ends_with(')') {
            assert!(
                first.chars().count() <= limit,
                "a flat condition overflowed at n={}:\n{}",
                n,
                result
            );
            continue;
        }

        broke = true;
        // The whole second clause belongs on one line. The generic layout puts
        // `AND`, `SECOND` and `STREQUAL` on three lines of their own.
        assert!(
            result
                .lines()
                .any(|l| l.trim().starts_with("AND SECOND STREQUAL")),
            "the clause layout gave way to the generic one at n={} (limit {}):\n{}",
            n,
            limit,
            result
        );
        assert_eq!(result, format_text(&result, config), "not idempotent");
        if n > 4 {
            break;
        }
    }

    assert!(broke, "the condition never wrapped at limit {}", limit);
}

#[test]
fn test_the_clause_layout_survives_every_column_term() {
    for limit in [40, 60, 80] {
        assert_clause_layout_survives_the_boundary(&FormatConfig {
            max_line_length: limit,
            ..Default::default()
        });
        // Pins the paren-space term in both the fit decision and the fill: with
        // it dropped, the layout declines one column early and the generic
        // one-argument-per-line shape appears instead
        assert_clause_layout_survives_the_boundary(&FormatConfig {
            max_line_length: limit,
            space_between_command_parens: true,
            ..Default::default()
        });
        // Pins the space-before-paren term, which a corpus sweep cannot catch:
        // it is a one-column threshold, and no corpus file sits on it
        assert_clause_layout_survives_the_boundary(&FormatConfig {
            max_line_length: limit,
            control_flow_space_before_paren: true,
            ..Default::default()
        });
        assert_clause_layout_survives_the_boundary(&FormatConfig {
            max_line_length: limit,
            space_between_command_parens: true,
            control_flow_space_before_paren: true,
            use_tabs: false,
            indent_width: 2,
            ..Default::default()
        });
    }
}

#[test]
fn test_a_multi_line_argument_is_measured_by_both_its_ends() {
    // A multi-line quoted or bracket argument consumes the current line only as
    // far as its first newline, and leaves the column at the width of whatever
    // follows its last. Measuring it once by its last line answered both
    // questions with the same number, so an argument was joined onto a line
    // that then overflowed — at the default 80 columns.
    let config = FormatConfig::default();
    let long = "y".repeat(78);
    let result = format_text(
        &format!("if(A \"{}\nz\")\n\tmessage(x)\nendif()\n", long),
        &config,
    );

    for line in result.lines() {
        if line.split_whitespace().count() > 1 {
            assert!(
                line.chars().count() <= 80,
                "line overflows: {} columns\n{}",
                line.chars().count(),
                result
            );
        }
    }
    assert_eq!(result, format_text(&result, &config), "not idempotent");

    // And a `)` on its own line still has to follow arguments that are
    // themselves split. (The shape where `broke` and `must_wrap` genuinely
    // disagree is narrower than this one — `if(A "xx\nyy")` at a 10-column
    // limit stays joined while `must_wrap` is true — so this asserts the
    // invariant rather than that particular discrimination.)
    let narrow = FormatConfig {
        max_line_length: 20,
        ..Default::default()
    };
    let result = format_text("if(A \"xxxxxxxxxxxxxxxxxx\nyy\")\nendif()\n", &narrow);
    if let Some(closer) = result.lines().position(|l| l.trim() == ")") {
        assert!(
            closer >= 2,
            "the closing paren was stranded under a joined condition:\n{}",
            result
        );
    }
    assert_eq!(result, format_text(&result, &narrow), "not idempotent");

    // After a multi-line argument the column is the width of its last line, not
    // that line added to what came before it. Getting that wrong made the
    // formatter think the line was full and break the next argument away.
    let at24 = FormatConfig {
        max_line_length: 24,
        ..Default::default()
    };
    let result = format_text(
        "if(A \"xxxxxxxxxxxxxxxx\nyy\" BBBB)\n\tmessage(x)\nendif()\n",
        &at24,
    );
    assert!(
        result.contains("yy\" BBBB)"),
        "an argument was broken away after a multi-line one:\n{}",
        result
    );
    assert_eq!(result, format_text(&result, &at24), "not idempotent");
}

#[test]
fn test_a_trailing_comment_counts_toward_the_conditions_line() {
    // The layout measured only the condition, not the comment the caller
    // appends after the `)`. So a condition that fits while its *line* does not:
    // the layout declined, the generic path broke the line because it can see
    // the comment, and on that broken input the layout took over and joined the
    // condition again. A two-pass cycle with no fixed point, at default
    // settings, which made `--check` reject the tool's own output forever.
    let config = default_config();

    for (name, closer) in [("if", "endif"), ("while", "endwhile")] {
        for condition_len in [4, 8, 20] {
            for comment_len in [40, 60, 67, 70] {
                let input = format!(
                    "{}({} BBBB) # {}\n{}()\n",
                    name,
                    "A".repeat(condition_len),
                    "c".repeat(comment_len),
                    closer
                );
                let once = format_text(&input, &config);
                assert_eq!(
                    once,
                    format_text(&once, &config),
                    "not a fixed point for a {}-column condition and a {}-column comment:\n{}",
                    condition_len,
                    comment_len,
                    once
                );
            }
        }
    }

    // And the clause layout is what breaks it: without the comment counted in
    // the fit decision the layout declines and the generic path puts every
    // argument on its own line
    let result = format_text(
        "if(FIRST_CONDITION AND SECOND_CONDITION) # explain why both of these have to hold\nendif()\n",
        &config,
    );
    assert!(
        result.contains("\tAND SECOND_CONDITION\n"),
        "the clause layout gave way to the generic one:\n{}",
        result
    );

    // The comment is what makes the difference: an 83-column line is broken,
    // and the same condition with no comment stays on one line
    let long = format!("if(AAAA BBBB) # {}\nendif()\n", "c".repeat(67));
    assert_eq!(long.lines().next().unwrap().chars().count(), 83);
    let with_comment = format_text(&long, &config);
    assert!(
        with_comment.lines().count() > 2,
        "an 83-column line should have been broken:\n{}",
        with_comment
    );
    assert_eq!(
        format_text("if(AAAA BBBB)\nendif()\n", &config),
        "if(AAAA BBBB)\nendif()\n"
    );
}

#[test]
fn test_the_column_after_a_multi_line_argument_is_its_last_line() {
    // The column after an argument that spans lines is the width of its last
    // line, not that width added to what came before it, and not the width of
    // its first line. Both errors let the next argument join a line that then
    // overflows.
    let config = FormatConfig {
        max_line_length: 30,
        ..Default::default()
    };
    let result = format_text(
        "if(A \"x\nyyyyyyyyyyyyyyyyyyyyyyyy\" BBBB)\n\tmessage(x)\nendif()\n",
        &config,
    );
    for line in result.lines() {
        if line.split_whitespace().count() > 1 {
            assert!(
                line.chars().count() <= 30,
                "line overflows: {} columns\n{}",
                line.chars().count(),
                result
            );
        }
    }
    assert_eq!(result, format_text(&result, &config), "not idempotent");

    // The column after it *replaces* what came before rather than adding to it:
    // at a 12-column limit `yy" B)` fits, and adding the widths breaks `B` away
    let narrow = FormatConfig {
        max_line_length: 12,
        ..Default::default()
    };
    let result = format_text("if(AAAA \"xx\nyy\" B)\n\tmessage(x)\nendif()\n", &narrow);
    assert!(
        result.contains("yy\" B)"),
        "the column after a multi-line argument was added, not replaced:\n{}",
        result
    );
    assert_eq!(result, format_text(&result, &narrow), "not idempotent");

    // And the flat closing space is emitted, which a mutation of it made
    // non-idempotent
    let spaced = FormatConfig {
        space_between_command_parens: true,
        ..Default::default()
    };
    let result = format_text("if(\n\tAAAA BBBB\n)\nendif()\n", &spaced);
    assert!(
        result.contains("if( AAAA BBBB )"),
        "the flat closing paren space is missing:\n{}",
        result
    );
    assert_eq!(result, format_text(&result, &spaced), "not idempotent");
}

#[test]
fn test_a_single_argument_condition_keeps_the_generic_layout() {
    // A one-argument condition has nothing to split into clauses, so the clause
    // layout must decline. Nothing pinned that boundary.
    let config = FormatConfig {
        max_line_length: 20,
        ..Default::default()
    };
    let result = format_text(
        "if(AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA)\nendif()\n",
        &config,
    );
    // The argument itself cannot be broken, so it overflows and the `)` takes
    // the next line — the generic path's answer, not the clause layout's
    assert_eq!(
        result, "if(AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n)\nendif()\n",
        "a one-argument condition should not reach the clause layout"
    );
    assert_eq!(result, format_text(&result, &config), "not idempotent");
}
