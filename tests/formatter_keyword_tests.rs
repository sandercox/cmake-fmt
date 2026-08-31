use cmake_fmt::formatter::{
    ClosingStyle, CommandCase, CommandGrammarConfig, FormatConfig, format_text,
};
use std::collections::HashMap;

fn default_config() -> FormatConfig {
    FormatConfig::default()
}

// ============================================================================
// KEYWORD-AWARE FORMATTING TESTS
// ============================================================================

#[test]
fn test_target_link_libraries_keywords() {
    let input =
        "target_link_libraries(myapp PUBLIC lib1 lib2 lib3 lib4 lib5 PRIVATE lib6 lib7 lib8)";
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
    assert_eq!(result, "target_link_libraries(myapp PRIVATE lib1)");
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
    assert_eq!(result, "add_library(mylib STATIC src/a.cpp)");
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
    assert_eq!(result, "set(MY_VAR \"some value\")");
}

#[test]
fn test_message_not_keyword_aware() {
    let input = "message(STATUS \"This is a message\")";
    let result = format_text(input, &default_config());
    // STATUS is a keyword, but message() is not keyword-aware
    // Should use simple formatting
    assert_eq!(result, "message(STATUS \"This is a message\")");
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
    let input =
        "target_compile_options(mylib PRIVATE -Wall -Wextra -Wpedantic -Werror PUBLIC -fPIC)";
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
    assert!(
        result.contains("# wui/patch/cord/cord_anchor.cpp"),
        "Comment was dropped from pre-keyword section"
    );
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
    assert!(
        result.contains("\n\n"),
        "Blank line in pre-keyword section was dropped"
    );
    assert!(result.contains("src/a.cpp"));
    assert!(result.contains("src/b.cpp"));
}

#[test]
fn test_trailing_inline_comment_preserved() {
    let input = "target_link_libraries(wui PUBLIC\n    juce::JUCE\n    rj::rj\n    wire_common::wire_common\n    WireDev # For Clock enum...but I'd rather not\n    WireResources\n)\n";
    let result = format_text(input, &default_config());
    // Trailing comment must stay on the same line as WireDev
    assert!(
        result.contains("WireDev # For Clock enum"),
        "Trailing inline comment was moved away from its argument. Got:\n{}",
        result
    );
    // WireResources must be on a separate line, not preceded by the comment
    let lines: Vec<&str> = result.lines().collect();
    let wiredev_line = lines.iter().find(|l| l.contains("WireDev")).unwrap();
    assert!(
        wiredev_line.contains("# For Clock enum"),
        "Comment not on same line as WireDev. WireDev line: {}",
        wiredev_line
    );
}

#[test]
fn test_trailing_inline_comment_simple_args() {
    let input = "set(FLAGS\n    -Wall\n    -Wextra # Extra warnings\n    -Wpedantic\n)\n";
    let result = format_text(input, &default_config());
    // Trailing comment must stay on same line as -Wextra
    assert!(
        result.contains("-Wextra # Extra warnings"),
        "Trailing comment moved away from -Wextra. Got:\n{}",
        result
    );
}

#[test]
fn test_leading_comment_own_line_still_works() {
    let input = "set(SOURCES\n    src/main.cpp\n    # Core library sources\n    src/core.cpp\n)\n";
    let result = format_text(input, &default_config());
    // Leading comment should be on its own line, before src/core.cpp
    let lines: Vec<&str> = result.lines().collect();
    let comment_idx = lines
        .iter()
        .position(|l| l.contains("# Core library sources"))
        .unwrap();
    let core_idx = lines
        .iter()
        .position(|l| l.contains("src/core.cpp"))
        .unwrap();
    assert!(
        comment_idx < core_idx,
        "Leading comment should appear before src/core.cpp"
    );
    // Comment should NOT be on the same line as src/main.cpp
    let main_line = lines.iter().find(|l| l.contains("src/main.cpp")).unwrap();
    assert!(
        !main_line.contains("#"),
        "Leading comment should not be on main.cpp's line"
    );
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
    let input =
        "target_link_libraries(myapp PUBLIC lib1 lib2 PRIVATE lib3 lib4 INTERFACE lib5 lib6)";
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
    assert!(result.ends_with(")"));
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
    assert_eq!(
        count, 1,
        "Comment should appear exactly once, found {} times",
        count
    );
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
        assert_eq!(
            once, twice,
            "Formatting should be idempotent for input:\n{}",
            input
        );
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
    assert_eq!(result, "find_package(Boost REQUIRED QUIET)");
}

#[test]
fn test_single_value_inline_short() {
    // Short command fits on one line — flat rendering
    let input = "install(TARGETS mylib DESTINATION lib)";
    let result = format_text(input, &default_config());
    assert_eq!(result, "install(TARGETS mylib DESTINATION lib)");
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
    let input =
        "my_custom_function(arg1 arg2 arg3 arg4 arg5 arg6 arg7 arg8 arg9 arg10 arg11 arg12 arg13)";
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
    assert!(
        line_count >= 2,
        "Expected multiline output with force_break_keywords, got {} lines",
        line_count
    );
}

#[test]
fn test_force_break_keywords_false_short_stays_inline() {
    let config = default_config(); // force_break_keywords = false by default
    let input = "find_package(Boost REQUIRED)";
    let result = format_text(input, &config);
    // Short command fits on one line — flat rendering
    assert_eq!(result, "find_package(Boost REQUIRED)");
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
        assert_eq!(
            first, second,
            "Idempotency failed with force_break for: {}",
            input
        );
    }
}

#[test]
fn test_backward_compat_existing_commands() {
    // These commands already worked before grammar system -- verify they still produce valid output
    let config = default_config();
    let test_cases = vec![
        ("set(MY_VAR value)", "set(MY_VAR value)"),
        ("message(STATUS \"hello\")", "message(STATUS \"hello\")"),
        ("if(TRUE)\nendif()", "if(TRUE)\nendif()"),
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
    assert_eq!(
        result,
        "install(FILES readme.txt license.txt DESTINATION share/doc)"
    );
}

#[test]
fn test_install_directory_mode_formatting() {
    let input = "install(DIRECTORY include/ DESTINATION include FILES_MATCHING PATTERN \"*.h\")";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);

    // Short command fits on one line — flat rendering
    assert_eq!(
        result,
        "install(DIRECTORY include/ DESTINATION include FILES_MATCHING PATTERN \"*.h\")"
    );
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
    assert_eq!(
        pass1, pass2,
        "DIRECTORY mode formatting should be idempotent"
    );
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
    let input =
        "file(GLOB_RECURSE headers LIST_DIRECTORIES false CONFIGURE_DEPENDS \"include/*.h\")";
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
    assert_eq!(
        result,
        "GenerateTestExecutionGitlabCI(OUTPUT file.yml PLATFORM Linux)"
    );
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
    assert!(result.ends_with(")"));
    // Should NOT have args on same line as command name
    assert!(!result.contains("GenerateTestExecutionGitlabCI(OUTPUT"));
}

#[test]
fn test_custom_function_short_args() {
    let input = "my_custom_function(arg1 arg2 arg3)";
    let result = format_text(input, &default_config());
    // Short custom command stays on one line
    assert_eq!(result, "my_custom_function(arg1 arg2 arg3)");
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
    let input =
        "set(MY_VARIABLE value1 value2 value3 value4 value5 value6 value7 value8 value9 value10)";
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
    let input =
        "target_link_libraries(myapp PUBLIC lib1 lib2 lib3 lib4 lib5 PRIVATE lib6 lib7 lib8 lib9)";
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
    assert_eq!(
        result,
        "set_source_files_properties(f.cpp PROPERTIES GENERATED TRUE)"
    );
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
    let input =
        "set_target_properties(mylib PROPERTIES VERSION 1.0 SOVERSION 1 OUTPUT_NAME \"mylib\")";
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
    assert!(result.contains(
        "PROGRAMS ${VCPKG_INSTALLED_DIR}/${VCPKG_TARGET_TRIPLET}/tools/crashpad_handler\n"
    ));
    assert!(result.contains("DESTINATION bin\n"));
    assert!(result.contains("COMPONENT crashpad\n"));

    // Verify idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(
        result, pass2,
        "MultiValue single-arg formatting must be idempotent"
    );
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
    assert_eq!(
        result, pass2,
        "MultiValue multi-arg formatting must be idempotent"
    );
}

// ============================================================================
// QUICK TASK 15: BinPack Formatting Tests
// ============================================================================

#[test]
fn test_command_short_stays_inline() {
    // Short COMMAND stays on one line
    let input = "execute_process(COMMAND echo hello OUTPUT_VARIABLE result)";
    let result = format_text(input, &default_config());
    assert_eq!(
        result,
        "execute_process(COMMAND echo hello OUTPUT_VARIABLE result)"
    );
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
    assert!(
        command_line.contains("${CMAKE_COMMAND}"),
        "First arg should be on COMMAND line"
    );
    assert!(
        command_line.contains("-E"),
        "Short args should pack on COMMAND line"
    );
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
    assert_eq!(
        result, pass2,
        "add_custom_target BinPack must be idempotent"
    );
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
    assert!(
        result.contains("# if the policy is set, add this to the codegen target"),
        "Trailing comment was dropped"
    );
    // ENABLE_CODEGEN should be present
    assert!(result.contains("${ENABLE_CODEGEN}"));
    // Verify idempotency
    let pass2 = format_text(&result, &default_config());
    assert_eq!(
        result, pass2,
        "BinPack with trailing comment must be idempotent"
    );
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
    assert!(
        command_line.contains("ToolVersionMapToCode"),
        "Tool name should be on same line as COMMAND"
    );
    // Args should be packed, not one-per-line
    // Count lines between COMMAND and the closing ) or next keyword
    let lines: Vec<&str> = result.lines().collect();
    let cmd_idx = lines.iter().position(|l| l.contains("COMMAND")).unwrap();
    // The next line after COMMAND args should be ")" - not many individual arg lines
    // With bin-packing, COMMAND + args should take at most 2-3 lines, not 7+ lines
    let arg_lines: Vec<&str> = lines[cmd_idx..]
        .iter()
        .take_while(|l| !l.trim().starts_with(')'))
        .copied()
        .collect();
    assert!(
        arg_lines.len() <= 3,
        "BinPack should use at most 3 lines for these args, got {}: {:?}",
        arg_lines.len(),
        arg_lines
    );
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
    assert!(
        result.contains("# this is important"),
        "Leading comment was dropped"
    );
    assert!(result.contains("--flag"));
    let pass2 = format_text(&result, &default_config());
    assert_eq!(
        result, pass2,
        "BinPack with leading comment must be idempotent"
    );
}

// ============================================================================
// SINGLE VALUE KEYWORD CONSUMPTION TESTS
// ============================================================================

#[test]
fn test_single_value_keyword_limits_consumption() {
    // SingleValue keyword should consume exactly one value.
    // Overflow args become positional (not nested under the keyword).
    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "my_command".to_string(),
        CommandGrammarConfig {
            one_value_keywords: vec!["OUTPUT".to_string(), "PLATFORM".to_string()],
            ..Default::default()
        },
    );

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
    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "generate_ci".to_string(),
        CommandGrammarConfig {
            one_value_keywords: vec![
                "OUTPUT".to_string(),
                "PLATFORM".to_string(),
                "ARCHITECTURE".to_string(),
            ],
            ..Default::default()
        },
    );

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
    assert_eq!(
        result, pass2,
        "SingleValue multiline overflow must be idempotent"
    );
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
    assert!(
        result.contains("list(APPEND SOURCES"),
        "APPEND and variable name should stay on the same line: {}",
        result
    );
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
    assert_eq!(
        result, "list(APPEND SOURCES \"item\")\n",
        "Short list(APPEND) should stay on one line"
    );
}

#[test]
fn test_list_sort_keeps_mode_and_variable_inline() {
    let config = FormatConfig::default();
    let input = "list(SORT\n    mylist\n    COMPARE STRING\n    CASE INSENSITIVE\n    ORDER DESCENDING\n)\n";
    let result = format_text(input, &config);
    // SORT and mylist should be on the same line
    assert!(
        result.contains("list(SORT mylist"),
        "SORT and variable name should stay on the same line: {}",
        result
    );
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
    assert_eq!(
        result, "list(REVERSE mylist)\n",
        "list(REVERSE) should stay on one line"
    );
}

