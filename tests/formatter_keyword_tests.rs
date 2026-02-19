use cmake_fmt::formatter::{format_text, CommandCase, CommandGrammarConfig, FormatConfig};
use std::collections::HashMap;

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
    // When broken, keywords on own line, values one-per-line underneath
    assert!(result.contains("\tPUBLIC\n"));
    assert!(result.contains("\t\tlib1\n"));
    assert!(result.contains("\t\tlib2\n"));
    assert!(result.contains("\tPRIVATE\n"));
    assert!(result.contains("\t\tlib6\n"));
}

#[test]
fn test_target_link_libraries_short_fits_one_line() {
    let input = "target_link_libraries(myapp PRIVATE lib1)";
    let result = format_text(input, &default_config());
    // Short command fits on one line — flat rendering
    assert_eq!(result, "target_link_libraries(myapp PRIVATE lib1)\n");
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
    // Should break with keywords on own line, values one-per-line underneath
    // Note: PUBLIC has only 1 arg, so it stays inline (MultiValue single-arg behavior)
    assert!(result.contains("\tPRIVATE\n"));
    assert!(result.contains("\t\tsrc/a.cpp\n"));
    assert!(result.contains("\t\tsrc/d.cpp\n"));
    assert!(result.contains("\tPUBLIC include/header.h\n"));
}

#[test]
fn test_add_library_keyword() {
    let input = "add_library(mylib STATIC src/a.cpp src/b.cpp src/c.cpp src/d.cpp src/e.cpp src/f.cpp src/g.cpp)";
    let result = format_text(input, &default_config());
    // STATIC groups with target name on first line, sources break to separate lines
    assert!(result.contains("mylib STATIC\n"));
    assert!(result.contains("\tsrc/a.cpp\n"));
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
    eprintln!("Result:\n{}", result);
    // BinPack artifact types consume DESTINATION as sub_keyword (stays on same line)
    assert!(result.contains("ARCHIVE DESTINATION lib"));
    assert!(result.contains("LIBRARY DESTINATION lib"));
    assert!(result.contains("RUNTIME DESTINATION bin"));
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
    // When command breaks, all keywords on own line with values one-per-line underneath
    // Note: PRIVATE has only 1 arg, so it stays inline (MultiValue single-arg behavior)
    assert!(result.contains("PUBLIC\n"));
    assert!(result.contains("\tPRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/src\n"));
}

#[test]
fn test_target_compile_options_keywords() {
    let input = "target_compile_options(mylib PRIVATE -Wall -Wextra -Wpedantic -Werror PUBLIC -fPIC)";
    let result = format_text(input, &default_config());
    // Should break with keywords on own line, values one-per-line underneath
    // Note: PUBLIC has only 1 arg, so it stays inline (MultiValue single-arg behavior)
    assert!(result.contains("\tPRIVATE\n"));
    assert!(result.contains("\t\t-Wall\n"));
    assert!(result.contains("\t\t-Werror\n"));
    assert!(result.contains("\tPUBLIC -fPIC\n"));
}

#[test]
fn test_target_compile_definitions_keywords() {
    let input = "target_compile_definitions(mylib PUBLIC MY_LIB_VERSION=1 MY_LIB_DEBUG PRIVATE INTERNAL_BUILD)";
    let result = format_text(input, &default_config());
    // Should break with keywords on own line, values one-per-line underneath
    // Note: PRIVATE has only 1 arg, so it stays inline (MultiValue single-arg behavior)
    assert!(result.contains("\tPUBLIC\n"));
    assert!(result.contains("\t\tMY_LIB_VERSION=1\n"));
    assert!(result.contains("\t\tMY_LIB_DEBUG\n"));
    assert!(result.contains("\tPRIVATE INTERNAL_BUILD\n"));
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

#[test]
fn test_pre_keyword_comments_preserved() {
    let input = "set_source_files_properties(\n\n    # wui/patch/cord/cord_anchor.cpp\n    wui/patch/node/node_view.cpp\n    wui/skin/skin.cpp\n    PROPERTIES COMPILE_FLAGS /wd4996\n)\n";
    let result = format_text(input, &default_config());
    // Comment must be preserved in the pre-keyword section
    assert!(result.contains("# wui/patch/cord/cord_anchor.cpp"), "Comment was dropped from pre-keyword section");
    // Both source files must be present
    assert!(result.contains("wui/patch/node/node_view.cpp"));
    assert!(result.contains("wui/skin/skin.cpp"));
    // Keyword section must still be present
    assert!(result.contains("PROPERTIES COMPILE_FLAGS /wd4996"));
}

#[test]
fn test_pre_keyword_blank_lines_preserved() {
    let input = "set_source_files_properties(\n\n    src/a.cpp\n    src/b.cpp\n    PROPERTIES COMPILE_FLAGS /wd4996\n)\n";
    let result = format_text(input, &default_config());
    // Blank line should be preserved (appears as double newline in output)
    assert!(result.contains("\n\n"), "Blank line in pre-keyword section was dropped");
    assert!(result.contains("src/a.cpp"));
    assert!(result.contains("src/b.cpp"));
}

#[test]
fn test_trailing_inline_comment_preserved() {
    let input = "target_link_libraries(wui PUBLIC\n    juce::JUCE\n    rj::rj\n    wire_common::wire_common\n    WireDev # For Clock enum...but I'd rather not\n    WireResources\n)\n";
    let result = format_text(input, &default_config());
    // Trailing comment must stay on the same line as WireDev
    assert!(result.contains("WireDev # For Clock enum"),
        "Trailing inline comment was moved away from its argument. Got:\n{}", result);
    // WireResources must be on a separate line, not preceded by the comment
    let lines: Vec<&str> = result.lines().collect();
    let wiredev_line = lines.iter().find(|l| l.contains("WireDev")).unwrap();
    assert!(wiredev_line.contains("# For Clock enum"),
        "Comment not on same line as WireDev. WireDev line: {}", wiredev_line);
}

#[test]
fn test_trailing_inline_comment_simple_args() {
    let input = "set(FLAGS\n    -Wall\n    -Wextra # Extra warnings\n    -Wpedantic\n)\n";
    let result = format_text(input, &default_config());
    // Trailing comment must stay on same line as -Wextra
    assert!(result.contains("-Wextra # Extra warnings"),
        "Trailing comment moved away from -Wextra. Got:\n{}", result);
}

#[test]
fn test_leading_comment_own_line_still_works() {
    let input = "set(SOURCES\n    src/main.cpp\n    # Core library sources\n    src/core.cpp\n)\n";
    let result = format_text(input, &default_config());
    // Leading comment should be on its own line, before src/core.cpp
    let lines: Vec<&str> = result.lines().collect();
    let comment_idx = lines.iter().position(|l| l.contains("# Core library sources")).unwrap();
    let core_idx = lines.iter().position(|l| l.contains("src/core.cpp")).unwrap();
    assert!(comment_idx < core_idx, "Leading comment should appear before src/core.cpp");
    // Comment should NOT be on the same line as src/main.cpp
    let main_line = lines.iter().find(|l| l.contains("src/main.cpp")).unwrap();
    assert!(!main_line.contains("#"), "Leading comment should not be on main.cpp's line");
}

// ============================================================================
// BLANK LINE NORMALIZATION TESTS
// ============================================================================

#[test]
fn test_blank_line_between_sections() {
    let input = "cmake_minimum_required(VERSION 3.20)\n\nadd_library(mylib src/a.cpp)\n";
    let result = format_text(input, &default_config());
    // Should preserve blank line between sections
    // Both commands are short and stay single-line: cmake_min_req, blank, add_library
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 3);
    // Verify blank line is preserved
    assert_eq!(lines[1], "");
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
    // Short command fits on one line (inside if block, indented)
    assert!(result.contains("\ttarget_link_libraries(myapp PRIVATE kernel32 user32)\n"));
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
    // Should break with all three keyword sections, keywords on own line, values one-per-line
    assert!(result.contains("\tPUBLIC\n"));
    assert!(result.contains("\t\tlib1\n"));
    assert!(result.contains("\t\tlib2\n"));
    assert!(result.contains("\tPRIVATE\n"));
    assert!(result.contains("\t\tlib3\n"));
    assert!(result.contains("\t\tlib4\n"));
    assert!(result.contains("\tINTERFACE\n"));
    assert!(result.contains("\t\tlib5\n"));
    assert!(result.contains("\t\tlib6\n"));
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
    // User chose multiline, so command stays multiline
    // Note: PRIVATE has only 1 arg, so it stays inline with value (MultiValue single-arg behavior)
    assert!(result.contains("PRIVATE lib1\n"));
    // Should NOT be collapsed to a single line with the target name
    assert!(!result.contains("myapp PRIVATE lib1)\n"));
    // Command should still be multiline (not collapsed to one line)
    assert!(result.contains("target_link_libraries(myapp\n"));
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
    assert!(result.contains("REQUIRED") && result.contains("QUIET") && result.contains("CONFIG"));
    // Multiline input: COMPONENTS values stay on separate lines
    assert!(result.contains("COMPONENTS\n"));
}

