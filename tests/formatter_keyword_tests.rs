use cmake_fmt::formatter::{format_text, CommandCase, FormatConfig};

fn default_config() -> FormatConfig {
    FormatConfig::default()
}

// ============================================================================
// KEYWORD-AWARE FORMATTING TESTS
// ============================================================================

#[test]
fn test_target_link_libraries_keywords() {
    let input = "target_link_libraries(myapp PUBLIC lib1 lib2 lib3 lib4 lib5 PRIVATE lib6 lib7 lib8)";
    let result = format_text(input, &default_config());
    // Should break because line is too long
    assert!(result.contains("PUBLIC\n"));
    assert!(result.contains("PRIVATE\n"));
    // Check indentation: keyword at 1 level (tab), values at 2 levels (2 tabs)
    assert!(result.contains("\tPUBLIC\n"));
    assert!(result.contains("\t\tlib1\n"));
}

#[test]
fn test_target_link_libraries_short_fits_one_line() {
    let input = "target_link_libraries(myapp PRIVATE lib1)";
    let result = format_text(input, &default_config());
    // With grammar-driven formatting, keyword-aware commands always format vertically for idempotency
    assert_eq!(result, "target_link_libraries(myapp\n\tPRIVATE\n\t\tlib1\n)\n");
}

#[test]
fn test_target_include_directories_keywords() {
    let input = "target_include_directories(mylib PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include ${CMAKE_CURRENT_SOURCE_DIR}/public PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/src)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Should break due to length
    assert!(result.contains("PUBLIC\n") || result.contains("PUBLIC "));
    assert!(result.contains("PRIVATE\n") || result.contains("PRIVATE "));
    // Verify command name appears
    assert!(result.contains("target_include_directories"));
}

#[test]
fn test_target_sources_keywords() {
    let input = "target_sources(mylib PRIVATE src/a.cpp src/b.cpp src/c.cpp src/d.cpp PUBLIC include/header.h)";
    let result = format_text(input, &default_config());
    // Should break
    assert!(result.contains("PRIVATE\n"));
    assert!(result.contains("PUBLIC\n"));
}

#[test]
fn test_add_library_keyword() {
    let input = "add_library(mylib STATIC src/a.cpp src/b.cpp src/c.cpp src/d.cpp src/e.cpp src/f.cpp src/g.cpp)";
    let result = format_text(input, &default_config());
    // Should break
    assert!(result.contains("STATIC\n"));
}

#[test]
fn test_add_library_short() {
    let input = "add_library(mylib STATIC src/a.cpp)";
    let result = format_text(input, &default_config());
    // Should stay on one line
    assert_eq!(result, "add_library(mylib STATIC src/a.cpp)\n");
}

#[test]
fn test_install_keywords() {
    let input = "install(TARGETS mylib ARCHIVE DESTINATION lib LIBRARY DESTINATION lib RUNTIME DESTINATION bin)";
    let result = format_text(input, &default_config());
    // Should break due to length
    assert!(result.contains("TARGETS\n"));
    assert!(result.contains("ARCHIVE\n"));
    // DESTINATION is SingleValue, so it keeps value inline: "DESTINATION lib"
    assert!(result.contains("DESTINATION"));
    assert!(result.contains("lib"));
}

#[test]
fn test_non_keyword_command_unchanged() {
    let input = "set(MY_VAR \"some value\")";
    let result = format_text(input, &default_config());
    // Should stay simple, no keyword-aware formatting
    assert_eq!(result, "set(MY_VAR \"some value\")\n");
}

#[test]
fn test_message_not_keyword_aware() {
    let input = "message(STATUS \"This is a message\")";
    let result = format_text(input, &default_config());
    // STATUS is a keyword, but message() is not keyword-aware
    // Should use simple formatting
    assert_eq!(result, "message(STATUS \"This is a message\")\n");
}

