use cmake_fmt::formatter::{FormatConfig, SourceGrouping, format_text};
use std::collections::HashMap;

#[test]
fn test_idempotency_config_grammar_command() {
    let input = r#"
my_install(
    OPTIONAL
    REQUIRED
    DESTINATION /usr/bin
    COMPONENT runtime
    FILES a.txt b.txt c.txt
    TARGETS mytarget
)
"#;

    // Define custom grammar via config
    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "my_install".to_string(),
        cmake_fmt::formatter::CommandGrammarConfig {
            options: vec!["OPTIONAL".to_string(), "REQUIRED".to_string()],
            one_value_keywords: vec!["DESTINATION".to_string(), "COMPONENT".to_string()],
            multi_value_keywords: vec!["FILES".to_string(), "TARGETS".to_string()],
            ..Default::default()
        },
    );

    let config = FormatConfig {
        command_grammars,
        max_line_length: 80,
        ..Default::default()
    };

    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);

    assert_eq!(
        pass1, pass2,
        "Config grammar formatting is not idempotent.\nPass1:\n{}\nPass2:\n{}",
        pass1, pass2
    );
}

#[test]
fn test_idempotency_source_grouping() {
    let input = r#"
target_sources(mylib
    PUBLIC
        widget.cpp
        widget.h
        util.hpp
        util.cpp
        standalone.c
)
"#;

    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 80,
        ..Default::default()
    };

    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);

    assert_eq!(
        pass1, pass2,
        "Source grouping formatting is not idempotent.\nPass1:\n{}\nPass2:\n{}",
        pass1, pass2
    );
}

#[test]
fn test_idempotency_user_command_with_cmake_parse_arguments() {
    let input = r#"
function(my_test)
    cmake_parse_arguments(
        ARG
        "QUICK;SLOW"
        "NAME;TIMEOUT"
        "SOURCES;DEPENDS"
        ${ARGN}
    )

    my_test(
        QUICK
        NAME mytest
        TIMEOUT 30
        SOURCES a.cpp b.cpp
        DEPENDS dep1 dep2
    )
endfunction()
"#;

    let config = FormatConfig {
        max_line_length: 80,
        ..Default::default()
    };

    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);

    assert_eq!(
        pass1, pass2,
        "User command with cmake_parse_arguments is not idempotent.\nPass1:\n{}\nPass2:\n{}",
        pass1, pass2
    );
}

#[test]
fn test_idempotency_mixed_features() {
    // Complex input combining:
    // - Config-defined grammar
    // - Source grouping
    // - Builtin keyword-aware commands
    // - User commands
    let input = r#"
project(MyProject VERSION 1.0)

function(my_deploy)
    cmake_parse_arguments(
        DEP
        "PRODUCTION;STAGING"
        "ENV;REGION"
        "FILES;SCRIPTS"
        ${ARGN}
    )
endfunction()

add_library(mylib STATIC)

target_sources(mylib
    PRIVATE
        core.cpp
        core.h
        util.hpp
        util.cpp
        main.cpp
)

target_link_libraries(mylib
    PUBLIC
        dep1
        dep2
    PRIVATE
        internal1
)

my_deploy(
    PRODUCTION
    ENV prod
    REGION us-west
    FILES config.yaml secrets.env
    SCRIPTS deploy.sh cleanup.sh
)

my_custom_command(
    FLAG1
    FLAG2
    OPTION value
    FILES a.txt b.txt
)
"#;

    // Config grammar for my_custom_command
    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "my_custom_command".to_string(),
        cmake_fmt::formatter::CommandGrammarConfig {
            options: vec!["FLAG1".to_string(), "FLAG2".to_string()],
            one_value_keywords: vec!["OPTION".to_string()],
            multi_value_keywords: vec!["FILES".to_string()],
            ..Default::default()
        },
    );

    let config = FormatConfig {
        command_grammars,
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 80,
        ..Default::default()
    };

    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);

    assert_eq!(
        pass1, pass2,
        "Mixed features formatting is not idempotent.\nPass1:\n{}\nPass2:\n{}",
        pass1, pass2
    );
}