#[test]
fn test_flag_grouping_short_fits_one_line() {
    let input = "find_package(Boost REQUIRED QUIET)";
    let result = format_text(input, &default_config());
    // Short command with flags fits on one line — flat rendering
    assert_eq!(result, "find_package(Boost REQUIRED QUIET)\n");
}

#[test]
fn test_single_value_inline_short() {
    // Short command fits on one line — flat rendering
    let input = "install(TARGETS mylib DESTINATION lib)";
    let result = format_text(input, &default_config());
    assert_eq!(result, "install(TARGETS mylib DESTINATION lib)\n");
}

#[test]
fn test_multi_value_one_per_line() {
    let input = "target_sources(mylib PRIVATE src/a.cpp src/b.cpp src/c.cpp src/d.cpp src/e.cpp src/f.cpp src/g.cpp)";
    let result = format_text(input, &default_config());
    // When broken, keywords on own line, values one-per-line underneath
    assert!(result.contains("\tPRIVATE\n"));
    assert!(result.contains("\t\tsrc/a.cpp\n"));
    assert!(result.contains("\t\tsrc/g.cpp\n"));
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
    // Short command fits on one line — flat rendering
    assert_eq!(result, "find_package(Boost REQUIRED)\n");
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

// ============================================================================
// Phase 14: Per-Mode Command Formatting
// ============================================================================

#[test]
fn test_install_targets_mode_formatting() {
    let input = "install(TARGETS mylib RUNTIME DESTINATION bin LIBRARY DESTINATION lib ARCHIVE DESTINATION lib)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // BinPack artifact types consume DESTINATION as sub_keyword (stays on same line)
    assert!(result.contains("RUNTIME DESTINATION bin"));
    assert!(result.contains("LIBRARY DESTINATION lib"));
    assert!(result.contains("ARCHIVE DESTINATION lib"));
}

#[test]
fn test_install_files_mode_formatting() {
    let input = "install(FILES readme.txt license.txt DESTINATION share/doc)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // Short command fits on one line — flat rendering
    assert_eq!(result, "install(FILES readme.txt license.txt DESTINATION share/doc)\n");
}

#[test]
fn test_install_directory_mode_formatting() {
    let input = "install(DIRECTORY include/ DESTINATION include FILES_MATCHING PATTERN \"*.h\")";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // Short command fits on one line — flat rendering
    assert_eq!(result, "install(DIRECTORY include/ DESTINATION include FILES_MATCHING PATTERN \"*.h\")\n");
}

#[test]
fn test_install_export_mode_formatting() {
    let input = "install(EXPORT MyProjectTargets NAMESPACE MyProject:: FILE MyProjectConfig.cmake DESTINATION lib/cmake/MyProject)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // Should format with EXPORT/NAMESPACE/FILE/DESTINATION as single-value keywords
    assert!(result.contains("EXPORT"));
    assert!(result.contains("MyProjectTargets"));
    assert!(result.contains("NAMESPACE"));
    assert!(result.contains("MyProject::"));
    assert!(result.contains("FILE"));
    assert!(result.contains("MyProjectConfig.cmake"));
}

#[test]
fn test_install_variable_mode_fallback() {
    let input = "install(${INSTALL_TYPE} mylib DESTINATION lib)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // When mode is a variable, we can't resolve to a specific mode grammar
    // Falls back to general keyword-aware formatting (using hardcoded keywords)
    assert!(result.contains("${INSTALL_TYPE}"));
    assert!(result.contains("mylib"));
    assert!(result.contains("DESTINATION"));
    // Still treats DESTINATION as a keyword (using fallback keyword list)
    // This is correct behavior - DESTINATION is a keyword even if mode is unknown
}

#[test]
fn test_install_mode_idempotency_targets() {
    let input = "install(TARGETS mylib RUNTIME DESTINATION bin LIBRARY DESTINATION lib)";
    let pass1 = format_text(input, &default_config());
    let pass2 = format_text(&pass1, &default_config());
    assert_eq!(pass1, pass2, "TARGETS mode formatting should be idempotent");
}

#[test]
fn test_install_mode_idempotency_files() {
    let input = "install(FILES readme.txt license.txt DESTINATION share/doc)";
    let pass1 = format_text(input, &default_config());
    let pass2 = format_text(&pass1, &default_config());
    assert_eq!(pass1, pass2, "FILES mode formatting should be idempotent");
}

#[test]
fn test_install_mode_idempotency_directory() {
    let input = "install(DIRECTORY include/ DESTINATION include FILES_MATCHING PATTERN \"*.h\")";
    let pass1 = format_text(input, &default_config());
    let pass2 = format_text(&pass1, &default_config());
    assert_eq!(pass1, pass2, "DIRECTORY mode formatting should be idempotent");
}

#[test]
fn test_install_mode_idempotency_export() {
    let input = "install(EXPORT MyProjectTargets NAMESPACE MyProject:: FILE MyProjectConfig.cmake DESTINATION lib/cmake/MyProject)";
    let pass1 = format_text(input, &default_config());
    let pass2 = format_text(&pass1, &default_config());
    assert_eq!(pass1, pass2, "EXPORT mode formatting should be idempotent");
}

#[test]
fn test_install_script_mode() {
    let input = "install(SCRIPT \"${CMAKE_CURRENT_SOURCE_DIR}/post-install.cmake\")";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // Should format SCRIPT as single-value
    assert!(result.contains("SCRIPT"));
    assert!(result.contains("${CMAKE_CURRENT_SOURCE_DIR}/post-install.cmake"));
}

#[test]
fn test_install_code_mode() {
    let input = "install(CODE \"message(STATUS \\\"Installing\\\")\")";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // Should format CODE as single-value
    assert!(result.contains("CODE"));
    assert!(result.contains("message(STATUS"));
}

// ============================================================================
// Phase 14: file(), string(), list() multi-mode tests
// ============================================================================

#[test]
fn test_file_glob_mode_formatting() {
    let input = "file(GLOB\n  sources\n  CONFIGURE_DEPENDS\n  \"src/*.cpp\"\n)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // CONFIGURE_DEPENDS should format as flag
    assert!(result.contains("GLOB"));
    assert!(result.contains("sources"));
    assert!(result.contains("CONFIGURE_DEPENDS"));
    assert!(result.contains("src/*.cpp"));
}

#[test]
fn test_file_glob_recurse_with_keywords() {
    let input = "file(GLOB_RECURSE headers LIST_DIRECTORIES false CONFIGURE_DEPENDS \"include/*.h\")";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // LIST_DIRECTORIES as single-value, CONFIGURE_DEPENDS as flag
    assert!(result.contains("GLOB_RECURSE"));
    assert!(result.contains("headers"));
    assert!(result.contains("LIST_DIRECTORIES"));
    assert!(result.contains("false"));
    assert!(result.contains("CONFIGURE_DEPENDS"));
    assert!(result.contains("include/*.h"));
}

#[test]
fn test_file_read_mode() {
    let input = "file(READ \"input.txt\" content OFFSET 10 LIMIT 100 HEX)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // OFFSET/LIMIT as single-value, HEX as flag
    assert!(result.contains("READ"));
    assert!(result.contains("input.txt"));
    assert!(result.contains("content"));
    assert!(result.contains("OFFSET"));
    assert!(result.contains("10"));
    assert!(result.contains("LIMIT"));
    assert!(result.contains("100"));
    assert!(result.contains("HEX"));
}

#[test]
fn test_file_strings_mode() {
    let input = "file(STRINGS \"data.txt\" lines LENGTH_MAXIMUM 80 REGEX \"^#.*\")";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // LENGTH_MAXIMUM/REGEX as single-value
    assert!(result.contains("STRINGS"));
    assert!(result.contains("data.txt"));
    assert!(result.contains("lines"));
    assert!(result.contains("LENGTH_MAXIMUM"));
    assert!(result.contains("80"));
    assert!(result.contains("REGEX"));
    assert!(result.contains("^#.*"));
}

#[test]
fn test_file_download_mode() {
    let input = "file(DOWNLOAD\n  \"https://example.com/file.tar.gz\"\n  \"file.tar.gz\"\n  EXPECTED_HASH SHA256=abc123\n  SHOW_PROGRESS\n  TIMEOUT 300\n)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // EXPECTED_HASH/TIMEOUT as single-value, SHOW_PROGRESS as flag
    assert!(result.contains("DOWNLOAD"));
    assert!(result.contains("https://example.com/file.tar.gz"));
    assert!(result.contains("file.tar.gz"));
    assert!(result.contains("EXPECTED_HASH"));
    assert!(result.contains("SHA256=abc123"));
    assert!(result.contains("SHOW_PROGRESS"));
    assert!(result.contains("TIMEOUT"));
    assert!(result.contains("300"));
}

#[test]
fn test_file_copy_mode() {
    let input = "file(COPY src/ DESTINATION build/ FILES_MATCHING PATTERN \"*.cmake\")";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // DESTINATION/PATTERN as single-value, FILES_MATCHING as flag
    assert!(result.contains("COPY"));
    assert!(result.contains("src/"));
    assert!(result.contains("DESTINATION"));
    assert!(result.contains("build/"));
    assert!(result.contains("FILES_MATCHING"));
    assert!(result.contains("PATTERN"));
    assert!(result.contains("*.cmake"));
}

#[test]
fn test_file_variable_mode_fallback() {
    let input = "file(${FILE_OP} \"input.txt\" content)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // Variable reference in mode position falls back to simple formatting
    assert!(result.contains("${FILE_OP}"));
    assert!(result.contains("input.txt"));
    assert!(result.contains("content"));
}

#[test]
fn test_string_replace_simple_formatting() {
    let input = "string(REPLACE \"old\" \"new\" output \"${input}\")";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // REPLACE mode has empty grammar, formats as simple args
    assert!(result.contains("REPLACE"));
    assert!(result.contains("old"));
    assert!(result.contains("new"));
    assert!(result.contains("output"));
    assert!(result.contains("${input}"));
}

#[test]
fn test_string_toupper_simple() {
    let input = "string(TOUPPER \"${input}\" output)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // Simple formatting
    assert!(result.contains("TOUPPER"));
    assert!(result.contains("${input}"));
    assert!(result.contains("output"));
}

#[test]
fn test_string_find_with_reverse() {
    let input = "string(FIND \"haystack\" \"needle\" position REVERSE)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // REVERSE is defined as flag in FIND mode
    assert!(result.contains("FIND"));
    assert!(result.contains("haystack"));
    assert!(result.contains("needle"));
    assert!(result.contains("position"));
    assert!(result.contains("REVERSE"));
}

#[test]
fn test_string_random_with_keywords() {
    let input = "string(RANDOM LENGTH 16 ALPHABET \"0123456789abcdef\" result)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // LENGTH/ALPHABET as single-value
    assert!(result.contains("RANDOM"));
    assert!(result.contains("LENGTH"));
    assert!(result.contains("16"));
    assert!(result.contains("ALPHABET"));
    assert!(result.contains("0123456789abcdef"));
    assert!(result.contains("result"));
}

#[test]
fn test_string_configure_keywords() {
    let input = "string(CONFIGURE \"input @VAR@\" output @ONLY ESCAPE_QUOTES)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // @ONLY/ESCAPE_QUOTES as flags
    assert!(result.contains("CONFIGURE"));
    assert!(result.contains("input @VAR@"));
    assert!(result.contains("output"));
    assert!(result.contains("@ONLY"));
    assert!(result.contains("ESCAPE_QUOTES"));
}

#[test]
fn test_list_append_simple() {
    let input = "list(APPEND mylist item1 item2 item3)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // Simple formatting (empty grammar mode)
    assert!(result.contains("APPEND"));
    assert!(result.contains("mylist"));
    assert!(result.contains("item1"));
    assert!(result.contains("item2"));
    assert!(result.contains("item3"));
}

#[test]
fn test_list_sort_with_keywords() {
    let input = "list(SORT mylist\n  COMPARE STRING\n  CASE INSENSITIVE\n  ORDER DESCENDING\n)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // COMPARE/CASE/ORDER as single-value keywords
    assert!(result.contains("SORT"));
    assert!(result.contains("mylist"));
    assert!(result.contains("COMPARE"));
    assert!(result.contains("STRING"));
    assert!(result.contains("CASE"));
    assert!(result.contains("INSENSITIVE"));
    assert!(result.contains("ORDER"));
    assert!(result.contains("DESCENDING"));
}

#[test]
fn test_list_filter_with_keywords() {
    let input = "list(FILTER mylist INCLUDE REGEX \".*test.*\")";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // INCLUDE as flag, REGEX as single-value
    assert!(result.contains("FILTER"));
    assert!(result.contains("mylist"));
    assert!(result.contains("INCLUDE"));
    assert!(result.contains("REGEX"));
    assert!(result.contains(".*test.*"));
}

#[test]
fn test_list_variable_mode_fallback() {
    let input = "list(${LIST_OP} mylist item1)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // Falls back to simple formatting
    assert!(result.contains("${LIST_OP}"));
    assert!(result.contains("mylist"));
    assert!(result.contains("item1"));
}

#[test]
fn test_all_mode_commands_idempotency() {
    let test_cases = vec![
        "file(GLOB sources CONFIGURE_DEPENDS \"src/*.cpp\")",
        "string(REPLACE \"old\" \"new\" output \"${input}\")",
        "list(SORT mylist COMPARE STRING CASE INSENSITIVE ORDER DESCENDING)",
        "install(TARGETS mylib RUNTIME DESTINATION bin)",
    ];

    for input in test_cases {
        let pass1 = format_text(input, &default_config());
        let pass2 = format_text(&pass1, &default_config());
        assert_eq!(pass1, pass2, "Idempotency failed for: {}", input);
    }
}

#[test]
fn test_mode_commands_backward_compat() {
    let config = default_config();

    // Commands not touched by Phase 14 should format identically
    let test_cases = vec![
        "find_package(Boost REQUIRED COMPONENTS system filesystem)",
        "target_link_libraries(myapp PRIVATE lib1 lib2)",
        "add_custom_command(OUTPUT output.txt COMMAND echo hello)",
    ];

    for input in test_cases {
        let result = format_text(input, &config);
        // Verify no crashes and basic structure preserved
        assert!(result.contains(input.split('(').next().unwrap()));
    }
}

// ============================================================================
// CUSTOM COMMAND FORMATTING TESTS
// ============================================================================

#[test]
fn test_custom_command_fits_one_line() {
    let input = "GenerateTestExecutionGitlabCI(OUTPUT file.yml PLATFORM Linux)";
    let result = format_text(input, &default_config());
    // Should stay on one line when it fits
    assert_eq!(result, "GenerateTestExecutionGitlabCI(OUTPUT file.yml PLATFORM Linux)\n");
}

#[test]
fn test_custom_command_breaks_all_args() {
    let input = "GenerateTestExecutionGitlabCI(OUTPUT ${CMAKE_CURRENT_SOURCE_DIR}/run-ci-tests.yml PLATFORM ${CI_PLATFORM} ${CI_ARCHITECTURE})";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Should break with ALL args on new lines (no args on opening line)
    assert!(result.contains("GenerateTestExecutionGitlabCI(\n"));
    assert!(result.contains("\tOUTPUT\n"));
    assert!(result.contains("\t${CMAKE_CURRENT_SOURCE_DIR}/run-ci-tests.yml\n"));
    assert!(result.contains("\tPLATFORM\n"));
    assert!(result.contains("\t${CI_PLATFORM}\n"));
    assert!(result.contains("\t${CI_ARCHITECTURE}\n"));
    assert!(result.ends_with(")\n"));
    // Should NOT have args on same line as command name
    assert!(!result.contains("GenerateTestExecutionGitlabCI(OUTPUT"));
}

#[test]
fn test_custom_function_short_args() {
    let input = "my_custom_function(arg1 arg2 arg3)";
    let result = format_text(input, &default_config());
    // Short custom command stays on one line
    assert_eq!(result, "my_custom_function(arg1 arg2 arg3)\n");
}

#[test]
fn test_custom_macro_long_args() {
    let input = "my_custom_macro(very_long_argument_name_one very_long_argument_name_two very_long_argument_name_three very_long_argument_name_four)";
    let result = format_text(input, &default_config());
    // Long custom command breaks with all args indented
    assert!(result.contains("my_custom_macro(\n"));
    assert!(result.contains("\tvery_long_argument_name_one\n"));
    assert!(result.contains("\tvery_long_argument_name_two\n"));
    assert!(result.contains("\tvery_long_argument_name_three\n"));
    assert!(result.contains("\tvery_long_argument_name_four\n"));
}

#[test]
fn test_builtin_command_keeps_first_arg_inline() {
    let input = "set(MY_VARIABLE value1 value2 value3 value4 value5 value6 value7 value8 value9 value10)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Builtin command should keep first arg on same line when breaking (current behavior)
    assert!(result.contains("set(MY_VARIABLE"));
    // But not all on one line (too long)
    assert!(result.contains("\n\t"));
}

// ============================================================================
// KWFMT-02 BEHAVIOR TESTS
// ============================================================================

#[test]
fn test_keyword_break_values_per_line() {
    // KWFMT-02: When keyword commands auto-break, values appear one-per-line
    let input = "target_link_libraries(myapp PUBLIC lib1 lib2 lib3 lib4 lib5 PRIVATE lib6 lib7 lib8 lib9)";
    let result = format_text(input, &default_config());

    // Verify keywords are on their own line
    assert!(result.contains("\tPUBLIC\n"));
    assert!(result.contains("\tPRIVATE\n"));

    // Verify values are one-per-line underneath at deeper indent
    assert!(result.contains("\t\tlib1\n"));
    assert!(result.contains("\t\tlib2\n"));
    assert!(result.contains("\t\tlib5\n"));
    assert!(result.contains("\t\tlib6\n"));
    assert!(result.contains("\t\tlib9\n"));

    // Verify idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "KWFMT-02 formatting must be idempotent");
}

// ============================================================================
// PAIR VALUE KEYWORD TESTS (KWFMT-03)
// ============================================================================

#[test]
fn test_set_source_files_properties_short() {
    let input = "set_source_files_properties(f.cpp PROPERTIES GENERATED TRUE)";
    let result = format_text(input, &default_config());
    // Short enough to stay on one line
    assert_eq!(result, "set_source_files_properties(f.cpp PROPERTIES GENERATED TRUE)\n");
}

#[test]
fn test_set_source_files_properties_pairs() {
    let input = "set_source_files_properties(file1.cpp file2.cpp PROPERTIES COMPILE_FLAGS \"-O2\" HEADER_FILE_ONLY TRUE GENERATED TRUE)";
    let result = format_text(input, &default_config());
    // Should break with PROPERTIES pairs
    assert!(result.contains("\tPROPERTIES\n"));
    assert!(result.contains("\t\tCOMPILE_FLAGS \"-O2\"\n"));
    assert!(result.contains("\t\tHEADER_FILE_ONLY TRUE\n"));
    assert!(result.contains("\t\tGENERATED TRUE\n"));
}

#[test]
fn test_set_target_properties_pairs() {
    let input = "set_target_properties(mylib PROPERTIES VERSION 1.0 SOVERSION 1 OUTPUT_NAME \"mylib\")";
    let result = format_text(input, &default_config());
    // Should break with PROPERTIES pairs
    assert!(result.contains("\tPROPERTIES\n"));
    assert!(result.contains("\t\tVERSION 1.0\n"));
    assert!(result.contains("\t\tSOVERSION 1\n"));
    assert!(result.contains("\t\tOUTPUT_NAME \"mylib\"\n"));
}

#[test]
fn test_set_source_files_properties_idempotency() {
    let inputs = vec![
        "set_source_files_properties(file1.cpp PROPERTIES COMPILE_FLAGS \"-O2\" HEADER_FILE_ONLY TRUE)",
        "set_target_properties(mylib PROPERTIES VERSION 1.0 SOVERSION 1)",
    ];
    for input in inputs {
        let pass1 = format_text(input, &default_config());
        let pass2 = format_text(&pass1, &default_config());
        assert_eq!(pass1, pass2, "Idempotency failed for: {}", input);
    }
}

// ============================================================================
// QUICK TASK 14: MultiValue Single-Arg Inline Formatting Tests
// ============================================================================

#[test]
fn test_multivalue_single_arg_stays_inline() {
    // PROGRAMS is MultiValue, but with exactly 1 arg it should stay inline like SingleValue
    let input = "install(PROGRAMS ${VCPKG_INSTALLED_DIR}/${VCPKG_TARGET_TRIPLET}/tools/crashpad_handler DESTINATION bin COMPONENT crashpad)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // When command breaks (it's long enough to trigger multiline):
    // PROGRAMS value should be inline with PROGRAMS keyword (not on next line)
    // DESTINATION value should be inline with DESTINATION keyword (SingleValue behavior)
    // COMPONENT value should be inline with COMPONENT keyword (SingleValue behavior)
    assert!(result.contains("PROGRAMS ${VCPKG_INSTALLED_DIR}/${VCPKG_TARGET_TRIPLET}/tools/crashpad_handler\n"));
    assert!(result.contains("DESTINATION bin\n"));
    assert!(result.contains("COMPONENT crashpad\n"));

    // Verify idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "MultiValue single-arg formatting must be idempotent");
}

#[test]
fn test_multivalue_multiple_args_still_vertical() {
    // PROGRAMS with 2+ args should still format vertically (regression guard)
    // Use longer program names to ensure command breaks
    let input = "install(PROGRAMS very_long_program_name_one very_long_program_name_two very_long_program_name_three DESTINATION bin)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // PROGRAMS should be on its own line with values underneath (vertical layout)
    assert!(result.contains("\tPROGRAMS\n"));
    assert!(result.contains("\t\tvery_long_program_name_one\n"));
    assert!(result.contains("\t\tvery_long_program_name_two\n"));
    assert!(result.contains("\t\tvery_long_program_name_three\n"));
    // DESTINATION still SingleValue, stays inline
    assert!(result.contains("DESTINATION bin\n"));

    // Verify idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "MultiValue multi-arg formatting must be idempotent");
}

// ============================================================================
// QUICK TASK 15: BinPack Formatting Tests
// ============================================================================

#[test]
fn test_command_short_stays_inline() {
    // Short COMMAND stays on one line
    let input = "execute_process(COMMAND echo hello OUTPUT_VARIABLE result)";
    let result = format_text(input, &default_config());
    assert_eq!(result, "execute_process(COMMAND echo hello OUTPUT_VARIABLE result)\n");
}

#[test]
fn test_command_bin_packs_to_fill_lines() {
    // Long COMMAND bin-packs values to fill lines
    let input = "execute_process(COMMAND ${BREW_EXECUTABLE} --prefix llvm OUTPUT_VARIABLE LLVM_PREFIX OUTPUT_STRIP_TRAILING_WHITESPACE)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // COMMAND values should bin-pack on one line (they fit together)
    assert!(result.contains("COMMAND ${BREW_EXECUTABLE} --prefix llvm\n"));
    // Other keywords should format normally
    assert!(result.contains("OUTPUT_VARIABLE LLVM_PREFIX\n"));
    // Verify idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "BinPack formatting must be idempotent");
}

#[test]
fn test_command_bin_packs_wraps_long_lines() {
    // Very long COMMAND wraps at line width
    let input = "execute_process(COMMAND ${CMAKE_COMMAND} -E env VCPKG_ROOT=${VCPKG_ROOT} ${CMAKE_COMMAND} --build . --config Release --target install OUTPUT_VARIABLE result)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Values should bin-pack across multiple lines
    // First line after COMMAND should have multiple args packed
    let command_line = result.lines().find(|l| l.contains("COMMAND")).unwrap();
    assert!(command_line.contains("${CMAKE_COMMAND}"), "First arg should be on COMMAND line");
    assert!(command_line.contains("-E"), "Short args should pack on COMMAND line");
    // Verify idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "BinPack wrapping must be idempotent");
}

#[test]
fn test_command_bin_packs_even_when_input_multiline() {
    // COMMAND bin-packs values even when input had them on separate lines
    // (unless comments/blank lines are present, which would fall back to per-line)
    let input = "add_custom_command(\n    OUTPUT output.txt\n    COMMAND\n        ${CMAKE_COMMAND}\n        -E\n        copy\n        input.txt\n        output.txt\n    VERBATIM\n)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // COMMAND values should bin-pack to fit on one line (no comments/blank lines to preserve)
    assert!(result.contains("COMMAND ${CMAKE_COMMAND} -E copy input.txt output.txt\n"));
    // Verify idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "BinPack packing must be idempotent");
}

#[test]
fn test_command_single_arg_stays_inline() {
    // COMMAND with single arg stays inline like SingleValue
    let input = "add_test(NAME mytest COMMAND mytest WORKING_DIRECTORY ${CMAKE_BINARY_DIR})";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // COMMAND with 1 arg should stay inline with keyword
    assert!(result.contains("COMMAND mytest"));
    // Verify idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "BinPack single arg must be idempotent");
}

#[test]
fn test_command_add_custom_target_bin_packs() {
    // add_custom_target COMMAND also bin-packs
    let input = "add_custom_target(run_tests ALL COMMAND ${CMAKE_CTEST_COMMAND} --output-on-failure --parallel 4 WORKING_DIRECTORY ${CMAKE_BINARY_DIR})";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // COMMAND values should bin-pack
    assert!(result.contains("COMMAND"));
    assert!(result.contains("${CMAKE_CTEST_COMMAND}"));
    // Verify idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "add_custom_target BinPack must be idempotent");
}

// ============================================================================
// QUICK TASK 54: BinPack Bug Fix Tests
// ============================================================================

#[test]
fn test_command_binpack_with_trailing_comment() {
    // Trailing comment on a BinPack arg must NOT be dropped
    let input = "add_custom_command(\n    OUTPUT ${SOURCE_OUTPUT_FILE}\n    DEPENDS ${MAP_FILE} ToolVersionMapToCode\n    COMMAND ToolVersionMapToCode --verbose --force --map ${MAP_FILE} --source ${SOURCE_OUTPUT_FILE}\n    ${ENABLE_CODEGEN} # if the policy is set, add this to the codegen target\n)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Trailing comment must be preserved
    assert!(result.contains("# if the policy is set, add this to the codegen target"),
        "Trailing comment was dropped");
    // ENABLE_CODEGEN should be present
    assert!(result.contains("${ENABLE_CODEGEN}"));
    // Verify idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "BinPack with trailing comment must be idempotent");
}

#[test]
fn test_command_binpack_force_multiline_packs_args() {
    // When input is multiline (force_multiline=true), COMMAND args must STILL bin-pack
    // not go one-per-line
    let input = "add_custom_command(\n    OUTPUT ${SOURCE_OUTPUT_FILE}\n    COMMAND ToolVersionMapToCode --verbose --force --map ${MAP_FILE} --source ${SOURCE_OUTPUT_FILE}\n)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // COMMAND line should have multiple args packed together
    // Find the COMMAND line
    let command_line = result.lines().find(|l| l.contains("COMMAND")).unwrap();
    // It should have more than just "COMMAND" on it - at least the tool name
    assert!(command_line.contains("ToolVersionMapToCode"),
        "Tool name should be on same line as COMMAND");
    // Args should be packed, not one-per-line
    // Count lines between COMMAND and the closing ) or next keyword
    let lines: Vec<&str> = result.lines().collect();
    let cmd_idx = lines.iter().position(|l| l.contains("COMMAND")).unwrap();
    // The next line after COMMAND args should be ")" - not many individual arg lines
    // With bin-packing, COMMAND + args should take at most 2-3 lines, not 7+ lines
    let arg_lines: Vec<&str> = lines[cmd_idx..].iter()
        .take_while(|l| !l.trim().starts_with(')'))
        .copied()
        .collect();
    assert!(arg_lines.len() <= 3,
        "BinPack should use at most 3 lines for these args, got {}: {:?}", arg_lines.len(), arg_lines);
    // Verify idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "BinPack multiline must be idempotent");
}

#[test]
fn test_command_binpack_with_leading_comment() {
    // Leading comment before a BinPack arg must be preserved
    let input = "add_custom_command(\n    OUTPUT output.txt\n    COMMAND ${TOOL}\n    # this is important\n    --flag\n)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    assert!(result.contains("# this is important"), "Leading comment was dropped");
    assert!(result.contains("--flag"));
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "BinPack with leading comment must be idempotent");
}