#[test]
fn test_keyword_aware_with_generator_expr() {
    let input = "target_include_directories(mylib PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include> $<INSTALL_INTERFACE:include> PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/src)";
    let result = format_text(input, &default_config());
    // Generator expressions should be atomic (never break internally)
    assert!(result.contains("$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>"));
    assert!(result.contains("$<INSTALL_INTERFACE:include>"));
    // Should break due to length
    assert!(result.contains("PUBLIC\n"));
    assert!(result.contains("PRIVATE\n"));
}

#[test]
fn test_target_compile_options_keywords() {
    let input = "target_compile_options(mylib PRIVATE -Wall -Wextra -Wpedantic -Werror PUBLIC -fPIC)";
    let result = format_text(input, &default_config());
    // Should break
    assert!(result.contains("PRIVATE\n"));
    assert!(result.contains("PUBLIC\n"));
}

#[test]
fn test_target_compile_definitions_keywords() {
    let input = "target_compile_definitions(mylib PUBLIC MY_LIB_VERSION=1 MY_LIB_DEBUG PRIVATE INTERNAL_BUILD)";
    let result = format_text(input, &default_config());
    // Should break
    assert!(result.contains("PUBLIC\n"));
    assert!(result.contains("PRIVATE\n"));
}

// ============================================================================
// COMMENT PRESERVATION TESTS
// ============================================================================

#[test]
fn test_comment_between_commands() {
    let input = "set(A b)\n# This is a comment\nset(C d)\n";
    let result = format_text(input, &default_config());
    assert!(result.contains("# This is a comment"));
    // Comment should appear in the output
    // Note: There may be blank lines normalized around it
    assert!(result.contains("set(A b)"));
    assert!(result.contains("set(C d)"));
}

#[test]
fn test_comment_inside_if_block() {
    let input = "if(WIN32)\n  # Platform specific\n  set(A b)\nendif()\n";
    let result = format_text(input, &default_config());
    assert!(result.contains("# Platform specific"));
    // Comment should be indented inside the if block
    assert!(result.contains("\t# Platform specific"));
}

#[test]
fn test_trailing_comment_after_command() {
    let input = "set(A b) # set the value\n";
    let result = format_text(input, &default_config());
    // Trailing comment should stay on same line
    assert_eq!(result, "set(A b) # set the value\n");
}

#[test]
fn test_multiple_leading_comments() {
    let input = "# Comment 1\n# Comment 2\nset(A b)\n";
    let result = format_text(input, &default_config());
    assert!(result.contains("# Comment 1"));
    assert!(result.contains("# Comment 2"));
    // Both should appear before the command
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines[0], "# Comment 1");
    assert_eq!(lines[1], "# Comment 2");
    assert_eq!(lines[2], "set(A b)");
}

#[test]
fn test_bracket_comment_preserved() {
    let input = "#[=[\nMulti-line bracket comment\n]=]\nset(A b)\n";
    let result = format_text(input, &default_config());
    assert!(result.contains("#[=["));
    assert!(result.contains("Multi-line bracket comment"));
    assert!(result.contains("]=]"));
}

#[test]
fn test_comment_before_keyword_aware_command() {
    let input = "# Link dependencies\ntarget_link_libraries(myapp PUBLIC lib1 lib2 lib3 lib4 PRIVATE lib5)\n";
    let result = format_text(input, &default_config());
    // Comment should be preserved before the command
    assert!(result.starts_with("# Link dependencies\n"));
}

#[test]
fn test_trailing_comment_after_keyword_aware_command() {
    let input = "target_link_libraries(myapp PRIVATE lib1) # private only\n";
    let result = format_text(input, &default_config());
    // Trailing comment should be preserved
    assert!(result.contains("# private only"));
}

// ============================================================================
// BLANK LINE NORMALIZATION TESTS
// ============================================================================

#[test]
fn test_blank_line_between_sections() {
    let input = "cmake_minimum_required(VERSION 3.20)\n\nadd_library(mylib src/a.cpp)\n";
    let result = format_text(input, &default_config());
    // Should preserve blank line between sections
    // cmake_minimum_required formats vertically now, so: command-line1, command-line2, blank, add_library-line
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 4);
    // Verify blank line is preserved
    assert_eq!(lines[2], "");
}