#[test]
fn test_idempotency_large_realistic_file() {
    // Realistic CMake file with diverse patterns
    let input = r#"
cmake_minimum_required(VERSION 3.20)
project(LargeProject VERSION 2.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Core library
add_library(core STATIC)

target_sources(core
    PRIVATE
        src/core/engine.cpp
        src/core/engine.h
        src/core/parser.hpp
        src/core/parser.cpp
        src/core/lexer.cpp
        src/core/lexer.h
        src/core/ast.cpp
        src/core/ast.hpp
        src/core/types.h
        src/core/visitor.cpp
)

target_include_directories(core
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/src
)

target_compile_definitions(core
    PRIVATE
        CORE_VERSION="${PROJECT_VERSION}"
        $<$<CONFIG:Debug>:DEBUG_MODE>
        $<$<CONFIG:Release>:RELEASE_MODE>
)

target_link_libraries(core
    PUBLIC
        fmt::fmt
        spdlog::spdlog
    PRIVATE
        Boost::filesystem
        Boost::system
)

# Utility library
add_library(util STATIC)

target_sources(util
    PRIVATE
        src/util/string.cpp
        src/util/string.h
        src/util/file.cpp
        src/util/file.hpp
        src/util/config.cpp
        src/util/config.h
)

target_link_libraries(util
    PUBLIC
        core
    PRIVATE
        yaml-cpp::yaml-cpp
)

# Main executable
add_executable(main src/main.cpp)

target_link_libraries(main
    PRIVATE
        core
        util
)

# Tests
if(BUILD_TESTING)
    enable_testing()

    add_executable(core_test)

    target_sources(core_test
        PRIVATE
            tests/engine_test.cpp
            tests/parser_test.cpp
            tests/lexer_test.cpp
            tests/ast_test.cpp
    )

    target_link_libraries(core_test
        PRIVATE
            core
            util
            GTest::gtest
            GTest::gtest_main
    )

    add_test(
        NAME core_tests
        COMMAND core_test
        WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
    )

    set_tests_properties(core_tests
        PROPERTIES
            TIMEOUT 300
            LABELS "unit"
    )
endif()

# Installation
install(
    TARGETS core util main
    EXPORT LargeProjectTargets
    ARCHIVE DESTINATION lib
    LIBRARY DESTINATION lib
    RUNTIME DESTINATION bin
    INCLUDES DESTINATION include
)

install(
    DIRECTORY include/
    DESTINATION include
    FILES_MATCHING PATTERN "*.h" PATTERN "*.hpp"
)
"#;

    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };

    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);

    assert_eq!(
        pass1, pass2,
        "Large realistic file formatting is not idempotent"
    );
}

#[test]
fn test_idempotency_all_keyword_types() {
    // Test all keyword types remain idempotent
    let input = r#"
# Flag keywords
target_compile_features(mylib PUBLIC cxx_std_17 PRIVATE cxx_constexpr)

# SingleValue keywords
set_target_properties(mylib PROPERTIES VERSION 1.0)

# MultiValue keywords
target_sources(mylib PRIVATE a.cpp b.cpp c.cpp)

# PairValue keywords
set_target_properties(mylib PROPERTIES
    VERSION 1.0
    SOVERSION 1
    OUTPUT_NAME mylib
    POSITION_INDEPENDENT_CODE ON
)

# Mixed in one command
target_link_libraries(mylib
    PUBLIC
        dep1::dep1
        dep2::dep2
    PRIVATE
        internal1
        internal2
    INTERFACE
        iface1
)
"#;

    let config = FormatConfig {
        max_line_length: 80,
        ..Default::default()
    };

    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);

    assert_eq!(
        pass1, pass2,
        "All keyword types formatting is not idempotent.\nPass1:\n{}\nPass2:\n{}",
        pass1, pass2
    );
}

#[test]
fn test_idempotency_with_comments_and_blank_lines() {
    let input = r#"
target_sources(mylib
    # Core files
    PRIVATE
        # Engine implementation
        engine.cpp
        engine.h

        # Parser implementation
        parser.cpp
        parser.hpp

        # Utilities
        util.c
        util.h
)
"#;

    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_blank_lines: 1,
        max_line_length: 80,
        ..Default::default()
    };

    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);

    assert_eq!(
        pass1, pass2,
        "Formatting with comments and blank lines is not idempotent.\nPass1:\n{}\nPass2:\n{}",
        pass1, pass2
    );
}