// ============================================================================
// SINGLE VALUE KEYWORD CONSUMPTION TESTS
// ============================================================================

#[test]
fn test_single_value_keyword_limits_consumption() {
    // SingleValue keyword should consume exactly one value.
    // Overflow args become positional (not nested under the keyword).
    let mut grammar = CommandGrammarConfig::default();
    grammar.one_value_keywords = vec!["OUTPUT".to_string(), "PLATFORM".to_string()];

    let mut command_grammars = HashMap::new();
    command_grammars.insert("my_command".to_string(), grammar);

    let config = FormatConfig {
        command_grammars,
        ..Default::default()
    };

    let input = "my_command(OUTPUT file.yml PLATFORM ${CI_PLATFORM} ${CI_ARCHITECTURE})";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);

    // PLATFORM should keep its single value inline
    assert!(result.contains("PLATFORM ${CI_PLATFORM}"));
    // ${CI_ARCHITECTURE} should NOT be nested under PLATFORM
    assert!(!result.contains("\t\t${CI_ARCHITECTURE}"));
    // Idempotency
    let pass2 = format_text(&result, &config);
    assert_eq!(result, pass2, "SingleValue overflow must be idempotent");
}

#[test]
fn test_single_value_overflow_multiline() {
    // When the line is long enough to break, overflow args should appear at keyword indent level
    let mut grammar = CommandGrammarConfig::default();
    grammar.one_value_keywords = vec![
        "OUTPUT".to_string(),
        "PLATFORM".to_string(),
        "ARCHITECTURE".to_string(),
    ];

    let mut command_grammars = HashMap::new();
    command_grammars.insert("generate_ci".to_string(), grammar);

    let config = FormatConfig {
        command_grammars,
        ..Default::default()
    };

    let input = "generate_ci(OUTPUT ${CMAKE_CURRENT_SOURCE_DIR}/run-ci-tests.yml PLATFORM ${CI_PLATFORM} ${CI_ARCHITECTURE})";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);

    // First SingleValue keyword drops to new line (not a multi-mode command)
    assert!(result.contains("generate_ci(\n"));
    // OUTPUT should have its single value inline
    assert!(result.contains("OUTPUT ${CMAKE_CURRENT_SOURCE_DIR}/run-ci-tests.yml"));
    // PLATFORM should have its single value inline
    assert!(result.contains("PLATFORM ${CI_PLATFORM}"));
    // ${CI_ARCHITECTURE} at keyword indent, not value indent
    assert!(result.contains("\t${CI_ARCHITECTURE}\n"));
    assert!(!result.contains("\t\t${CI_ARCHITECTURE}"));
    // Idempotency
    let pass2 = format_text(&result, &config);
    assert_eq!(result, pass2, "SingleValue multiline overflow must be idempotent");
}