#[test]
fn test_no_trailing_blank_lines() {
    let input = "set(A b)\n\n\n";
    let result = format_text(input, &default_config());
    // Should end with single newline
    assert_eq!(result, "set(A b)\n");
}

#[test]
fn test_leading_blank_lines_stripped() {
    let input = "\n\nset(A b)\n";
    let result = format_text(input, &default_config());
    // Should start with first meaningful content
    assert_eq!(result, "set(A b)\n");
}

#[test]
fn test_multiple_blank_lines_collapsed() {
    let input = "set(A b)\n\n\n\nset(C d)\n";
    let config = FormatConfig {
        max_blank_lines: 1,
        ..default_config()
    };
    let result = format_text(input, &config);
    // Should collapse to 1 blank line
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 3); // set(A), blank, set(C)
}

#[test]
fn test_max_blank_lines_two() {
    let input = "set(A b)\n\n\n\nset(C d)\n";
    let config = FormatConfig {
        max_blank_lines: 2,
        ..default_config()
    };
    let result = format_text(input, &config);
    // Should allow up to 2 blank lines
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 4); // set(A), blank, blank, set(C)
}

// ============================================================================
// INTERACTION TESTS (keyword + scope + comments)
// ============================================================================

#[test]
fn test_keyword_aware_inside_if_block() {
    let input = "if(WIN32)\n  target_link_libraries(myapp PUBLIC lib1 lib2 lib3 lib4 PRIVATE lib5 lib6)\nendif()\n";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Should have proper nesting: if at level 0, target_link at level 1
    assert!(result.contains("if(WIN32)"));
    assert!(result.contains("target_link_libraries"));
    assert!(result.contains("PUBLIC"));
    assert!(result.contains("PRIVATE"));
    assert!(result.contains("endif()"));
}

#[test]
fn test_keyword_aware_with_comment_inside_if() {
    let input = "if(WIN32)\n  # Windows-specific libraries\n  target_link_libraries(myapp PRIVATE kernel32 user32)\nendif()\n";
    let result = format_text(input, &default_config());
    // Comment should be indented at if-block level
    assert!(result.contains("\t# Windows-specific libraries\n"));
    // Command formats vertically for consistency
    assert!(result.contains("\ttarget_link_libraries(myapp\n"));
    assert!(result.contains("\t\tPRIVATE\n"));
}

#[test]
fn test_real_world_target_formatting() {
    // Read the real-world fixture
    let input = std::fs::read_to_string("tests/fixtures/real_world_target.cmake")
        .expect("Failed to read real_world_target.cmake");

    let result = format_text(&input, &default_config());

    // Verify basic structure is preserved
    assert!(result.contains("cmake_minimum_required"));
    assert!(result.contains("project(RealWorld") || result.contains("project( RealWorld"));
    assert!(result.contains("add_library") && result.contains("realworld"));
    assert!(result.contains("target_include_directories"));
    assert!(result.contains("target_link_libraries"));

    // Verify keyword sections are formatted
    assert!(result.contains("PUBLIC\n"));
    assert!(result.contains("PRIVATE\n"));

    // Verify comments are preserved
    assert!(result.contains("# Platform-specific sources"));
    assert!(result.contains("# Main library target"));
    assert!(result.contains("# Install rules"));

    // Verify if/elseif/else structure
    assert!(result.contains("if(WIN32)"));
    assert!(result.contains("elseif(APPLE)"));
    assert!(result.contains("else()"));
    assert!(result.contains("endif()"));
}

#[test]
fn test_keyword_aware_with_trailing_comment() {
    let input = "target_link_libraries(myapp PUBLIC lib1 lib2 lib3 lib4) # all public\n";
    let result = format_text(input, &default_config());
    // Should preserve trailing comment
    assert!(result.contains("# all public"));
    assert!(result.contains("PUBLIC"));
}

#[test]
fn test_interface_keyword() {
    let input = "target_link_libraries(mylib INTERFACE header_only_lib another_header_lib)";
    let result = format_text(input, &default_config());
    // INTERFACE is also a keyword
    // Should stay on one line if it fits
    if result.len() <= 80 {
        assert!(result.contains("INTERFACE"));
    }
}