#[test]
fn test_define_property_keeps_scope_property_name_inline() {
    let config = FormatConfig::default();
    let input = "define_property(TEST\n    PROPERTY\n    SEPARATE_JOB\n    BRIEF_DOCS\n    \"Run as separate job\"\n    FULL_DOCS\n    \"Switches the job to run separately\"\n)\n";
    let result = format_text(input, &config);
    // Scope + PROPERTY + name should be on the same line
    assert!(
        result.contains("define_property(TEST PROPERTY SEPARATE_JOB"),
        "TEST PROPERTY SEPARATE_JOB should stay on the same line: {}",
        result
    );
    // BRIEF_DOCS and FULL_DOCS should be keyword sections on new lines
    assert!(
        result.contains("\tBRIEF_DOCS \"Run as separate job\""),
        "BRIEF_DOCS should be on its own line with value: {}",
        result
    );
    assert!(
        result.contains("\tFULL_DOCS \"Switches the job to run separately\""),
        "FULL_DOCS should be on its own line with value: {}",
        result
    );
    // Idempotency
    let pass2 = format_text(&result, &config);
    assert_eq!(result, pass2, "define_property should be idempotent");
}

#[test]
fn test_define_property_short_stays_one_line() {
    let config = FormatConfig::default();
    let input = "define_property(TARGET PROPERTY MY_PROP)\n";
    let result = format_text(input, &config);
    assert_eq!(
        result, "define_property(TARGET PROPERTY MY_PROP)\n",
        "Short define_property should stay on one line"
    );
}

#[test]
fn test_define_property_with_inherited() {
    let config = FormatConfig::default();
    let input = "define_property(DIRECTORY\n    PROPERTY\n    MY_CUSTOM_PROP\n    INHERITED\n    BRIEF_DOCS\n    \"A custom property\"\n    FULL_DOCS\n    \"This is a custom directory property that is inherited by subdirectories\"\n)\n";
    let result = format_text(input, &config);
    // Scope + PROPERTY + name should be on the same line
    assert!(
        result.contains("define_property(DIRECTORY PROPERTY MY_CUSTOM_PROP"),
        "DIRECTORY PROPERTY MY_CUSTOM_PROP should stay together: {}",
        result
    );
    // INHERITED should appear (as a flag, grouped with scope or on its own)
    assert!(
        result.contains("INHERITED"),
        "INHERITED flag should be present: {}",
        result
    );
    // BRIEF_DOCS and FULL_DOCS should be keyword sections
    assert!(
        result.contains("BRIEF_DOCS"),
        "BRIEF_DOCS should be present: {}",
        result
    );
    assert!(
        result.contains("FULL_DOCS"),
        "FULL_DOCS should be present: {}",
        result
    );
    // Idempotency
    let pass2 = format_text(&result, &config);
    assert_eq!(
        result, pass2,
        "define_property with INHERITED should be idempotent"
    );
}

#[test]
fn test_define_property_multiple_brief_docs() {
    let config = FormatConfig::default();
    let input = "define_property(GLOBAL PROPERTY MY_GLOBAL_PROP BRIEF_DOCS \"doc1\" \"doc2\" FULL_DOCS \"full doc\")\n";
    let result = format_text(input, &config);
    // Should contain the property name
    assert!(
        result.contains("PROPERTY MY_GLOBAL_PROP"),
        "PROPERTY and name should stay together: {}",
        result
    );
    // Multiple BRIEF_DOCS values
    assert!(
        result.contains("BRIEF_DOCS"),
        "BRIEF_DOCS should be present: {}",
        result
    );
    assert!(
        result.contains("\"doc1\""),
        "First doc string should be present: {}",
        result
    );
    assert!(
        result.contains("\"doc2\""),
        "Second doc string should be present: {}",
        result
    );
    // Idempotency
    let pass2 = format_text(&result, &config);
    assert_eq!(
        result, pass2,
        "define_property with multiple docs should be idempotent"
    );
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
)";
    let result = format_text(input, &config);
    assert_eq!(result, expected);
}

#[test]
fn test_file_copy_single_file_stays_inline() {
    let input = "file(COPY src/ DESTINATION build/)";
    let result = format_text(input, &default_config());
    assert_eq!(result, "file(COPY src/ DESTINATION build/)");
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
)";
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
    assert!(
        result.contains("# delay_meter/class.cpp\n\n\t# fdn_reverb/class.hpp"),
        "Expected blank line between comment groups, got:\n{}",
        result
    );
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
    assert!(
        result.contains("# group A continued\n\n\t# group B"),
        "Expected blank line between groups A and B, got:\n{}",
        result
    );
    assert!(
        result.contains("# group B continued\n\n\t# group C"),
        "Expected blank line between groups B and C, got:\n{}",
        result
    );
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
    assert!(
        result.contains("# audio/engine.cpp\n\n\t\t# video sources"),
        "Expected blank line between comment groups under keyword, got:\n{}",
        result
    );
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
    assert!(
        result.contains("# module_a/foo.cpp\n\n\t# disabled module B"),
        "Expected blank line between trailing comment blocks, got:\n{}",
        result
    );
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
    assert!(
        result.contains("file1.cpp\n\n\t# section header\n\tfile2.cpp"),
        "Expected preserved blank line + comment + arg ordering, got:\n{}",
        result
    );
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
    assert_eq!(
        result, pass2,
        "install TARGETS with EXPORT should be idempotent"
    );
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
    assert_eq!(
        result, pass2,
        "install TARGETS with multiline artifact should be idempotent"
    );
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
    assert_eq!(
        result, pass2,
        "install TARGETS with EXPORT should be idempotent"
    );
}

#[test]
fn test_install_targets_global_destination_component() {
    // Global DESTINATION/COMPONENT (no artifact type) must work
    let input = "install(TARGETS libwire COMPONENT wire DESTINATION lib)\n";
    let result = format_text(input, &default_config());
    // COMPONENT and DESTINATION should be recognized as keywords
    assert!(
        result.contains("COMPONENT wire"),
        "COMPONENT should be a keyword"
    );
    assert!(
        result.contains("DESTINATION lib"),
        "DESTINATION should be a keyword"
    );
    // They should NOT be consumed as target names
    assert!(
        !result.contains("TARGETS\n\t\tlibwire\n\t\tCOMPONENT"),
        "COMPONENT should not be a target arg"
    );
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
    assert!(
        result.contains("TARGETS Wire"),
        "Wire should be inline with TARGETS"
    );
    // DESTINATION and COMPONENT must be separate keyword sections
    assert!(
        result.contains("\tDESTINATION bin"),
        "DESTINATION should be a separate keyword"
    );
    assert!(
        result.contains("\tCOMPONENT wire"),
        "COMPONENT should be a separate keyword"
    );
    // Must NOT have DESTINATION/COMPONENT indented under TARGETS
    assert!(
        !result.contains("\t\tDESTINATION"),
        "DESTINATION must not be a TARGETS value"
    );
    assert!(
        !result.contains("\t\tCOMPONENT"),
        "COMPONENT must not be a TARGETS value"
    );
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
    assert!(
        result.contains("TARGETS mylib"),
        "mylib should be inline with TARGETS"
    );
    // Artifact types should consume DESTINATION as sub_keyword (BinPack behavior)
    assert!(
        result.contains("RUNTIME DESTINATION bin"),
        "RUNTIME should consume DESTINATION"
    );
    assert!(
        result.contains("LIBRARY DESTINATION lib"),
        "LIBRARY should consume DESTINATION"
    );
    assert!(
        result.contains("ARCHIVE DESTINATION lib"),
        "ARCHIVE should consume DESTINATION"
    );
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
    assert!(
        result.contains("FILES_MATCHING PATTERN \"*.h\""),
        "PATTERN should be grouped with FILES_MATCHING"
    );
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
    assert!(
        result.contains("FILES_MATCHING PATTERN \"*.h\""),
        "Single PATTERN should be inline with FILES_MATCHING"
    );
}

#[test]
fn test_files_matching_multiple_patterns_indented() {
    let mut config = default_config();
    config.max_line_length = 80;
    let input = "install(\n    DIRECTORY \"${VST24_SOURCE}/public.sdk\"\n    DESTINATION include\n    FILES_MATCHING\n    PATTERN \"*.h\"\n    PATTERN \"*.cpp\"\n    EXCLUDE PATTERN \"internal/*\"\n)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Multiple patterns: each on indented line below FILES_MATCHING
    assert!(
        result.contains("FILES_MATCHING\n"),
        "FILES_MATCHING should be on its own line with multiple patterns"
    );
    assert!(
        result.contains("\t\tPATTERN \"*.h\"\n"),
        "First PATTERN should be indented under FILES_MATCHING"
    );
    assert!(
        result.contains("\t\tPATTERN \"*.cpp\"\n"),
        "Second PATTERN should be indented under FILES_MATCHING"
    );
    assert!(
        result.contains("\t\tEXCLUDE PATTERN \"internal/*\"\n"),
        "EXCLUDE PATTERN should be indented under FILES_MATCHING"
    );
}

#[test]
fn test_files_matching_single_regex_inline() {
    let input = "install(\n    DIRECTORY src/\n    DESTINATION include\n    FILES_MATCHING\n    REGEX \".*\\.h$\"\n)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Single REGEX: should be inline with FILES_MATCHING
    assert!(
        result.contains("FILES_MATCHING REGEX"),
        "Single REGEX should be inline with FILES_MATCHING"
    );
}

#[test]
fn test_files_matching_idempotent_single() {
    let input = "install(\n    DIRECTORY include/\n    DESTINATION include\n    FILES_MATCHING\n    PATTERN \"*.h\"\n)";
    let pass1 = format_text(input, &default_config());
    let pass2 = format_text(&pass1, &default_config());
    assert_eq!(
        pass1, pass2,
        "FILES_MATCHING single pattern should be idempotent"
    );
}

#[test]
fn test_files_matching_idempotent_multiple() {
    let input = "install(\n    DIRECTORY include/\n    DESTINATION include\n    FILES_MATCHING\n    PATTERN \"*.h\"\n    PATTERN \"*.cpp\"\n)";
    let pass1 = format_text(input, &default_config());
    eprintln!("Pass 1:\n{}", pass1);
    let pass2 = format_text(&pass1, &default_config());
    eprintln!("Pass 2:\n{}", pass2);
    assert_eq!(
        pass1, pass2,
        "FILES_MATCHING multiple patterns should be idempotent"
    );
}

#[test]
fn test_files_matching_file_copy_mode() {
    let input = "file(\n    COPY src/\n    DESTINATION build/\n    FILES_MATCHING\n    PATTERN \"*.cmake\"\n)";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // file(COPY) should also support FILES_MATCHING collection formatting
    assert!(
        result.contains("FILES_MATCHING PATTERN \"*.cmake\""),
        "file(COPY) should format FILES_MATCHING inline for single pattern"
    );
}

#[test]
fn test_files_matching_flat_short_command() {
    let input = "install(DIRECTORY include/ DESTINATION include FILES_MATCHING PATTERN \"*.h\")";
    let result = format_text(input, &default_config());
    eprintln!("Result:\n{}", result);
    // Short command fits on one line — flat rendering
    assert_eq!(
        result,
        "install(DIRECTORY include/ DESTINATION include FILES_MATCHING PATTERN \"*.h\")"
    );
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
    assert!(
        result.contains("mylib STATIC\n"),
        "STATIC should collapse with target name"
    );
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
    assert!(
        result.contains("mylib STATIC"),
        "STATIC should stay on same line as target name (type-selector flag with trailing args)"
    );
    assert!(
        !result.contains("\tSTATIC\n"),
        "STATIC should NOT be on its own line"
    );
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
    assert!(
        result.contains("\tREQUIRED\n"),
        "REQUIRED should be on its own line"
    );
    assert!(
        result.contains("\tCONFIG\n"),
        "CONFIG should be on its own line"
    );
}

#[test]
fn test_collapse_empty_flags_idempotent() {
    let mut config = default_config();
    config.collapse_empty_flags = false;
    let input = "add_library(mylib STATIC src/a.cpp src/b.cpp src/c.cpp src/d.cpp src/e.cpp src/f.cpp src/g.cpp)";
    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);
    assert_eq!(
        pass1, pass2,
        "Formatting with collapse_empty_flags=false should be idempotent"
    );
}