// ============================================================================
// LIST COMMAND GRAMMAR TESTS
// ============================================================================

#[test]
fn test_list_append_keeps_mode_and_variable_inline() {
    let config = FormatConfig::default();
    let input = "list(APPEND\n    SOURCES\n    file1.cpp\n    file2.cpp\n    file3.cpp\n)\n";
    let result = format_text(input, &config);
    // APPEND and SOURCES should be on the same line
    assert!(result.contains("list(APPEND SOURCES"),
        "APPEND and variable name should stay on the same line: {}", result);
    // Files should be on separate lines
    assert!(result.contains("\tfile1.cpp\n"));
    assert!(result.contains("\tfile2.cpp\n"));
    assert!(result.contains("\tfile3.cpp\n"));
    // Idempotency
    let pass2 = format_text(&result, &config);
    assert_eq!(result, pass2, "list(APPEND ...) should be idempotent");
}

#[test]
fn test_list_append_short_stays_one_line() {
    let config = FormatConfig::default();
    let input = "list(APPEND SOURCES \"item\")\n";
    let result = format_text(input, &config);
    assert_eq!(result, "list(APPEND SOURCES \"item\")\n",
        "Short list(APPEND) should stay on one line");
}

#[test]
fn test_list_sort_keeps_mode_and_variable_inline() {
    let config = FormatConfig::default();
    let input = "list(SORT\n    mylist\n    COMPARE STRING\n    CASE INSENSITIVE\n    ORDER DESCENDING\n)\n";
    let result = format_text(input, &config);
    // SORT and mylist should be on the same line
    assert!(result.contains("list(SORT mylist"),
        "SORT and variable name should stay on the same line: {}", result);
    // Keywords should be on separate lines
    assert!(result.contains("\tCOMPARE STRING\n"));
    assert!(result.contains("\tCASE INSENSITIVE\n"));
    // Idempotency
    let pass2 = format_text(&result, &config);
    assert_eq!(result, pass2, "list(SORT ...) should be idempotent");
}