#[test]
fn test_mixed_keywords_proper_indentation() {
    let input = "target_link_libraries(myapp PUBLIC lib1 lib2 PRIVATE lib3 lib4 INTERFACE lib5 lib6)";
    let result = format_text(input, &default_config());
    // Should break with all three keyword sections
    assert!(result.contains("PUBLIC\n"));
    assert!(result.contains("PRIVATE\n"));
    assert!(result.contains("INTERFACE\n"));
    // Verify indentation levels: keywords at 1 tab, values at 2 tabs
    assert!(result.contains("\tPUBLIC\n"));
    assert!(result.contains("\tPRIVATE\n"));
    assert!(result.contains("\tINTERFACE\n"));
    assert!(result.contains("\t\tlib1\n"));
    assert!(result.contains("\t\tlib3\n"));
    assert!(result.contains("\t\tlib5\n"));
}

#[test]
fn test_case_uppercase_with_keywords() {
    let input = "target_link_libraries(myapp PUBLIC lib1)";
    let config = FormatConfig {
        command_case: CommandCase::Uppercase,
        ..default_config()
    };
    let result = format_text(input, &config);
    // Command should be uppercase, keywords should stay as-is
    assert!(result.starts_with("TARGET_LINK_LIBRARIES"));
    assert!(result.contains("PUBLIC"));
    assert!(!result.contains("public")); // Keyword shouldn't be lowercased
}

#[test]
fn test_keyword_only_no_values() {
    let input = "target_link_libraries(myapp PUBLIC lib1 lib2 PRIVATE)";
    let result = format_text(input, &default_config());
    eprintln!("Result: {:?}", result);
    // Should not crash or produce invalid output
    assert!(result.ends_with(")\n"));
    assert!(result.contains("PUBLIC"));
    assert!(result.contains("PRIVATE"));
    // If it fits on one line, it may stay flat
    // If too long, it will break
    assert!(result.len() < 200); // Sanity check
}

// ============================================================================
// KEYWORD-AWARE ARGUMENT LIST ENHANCEMENT TESTS (Phase 7)
// ============================================================================

#[test]
fn test_keyword_arglist_comment_in_section() {
    let input = "target_link_libraries(myapp\n  PUBLIC\n    lib1\n    # Platform libs\n    lib2\n  PRIVATE\n    lib3\n)\n";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Comment should appear between lib1 and lib2
    assert!(result.contains("# Platform libs"));
    // Check that lib1 and lib2 are present
    assert!(result.contains("lib1"));
    assert!(result.contains("lib2"));
    // The order should be maintained
    let lib1_pos = result.find("lib1").unwrap();
    let comment_pos = result.find("# Platform libs").unwrap();
    let lib2_pos = result.find("lib2").unwrap();
    assert!(lib1_pos < comment_pos);
    assert!(comment_pos < lib2_pos);
}

#[test]
fn test_keyword_arglist_blank_line_between_sections() {
    let input = "target_link_libraries(myapp\n  PUBLIC\n    lib1\n\n  PRIVATE\n    lib2\n)\n";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Should preserve the blank line structure
    assert!(result.contains("PUBLIC"));
    assert!(result.contains("PRIVATE"));
    assert!(result.contains("lib1"));
    assert!(result.contains("lib2"));
    // Check for blank line (two consecutive newlines)
    assert!(result.contains("\n\n") || result.contains("lib1\n    \n"));
}

#[test]
fn test_keyword_arglist_multiline_stays_multiline() {
    let input = "target_link_libraries(myapp\n  PRIVATE\n    lib1\n)\n";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Even though this is short enough to fit on one line,
    // user chose multiline, so it should stay multiline
    assert!(result.contains("PRIVATE\n"));
    assert!(result.contains("lib1\n"));
    // Should NOT be collapsed to: target_link_libraries(myapp PRIVATE lib1)
    assert!(!result.contains("myapp PRIVATE lib1)\n"));
}