#[test]
fn test_collapse_empty_flags_short_command_still_flat() {
    let mut config = default_config();
    config.collapse_empty_flags = false;
    let input = "add_library(mylib STATIC src/a.cpp)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Short command still fits on one line (flat rendering, no breaking needed)
    assert_eq!(result, "add_library(mylib STATIC src/a.cpp)");
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
    assert!(
        result.contains("AdobeAfterEffectsSDK STATIC"),
        "STATIC must stay inline with target name"
    );
    assert!(
        !result.contains("AdobeAfterEffectsSDK\n\tSTATIC"),
        "STATIC must NOT drop to a new line"
    );
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
    assert!(
        result.contains("mylib SHARED"),
        "SHARED must stay inline with target name"
    );
}

#[test]
fn test_collapse_empty_flags_false_add_executable_win32_stays_inline() {
    // WIN32 in add_executable is also a type-selector with trailing source args
    let mut config = default_config();
    config.collapse_empty_flags = false;
    let input = "add_executable(myapp WIN32 src/main.cpp src/app.cpp src/util.cpp src/config.cpp src/render.cpp src/input.cpp)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    assert!(
        result.contains("myapp WIN32"),
        "WIN32 must stay inline with target name"
    );
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
    assert!(
        result.contains("\tSTATIC\n"),
        "STATIC with no trailing args should be on own line"
    );
    assert!(
        result.contains("\tIMPORTED\n"),
        "IMPORTED with no trailing args should be on own line"
    );
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

// ============================================================================
// INLINE SINGLE KEYWORD TESTS
// ============================================================================

#[test]
fn test_inline_single_keyword_true_single_section() {
    // Single keyword section uses inline layout when inline_single_keyword=true.
    // Use max_line_length=60 to ensure the command wraps to multiline.
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.max_line_length = 60;
    let input = "target_sources(mylib PUBLIC src/a.cpp src/b.cpp src/c.cpp src/d.cpp src/e.cpp)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Keyword PUBLIC is on same line as mylib (inline), values are single-tab indented
    assert!(
        result.contains("mylib PUBLIC\n"),
        "PUBLIC must be on same line as mylib"
    );
    assert!(
        result.contains("\tsrc/a.cpp\n"),
        "values must be single-tab indented"
    );
    assert!(
        result.contains("\tsrc/b.cpp\n"),
        "values must be single-tab indented"
    );
    // Must NOT use double-tab indent
    assert!(
        !result.contains("\t\tsrc/a.cpp"),
        "values must NOT be double-tab indented"
    );
    // Exact expected output
    let expected = "target_sources(mylib PUBLIC\n\tsrc/a.cpp\n\tsrc/b.cpp\n\tsrc/c.cpp\n\tsrc/d.cpp\n\tsrc/e.cpp\n)";
    assert_eq!(result, expected);
}

#[test]
fn test_inline_single_keyword_true_multiple_sections() {
    // Multiple keyword sections use standard double-indent layout even when inline_single_keyword=true.
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.max_line_length = 60;
    let input = "target_sources(mylib PUBLIC src/a.cpp src/b.cpp PRIVATE src/c.cpp src/d.cpp)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Standard layout: keywords on own lines, values double-indented
    assert!(
        result.contains("\tPUBLIC\n"),
        "PUBLIC must be on its own line (standard layout)"
    );
    assert!(
        result.contains("\t\tsrc/a.cpp\n"),
        "values must be double-tab indented"
    );
    assert!(
        result.contains("\tPRIVATE\n"),
        "PRIVATE must be on its own line"
    );
    assert!(
        result.contains("\t\tsrc/c.cpp\n"),
        "values must be double-tab indented"
    );
}

#[test]
fn test_inline_single_keyword_false_default() {
    // Default (false) preserves existing behavior: keywords on own lines, values double-indented.
    let mut config = default_config();
    config.max_line_length = 60;
    let input = "target_sources(mylib PUBLIC src/a.cpp src/b.cpp src/c.cpp src/d.cpp src/e.cpp)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Standard layout
    assert!(
        result.contains("\tPUBLIC\n"),
        "PUBLIC must be on its own line with default config"
    );
    assert!(
        result.contains("\t\tsrc/a.cpp\n"),
        "values must be double-tab indented with default config"
    );
}

#[test]
fn test_inline_single_keyword_short_fits_one_line() {
    // Short command stays on one line regardless of inline_single_keyword setting.
    let mut config = default_config();
    config.inline_single_keyword = true;
    let input = "target_link_libraries(myapp PRIVATE lib1)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Fits on one line — flat rendering
    assert_eq!(result, "target_link_libraries(myapp PRIVATE lib1)");
}

#[test]
fn test_inline_single_keyword_target_link_libraries() {
    // Works with target_link_libraries: single keyword PUBLIC stays inline.
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.max_line_length = 60;
    let input = "target_link_libraries(myapp PUBLIC lib1 lib2 lib3 lib4 lib5 lib6 lib7 lib8)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // PUBLIC on same line as myapp
    assert!(
        result.contains("myapp PUBLIC\n"),
        "PUBLIC must be on same line as myapp"
    );
    // Values single-tab indented
    assert!(
        result.contains("\tlib1\n"),
        "libs must be single-tab indented"
    );
    assert!(
        !result.contains("\t\tlib1"),
        "libs must NOT be double-tab indented"
    );
}

#[test]
fn test_inline_single_keyword_idempotent() {
    // Formatting inline_single_keyword output again produces identical result (idempotency).
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.max_line_length = 60;
    let input = "target_sources(mylib PUBLIC src/a.cpp src/b.cpp src/c.cpp src/d.cpp src/e.cpp)";
    let result = format_text(input, &config);
    let pass2 = format_text(&result, &config);
    eprintln!("Pass 1:\n{}", result);
    eprintln!("Pass 2:\n{}", pass2);
    assert_eq!(
        result, pass2,
        "inline_single_keyword formatting must be idempotent"
    );
}

#[test]
fn test_inline_single_keyword_with_force_break() {
    // force_break_keywords forces multiline; with inline_single_keyword the keyword still
    // stays inline with the target name, values are single-indented.
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.force_break_keywords = true;
    let input = "target_link_libraries(myapp PUBLIC lib1)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Keyword stays inline, value single-indented
    assert!(
        result.contains("myapp PUBLIC\n"),
        "PUBLIC must be inline with myapp even with force_break"
    );
    assert!(
        result.contains("\tlib1\n"),
        "lib1 must be single-tab indented"
    );
    assert!(
        !result.contains("\t\tlib1"),
        "lib1 must NOT be double-tab indented"
    );
}

#[test]
fn test_inline_single_keyword_multiline_pre_args_no_inline() {
    // When pre-keyword args don't fit on the opening line, keyword should NOT be inlined.
    // Falls back to standard keyword formatting with keyword on its own line.
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.max_line_length = 60;
    let input = "set_source_files_properties(\n\t# wui/patch/cord/cord_anchor.cpp\n\twui/patch/node/node_view.cpp\n\twui/skin/skin.cpp PROPERTIES\n\tCOMPILE_FLAGS\n\t/wd4996\n)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // PROPERTIES must NOT be on the same line as skin.cpp — it should be on its own line
    assert!(
        !result.contains("skin.cpp PROPERTIES"),
        "keyword must NOT be inlined when pre-keyword args are multiline"
    );
    // Standard layout: keyword on own line, indented (values may follow on same line)
    assert!(
        result.contains("\tPROPERTIES"),
        "PROPERTIES must be on its own indented line"
    );
}

#[test]
fn test_inline_single_keyword_multiline_pre_args_idempotent() {
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.max_line_length = 60;
    let input = "set_source_files_properties(\n\twui/patch/node/node_view.cpp\n\twui/skin/skin.cpp\n\tPROPERTIES\n\t\tCOMPILE_FLAGS\n\t\t/wd4996\n)";
    let result = format_text(input, &config);
    let pass2 = format_text(&result, &config);
    eprintln!("Pass 1:\n{}", result);
    eprintln!("Pass 2:\n{}", pass2);
    assert_eq!(
        result, pass2,
        "multiline pre-keyword args must be idempotent"
    );
}

#[test]
fn test_inline_single_keyword_two_short_pre_args_still_inlines() {
    // Two short pre-keyword args that fit on the line: keyword should still inline.
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.max_line_length = 80;
    let input = "set_source_files_properties(a.cpp b.cpp PROPERTIES COMPILE_FLAGS /wd4996)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // With max_line_length=80, "set_source_files_properties(a.cpp b.cpp PROPERTIES" = 51 chars, fits
    // So inline should still apply
    assert!(
        result.contains("b.cpp PROPERTIES"),
        "keyword should be inlined when pre-keyword args fit on opening line"
    );
}

// ============================================================================
// INLINE SINGLE KEYWORD + LIST(APPEND) TESTS
// ============================================================================

#[test]
fn test_inline_single_keyword_list_append() {
    // list(APPEND SOURCES ...) with inline_single_keyword=true keeps "APPEND SOURCES"
    // on the command opening line with values indented below.
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.max_line_length = 60;
    let input = "list(APPEND SOURCES file1.cpp file2.cpp file3.cpp file4.cpp file5.cpp)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // APPEND SOURCES must be on the opening line (inline with the command)
    assert!(
        result.contains("list(APPEND SOURCES\n"),
        "APPEND SOURCES must be on the command opening line"
    );
    // Files must be single-tab indented
    assert!(
        result.contains("\tfile1.cpp\n"),
        "files must be single-tab indented"
    );
    assert!(
        result.contains("\tfile2.cpp\n"),
        "files must be single-tab indented"
    );
    // APPEND must NOT be on its own line
    assert!(
        !result.contains("\nAPPEND\n"),
        "APPEND must NOT be on its own line"
    );
    assert!(
        !result.contains("\tAPPEND\n"),
        "APPEND must NOT be on its own indented line"
    );
}

#[test]
fn test_inline_single_keyword_list_append_with_space_parens() {
    // Same as above but with space_between_command_parens=true.
    // Expected: list( APPEND SOURCES\n\tfile1.cpp\n...
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.space_between_command_parens = true;
    config.max_line_length = 60;
    let input = "list(APPEND SOURCES file1.cpp file2.cpp file3.cpp file4.cpp file5.cpp)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // With space_between_command_parens, opening has "list( APPEND SOURCES"
    assert!(
        result.starts_with("list( APPEND SOURCES\n"),
        "must start with 'list( APPEND SOURCES\\n'"
    );
    // Files must be single-tab indented
    assert!(
        result.contains("\tfile1.cpp\n"),
        "files must be single-tab indented"
    );
    assert!(
        result.contains("\tfile2.cpp\n"),
        "files must be single-tab indented"
    );
}

#[test]
fn test_inline_single_keyword_list_append_multiline_input() {
    // The exact user-reported case: multiline input with space_between_command_parens.
    // Formatting should preserve structure: APPEND SOURCES inline on opening line.
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.space_between_command_parens = true;
    config.max_line_length = 160;
    let input = "list( APPEND SOURCES\n    TestCoreJSON.cpp\n    TestPrimitiveJSON.cpp\n)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // APPEND SOURCES must be on the opening line
    assert!(
        result.contains("list( APPEND SOURCES\n"),
        "APPEND SOURCES must be on the command opening line"
    );
    // Files must be single-tab indented
    assert!(
        result.contains("\tTestCoreJSON.cpp\n"),
        "files must be single-tab indented"
    );
    assert!(
        result.contains("\tTestPrimitiveJSON.cpp\n"),
        "files must be single-tab indented"
    );
    // APPEND must NOT be on its own line
    assert!(
        !result.contains("\tAPPEND\n"),
        "APPEND must NOT be on its own indented line"
    );
}

#[test]
fn test_inline_single_keyword_list_append_idempotent() {
    // Formatting the output of list(APPEND) again produces identical output.
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.max_line_length = 60;
    let input = "list(APPEND SOURCES file1.cpp file2.cpp file3.cpp file4.cpp file5.cpp)";
    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);
    eprintln!("Pass 1:\n{}", pass1);
    eprintln!("Pass 2:\n{}", pass2);
    assert_eq!(
        pass1, pass2,
        "list(APPEND) + inline_single_keyword formatting must be idempotent"
    );
}

#[test]
fn test_inline_single_keyword_list_append_short_fits_one_line() {
    // Short list(APPEND mylist item) stays on one line with inline_single_keyword=true.
    let mut config = default_config();
    config.inline_single_keyword = true;
    let input = "list(APPEND mylist item)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Short enough to fit on one line
    assert_eq!(result, "list(APPEND mylist item)");
}

// ============================================================================
// KEYWORD SPACE BEFORE PAREN TESTS
// ============================================================================

#[test]
fn test_control_flow_space_before_paren_if_enabled() {
    // When enabled, if/endif get a space before (, but message does not.
    let mut config = default_config();
    config.control_flow_space_before_paren = true;
    let input = "if(TRUE)\n  message(\"hello\")\nendif()";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    assert!(result.contains("if (TRUE)"), "if must have space before (");
    assert!(
        result.contains("endif ()"),
        "endif must have space before ("
    );
    assert!(
        result.contains("message(\"hello\")"),
        "message must NOT have space before ("
    );
}

#[test]
fn test_control_flow_space_before_paren_foreach() {
    // foreach and endforeach both get a space before (.
    let mut config = default_config();
    config.control_flow_space_before_paren = true;
    let input = "foreach(item IN LISTS a b c)\nendforeach()";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    assert!(
        result.contains("foreach (item"),
        "foreach must have space before ("
    );
    assert!(
        result.contains("endforeach ()"),
        "endforeach must have space before ("
    );
}

#[test]
fn test_control_flow_space_before_paren_disabled_default() {
    // Default config (false) produces no space before (.
    let config = default_config();
    let input = "if(TRUE)\nendif()";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    assert!(
        result.contains("if(TRUE)"),
        "if must NOT have space before ( with default config"
    );
    assert!(
        result.contains("endif()"),
        "endif must NOT have space before ( with default config"
    );
}

#[test]
fn test_control_flow_space_before_paren_regular_commands_unaffected() {
    // Regular commands are never affected, even when control_flow_space_before_paren=true.
    let mut config = default_config();
    config.control_flow_space_before_paren = true;
    let input = "set(MY_VAR \"value\")\nmessage(\"hello\")\nadd_library(mylib STATIC src/a.cpp)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    assert!(
        result.contains("set(MY_VAR"),
        "set must NOT have space before ("
    );
    assert!(
        result.contains("message(\"hello\")"),
        "message must NOT have space before ("
    );
    assert!(
        result.contains("add_library(mylib"),
        "add_library must NOT have space before ("
    );
}

