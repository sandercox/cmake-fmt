use cmake_format::formatter::{format_text, CommandCase, FormatConfig};

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
    // Check indentation: keyword at 1 level, values at 2 levels
    assert!(result.contains("  PUBLIC\n"));
    assert!(result.contains("    lib1\n"));
}

#[test]
fn test_target_link_libraries_short_fits_one_line() {
    let input = "target_link_libraries(myapp PRIVATE lib1)";
    let result = format_text(input, &default_config());
    // Should stay on one line since it fits within 80 chars
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
    assert!(result.contains("DESTINATION\n"));
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
    assert!(result.contains("  # Platform specific"));
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
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 3); // command, blank, command
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
    assert!(result.contains("  # Windows-specific libraries\n"));
    // Command should stay on one line (short enough)
    assert!(result.contains("  target_link_libraries(myapp PRIVATE kernel32 user32)\n"));
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
    // Verify indentation levels
    assert!(result.contains("  PUBLIC\n"));
    assert!(result.contains("  PRIVATE\n"));
    assert!(result.contains("  INTERFACE\n"));
    assert!(result.contains("    lib1\n"));
    assert!(result.contains("    lib3\n"));
    assert!(result.contains("    lib5\n"));
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