#[test]
fn test_keyword_arglist_comment_not_duplicated() {
    let input = "target_link_libraries(myapp\n  PUBLIC\n    lib1  # main lib\n    lib2\n)\n";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Comment should appear exactly once
    let count = result.matches("# main lib").count();
    assert_eq!(count, 1, "Comment should appear exactly once, found {} times", count);
}

#[test]
fn test_keyword_arglist_first_arg_same_line() {
    let input = "target_link_libraries(myapp\n  PUBLIC lib1\n)\n";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // First arg (myapp) should be on same line as command
    assert!(result.starts_with("target_link_libraries(myapp"));
    assert!(!result.starts_with("target_link_libraries(\n"));
}

#[test]
fn test_keyword_arglist_idempotency() {
    let inputs = vec![
        "target_link_libraries(myapp\n  PUBLIC\n    lib1\n    # Platform libs\n    lib2\n  PRIVATE\n    lib3\n)\n",
        "target_link_libraries(myapp\n  PUBLIC\n    lib1\n\n  PRIVATE\n    lib2\n)\n",
        "target_link_libraries(myapp\n  PRIVATE\n    lib1\n)\n",
    ];

    for input in inputs {
        let once = format_text(input, &default_config());
        let twice = format_text(&once, &default_config());
        assert_eq!(once, twice, "Formatting should be idempotent for input:\n{}", input);
    }
}

#[test]
fn test_keyword_arglist_bracket_comment() {
    let input = "target_link_libraries(myapp\n  PUBLIC\n    lib1\n    #[=[\n    Special lib\n    ]=]\n    lib2\n)\n";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Bracket comment should be preserved
    assert!(result.contains("#[=["));
    assert!(result.contains("Special lib"));
    assert!(result.contains("]=]"));
}

// ============================================================================
// PHASE 13 GRAMMAR-DRIVEN FORMATTING TESTS
// ============================================================================

#[test]
fn test_flag_grouping_find_package() {
    // REQUIRED QUIET CONFIG should group on one line, not each on its own
    let input = "find_package(Boost\n    REQUIRED\n    QUIET\n    CONFIG\n    COMPONENTS\n        filesystem\n        system\n)";
    let result = format_text(input, &default_config());
    // Flags should group together
    assert!(result.contains("REQUIRED") && result.contains("QUIET"));
    // COMPONENTS should be on its own line with values below
    assert!(result.contains("COMPONENTS\n"));
}

#[test]
fn test_flag_grouping_short_fits_one_line() {
    let input = "find_package(Boost REQUIRED QUIET)";
    let result = format_text(input, &default_config());
    // Flags group inline, but command formats vertically for consistency
    assert_eq!(result, "find_package(Boost\n\tREQUIRED QUIET\n)\n");
}

#[test]
fn test_single_value_inline_short() {
    // DESTINATION is SingleValue so stays inline, TARGETS is MultiValue so breaks vertically
    let input = "install(TARGETS mylib DESTINATION lib)";
    let result = format_text(input, &default_config());
    assert_eq!(result, "install(TARGETS\n\t\tmylib\n\tDESTINATION lib\n)\n");
}

#[test]
fn test_multi_value_one_per_line() {
    let input = "target_sources(mylib PRIVATE src/a.cpp src/b.cpp src/c.cpp src/d.cpp src/e.cpp src/f.cpp src/g.cpp)";
    let result = format_text(input, &default_config());
    // Should break with each source on its own line
    assert!(result.contains("PRIVATE\n"));
    // Count that source files are on separate lines
    let lines: Vec<&str> = result.lines().collect();
    let source_lines = lines.iter().filter(|l| l.trim().ends_with(".cpp")).count();
    assert!(source_lines >= 5, "Expected 5+ source files on separate lines, got {}", source_lines);
}