#[test]
fn test_control_flow_space_before_paren_all_block_commands() {
    // All 14 block commands get a space before ( when enabled.
    let mut config = default_config();
    config.control_flow_space_before_paren = true;
    let input = concat!(
        "if(cond)\n",
        "elseif(cond2)\n",
        "else()\n",
        "endif()\n",
        "foreach(x IN LISTS a)\n",
        "endforeach()\n",
        "while(cond)\n",
        "endwhile()\n",
        "macro(my_macro)\n",
        "endmacro()\n",
        "function(my_func)\n",
        "endfunction()\n",
        "block()\n",
        "endblock()\n",
    );
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    assert!(result.contains("if (cond)"), "if must have space");
    assert!(result.contains("elseif (cond2)"), "elseif must have space");
    assert!(result.contains("else ()"), "else must have space");
    assert!(result.contains("endif ()"), "endif must have space");
    assert!(result.contains("foreach (x"), "foreach must have space");
    assert!(
        result.contains("endforeach ()"),
        "endforeach must have space"
    );
    assert!(result.contains("while (cond)"), "while must have space");
    assert!(result.contains("endwhile ()"), "endwhile must have space");
    assert!(result.contains("macro (my_macro)"), "macro must have space");
    assert!(result.contains("endmacro ()"), "endmacro must have space");
    assert!(
        result.contains("function (my_func)"),
        "function must have space"
    );
    assert!(
        result.contains("endfunction ()"),
        "endfunction must have space"
    );
    assert!(result.contains("block ()"), "block must have space");
    assert!(result.contains("endblock ()"), "endblock must have space");
}

#[test]
fn test_control_flow_space_before_paren_with_closing_style_remove() {
    // Space is inserted before ( AND closing args are removed.
    let mut config = default_config();
    config.control_flow_space_before_paren = true;
    config.closing_style = ClosingStyle::Remove;
    let input = "if(condition)\nendif(condition)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    assert!(
        result.contains("if (condition)"),
        "if must have space before ("
    );
    assert!(
        result.contains("endif ()"),
        "endif must have space before ( and closing args removed"
    );
}

#[test]
fn test_control_flow_space_before_paren_idempotent() {
    // Formatting twice produces the same result.
    let mut config = default_config();
    config.control_flow_space_before_paren = true;
    let input = "if(TRUE)\n  message(\"hello\")\nendif()";
    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);
    eprintln!("Pass 1:\n{}", pass1);
    eprintln!("Pass 2:\n{}", pass2);
    assert_eq!(
        pass1, pass2,
        "control_flow_space_before_paren formatting must be idempotent"
    );
}

#[test]
fn test_control_flow_space_before_paren_nested() {
    // Nested if/foreach get correct indentation and spaces on all block commands.
    let mut config = default_config();
    config.control_flow_space_before_paren = true;
    let input = concat!(
        "if(OUTER)\n",
        "foreach(item IN LISTS my_list)\n",
        "message(${item})\n",
        "endforeach()\n",
        "endif()\n",
    );
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    assert!(result.contains("if (OUTER)"), "outer if must have space");
    assert!(result.contains("foreach (item"), "foreach must have space");
    assert!(
        result.contains("endforeach ()"),
        "endforeach must have space"
    );
    assert!(result.contains("endif ()"), "endif must have space");
    // Regular command inside block — no space
    assert!(
        result.contains("message(${item})"),
        "message must NOT have space"
    );
}

// ============================================================================
// SPACE BETWEEN COMMAND PARENS TESTS
// ============================================================================