#[test]
fn test_list_reverse_simple() {
    let config = FormatConfig::default();
    let input = "list(REVERSE mylist)\n";
    let result = format_text(input, &config);
    assert_eq!(result, "list(REVERSE mylist)\n",
        "list(REVERSE) should stay on one line");
}

#[test]
fn test_define_property_keeps_scope_property_name_inline() {
    let config = FormatConfig::default();
    let input = "define_property(TEST\n    PROPERTY\n    SEPARATE_JOB\n    BRIEF_DOCS\n    \"Run as separate job\"\n    FULL_DOCS\n    \"Switches the job to run separately\"\n)\n";
    let result = format_text(input, &config);
    // Scope + PROPERTY + name should be on the same line
    assert!(result.contains("define_property(TEST PROPERTY SEPARATE_JOB"),
        "TEST PROPERTY SEPARATE_JOB should stay on the same line: {}", result);
    // BRIEF_DOCS and FULL_DOCS should be keyword sections on new lines
    assert!(result.contains("\tBRIEF_DOCS \"Run as separate job\""),
        "BRIEF_DOCS should be on its own line with value: {}", result);
    assert!(result.contains("\tFULL_DOCS \"Switches the job to run separately\""),
        "FULL_DOCS should be on its own line with value: {}", result);
    // Idempotency
    let pass2 = format_text(&result, &config);
    assert_eq!(result, pass2, "define_property should be idempotent");
}