#[test]
fn test_idempotency_edge_case_empty_sections() {
    let input = r#"
target_link_libraries(mylib
    PUBLIC
    PRIVATE
        dep1
    INTERFACE
)
"#;

    let config = FormatConfig::default();

    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);

    assert_eq!(
        pass1, pass2,
        "Empty sections formatting is not idempotent.\nPass1:\n{}\nPass2:\n{}",
        pass1, pass2
    );
}

#[test]
fn test_idempotency_nested_generator_expressions() {
    let input = r#"
target_compile_definitions(mylib
    PRIVATE
        VERSION="$<IF:$<CONFIG:Debug>,debug,release>"
        BUILD_TYPE="$<CONFIG>"
        PLATFORM="$<PLATFORM_ID>"
)
"#;

    let config = FormatConfig::default();

    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);

    assert_eq!(
        pass1, pass2,
        "Nested generator expressions formatting is not idempotent.\nPass1:\n{}\nPass2:\n{}",
        pass1, pass2
    );
}

#[test]
fn test_idempotency_unbalanced_parens_converges() {
    // Input CMake itself rejects. The formatter supplies one missing closer per
    // pass, so it takes as many passes as there are unclosed parens — not a
    // fixed two, as this test first claimed. `--check` on the formatter's own
    // output can disagree until it settles.
    //
    // The deficit shrinks by one per pass only because the closer lands where
    // the parser can see it next time. That is not true in general: where the
    // group's text ends inside an unterminated construct — a bracket comment,
    // a quote, a bracket argument — the closer is swallowed by it and the next
    // pass appends another, so the output grows without bound. Those shapes are
    // covered by the content guard, which refuses to write any of this to disk;
    // the lexer flag that would fix the cause is a follow-up.
    //
    // Pinned rather than fixed: closing them all at once would invent more
    // syntax into a file that is already invalid.
    let config = FormatConfig::default();

    for (input, expected_passes) in [("cmd((# c\n\n", 2), ("cmd(((# c\n\n", 3)] {
        let mut current = input.to_string();
        let mut passes = 0;

        for _ in 0..8 {
            let next = format_text(&current, &config);
            passes += 1;
            if next == current {
                break;
            }
            current = next;
        }

        // A bound, not an exact count: how many passes recovery takes is not a
        // promise to anyone, and pinning it exactly turns any change to error
        // recovery into a test edit.
        assert!(
            passes <= expected_passes,
            "{:?} should reach a fixed point within {} passes, took {}",
            input,
            expected_passes,
            passes
        );
        assert!(current.contains("# c"), "comment lost:\n{}", current);
        assert!(current.contains("cmd("), "command lost:\n{}", current);
    }
}

#[test]
fn test_idempotency_unterminated_group_with_comment() {
    // The verbatim text of a comment-bearing group ends inside the open line
    // comment when the parser never saw the closing paren, so the caller's
    // paren landed inside the comment and the next pass appended another —
    // one byte per run, forever, with `--check` never going green.
    let config = FormatConfig::default();

    for input in [
        "f((A # c",
        "f((A\n# c",
        "set(V (a # c",
        "target_sources(t PRIVATE (a.cpp # c",
    ] {
        let once = format_text(input, &config);
        assert_eq!(
            once,
            format_text(&once, &config),
            "still growing for {:?}",
            input
        );
    }
}

#[test]
fn test_idempotency_balanced_nested_groups() {
    // The ordinary case is a fixed point on the first pass, *and* the group is
    // still there. Asserting only idempotency passed before the fix too:
    // deleting the group is idempotent as well.
    let config = FormatConfig::default();
    for (input, expected) in [
        ("if((TRUE))\nendif()\n", "if((TRUE))\nendif()\n"),
        (
            "if((A AND B) OR (C AND D))\nendif()\n",
            "if((A AND B) OR (C AND D))\nendif()\n",
        ),
        ("if(NOT(TRUE))\nendif()\n", "if(NOT(TRUE))\nendif()\n"),
        ("set(V (a b) c.cpp)\n", "set(V (a b) c.cpp)\n"),
    ] {
        let once = format_text(input, &config);
        assert_eq!(once, expected, "wrong output for {:?}", input);
        assert_eq!(
            once,
            format_text(&once, &config),
            "not idempotent: {}",
            input
        );
    }
}