#[test]
fn test_space_between_command_parens_single_line() {
    let mut config = default_config();
    config.space_between_command_parens = true;
    let input = r#"set(MY_VAR "value")"#;
    let result = format_text(input, &config);
    assert_eq!(result, r#"set( MY_VAR "value" )"#);
}

#[test]
fn test_space_between_command_parens_multiline_builtin() {
    let mut config = default_config();
    config.space_between_command_parens = true;
    let input =
        "set(MY_LONG_VARIABLE_NAME value1 value2 value3 value4 value5 value6 value7 value8 value9)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Space after opening paren (first arg follows on same line for builtin)
    assert!(
        result.starts_with("set( MY_LONG_VARIABLE_NAME"),
        "opening paren must have space before first arg"
    );
    // Closing ) at base indent (no extra space before it)
    let last_line = result.trim_end_matches('\n').lines().last().unwrap_or("");
    assert_eq!(
        last_line, ")",
        "closing paren must be at column 0 with no leading space"
    );
}

#[test]
fn test_space_between_command_parens_empty_args() {
    let mut config = default_config();
    config.space_between_command_parens = true;
    let input = "some_command()";
    let result = format_text(input, &config);
    // No space inside empty parens
    assert_eq!(result, "some_command()");
}

#[test]
fn test_space_between_command_parens_keyword_aware() {
    let mut config = default_config();
    config.space_between_command_parens = true;
    let input = "target_link_libraries(myapp PRIVATE lib1)";
    let result = format_text(input, &config);
    // Short command fits on one line with spaces inside parens
    assert_eq!(result, "target_link_libraries( myapp PRIVATE lib1 )");
}

#[test]
fn test_space_between_command_parens_keyword_multiline() {
    let mut config = default_config();
    config.space_between_command_parens = true;
    let input =
        "target_link_libraries(myapp PUBLIC lib1 lib2 lib3 lib4 lib5 lib6 lib7 lib8 lib9 lib10)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // First line has space after ( before first arg
    assert!(
        result.starts_with("target_link_libraries( myapp"),
        "first line must have space after ("
    );
    // Closing ) at base indent
    let last_line = result.trim_end_matches('\n').lines().last().unwrap_or("");
    assert_eq!(
        last_line, ")",
        "closing paren must be at column 0 with no leading space"
    );
}

#[test]
fn test_space_between_command_parens_false_default() {
    let config = default_config();
    let input = r#"set(MY_VAR "value")"#;
    let result = format_text(input, &config);
    // Default: no extra spaces
    assert_eq!(result, r#"set(MY_VAR "value")"#);
}

#[test]
fn test_space_between_command_parens_idempotent() {
    let mut config = default_config();
    config.space_between_command_parens = true;
    let input = r#"set(MY_VAR "value")"#;
    let first = format_text(input, &config);
    let second = format_text(&first, &config);
    assert_eq!(first, second, "formatting must be idempotent");
}

#[test]
fn test_space_between_command_parens_no_trailing_whitespace() {
    let mut config = default_config();
    config.space_between_command_parens = true;
    // A command that will break to multiline
    let input =
        "set(MY_LONG_VARIABLE_NAME value1 value2 value3 value4 value5 value6 value7 value8 value9)";
    let result = format_text(input, &config);
    for line in result.lines() {
        assert_eq!(
            line,
            line.trim_end(),
            "no line should have trailing whitespace, got: {:?}",
            line
        );
    }
}

#[test]
fn test_space_between_command_parens_single_arg() {
    // Core bug: single-arg commands must get space before closing paren
    let mut config = default_config();
    config.space_between_command_parens = true;
    let input = r#"add_subdirectory("mevi-vktools/")"#;
    let result = format_text(input, &config);
    assert_eq!(result, r#"add_subdirectory( "mevi-vktools/" )"#);
}

#[test]
fn test_space_between_command_parens_single_arg_unquoted() {
    let mut config = default_config();
    config.space_between_command_parens = true;
    let input = "add_subdirectory(subdir)";
    let result = format_text(input, &config);
    assert_eq!(result, "add_subdirectory( subdir )");
}

#[test]
fn test_space_between_command_parens_single_arg_custom_command() {
    // Custom commands also go through the single-arg fast path
    let mut config = default_config();
    config.space_between_command_parens = true;
    let input = "my_custom_func(arg1)";
    let result = format_text(input, &config);
    assert_eq!(result, "my_custom_func( arg1 )");
}

#[test]
fn test_space_between_command_parens_single_arg_idempotent() {
    let mut config = default_config();
    config.space_between_command_parens = true;
    let input = r#"add_subdirectory("mevi-vktools/")"#;
    let first = format_text(input, &config);
    let second = format_text(&first, &config);
    assert_eq!(
        first, second,
        "formatting must be idempotent for single-arg commands"
    );
}

// ============================================================================
// INDENT CLOSING PAREN TESTS
// ============================================================================

#[test]
fn test_indent_closing_paren_multiline() {
    let mut config = default_config();
    config.indent_closing_paren = true;
    let input =
        "set(MY_LONG_VARIABLE_NAME value1 value2 value3 value4 value5 value6 value7 value8 value9)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Closing ) is indented one level
    let last_line = result.trim_end_matches('\n').lines().last().unwrap_or("");
    assert_eq!(last_line, "\t)", "closing paren must be indented one tab");
}

#[test]
fn test_indent_closing_paren_single_line_unchanged() {
    let mut config = default_config();
    config.indent_closing_paren = true;
    let input = r#"set(MY_VAR "value")"#;
    let result = format_text(input, &config);
    // Single-line command is unchanged
    assert_eq!(result, r#"set(MY_VAR "value")"#);
}

#[test]
fn test_indent_closing_paren_keyword_aware() {
    let mut config = default_config();
    config.indent_closing_paren = true;
    let input =
        "target_link_libraries(myapp PUBLIC lib1 lib2 lib3 lib4 lib5 lib6 lib7 lib8 lib9 lib10)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Closing ) indented one level
    let last_line = result.trim_end_matches('\n').lines().last().unwrap_or("");
    assert_eq!(last_line, "\t)", "closing paren must be indented one tab");
}

#[test]
fn test_indent_closing_paren_false_default() {
    let config = default_config();
    let input =
        "set(MY_LONG_VARIABLE_NAME value1 value2 value3 value4 value5 value6 value7 value8 value9)";
    let result = format_text(input, &config);
    // Default: closing ) at column 0
    let last_line = result.trim_end_matches('\n').lines().last().unwrap_or("");
    assert_eq!(
        last_line, ")",
        "closing paren must be at column 0 by default"
    );
}

#[test]
fn test_indent_closing_paren_idempotent() {
    let mut config = default_config();
    config.indent_closing_paren = true;
    let input =
        "set(MY_LONG_VARIABLE_NAME value1 value2 value3 value4 value5 value6 value7 value8 value9)";
    let first = format_text(input, &config);
    let second = format_text(&first, &config);
    assert_eq!(first, second, "formatting must be idempotent");
}

#[test]
fn test_indent_closing_paren_single_arg_force_multiline() {
    // Force-multiline single-arg builtin: closing ) must be on its own indented line
    let mut config = default_config();
    config.indent_closing_paren = true;
    // A command with a newline inside forces multiline rendering
    let input = "set(\n  MY_VAR\n)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // The closing ) must be on its own line, indented one tab
    let last_line = result.trim_end_matches('\n').lines().last().unwrap_or("");
    assert_eq!(
        last_line, "\t)",
        "closing paren must be indented one tab for single-arg force-multiline"
    );
}

// ============================================================================
// COMBINED: SPACE BETWEEN PARENS + INDENT CLOSING PAREN
// ============================================================================

#[test]
fn test_combined_space_parens_and_indent_closing() {
    let mut config = default_config();
    config.space_between_command_parens = true;
    config.indent_closing_paren = true;
    let input = r#"set(MY_VAR "value")"#;
    let result = format_text(input, &config);
    // Single-line: spaces inside parens, no indent effect (no multiline)
    assert_eq!(result, r#"set( MY_VAR "value" )"#);
}

#[test]
fn test_combined_space_parens_and_indent_closing_multiline() {
    let mut config = default_config();
    config.space_between_command_parens = true;
    config.indent_closing_paren = true;
    let input =
        "set(MY_LONG_VARIABLE_NAME value1 value2 value3 value4 value5 value6 value7 value8 value9)";
    let result = format_text(input, &config);
    eprintln!("Result:\n{}", result);
    // Space after opening paren on first line
    assert!(
        result.starts_with("set( MY_LONG_VARIABLE_NAME"),
        "opening paren must have space"
    );
    // Closing ) indented one level, no extra space before it
    let last_line = result.trim_end_matches('\n').lines().last().unwrap_or("");
    assert_eq!(
        last_line, "\t)",
        "closing paren must be indented one tab without extra space"
    );
}

#[test]
fn test_combined_idempotent() {
    let mut config = default_config();
    config.space_between_command_parens = true;
    config.indent_closing_paren = true;
    let input =
        "set(MY_LONG_VARIABLE_NAME value1 value2 value3 value4 value5 value6 value7 value8 value9)";
    let first = format_text(input, &config);
    let second = format_text(&first, &config);
    assert_eq!(first, second, "combined options must be idempotent");
}

// Regression: inline_single_keyword + PairValue keyword (PROPERTIES) should keep
// key-value pairs together, not split them onto separate lines
#[test]
fn test_inline_single_keyword_pair_value_properties() {
    let mut config = default_config();
    config.inline_single_keyword = true;
    let input = "set_target_properties(myTarget PROPERTIES\n\tMACOSX_BUNDLE TRUE\n\tXCODE_ATTRIBUTE_CODE_SIGN_IDENTITY \"\"\n\tINSTALL_RPATH @executable_path/../Frameworks\n)\n";
    let result = format_text(input, &config);
    assert_eq!(
        result, input,
        "PairValue properties should stay as key-value pairs"
    );
}

#[test]
fn test_inline_single_keyword_pair_value_with_space_parens() {
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.space_between_command_parens = true;
    let input = "set_target_properties( ${targetName} PROPERTIES\n\tMACOSX_BUNDLE TRUE\n\tXCODE_ATTRIBUTE_CODE_SIGN_IDENTITY \"\"\n\tINSTALL_RPATH @executable_path/../Frameworks\n)\n";
    let result = format_text(input, &config);
    assert_eq!(
        result, input,
        "PairValue with space_between_command_parens should be idempotent"
    );
}

#[test]
fn test_inline_single_keyword_pair_value_single_pair() {
    let mut config = default_config();
    config.inline_single_keyword = true;
    let input = "set_target_properties(myTarget PROPERTIES MACOSX_BUNDLE TRUE)\n";
    let result = format_text(input, &config);
    assert_eq!(
        result, input,
        "Single property pair should stay inline with keyword"
    );
}

// Regression: source_grouping headers_first should apply to keyword section args
// in the inline_single_keyword path (e.g., files after PUBLIC in target_sources)
#[test]
fn test_inline_single_keyword_source_grouping_headers_first() {
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.source_grouping = cmake_fmt::formatter::SourceGrouping::HeadersFirst;
    let input = "target_sources(myTarget PUBLIC\n\tfoo.h\n\tfoo.cpp\n\tbar.h\n\tbaz.cpp\n)\n";
    let result = format_text(input, &config);
    // foo.h + foo.cpp should be paired on same line
    assert!(
        result.contains("foo.h foo.cpp"),
        "header+source pair should be grouped: got {}",
        result
    );
    // bar.h has no matching bar.cpp so stays alone
    assert!(
        result.contains("\tbar.h\n"),
        "unmatched header stays on its own line: got {}",
        result
    );
}

#[test]
fn test_inline_single_keyword_source_grouping_with_blank_lines() {
    let mut config = default_config();
    config.inline_single_keyword = true;
    config.source_grouping = cmake_fmt::formatter::SourceGrouping::HeadersFirst;
    config.space_between_command_parens = true;
    // Blank line between groups should be preserved
    let input = "target_sources( myTarget PUBLIC\n\talpha.h alpha.cpp\n\n\tbeta.h beta.cpp\n)\n";
    let result = format_text(input, &config);
    let second = format_text(&result, &config);
    assert_eq!(
        result, second,
        "source_grouping + inline_single_keyword must be idempotent: got {}",
        result
    );
}

#[test]
fn test_an_own_line_comment_in_a_keyword_section_survives() {
    // The shortcut that puts a lone value inline with its keyword emitted the
    // keyword and the value and nothing else, so a comment written on its own
    // line inside that section was deleted outright — silent content loss in an
    // everyday shape. It now only applies when there is nothing else to place.
    let config = FormatConfig::default();

    for input in [
        "target_sources(t PRIVATE\n\t# impl\n\tb.cpp\n)\n",
        "install(TARGETS t\n\t# note\n\tDESTINATION lib)\n",
        "target_sources(t PRIVATE\n\t# one\n\t# two\n\tb.cpp\n)\n",
        "target_compile_definitions(t PUBLIC\n\t# why\n\tFOO=1\n)\n",
    ] {
        let result = format_text(input, &config);
        for comment in ["# impl", "# note", "# one", "# two", "# why"] {
            assert_eq!(
                input.contains(comment),
                result.contains(comment),
                "comment {} lost for {:?}:\n{}",
                comment,
                input,
                result
            );
        }
        assert_eq!(result, format_text(&result, &config), "not idempotent");
    }
}

#[test]
fn test_a_lone_value_still_goes_inline_with_its_keyword() {
    // The guard above must not cost the layout it guards: with no comment, a
    // single value still sits on the keyword's line.
    let config = FormatConfig::default();
    let result = format_text("target_sources(t PRIVATE\n\tb.cpp\n)\n", &config);
    assert!(
        result.contains("PRIVATE b.cpp"),
        "a lone value should stay inline:\n{}",
        result
    );
}

#[test]
fn test_comments_survive_every_keyword_shortcut() {
    // Three arms had the same defect: they emit the keyword and its value and
    // nothing else, so anything else the section carried was deleted. One was
    // fixed; these are the other two, plus the `PairValue` branch that computed
    // the comments into its own gate and then never emitted them.
    let config = FormatConfig::default();

    for (input, comments) in [
        (
            "list(APPEND V\n\t# note\n\ta.cpp\n\tb.cpp\n)\n",
            &["# note"][..],
        ),
        (
            "install(TARGETS t\n\t# note\n\tDESTINATION lib)\n",
            &["# note"][..],
        ),
        (
            "set_target_properties(t PROPERTIES\n\tK1 v1 # one\n\tK2 v2 # two\n)\n",
            &["# one", "# two"][..],
        ),
        (
            "set_target_properties(t PROPERTIES\n\t# why\n\tK1 v1\n\tK2 v2\n)\n",
            &["# why"][..],
        ),
        (
            "add_test(NAME t\n\t# note\n\tCOMMAND runner)\n",
            &["# note"][..],
        ),
        // A comment on a bare key — the last pair with no value — is indexed to
        // the key, not the value, and only the value's index was emitted
        (
            "set_target_properties(t PROPERTIES\n\tK1 v1\n\tK2 # dangling\n)\n",
            &["# dangling"][..],
        ),
        (
            "file(WRITE out.txt\n\t# note\n\t\"body\")\n",
            &["# note"][..],
        ),
    ] {
        let result = format_text(input, &config);
        for comment in comments {
            assert!(
                result.contains(comment),
                "comment {} lost for {:?}:\n{}",
                comment,
                input,
                result
            );
        }
        assert_eq!(result, format_text(&result, &config), "not idempotent");
    }
}

#[test]
fn test_a_keyword_shortcut_still_applies_without_comments() {
    // The guards must not cost the layout they guard.
    let config = FormatConfig::default();
    assert_eq!(
        format_text("set_target_properties(t PROPERTIES K1 v1 K2 v2)\n", &config),
        "set_target_properties(t PROPERTIES K1 v1 K2 v2)\n"
    );
    assert_eq!(
        format_text("list(APPEND V a.cpp b.cpp)\n", &config),
        "list(APPEND V a.cpp b.cpp)\n"
    );
}

#[test]
fn test_a_property_value_never_lands_inside_a_comment() {
    // A comment runs to the end of its line, so emitting a key's trailing
    // comment before the key's value put the value *inside* the comment. The
    // token count looked right on the first pass, and each pass after it
    // swallowed one more token: `CXX_STANDARD # note 17`, then `... 17 AUTOMOC`,
    // then the comment, `17` and `AUTOMOC` were simply gone.
    let config = FormatConfig::default();
    let input = concat!(
        "set_target_properties(mytarget PROPERTIES\n",
        "\tCXX_STANDARD # vendor SDK needs this\n",
        "\t17\n",
        "\tAUTOMOC\n",
        "\tON\n",
        ")\n"
    );

    let mut current = format_text(input, &config);
    assert!(
        current.contains("CXX_STANDARD 17 # vendor SDK needs this"),
        "the value must come before the comment:\n{}",
        current
    );
    assert_eq!(current, format_text(&current, &config), "not idempotent");

    // Nothing may erode over repeated passes
    for _ in 0..4 {
        current = format_text(&current, &config);
        for token in [
            "CXX_STANDARD",
            "17",
            "AUTOMOC",
            "ON",
            "vendor SDK needs this",
        ] {
            assert!(current.contains(token), "{} lost:\n{}", token, current);
        }
    }

    // Odd arity: a key with no value keeps its comment too
    let result = format_text(
        "set_target_properties(t PROPERTIES\n\tCXX_STANDARD # note\n\t17\n\tAUTOMOC\n)\n",
        &config,
    );
    assert!(result.contains("CXX_STANDARD 17 # note"), "{}", result);
    assert!(result.contains("AUTOMOC"), "AUTOMOC lost:\n{}", result);
}

#[test]
fn test_a_single_property_pair_keeps_its_comments() {
    // The single-pair shortcut is tested *before* the flag that comments set, so
    // the comments that were supposed to force the per-line layout never reached
    // it and were dropped. Both the general path and the inline twin.
    for style in [false, true] {
        let config = FormatConfig {
            inline_single_keyword: style,
            ..Default::default()
        };
        for (input, comment) in [
            (
                "set_target_properties(t PROPERTIES\n\t# keep the standard\n\tCXX_STANDARD\n\t17\n)\n",
                "# keep the standard",
            ),
            (
                "set_target_properties(t PROPERTIES\n\tCXX_STANDARD\n\t17 # note\n)\n",
                "# note",
            ),
        ] {
            let result = format_text(input, &config);
            assert!(
                result.contains(comment),
                "{} lost with inline_single_keyword={}:\n{}",
                comment,
                style,
                result
            );
            assert_eq!(result, format_text(&result, &config), "not idempotent");
        }
    }
}

#[test]
fn test_the_inline_twin_emits_own_line_property_comments() {
    // The inline_single_keyword path emitted only trailing comments, so every
    // own-line comment inside a PROPERTIES run was deleted.
    let config = FormatConfig {
        inline_single_keyword: true,
        ..Default::default()
    };
    let result = format_text(
        "set_target_properties(t PROPERTIES\n\t# note\n\tCXX_STANDARD\n\t17\n\tAUTOMOC\n\tON\n)\n",
        &config,
    );
    assert!(
        result.contains("# note"),
        "own-line comment lost:\n{}",
        result
    );
    for token in ["CXX_STANDARD", "17", "AUTOMOC", "ON"] {
        assert!(result.contains(token), "{} lost:\n{}", token, result);
    }
    assert_eq!(result, format_text(&result, &config), "not idempotent");
}

#[test]
fn test_a_flag_with_no_values_keeps_its_comments() {
    // The Flag arm's whole comment machinery sat inside `!args.is_empty()`, so a
    // flag followed by another keyword lost its comment — and
    // `find_package(Foo REQUIRED # note ...)` is everyday CMake.
    //
    // The *follower* is swept rather than chosen. This test first used
    // `COMPONENTS`, a MultiValue keyword, which turned out to be the one
    // follower class whose separator is safe — so it passed while the fix it
    // guarded was destroying the following keyword for every other class. A
    // comment runs to end of line, so whatever follows it has to start a new
    // one, and nothing here may go missing.
    let config = FormatConfig::default();

    for follower in [
        "QUIET",          // Flag: collapses after the previous section
        "CONFIG",         // Flag
        "COMPONENTS b a", // MultiValue
        "NAMES foo",      // MultiValue
        "PATHS /opt",     // MultiValue
    ] {
        let input = format!(
            "find_package(Foo\n\tREQUIRED # only on windows\n\t{}\n)\n",
            follower
        );
        let result = format_text(&input, &config);

        assert!(
            result.contains("# only on windows"),
            "the flag's comment was lost before {}:\n{}",
            follower,
            result
        );
        for token in ["REQUIRED"]
            .iter()
            .chain(follower.split(' ').collect::<Vec<_>>().iter())
        {
            assert!(
                appears_as_code(&result, token),
                "{} was swallowed by the comment before {}:\n{}",
                token,
                follower,
                result
            );
        }
        assert_eq!(
            result,
            format_text(&result, &config),
            "not idempotent for follower {}",
            follower
        );
    }

    // The shapes that lose a *flag* rather than a comment, which is what the
    // collapsing separator did: every keyword after the comment vanished, as a
    // stable fixed point that `--check` then called formatted.
    for (input, tokens) in [
        (
            "find_package(Foo\n\tREQUIRED # a\n\tQUIET # b\n\tCONFIG # c\n\tGLOBAL)\n",
            &["REQUIRED", "QUIET", "CONFIG", "GLOBAL", "# a", "# b", "# c"][..],
        ),
        (
            "add_library(mylib STATIC # vendor blob\n\tEXCLUDE_FROM_ALL)\n",
            &["STATIC", "EXCLUDE_FROM_ALL", "# vendor blob"][..],
        ),
        (
            "add_executable(myexe WIN32 # note\n\tMACOSX_BUNDLE a.cpp)\n",
            &["WIN32", "MACOSX_BUNDLE", "a.cpp", "# note"][..],
        ),
    ] {
        let mut current = format_text(input, &config);
        for pass in 1..=3 {
            for token in tokens {
                let present = if token.starts_with('#') {
                    current.contains(token)
                } else {
                    appears_as_code(&current, token)
                };
                assert!(
                    present,
                    "{} lost by pass {} of {:?}:\n{}",
                    token, pass, input, current
                );
            }
            current = format_text(&current, &config);
        }
    }
}

/// Whether `token` appears as a real argument rather than as comment prose.
///
/// `contains` cannot tell the two apart, which is exactly how a bug that turned
/// `QUIET` into part of `# note QUIET` passed a test asserting `contains("QUIET")`.
/// A comment runs to the end of its line, so everything from the first `#`
/// onwards is prose. Parentheses are separators too: a first argument is glued
/// to its command name (`define_property(TEST`), and trimming only the ends of a
/// word left it unfindable, so the oracle silently passed for the token that
/// mattered most.
fn appears_as_code(text: &str, token: &str) -> bool {
    text.lines().any(|line| {
        line.split('#')
            .next()
            .unwrap_or("")
            .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
            .any(|word| word == token)
    })
}

#[test]
fn test_a_valueless_keyword_keeps_its_comments_in_every_arm() {
    // Five arms render a keyword section, and each guarded its whole comment
    // machinery on the section having values — so a comment attached to a
    // keyword with none had nowhere to go and was deleted. One was fixed at a
    // time over two rounds; these are all of them.
    let config = FormatConfig::default();
    let inline = FormatConfig {
        inline_single_keyword: true,
        ..Default::default()
    };

    let cases: &[(&str, &FormatConfig)] = &[
        // Flag
        (
            "find_package(Foo\n\tREQUIRED # note\n\tCOMPONENTS b\n)\n",
            &config,
        ),
        // PairValue
        ("set_target_properties(t\n\tPROPERTIES # note\n)\n", &config),
        // BinPack
        (
            "add_custom_command(TARGET t POST_BUILD\n\tCOMMAND # note\n)\n",
            &config,
        ),
        // MultiValue / SingleValue catch-all
        ("target_sources(t\n\tPRIVATE # note\n)\n", &config),
        ("install(TARGETS t DESTINATION\n\t# note\n)\n", &config),
        // the inline_single_keyword twin
        ("find_package(Foo REQUIRED\n\t# note\n)\n", &inline),
        ("target_sources(t\n\tPRIVATE # note\n)\n", &inline),
    ];

    for (input, cfg) in cases {
        let result = format_text(input, cfg);
        assert!(
            result.contains("# note"),
            "the comment was deleted for {:?}:\n{}",
            input,
            result
        );
        // `contains` alone would pass on a comment that had swallowed the code
        // around it, which is the mistake this file has caught three times.
        // Every non-comment token of the input has to still be an argument.
        for token in input
            .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
            .filter(|word| !word.is_empty())
            .take_while(|word| !word.starts_with('#'))
            .skip(1)
        {
            assert!(
                appears_as_code(&result, token),
                "{} was swallowed by the comment for {:?}:\n{}",
                token,
                input,
                result
            );
        }
        assert_eq!(
            result,
            format_text(&result, cfg),
            "not idempotent for {:?}",
            input
        );
    }
}

#[test]
fn test_a_valueless_keywords_comment_sits_at_the_keywords_indent() {
    // The whole point of moving these to `keyword_indent`: there are no values
    // for the comment to sit under. Three arms used `value_indent`, so the same
    // construct came out one tab in from one command and two from another.
    //
    // The sibling test asserts the comment survives and no code is swallowed —
    // which every wrong indent, and even gluing the comment onto the keyword's
    // own line, satisfies. Exact output is the only thing that sees it.
    let config = FormatConfig::default();
    let inline = FormatConfig {
        inline_single_keyword: true,
        ..Default::default()
    };
    for (input, expected, cfg) in [
        // MultiValue
        (
            "target_sources(t\n\tPRIVATE # note\n)\n",
            "target_sources(t\n\tPRIVATE\n\t# note\n)\n",
            &config,
        ),
        // PairValue
        (
            "set_target_properties(t\n\tPROPERTIES # note\n)\n",
            "set_target_properties(t\n\tPROPERTIES\n\t# note\n)\n",
            &config,
        ),
        // BinPack
        (
            "add_custom_command(TARGET t POST_BUILD\n\tCOMMAND # note\n)\n",
            "add_custom_command(\n\tTARGET t\n\tPOST_BUILD\n\tCOMMAND\n\t# note\n)\n",
            &config,
        ),
        // Flag
        (
            "find_package(Foo REQUIRED\n\t# note\n)\n",
            "find_package(Foo REQUIRED\n\t# note\n)\n",
            &config,
        ),
        // the inline_single_keyword twin
        (
            "target_sources(t\n\tPRIVATE # note\n)\n",
            "target_sources(t PRIVATE\n\t# note\n)\n",
            &inline,
        ),
    ] {
        let result = format_text(input, cfg);
        assert_eq!(
            result, expected,
            "the comment of a valueless keyword is not at the keyword's indent"
        );
        assert_eq!(result, format_text(&result, cfg), "not idempotent");
    }
}

#[test]
fn test_a_comment_before_a_commands_first_keyword_survives() {
    // The section parser pushed its leading section only when that section held
    // an argument or a keyword, so a section holding *only* comments was thrown
    // away — deleting every comment written before a command's first argument
    // when that argument is a keyword. Stable fixed point, exit 0.
    //
    // The cause predates this branch, but the branch decides which spellings
    // reach it: adding `TYPE`, `FILES`, `BASE_DIRS` to `target_sources` and
    // creating `source_group` brought six new (command, keyword) pairs into it,
    // all of which the previous release formatted correctly.
    //
    // Over every (command, keyword) pair the grammar knows, the previous release
    // loses this comment on 227 of 375; this loses none.
    let config = FormatConfig::default();
    for (command, keyword) in [
        // the six this branch would have added to the defect
        ("source_group", "FILES"),
        ("source_group", "PREFIX"),
        ("source_group", "TREE"),
        ("target_sources", "BASE_DIRS"),
        ("target_sources", "FILES"),
        ("target_sources", "TYPE"),
        // and the shapes that were already reaching it
        ("install", "FILES"),
        ("target_sources", "PUBLIC"),
        ("find_package", "REQUIRED"),
        ("execute_process", "COMMAND"),
        // multi-mode, where the mode keyword is emitted inline with the command
        // name — it must not be inlined onto a line the comment has ended
        ("list", "APPEND"),
        ("file", "APPEND"),
        ("define_property", "CACHED_VARIABLE"),
    ] {
        let input = format!("{command}(\n\t# ship the headers\n\t{keyword} x\n)\n");
        let result = format_text(&input, &config);
        assert!(
            result.contains("# ship the headers"),
            "the leading comment was deleted for {} {}:\n{}",
            command,
            keyword,
            result
        );
        assert!(
            appears_as_code(&result, keyword),
            "{} was swallowed by the comment in {}:\n{}",
            keyword,
            command,
            result
        );
        assert!(
            appears_as_code(&result, "x"),
            "the value was swallowed by the comment in {} {}:\n{}",
            command,
            keyword,
            result
        );
        assert_eq!(
            result,
            format_text(&result, &config),
            "not idempotent for {} {}",
            command,
            keyword
        );
    }
}

#[test]
fn test_a_single_value_keyword_keeps_a_comment_on_its_value() {
    // The arm that stopped a comment demoting a `SingleValue` section emits two
    // kinds after the value: the trailing comment on the value's own line, and
    // the own-line comments written around it. Both are new, and both were
    // unpinned — deleting the trailing-comment loop left the whole suite green
    // while `list(APPEND V # note)` came back without `# note`, at exit 0 and a
    // stable fixed point.
    //
    // Exact output, because order is the other half of it: emitting the comment
    // *before* the value also passed, and produced `APPEND # note V # note` —
    // the value swallowed into a comment and the comment duplicated. That is the
    // mistake the `PairValue` arm already carries a comment about.
    let config = FormatConfig::default();
    for (input, expected) in [
        // a trailing comment on the value
        (
            "list(APPEND V # note\n\ta.cpp\n)\n",
            "list(APPEND V # note\n\ta.cpp\n)\n",
        ),
        (
            "add_custom_command(TARGET t POST_BUILD\n\tCOMMENT c # note\n\tCOMMAND x)\n",
            "add_custom_command(\n\tTARGET t\n\tPOST_BUILD\n\tCOMMENT c # note\n\tCOMMAND x\n)\n",
        ),
        // an own-line comment written before the value: it cannot stay there —
        // a comment runs to end of line and would swallow the value — so it
        // moves after, at the keyword's indent
        (
            "add_custom_command(TARGET t POST_BUILD\n\tCOMMENT\n\t# note\n\tc\n\tCOMMAND x)\n",
            "add_custom_command(\n\tTARGET t\n\tPOST_BUILD\n\tCOMMENT c\n\t# note\n\tCOMMAND x\n)\n",
        ),
        // and both kinds at once, in that order
        (
            "list(APPEND V # trail\n\t# own\n\ta.cpp\n)\n",
            "list(APPEND V # trail\n\t# own\n\ta.cpp\n)\n",
        ),
    ] {
        let result = format_text(input, &config);
        assert_eq!(result, expected, "wrong placement for {:?}", input);
        assert_eq!(result, format_text(&result, &config), "not idempotent");
        // and nothing became comment prose. The tokens are taken from the
        // input rather than listed, so a shape added later cannot skip the
        // check by not mentioning them.
        for line in input.lines() {
            for token in line
                .split('#')
                .next()
                .unwrap_or("")
                .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
                .filter(|word| !word.is_empty())
                .skip(usize::from(line.starts_with(char::is_alphabetic)))
            {
                assert!(
                    appears_as_code(&result, token),
                    "{} was swallowed by the comment for {:?}:\n{}",
                    token,
                    input,
                    result
                );
            }
        }
    }
}

#[test]
fn test_a_comment_does_not_move_a_mode_keyword_off_its_line() {
    // The `SingleValue` shortcut keeps a multi-mode command's mode keyword and
    // its value on the opening line — `list(APPEND V …)`. A comment used to
    // demote the whole section into the catch-all arm, which puts the keyword on
    // its own line and its value one level deeper than the elements that follow:
    //
    //     list(            <- APPEND left the line
    //     \tAPPEND
    //     \t\tV            <- and its value is deeper than a.cpp
    //     \t\t# note
    //     \ta.cpp
    //     )
    //
    // 68 of 345 mode/style shapes did that. The comments are emitted after the
    // value now, so the arm renders one shape with them or without.
    let config = FormatConfig::default();
    for (command, mode) in [
        ("list", "APPEND"),
        ("list", "PREPEND"),
        ("list", "REMOVE_ITEM"),
        ("file", "GLOB"),
        ("file", "WRITE"),
        ("install", "SCRIPT"),
        ("install", "CODE"),
        ("file", "READ"),
        // `string(REGEX …)` and `define_property(TEST …)` are not this arm:
        // their mode keyword is a Flag, so they go through the Flag arm, which
        // has always honoured `first_keyword_inline`.
    ] {
        for body in ["\t# note\n\ta.cpp\n", "\ta.cpp\n\t# note\n"] {
            let input = format!("{command}({mode} V\n{body})\n");
            let result = format_text(&input, &config);
            assert!(
                result.starts_with(&format!("{command}({mode} V")),
                "the mode keyword left its line:\n{}",
                result
            );
            assert!(
                result.contains("# note"),
                "the comment was dropped:\n{}",
                result
            );
            for token in ["V", "a.cpp"] {
                assert!(
                    appears_as_code(&result, token),
                    "{} was swallowed by the comment:\n{}",
                    token,
                    result
                );
            }
            assert_eq!(result, format_text(&result, &config), "not idempotent");
        }
    }
}

#[test]
fn test_the_inline_twin_keeps_a_single_values_comment_too() {
    // `format_keyword_aware_args_inline_single` kept the guard the general path
    // shed: its shortcut required the section to carry no comments, so a comment
    // put the mode keyword on its own line and cost the first-pass fixed point
    // under sorting, exactly as it did on the general path before. Same input,
    // same failure, one function over — which is why the test that caught it on
    // the general path now runs against both configurations.
    let inline = FormatConfig {
        inline_single_keyword: true,
        ..Default::default()
    };
    assert_eq!(
        format_text("list(APPEND V # note\n\ta.cpp\n)\n", &inline),
        "list(APPEND V # note\n\ta.cpp\n)\n",
        "the trailing comment moved the mode keyword off its line"
    );
    assert_eq!(
        format_text("list(APPEND V\n\t# note\n\ta.cpp\n)\n", &inline),
        "list(APPEND V\n\t# note\n\ta.cpp\n)\n",
        "the own-line comment moved the mode keyword off its line"
    );
    // A blank line still disqualifies the shortcut — it has nowhere to put one —
    // so the keyword drops to its own line. Byte-identical to the previous
    // release, which is the point: the guard was narrowed to comments only.
    assert_eq!(
        format_text("list(APPEND V\n\n\ta.cpp\n)\n", &inline),
        "list(APPEND\n\tV\n\ta.cpp\n)\n",
        "a blank line should still disqualify the shortcut"
    );
}

#[test]
fn test_sorting_a_run_past_a_comment_still_reaches_a_fixed_point() {
    // The parser gives a comment written after a `SingleValue` keyword's one
    // value to *that* section, while a `list(APPEND V …)` run's elements live in
    // the following keyword-less section. So sorting the run to put the
    // commented element first moved the comment into that slot, and the *next*
    // pass laid the whole command out differently — `--check` rejecting freshly
    // formatted output. 12 of 345 shapes; the layout no longer depends on
    // whether the section carries a comment.
    for config in [
        FormatConfig {
            sort_sources: cmake_fmt::formatter::SortSources::Alphabetical,
            ..Default::default()
        },
        FormatConfig {
            source_grouping: cmake_fmt::formatter::SourceGrouping::HeadersFirst,
            ..Default::default()
        },
        FormatConfig {
            sort_sources: cmake_fmt::formatter::SortSources::Alphabetical,
            source_grouping: cmake_fmt::formatter::SourceGrouping::HeadersFirst,
            ..Default::default()
        },
        // The same shapes through the inline twin, which is a separate function
        // and kept the same bug for a round after the general path lost it
        FormatConfig {
            inline_single_keyword: true,
            sort_sources: cmake_fmt::formatter::SortSources::Alphabetical,
            ..Default::default()
        },
        FormatConfig {
            inline_single_keyword: true,
            sort_sources: cmake_fmt::formatter::SortSources::Alphabetical,
            source_grouping: cmake_fmt::formatter::SourceGrouping::HeadersFirst,
            ..Default::default()
        },
    ] {
        for input in [
            "list(APPEND V\n\tz.cpp\n\t# note\n\ta.cpp\n)\n",
            "list(PREPEND V\n\tz.cpp\n\t# note\n\ta.cpp\n)\n",
            "list(APPEND V\n\tb.cpp\n\t# note\n\tb.h\n)\n",
        ] {
            let once = format_text(input, &config);
            let twice = format_text(&once, &config);
            assert_eq!(
                once, twice,
                "sorting past a comment never settles:\n--- pass 1 ---\n{}\n--- pass 2 ---\n{}",
                once, twice
            );
            assert!(
                once.contains("# note"),
                "the comment was dropped:\n{}",
                once
            );
        }
    }
}

#[test]
fn test_only_the_second_section_groups_with_the_leading_flag() {
    // `define_property(TEST PROPERTY name)` puts the mode flag and the keyword
    // after it on one line. Section index 1 only: a *later* `SingleValue` is an
    // ordinary keyword and takes its own line. Loosening the index test to
    // `>= 1` survived the suite and is observable on 9 of 18 probed shapes —
    // `string(RANDOM LENGTH 8 ALPHABET abc)` collapses two keywords onto the
    // opening line instead of one.
    let config = FormatConfig::default();
    for (input, expected) in [
        (
            "string(RANDOM\n\tLENGTH 8\n\tALPHABET abc\n\tv\n)\n",
            "string(RANDOM LENGTH 8\n\tALPHABET abc\n\tv\n)\n",
        ),
        (
            "define_property(TEST\n\tPROPERTY x\n\tBRIEF_DOCS \"b\"\n)\n",
            "define_property(TEST PROPERTY x\n\tBRIEF_DOCS \"b\"\n)\n",
        ),
    ] {
        let result = format_text(input, &config);
        assert_eq!(
            result, expected,
            "only the section right after the leading flag joins its line"
        );
        assert_eq!(result, format_text(&result, &config), "not idempotent");
    }
}

#[test]
fn test_a_multi_mode_commands_first_flag_keeps_its_comment() {
    // The `SingleValue` arm collapses a multi-mode command's mode keyword onto
    // the following keyword — `define_property(TEST PROPERTY foo)` — and its
    // separator was an unconditional space, the one arm the previous round's
    // guard was not wired into. So the follower became comment prose:
    // `# note PROPERTY foo`, with the code gone from the file, stable, at exit 0.
    //
    // `file(GENERATE …)` escalated: pass 1 swallowed `OUTPUT out.txt`, pass 2
    // also swallowed `CONTENT "x"`, so two runs of a pre-commit hook ate the
    // command body.
    //
    // The arm is reached only by a multi-mode command whose first section is a
    // valueless flag and whose second is a `SingleValue` — a *command shape*,
    // not a follower type, which is the dimension the sweep above cannot see.
    let config = FormatConfig::default();

    let cases: &[(&str, &[&str])] = &[
        (
            "define_property(TEST # note\n\tPROPERTY foo\n\tBRIEF_DOCS \"b\"\n)\n",
            &["TEST", "PROPERTY", "foo", "BRIEF_DOCS"],
        ),
        (
            "define_property(GLOBAL # note\n\tPROPERTY foo\n)\n",
            &["GLOBAL", "PROPERTY", "foo"],
        ),
        (
            "define_property(DIRECTORY # note\n\tPROPERTY foo\n)\n",
            &["DIRECTORY", "PROPERTY", "foo"],
        ),
        (
            "define_property(SOURCE # note\n\tPROPERTY foo\n)\n",
            &["SOURCE", "PROPERTY", "foo"],
        ),
        (
            "define_property(CACHED_VARIABLE # note\n\tPROPERTY foo\n)\n",
            &["CACHED_VARIABLE", "PROPERTY", "foo"],
        ),
        (
            "file(GENERATE # note\n\tOUTPUT out.txt\n\tCONTENT \"x\"\n)\n",
            &["GENERATE", "OUTPUT", "out.txt", "CONTENT"],
        ),
        (
            "file(CONFIGURE # note\n\tOUTPUT out.txt\n\tCONTENT \"x\"\n)\n",
            &["CONFIGURE", "OUTPUT", "out.txt", "CONTENT"],
        ),
        (
            "file(ARCHIVE_CREATE # note\n\tOUTPUT a.tar\n\tPATHS p\n)\n",
            &["ARCHIVE_CREATE", "OUTPUT", "a.tar", "PATHS"],
        ),
        (
            "file(GET_RUNTIME_DEPENDENCIES # note\n\tRESOLVED_DEPENDENCIES_VAR v\n)\n",
            &["GET_RUNTIME_DEPENDENCIES", "RESOLVED_DEPENDENCIES_VAR", "v"],
        ),
        (
            "string(RANDOM # note\n\tLENGTH 8\n\tv\n)\n",
            &["RANDOM", "LENGTH", "8", "v"],
        ),
        (
            "string(UUID # note\n\tNAMESPACE ns\n\tNAME n\n\tTYPE MD5\n\tv\n)\n",
            &["UUID", "NAMESPACE", "ns", "NAME", "TYPE", "v"],
        ),
    ];

    // Three passes: the escalating shape looks partly intact after one.
    for (input, tokens) in cases {
        let mut current = format_text(input, &config);
        for pass in 1..=3 {
            assert!(
                current.contains("# note"),
                "the comment was lost by pass {} of {:?}:\n{}",
                pass,
                input,
                current
            );
            for token in *tokens {
                assert!(
                    appears_as_code(&current, token),
                    "{} was swallowed by the comment by pass {} of {:?}:\n{}",
                    token,
                    pass,
                    input,
                    current
                );
            }
            current = format_text(&current, &config);
        }
    }
}

#[test]
fn test_a_trailing_comment_on_a_section_with_values_holds_the_line_too() {
    // The guard reads both halves of a section's comments, and only the own-line
    // half was pinned — cutting `trailing_comments` out of it survived the whole
    // suite. It is not dead: these are the shapes where the *previous* section
    // has values and its comment trails them, which is the commonest spelling of
    // all and the one the collapsing separator ate.
    let config = FormatConfig::default();
    for (input, tokens) in [
        (
            "add_library(l STATIC a.cpp # note\n\tEXCLUDE_FROM_ALL)\n",
            &["l", "STATIC", "a.cpp", "EXCLUDE_FROM_ALL"][..],
        ),
        (
            "find_package(Foo # note\n\tREQUIRED)\n",
            &["Foo", "REQUIRED"][..],
        ),
        (
            "add_executable(e a.cpp # note\n\tWIN32)\n",
            &["e", "a.cpp", "WIN32"][..],
        ),
    ] {
        let result = format_text(input, &config);
        assert!(
            result.contains("# note"),
            "the trailing comment was lost for {:?}:\n{}",
            input,
            result
        );
        for token in tokens {
            assert!(
                appears_as_code(&result, token),
                "{} was swallowed by the trailing comment for {:?}:\n{}",
                token,
                input,
                result
            );
        }
        assert_eq!(result, format_text(&result, &config), "not idempotent");
    }
}

#[test]
fn test_the_inline_twin_does_not_collapse_onto_a_commented_line() {
    // The inline twin's "keyword follows pre-keyword args" separator is an
    // unconditional space as well, so under `inline_single_keyword` the keyword
    // joined a line the pre-keyword args had already ended with a comment. The
    // mechanism predates the branch, but new grammar entries for `target_sources`
    // and `source_group` brought eighteen more shapes into it — and a file can
    // turn the setting on for itself.
    let config = FormatConfig {
        inline_single_keyword: true,
        ..Default::default()
    };
    for (input, tokens) in [
        (
            "target_sources(t # c\n\tTYPE sv1\n)\n",
            &["t", "TYPE", "sv1"][..],
        ),
        (
            "target_sources(t # c\n\tPRIVATE a.cpp\n)\n",
            &["t", "PRIVATE", "a.cpp"][..],
        ),
        (
            "source_group(g # c\n\tFILES a.cpp\n)\n",
            &["g", "FILES", "a.cpp"][..],
        ),
        (
            "# cmake-fmt: inline_single_keyword=true\ntarget_sources(t # c\n\tTYPE sv1\n)\n",
            &["t", "TYPE", "sv1"][..],
        ),
    ] {
        let result = format_text(input, &config);
        assert!(
            result.contains("# c"),
            "the comment was lost for {:?}:\n{}",
            input,
            result
        );
        for token in tokens {
            assert!(
                appears_as_code(&result, token),
                "{} was swallowed by the comment for {:?}:\n{}",
                token,
                input,
                result
            );
        }
        assert_eq!(result, format_text(&result, &config), "not idempotent");
    }

    // And without a comment the keyword still shares the line — that collapse is
    // the whole point of the setting.
    assert!(
        format_text("target_sources(t\n\tPRIVATE a.cpp b.cpp\n)\n", &config)
            .starts_with("target_sources(t PRIVATE"),
        "the inline collapse stopped happening"
    );
}

#[test]
fn test_a_comment_after_the_last_property_pair_survives() {
    // The inline twin's "anything written after the last pair" loop had no test:
    // deleting it left the whole suite green while the comment was dropped.
    let config = FormatConfig {
        inline_single_keyword: true,
        ..Default::default()
    };
    for input in [
        "set_target_properties(t PROPERTIES\n\tK1 V1\n\tK2 V2\n\t# after all\n)\n",
        "set_target_properties(t PROPERTIES\n\tK1\n\t# mid\n\tV1\n)\n",
    ] {
        let result = format_text(input, &config);
        assert!(
            result.contains("# after all") || result.contains("# mid"),
            "the comment was dropped for {:?}:\n{}",
            input,
            result
        );
        assert_eq!(result, format_text(&result, &config), "not idempotent");
    }
}

#[test]
fn test_the_inline_twins_pair_comments_stay_with_their_pairs() {
    // The inline twin's per-pair comment loop was pinned for *presence* only:
    // making it never fire left every comment preserved and every one detached
    // from the pair it annotates, falling through to the drain that runs after
    // the last pair —
    //
    //     PROPERTIES / A 1 / B 2 / # about A / # about B
    //
    // No content lost, so no oracle that counts tokens or comments notices.
    // Exact output is what sees it.
    let inline = FormatConfig {
        inline_single_keyword: true,
        ..Default::default()
    };
    assert_eq!(
        format_text(
            "set_target_properties(t PROPERTIES\n\t# about A\n\tA 1\n\t# about B\n\tB 2\n)\n",
            &inline
        ),
        "set_target_properties(t PROPERTIES\n\t# about A\n\tA 1\n\t# about B\n\tB 2\n)\n",
        "a pair's comment came away from its pair"
    );
    // A comment after the last pair still belongs after it
    assert_eq!(
        format_text(
            "set_target_properties(t PROPERTIES\n\tA 1\n\tB 2\n\t# after all\n)\n",
            &inline
        ),
        "set_target_properties(t PROPERTIES\n\tA 1\n\tB 2\n\t# after all\n)\n",
        "a trailing comment moved"
    );
}

#[test]
fn test_a_blank_line_before_a_leading_comment_is_written_once() {
    // Two rules wrote the same blank. A comments-only section holds no
    // arguments, so its own `blank_lines[0]` and the "blank line between
    // sections" rule — which looks at `blank_lines.contains(&args.len())` —
    // are the same entry, and both fired. One blank line in the source came
    // back as two, on 233 of 233 grammar shapes; the previous release cannot
    // show it only because it deletes the comment.
    let config = FormatConfig::default();
    for (command, keyword) in [
        ("install", "FILES"),
        ("target_sources", "PRIVATE"),
        ("source_group", "FILES"),
        ("find_package", "COMPONENTS"),
    ] {
        let input = format!("{command}(\n\n\t# ship\n\t{keyword} a.h\n)\n");
        let result = format_text(&input, &config);
        assert!(
            !result.contains("\n\n\t# ship\n\n"),
            "one blank line came back as two for {} {}:\n{:?}",
            command,
            keyword,
            result
        );
        assert_eq!(
            result.matches("\n\n").count(),
            input.matches("\n\n").count(),
            "the number of blank lines changed for {} {}:\n{:?}",
            command,
            keyword,
            result
        );
        assert_eq!(result, format_text(&result, &config), "not idempotent");
    }

    // A positional argument in the same slot is not a comments-only section and
    // must keep the behaviour it had
    let control = format_text("install(\n\n\tq\n\tFILES a.h\n)\n", &config);
    assert_eq!(
        control,
        format_text(&control, &config),
        "control not idempotent"
    );
}

#[test]
fn test_a_blank_after_the_last_argument_survives_grouping() {
    // Blanks are re-emitted at segment boundaries, so a blank whose recorded
    // position equals `args.len()` had no boundary to be written at and was
    // dropped — while the comment-blank bookkeeping passed through unchanged.
    // The next parse read the survivor as a different kind of entry and laid the
    // section out differently, so `--check` rejected freshly formatted output.
    for config in [
        FormatConfig {
            source_grouping: cmake_fmt::formatter::SourceGrouping::HeadersFirst,
            ..Default::default()
        },
        FormatConfig {
            sort_sources: cmake_fmt::formatter::SortSources::Alphabetical,
            ..Default::default()
        },
    ] {
        for input in [
            "define_property(\n\n\t# c\n\n\t# d\n\tDIRECTORY\n\tSRCS\n\tz.cpp\n\ta.cpp\n)\n",
            "set(SRCS\n\tz.cpp\n\ta.cpp\n\n\t# after the last\n)\n",
            // Two comment groups separated by a blank, inside a sortable
            // keyword section: `comment_blank_indices` names a comment by index
            // and sorting rebuilds the comments in a new order, so the entry
            // came to name a different comment than the author meant and the
            // next parse recorded a different index again. The previous release
            // fails this at one blank and passes at two and three by accident —
            // its blank record fired a spurious second time, which happened to
            // make those spellings self-consistent. All three settle here.
            "install(\n\tFILES\n\tSRCS\n\n\t# c\n\n\t# d\n\tz.cpp\n\ta.cpp\n)\n",
            "install(\n\tFILES\n\tSRCS\n\n\n\t# c\n\n\n\t# d\n\tz.cpp\n\ta.cpp\n)\n",
            "install(\n\tFILES\n\tSRCS\n\n\n\n\t# c\n\n\n\n\t# d\n\tz.cpp\n\ta.cpp\n)\n",
        ] {
            let once = format_text(input, &config);
            let twice = format_text(&once, &config);
            assert_eq!(
                once, twice,
                "no first-pass fixed point:\n--- pass 1 ---\n{}\n--- pass 2 ---\n{}",
                once, twice
            );
        }
    }
}

#[test]
fn test_a_blank_run_before_a_comment_reaches_a_fixed_point() {
    // The blank-line record fired once per newline past the first, so the third
    // newline of one blank run took the "blank line between comment groups"
    // branch and recorded a blank before comment index 0 — for a comment that
    // sits at a later argument position. The next pass then wrote a blank in
    // front of that comment, so `--check` rejected freshly formatted output.
    //
    // Whitespace only, no content at risk, but the previous release has 99 of
    // these across the (command, keyword) pairs the grammar knows and this has
    // none.
    let config = FormatConfig::default();
    for input in [
        "add_custom_command(\n\n\n\tq\n\t#b\n\tVERBATIM\n)\n",
        "target_sources(\n\n\n\tt\n\t# note\n\tPRIVATE a.cpp\n)\n",
        "install(\n\n\n\tx\n\t# note\n\tFILES a.h\n)\n",
        // more than one blank run, and a blank between comment groups, which
        // still has to keep its blank
        "set(SRCS\n\n\n\tx.cpp\n\n\n\t# a\n\ty.cpp\n)\n",
        "set(SRCS\n\t# a\n\n\t# b\n\tx.cpp\n)\n",
    ] {
        let once = format_text(input, &config);
        let twice = format_text(&once, &config);
        assert_eq!(
            once, twice,
            "no first-pass fixed point for {:?}:\n--- pass 1 ---\n{}\n--- pass 2 ---\n{}",
            input, once, twice
        );
    }
}

#[test]
fn test_a_blank_between_comment_groups_names_the_right_comment() {
    // `comment_blank_indices` promises "a blank line before the *next* comment
    // at this argument position". Three ways that promise went wrong, all
    // whitespace, all costing the output its first-pass fixed point or adding a
    // line nobody wrote:
    //
    //  - an argument arriving instead of a second comment left the promise
    //    unkept, and the entry went on to name whatever comment landed at that
    //    index later — where `blank_lines` already carried a blank of its own
    //  - the "already wrote it" guard read `args.is_empty()`, which is only one
    //    of the three section shapes that emit their own blank
    //  - clearing the vector after a sort was right when the sort reordered the
    //    comments and wrong when it was the identity
    let config = FormatConfig::default();
    let sorting = FormatConfig {
        sort_sources: cmake_fmt::formatter::SortSources::Alphabetical,
        ..Default::default()
    };

    // (a) an argument between the two comment groups
    for input in [
        "install(\n\tFILES\n\n\t# note\n\n\tx\n\n\t# c\n)\n",
        "target_sources(t\n\tPRIVATE\n\n\t# note\n\n\ta.cpp\n\n\t# c\n)\n",
    ] {
        let once = format_text(input, &config);
        assert_eq!(
            once,
            format_text(&once, &config),
            "no first-pass fixed point:\n--- pass 1 ---\n{}\n--- pass 2 ---\n{}",
            once,
            format_text(&once, &config)
        );
    }

    // (b) every shape that writes its own blank writes exactly one
    for input in [
        "install(\n\n\t# ship\n\tFILES a.h\n)\n",
        "install(\n\tFILES a.h\n\n\t# c\n\tDESTINATION inc)\n",
        "target_sources(t\n\n\t# c\n\tPRIVATE a.cpp\n)\n",
    ] {
        let result = format_text(input, &config);
        assert_eq!(
            result.matches("\n\n").count(),
            input.matches("\n\n").count(),
            "the number of blank lines changed for {:?}:\n{:?}",
            input,
            result
        );
        assert_eq!(result, format_text(&result, &config), "not idempotent");
    }

    // (c) an identity sort keeps the blank; a real sort settles
    assert_eq!(
        format_text(
            "add_executable(\n\tEXCLUDE_FROM_ALL\n\n\t# c\n\n\t# d\n\ta.cpp\n\tz.cpp\n)\n",
            &sorting
        ),
        "add_executable(\n\tEXCLUDE_FROM_ALL\n\n\t# c\n\n\t# d\n\ta.cpp\n\tz.cpp\n)\n",
        "an identity sort should not delete the blank between comment groups"
    );
    for input in [
        "install(\n\tFILES\n\tSRCS\n\n\t# c\n\n\t# d\n\tz.cpp\n\ta.cpp\n)\n",
        "install(\n\tFILES\n\tSRCS\n\n\n\t# c\n\n\n\t# d\n\tz.cpp\n\ta.cpp\n)\n",
    ] {
        let once = format_text(input, &sorting);
        assert_eq!(
            once,
            format_text(&once, &sorting),
            "a real sort never settles"
        );
    }
}