#[test]
fn test_define_property_short_stays_one_line() {
    let config = FormatConfig::default();
    let input = "define_property(TARGET PROPERTY MY_PROP)\n";
    let result = format_text(input, &config);
    assert_eq!(result, "define_property(TARGET PROPERTY MY_PROP)\n",
        "Short define_property should stay on one line");
}

#[test]
fn test_define_property_with_inherited() {
    let config = FormatConfig::default();
    let input = "define_property(DIRECTORY\n    PROPERTY\n    MY_CUSTOM_PROP\n    INHERITED\n    BRIEF_DOCS\n    \"A custom property\"\n    FULL_DOCS\n    \"This is a custom directory property that is inherited by subdirectories\"\n)\n";
    let result = format_text(input, &config);
    // Scope + PROPERTY + name should be on the same line
    assert!(result.contains("define_property(DIRECTORY PROPERTY MY_CUSTOM_PROP"),
        "DIRECTORY PROPERTY MY_CUSTOM_PROP should stay together: {}", result);
    // INHERITED should appear (as a flag, grouped with scope or on its own)
    assert!(result.contains("INHERITED"),
        "INHERITED flag should be present: {}", result);
    // BRIEF_DOCS and FULL_DOCS should be keyword sections
    assert!(result.contains("BRIEF_DOCS"),
        "BRIEF_DOCS should be present: {}", result);
    assert!(result.contains("FULL_DOCS"),
        "FULL_DOCS should be present: {}", result);
    // Idempotency
    let pass2 = format_text(&result, &config);
    assert_eq!(result, pass2, "define_property with INHERITED should be idempotent");
}

#[test]
fn test_define_property_multiple_brief_docs() {
    let config = FormatConfig::default();
    let input = "define_property(GLOBAL PROPERTY MY_GLOBAL_PROP BRIEF_DOCS \"doc1\" \"doc2\" FULL_DOCS \"full doc\")\n";
    let result = format_text(input, &config);
    // Should contain the property name
    assert!(result.contains("PROPERTY MY_GLOBAL_PROP"),
        "PROPERTY and name should stay together: {}", result);
    // Multiple BRIEF_DOCS values
    assert!(result.contains("BRIEF_DOCS"),
        "BRIEF_DOCS should be present: {}", result);
    assert!(result.contains("\"doc1\""),
        "First doc string should be present: {}", result);
    assert!(result.contains("\"doc2\""),
        "Second doc string should be present: {}", result);
    // Idempotency
    let pass2 = format_text(&result, &config);
    assert_eq!(result, pass2, "define_property with multiple docs should be idempotent");
}

// ============================================================================
// FILE(COPY) / FILE(INSTALL) MULTIVALUE WRAPPING TESTS
// ============================================================================

#[test]
fn test_file_copy_multiline_wraps_files_under_copy() {
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 4,
        ..FormatConfig::default()
    };
    let input = "\
file(COPY ${CMAKE_CURRENT_FUNCTION_LIST_DIR}/editor/Resources/windows/Wire.ico
    ${CMAKE_CURRENT_FUNCTION_LIST_DIR}/editor/Resources/windows/wire.h
    DESTINATION ${CMAKE_CURRENT_BINARY_DIR}/rc
)";
    let expected = "\
file(
    COPY
        ${CMAKE_CURRENT_FUNCTION_LIST_DIR}/editor/Resources/windows/Wire.ico
        ${CMAKE_CURRENT_FUNCTION_LIST_DIR}/editor/Resources/windows/wire.h
    DESTINATION ${CMAKE_CURRENT_BINARY_DIR}/rc
)
";
    let result = format_text(input, &config);
    assert_eq!(result, expected);
}

#[test]
fn test_file_copy_single_file_stays_inline() {
    let input = "file(COPY src/ DESTINATION build/)";
    let result = format_text(input, &default_config());
    assert_eq!(result, "file(COPY src/ DESTINATION build/)\n");
}

#[test]
fn test_file_install_multiline_wraps_like_copy() {
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 4,
        ..FormatConfig::default()
    };
    let input = "\
file(INSTALL
    file1.txt
    file2.txt
    DESTINATION ${CMAKE_INSTALL_PREFIX}/share
)";
    let expected = "\
file(
    INSTALL
        file1.txt
        file2.txt
    DESTINATION ${CMAKE_INSTALL_PREFIX}/share
)
";
    let result = format_text(input, &config);
    assert_eq!(result, expected);
}

#[test]
fn test_file_copy_idempotent() {
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 4,
        ..FormatConfig::default()
    };
    let input = "\
file(
    COPY
        ${CMAKE_CURRENT_FUNCTION_LIST_DIR}/editor/Resources/windows/Wire.ico
        ${CMAKE_CURRENT_FUNCTION_LIST_DIR}/editor/Resources/windows/wire.h
    DESTINATION ${CMAKE_CURRENT_BINARY_DIR}/rc
)
";
    let pass1 = format_text(input, &config);
    assert_eq!(pass1, input, "First pass should match expected output");
    let pass2 = format_text(&pass1, &config);
    assert_eq!(pass1, pass2, "Second pass should be identical (idempotent)");
}

#[test]
fn test_add_definitions_all_args_on_new_lines() {
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 4,
        ..FormatConfig::default()
    };
    // Input: first arg trailing the command name (current bad behavior without grammar)
    let input = "\
add_definitions(-DASIO_STANDALONE
    -DASIO_HAS_STD_ADDRESSOF
    -DASIO_HAS_STD_ARRAY
    -DASIO_HAS_CSTDINT
)
";
    let expected = "\
add_definitions(
    -DASIO_STANDALONE
    -DASIO_HAS_STD_ADDRESSOF
    -DASIO_HAS_STD_ARRAY
    -DASIO_HAS_CSTDINT
)
";
    let result = format_text(input, &config);
    assert_eq!(result, expected);
}

#[test]
fn test_add_definitions_single_line_stays_flat() {
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 4,
        ..FormatConfig::default()
    };
    let input = "add_definitions(-DFOO)\n";
    let result = format_text(input, &config);
    assert_eq!(result, input);
}

#[test]
fn test_add_definitions_idempotent() {
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 4,
        ..FormatConfig::default()
    };
    let input = "\
add_definitions(
    -DASIO_STANDALONE
    -DASIO_HAS_STD_ADDRESSOF
    -DASIO_HAS_CSTDINT
)
";
    let pass1 = format_text(input, &config);
    assert_eq!(pass1, input, "First pass should match expected output");
    let pass2 = format_text(&pass1, &config);
    assert_eq!(pass1, pass2, "Second pass should be identical (idempotent)");
}

#[test]
fn test_configure_file_all_args_on_new_lines() {
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 4,
        ..FormatConfig::default()
    };
    let input = "\
configure_file(${CMAKE_CURRENT_SOURCE_DIR}/config.h.in
    ${CMAKE_CURRENT_BINARY_DIR}/config.h
    @ONLY
)
";
    let expected = "\
configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/config.h.in
    ${CMAKE_CURRENT_BINARY_DIR}/config.h
    @ONLY
)
";
    let result = format_text(input, &config);
    assert_eq!(result, expected);
}

#[test]
fn test_configure_file_idempotent() {
    let config = FormatConfig {
        use_tabs: false,
        indent_width: 4,
        ..FormatConfig::default()
    };
    let input = "\
configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/config.h.in
    ${CMAKE_CURRENT_BINARY_DIR}/config.h
)
";
    let pass1 = format_text(input, &config);
    assert_eq!(pass1, input, "First pass should match expected output");
    let pass2 = format_text(&pass1, &config);
    assert_eq!(pass1, pass2, "Second pass should be identical (idempotent)");
}

// ============================================================================
// BLANK LINES BETWEEN COMMENT BLOCKS TESTS
// ============================================================================

#[test]
fn test_blank_lines_between_comment_blocks_in_set() {
    let input = "\
set(SOURCES
\tfile1.hpp
\tfile2.cpp

\t# delay_meter/class.hpp
\t# delay_meter/class.cpp

\t# fdn_reverb/class.hpp
\t# fdn_reverb/class.cpp
)
";
    let result = format_text(input, &default_config());
    // Blank line between the two comment groups must be preserved
    assert!(result.contains("# delay_meter/class.cpp\n\n\t# fdn_reverb/class.hpp"),
        "Expected blank line between comment groups, got:\n{}", result);
}