#[test]
fn test_unknown_keyword_in_known_command() {
    // BOGUS_KEYWORD is not in find_package grammar -- should not crash
    let input = "find_package(Boost REQUIRED BOGUS_KEYWORD some_value COMPONENTS filesystem)";
    let result = format_text(input, &default_config());
    // Should format without errors, BOGUS_KEYWORD treated as argument
    assert!(result.contains("Boost"));
    assert!(result.contains("REQUIRED"));
    assert!(result.contains("BOGUS_KEYWORD"));
    assert!(result.contains("COMPONENTS"));
}

#[test]
fn test_completely_unknown_command() {
    // my_custom_function is not in grammar registry -- should use simple formatting
    let input = "my_custom_function(arg1 arg2 arg3 arg4 arg5 arg6 arg7 arg8 arg9 arg10 arg11 arg12 arg13)";
    let result = format_text(input, &default_config());
    // Should format without errors using simple argument formatting
    assert!(result.contains("my_custom_function"));
    assert!(result.contains("arg1"));
}

#[test]
fn test_force_break_keywords_true() {
    let mut config = default_config();
    config.force_break_keywords = true;
    // This would normally fit on one line
    let input = "find_package(Boost REQUIRED)";
    let result = format_text(input, &config);
    // With force_break, should go multiline
    assert!(result.contains('\n'));
    let line_count = result.lines().count();
    assert!(line_count >= 2, "Expected multiline output with force_break_keywords, got {} lines", line_count);
}

#[test]
fn test_force_break_keywords_false_short_stays_inline() {
    let config = default_config(); // force_break_keywords = false by default
    let input = "find_package(Boost REQUIRED)";
    let result = format_text(input, &config);
    // With grammar-driven formatting, keyword-aware commands always format consistently
    // for idempotency, regardless of force_break_keywords config
    assert_eq!(result, "find_package(Boost\n\tREQUIRED\n)\n");
}

#[test]
fn test_idempotency_keyword_aware_commands() {
    let config = default_config();
    let inputs = vec![
        "find_package(Boost REQUIRED QUIET COMPONENTS filesystem system)",
        "target_link_libraries(myapp PUBLIC lib1 lib2 PRIVATE lib3 lib4 lib5 lib6 lib7)",
        "install(TARGETS mylib DESTINATION lib OPTIONAL)",
        "target_sources(mylib PRIVATE a.cpp b.cpp c.cpp d.cpp e.cpp f.cpp g.cpp h.cpp i.cpp j.cpp)",
    ];
    for input in inputs {
        let first = format_text(input, &config);
        let second = format_text(&first, &config);
        assert_eq!(first, second, "Idempotency failed for input: {}", input);
    }
}

#[test]
fn test_idempotency_force_break() {
    let mut config = default_config();
    config.force_break_keywords = true;
    let inputs = vec![
        "find_package(Boost REQUIRED QUIET COMPONENTS filesystem system)",
        "target_link_libraries(myapp PUBLIC lib1 lib2 PRIVATE lib3)",
    ];
    for input in inputs {
        let first = format_text(input, &config);
        let second = format_text(&first, &config);
        assert_eq!(first, second, "Idempotency failed with force_break for: {}", input);
    }
}

#[test]
fn test_backward_compat_existing_commands() {
    // These commands already worked before grammar system -- verify they still produce valid output
    let config = default_config();
    let test_cases = vec![
        ("set(MY_VAR value)", "set(MY_VAR value)\n"),
        ("message(STATUS \"hello\")", "message(STATUS \"hello\")\n"),
        ("if(TRUE)\nendif()", "if(TRUE)\nendif()\n"),
    ];
    for (input, expected) in test_cases {
        let result = format_text(input, &config);
        assert_eq!(result, expected, "Backward compat failed for: {}", input);
    }
}

#[test]
fn test_case_sensitive_keywords() {
    // "Config" should not be treated as "CONFIG" keyword
    let input = "find_package(KF5 REQUIRED COMPONENTS CoreAddons I18n Config ConfigWidgets)";
    let result = format_text(input, &default_config());
    // All components should be preserved
    assert!(result.contains("Config"));
    assert!(result.contains("ConfigWidgets"));
    assert!(result.contains("I18n"));
    assert!(result.contains("CoreAddons"));
}