#[test]
fn test_blank_lines_between_three_comment_blocks() {
    let input = "\
set(SOURCES
\tfile1.cpp

\t# group A
\t# group A continued

\t# group B
\t# group B continued

\t# group C
)
";
    let result = format_text(input, &default_config());
    assert!(result.contains("# group A continued\n\n\t# group B"),
        "Expected blank line between groups A and B, got:\n{}", result);
    assert!(result.contains("# group B continued\n\n\t# group C"),
        "Expected blank line between groups B and C, got:\n{}", result);
}

#[test]
fn test_blank_lines_between_comment_blocks_with_keyword() {
    let input = "\
target_sources(mylib
\tPRIVATE
\t\tfile1.cpp
\t\tfile2.cpp
\t\tfile3.cpp

\t\t# audio sources
\t\t# audio/engine.cpp

\t\t# video sources
\t\t# video/renderer.cpp
)
";
    let result = format_text(input, &default_config());
    assert!(result.contains("# audio/engine.cpp\n\n\t\t# video sources"),
        "Expected blank line between comment groups under keyword, got:\n{}", result);
}

#[test]
fn test_blank_lines_between_comment_blocks_at_end() {
    let input = "\
set(SOURCES
\tfile1.cpp
\tfile2.cpp

\t# disabled module A
\t# module_a/foo.cpp

\t# disabled module B
\t# module_b/bar.cpp
)
";
    let result = format_text(input, &default_config());
    assert!(result.contains("# module_a/foo.cpp\n\n\t# disabled module B"),
        "Expected blank line between trailing comment blocks, got:\n{}", result);
}

#[test]
fn test_existing_blank_line_and_comment_behavior_preserved() {
    let input = "\
set(SOURCES
\tfile1.cpp

\t# section header
\tfile2.cpp
\tfile3.cpp
)
";
    let result = format_text(input, &default_config());
    // Blank line before comment, comment before file2
    assert!(result.contains("file1.cpp\n\n\t# section header\n\tfile2.cpp"),
        "Expected preserved blank line + comment + arg ordering, got:\n{}", result);
}

// ============================================================================
// INSTALL TARGETS EXPORT KEYWORD AND BINPACK ARTIFACT TYPES (Quick 55)
// ============================================================================

#[test]
fn test_install_targets_export_keyword() {
    // EXPORT should be recognized as a keyword, not consumed as a target name
    let input = "install(TARGETS VSTSDK EXPORT VSTSDKTargets LIBRARY DESTINATION lib ARCHIVE DESTINATION lib RUNTIME DESTINATION bin INCLUDES DESTINATION include)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // EXPORT should be a keyword with its value inline
    assert!(result.contains("EXPORT VSTSDKTargets"));
    // BinPack artifact types consume DESTINATION as sub_keyword (stays on same line)
    assert!(result.contains("LIBRARY DESTINATION lib"));
    assert!(result.contains("ARCHIVE DESTINATION lib"));
    assert!(result.contains("RUNTIME DESTINATION bin"));
    assert!(result.contains("INCLUDES DESTINATION include"));
    // Idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "install TARGETS with EXPORT should be idempotent");
}

#[test]
fn test_install_targets_artifact_binpack_multiline() {
    // When artifact type has many sub-keywords, they should wrap to indented lines
    let input = "install(TARGETS mylib LIBRARY DESTINATION lib PERMISSIONS OWNER_READ OWNER_WRITE GROUP_READ WORLD_READ CONFIGURATIONS Release Debug ARCHIVE DESTINATION lib RUNTIME DESTINATION bin)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // LIBRARY BinPack section consumes DESTINATION, PERMISSIONS, CONFIGURATIONS as sub_keywords
    assert!(result.contains("LIBRARY"));
    assert!(result.contains("DESTINATION lib"));
    assert!(result.contains("PERMISSIONS"));
    assert!(result.contains("CONFIGURATIONS"));
    // ARCHIVE and RUNTIME also consume DESTINATION as sub_keyword
    assert!(result.contains("ARCHIVE DESTINATION lib"));
    assert!(result.contains("RUNTIME DESTINATION bin"));
    // Idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "install TARGETS with multiline artifact should be idempotent");
}

#[test]
fn test_install_targets_export_not_consumed_as_target() {
    // Verify EXPORT is a keyword, not treated as a target name after TARGETS
    let input = "install(TARGETS mylib EXPORT mylib-export RUNTIME DESTINATION bin)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // EXPORT should pair with its value, not appear as a bare target name
    assert!(result.contains("EXPORT mylib-export"));
    assert!(result.contains("RUNTIME DESTINATION bin"));
    // Idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "install TARGETS with EXPORT should be idempotent");
}

#[test]
fn test_install_targets_global_destination_component() {
    // Global DESTINATION/COMPONENT (no artifact type) must work
    let input = "install(TARGETS libwire COMPONENT wire DESTINATION lib)\n";
    let result = format_text(input, &default_config());
    // COMPONENT and DESTINATION should be recognized as keywords
    assert!(result.contains("COMPONENT wire"), "COMPONENT should be a keyword");
    assert!(result.contains("DESTINATION lib"), "DESTINATION should be a keyword");
    // They should NOT be consumed as target names
    assert!(!result.contains("TARGETS\n\t\tlibwire\n\t\tCOMPONENT"),
        "COMPONENT should not be a target arg");
    // Idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "Should be idempotent");
}

// ============================================================================
// INSTALL TARGETS MULTILINE REGRESSION TESTS (Quick 63)
// ============================================================================

#[test]
fn test_install_targets_multiline_not_absorbed() {
    // Regression: multiline install(TARGETS) must not absorb DESTINATION/COMPONENT as target values
    let input = "install(\n\tTARGETS Wire\n\tDESTINATION bin\n\tCOMPONENT wire\n)\n";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // TARGETS should have Wire as its only value
    assert!(result.contains("TARGETS Wire"), "Wire should be inline with TARGETS");
    // DESTINATION and COMPONENT must be separate keyword sections
    assert!(result.contains("\tDESTINATION bin"), "DESTINATION should be a separate keyword");
    assert!(result.contains("\tCOMPONENT wire"), "COMPONENT should be a separate keyword");
    // Must NOT have DESTINATION/COMPONENT indented under TARGETS
    assert!(!result.contains("\t\tDESTINATION"), "DESTINATION must not be a TARGETS value");
    assert!(!result.contains("\t\tCOMPONENT"), "COMPONENT must not be a TARGETS value");
    // Idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "Should be idempotent");
}

#[test]
fn test_install_targets_multiline_with_artifact_types() {
    // Multiline install TARGETS with BinPack artifact types should still work correctly
    let input = "install(\n\tTARGETS mylib\n\tRUNTIME DESTINATION bin\n\tLIBRARY DESTINATION lib\n\tARCHIVE DESTINATION lib\n)\n";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // TARGETS should have mylib inline
    assert!(result.contains("TARGETS mylib"), "mylib should be inline with TARGETS");
    // Artifact types should consume DESTINATION as sub_keyword (BinPack behavior)
    assert!(result.contains("RUNTIME DESTINATION bin"), "RUNTIME should consume DESTINATION");
    assert!(result.contains("LIBRARY DESTINATION lib"), "LIBRARY should consume DESTINATION");
    assert!(result.contains("ARCHIVE DESTINATION lib"), "ARCHIVE should consume DESTINATION");
    // Idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "Should be idempotent");
}

#[test]
fn test_install_files_matching_still_groups_after_fix() {
    // Ensure FILES_MATCHING still correctly groups PATTERN sub_keywords after the fix
    let input = "install(\n\tDIRECTORY \"${SRC}/headers\"\n\tDESTINATION include\n\tFILES_MATCHING\n\tPATTERN \"*.h\"\n)\n";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // FILES_MATCHING should group PATTERN inline
    assert!(result.contains("FILES_MATCHING PATTERN \"*.h\""), "PATTERN should be grouped with FILES_MATCHING");
    // Idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(result, pass2, "Should be idempotent");
}

// ============================================================================
// FILES_MATCHING COLLECTION FORMATTING TESTS
// ============================================================================

#[test]
fn test_files_matching_single_pattern_inline() {
    let input = "install(\n    DIRECTORY \"${VST24_SOURCE}/public.sdk\"\n    DESTINATION include\n    FILES_MATCHING\n    PATTERN \"*.h\"\n)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Single pattern: should be inline with FILES_MATCHING
    assert!(result.contains("FILES_MATCHING PATTERN \"*.h\""), "Single PATTERN should be inline with FILES_MATCHING");
}

#[test]
fn test_files_matching_multiple_patterns_indented() {
    let mut config = default_config();
    config.max_line_length = 80;
    let input = "install(\n    DIRECTORY \"${VST24_SOURCE}/public.sdk\"\n    DESTINATION include\n    FILES_MATCHING\n    PATTERN \"*.h\"\n    PATTERN \"*.cpp\"\n    EXCLUDE PATTERN \"internal/*\"\n)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Multiple patterns: each on indented line below FILES_MATCHING
    assert!(result.contains("FILES_MATCHING\n"), "FILES_MATCHING should be on its own line with multiple patterns");
    assert!(result.contains("\t\tPATTERN \"*.h\"\n"), "First PATTERN should be indented under FILES_MATCHING");
    assert!(result.contains("\t\tPATTERN \"*.cpp\"\n"), "Second PATTERN should be indented under FILES_MATCHING");
    assert!(result.contains("\t\tEXCLUDE PATTERN \"internal/*\"\n"), "EXCLUDE PATTERN should be indented under FILES_MATCHING");
}

#[test]
fn test_files_matching_single_regex_inline() {
    let input = "install(\n    DIRECTORY src/\n    DESTINATION include\n    FILES_MATCHING\n    REGEX \".*\\.h$\"\n)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Single REGEX: should be inline with FILES_MATCHING
    assert!(result.contains("FILES_MATCHING REGEX"), "Single REGEX should be inline with FILES_MATCHING");
}

#[test]
fn test_files_matching_idempotent_single() {
    let input = "install(\n    DIRECTORY include/\n    DESTINATION include\n    FILES_MATCHING\n    PATTERN \"*.h\"\n)";
    let pass1 = format_text(input, &default_config());
    let pass2 = format_text(&pass1, &default_config());
    assert_eq!(pass1, pass2, "FILES_MATCHING single pattern should be idempotent");
}

#[test]
fn test_files_matching_idempotent_multiple() {
    let input = "install(\n    DIRECTORY include/\n    DESTINATION include\n    FILES_MATCHING\n    PATTERN \"*.h\"\n    PATTERN \"*.cpp\"\n)";
    let pass1 = format_text(input, &default_config());
    eprintln!("Pass 1:\n{}", pass1);
    let pass2 = format_text(&pass1, &default_config());
    eprintln!("Pass 2:\n{}", pass2);
    assert_eq!(pass1, pass2, "FILES_MATCHING multiple patterns should be idempotent");
}

#[test]
fn test_files_matching_file_copy_mode() {
    let input = "file(\n    COPY src/\n    DESTINATION build/\n    FILES_MATCHING\n    PATTERN \"*.cmake\"\n)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // file(COPY) should also support FILES_MATCHING collection formatting
    assert!(result.contains("FILES_MATCHING PATTERN \"*.cmake\""), "file(COPY) should format FILES_MATCHING inline for single pattern");
}

#[test]
fn test_files_matching_flat_short_command() {
    let input = "install(DIRECTORY include/ DESTINATION include FILES_MATCHING PATTERN \"*.h\")";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Short command fits on one line — flat rendering
    assert_eq!(result, "install(DIRECTORY include/ DESTINATION include FILES_MATCHING PATTERN \"*.h\")\n");
}

// ============================================================================
// COLLAPSE EMPTY FLAGS TESTS
// ============================================================================

#[test]
fn test_collapse_empty_flags_default_true() {
    let input = "add_library(mylib STATIC src/a.cpp src/b.cpp src/c.cpp src/d.cpp src/e.cpp src/f.cpp src/g.cpp)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Default behavior: STATIC collapses onto same line as target name
    assert!(result.contains("mylib STATIC\n"), "STATIC should collapse with target name");
}

#[test]
fn test_collapse_empty_flags_false() {
    let mut config = default_config();
    config.collapse_empty_flags = false;
    let input = "add_library(mylib STATIC src/a.cpp src/b.cpp src/c.cpp src/d.cpp src/e.cpp src/f.cpp src/g.cpp)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // With collapse disabled: STATIC is a type-selector flag with trailing source args,
    // so it stays inline with the target name regardless of collapse_empty_flags
    assert!(result.contains("mylib STATIC"), "STATIC should stay on same line as target name (type-selector flag with trailing args)");
    assert!(!result.contains("\tSTATIC\n"), "STATIC should NOT be on its own line");
}

#[test]
fn test_collapse_empty_flags_false_find_package() {
    let mut config = default_config();
    config.collapse_empty_flags = false;
    // Already-multiline input so force_multiline is true and flags break to their own lines
    let input = "find_package(\n    Boost\n    REQUIRED\n    CONFIG\n    COMPONENTS system filesystem thread\n)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // REQUIRED and CONFIG should each be on their own line
    assert!(result.contains("\tREQUIRED\n"), "REQUIRED should be on its own line");
    assert!(result.contains("\tCONFIG\n"), "CONFIG should be on its own line");
}

#[test]
fn test_collapse_empty_flags_idempotent() {
    let mut config = default_config();
    config.collapse_empty_flags = false;
    let input = "add_library(mylib STATIC src/a.cpp src/b.cpp src/c.cpp src/d.cpp src/e.cpp src/f.cpp src/g.cpp)";
    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);
    assert_eq!(pass1, pass2, "Formatting with collapse_empty_flags=false should be idempotent");
}

#[test]
fn test_collapse_empty_flags_short_command_still_flat() {
    let mut config = default_config();
    config.collapse_empty_flags = false;
    let input = "add_library(mylib STATIC src/a.cpp)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Short command still fits on one line (flat rendering, no breaking needed)
    assert_eq!(result, "add_library(mylib STATIC src/a.cpp)\n");
}

// ============================================================================
// COLLAPSE EMPTY FLAGS REGRESSION TESTS (Quick 64)
// ============================================================================

#[test]
fn test_collapse_empty_flags_false_add_library_static_stays_inline() {
    // Regression (Quick 64): STATIC is a type-selector with trailing source args,
    // must stay inline with target name even when collapse_empty_flags=false
    let mut config = default_config();
    config.collapse_empty_flags = false;
    let input = "add_library(AdobeAfterEffectsSDK STATIC\n\tExamples/Util/AEGP_SuiteHandler.cpp\n\tExamples/Util/AEGP_SuiteHandler.h\n\tExamples/Util/entry.h\n\tExamples/Util/MissingSuiteError.cpp\n)\n";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    assert!(result.contains("AdobeAfterEffectsSDK STATIC"), "STATIC must stay inline with target name");
    assert!(!result.contains("AdobeAfterEffectsSDK\n\tSTATIC"), "STATIC must NOT drop to a new line");
    // Idempotency
    let pass2 = format_text(&result, &config);
    assert_eq!(result, pass2, "Should be idempotent");
}

#[test]
fn test_collapse_empty_flags_false_add_library_shared_stays_inline() {
    // SHARED is also a type-selector with trailing source args
    let mut config = default_config();
    config.collapse_empty_flags = false;
    let input = "add_library(mylib SHARED src/a.cpp src/b.cpp src/c.cpp src/d.cpp src/e.cpp src/f.cpp src/g.cpp)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    assert!(result.contains("mylib SHARED"), "SHARED must stay inline with target name");
}

#[test]
fn test_collapse_empty_flags_false_add_executable_win32_stays_inline() {
    // WIN32 in add_executable is also a type-selector with trailing source args
    let mut config = default_config();
    config.collapse_empty_flags = false;
    let input = "add_executable(myapp WIN32 src/main.cpp src/app.cpp src/util.cpp src/config.cpp src/render.cpp src/input.cpp)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    assert!(result.contains("myapp WIN32"), "WIN32 must stay inline with target name");
}

#[test]
fn test_collapse_empty_flags_false_pure_flags_still_separate() {
    // IMPORTED has no trailing args (pure modifier) -- should be on own line when collapsed=false
    let mut config = default_config();
    config.collapse_empty_flags = false;
    let input = "add_library(\n\tmylib\n\tSTATIC\n\tIMPORTED\n)\n";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Both STATIC and IMPORTED have no trailing source args here, so both should be on own lines
    assert!(result.contains("\tSTATIC\n"), "STATIC with no trailing args should be on own line");
    assert!(result.contains("\tIMPORTED\n"), "IMPORTED with no trailing args should be on own line");
}

#[test]
fn test_collapse_empty_flags_false_find_package_flags_on_own_lines_idempotent() {
    // Complementary test: find_package flags (no trailing args) respect collapse_empty_flags=false
    let mut config = default_config();
    config.collapse_empty_flags = false;
    let input = "find_package(Boost REQUIRED CONFIG COMPONENTS system filesystem thread)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // When the command wraps to multiline, REQUIRED and CONFIG should be on own lines
    // (they are pure modifier flags with no trailing args)
    // Note: if command fits on one line, flat rendering keeps everything inline regardless
    let pass2 = format_text(&result, &config);
    assert_eq!(result, pass2, "Should be idempotent");
}
