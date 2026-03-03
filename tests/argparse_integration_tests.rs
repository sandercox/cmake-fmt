use cmake_fmt::formatter::{format_text_with_diagnostics, format_text_with_diagnostics_and_path, FormatConfig};
use std::fs;
use tempfile::TempDir;

/// Helper to set up a temporary project with CMake files and format a file within it
fn setup_and_format(
    files: &[(&str, &str)],
    file_to_format: &str,
    config: &FormatConfig,
) -> String {
    // Clear caches for test isolation
    cmake_fmt::formatter::grammar::clear_project_scan_cache();
    cmake_fmt::formatter::grammar::clear_project_grammar_cache();

    // Create temp directory
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Write .cmake-fmt.toml to mark project root
    fs::write(project_root.join(".cmake-fmt.toml"), "").unwrap();

    // Write all files
    for (rel_path, content) in files {
        let full_path = project_root.join(rel_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full_path, content).unwrap();
    }

    // Format the specified file
    let file_path = project_root.join(file_to_format);
    let content = fs::read_to_string(&file_path).unwrap();
    let (formatted, _warnings) = format_text_with_diagnostics_and_path(&content, config, Some(&file_path), false);
    formatted
}

#[test]
fn test_user_command_keyword_formatting() {
    let files = vec![
        (
            "cmake/MyModule.cmake",
            r#"function(my_install)
    cmake_parse_arguments(MY_INSTALL "OPTIONAL" "DESTINATION;COMPONENT" "TARGETS;CONFIGURATIONS" ${ARGN})
    # ... implementation
endfunction()
"#,
        ),
        (
            "CMakeLists.txt",
            r#"include(cmake/MyModule.cmake)
my_install(TARGETS mylib mylib2 DESTINATION lib COMPONENT runtime CONFIGURATIONS Release Debug)
"#,
        ),
    ];

    let mut config = FormatConfig::default();
    config.max_line_length = 40; // Force breaking

    let result = setup_and_format(&files, "CMakeLists.txt", &config);

    // Expected: keyword-aware formatting with proper structure
    // OPTIONAL is Flag, DESTINATION/COMPONENT are SingleValue (inline), TARGETS/CONFIGURATIONS are MultiValue (one-per-line)
    let expected = "include(cmake/MyModule.cmake)\nmy_install(\n\tTARGETS\n\t\tmylib\n\t\tmylib2\n\tDESTINATION lib\n\tCOMPONENT runtime\n\tCONFIGURATIONS\n\t\tRelease\n\t\tDebug\n)\n";

    assert_eq!(result, expected);
}

#[test]
fn test_user_command_short_stays_inline() {
    let files = vec![
        (
            "cmake/MyModule.cmake",
            r#"function(my_install)
    cmake_parse_arguments(MY_INSTALL "OPTIONAL" "DESTINATION;COMPONENT" "TARGETS;CONFIGURATIONS" ${ARGN})
endfunction()
"#,
        ),
        (
            "CMakeLists.txt",
            r#"include(cmake/MyModule.cmake)
my_install(TARGETS mylib DESTINATION lib)
"#,
        ),
    ];

    let config = FormatConfig::default();
    let result = setup_and_format(&files, "CMakeLists.txt", &config);

    // Expected: stays on one line (fits within max_line_length)
    let expected = r#"include(cmake/MyModule.cmake)
my_install(TARGETS mylib DESTINATION lib)
"#;

    assert_eq!(result, expected);
}

#[test]
fn test_user_command_no_cmake_parse_arguments() {
    let files = vec![
        (
            "cmake/MyModule.cmake",
            r#"function(my_helper ARG1 ARG2)
    message(STATUS "${ARG1} ${ARG2}")
endfunction()
"#,
        ),
        (
            "CMakeLists.txt",
            r#"include(cmake/MyModule.cmake)
my_helper(hello world)
"#,
        ),
    ];

    let config = FormatConfig::default();
    let result = setup_and_format(&files, "CMakeLists.txt", &config);

    // Expected: simple formatting, no keyword awareness
    let expected = r#"include(cmake/MyModule.cmake)
my_helper(hello world)
"#;

    assert_eq!(result, expected);
}

#[test]
fn test_builtin_takes_precedence() {
    let files = vec![
        (
            "cmake/MyModule.cmake",
            r#"function(find_package)
    cmake_parse_arguments(FP "CUSTOM_FLAG" "" "" ${ARGN})
endfunction()
"#,
        ),
        (
            "CMakeLists.txt",
            r#"include(cmake/MyModule.cmake)
find_package(Boost REQUIRED COMPONENTS filesystem)
"#,
        ),
    ];

    let config = FormatConfig::default();
    let result = setup_and_format(&files, "CMakeLists.txt", &config);

    // Expected: REQUIRED and COMPONENTS are formatted per the builtin grammar, not CUSTOM_FLAG
    // The builtin find_package grammar should be used, not the user-defined one
    let expected = r#"include(cmake/MyModule.cmake)
find_package(Boost REQUIRED COMPONENTS filesystem)
"#;

    assert_eq!(result, expected);
}

#[test]
fn test_idempotency_user_command_grammar() {
    let files = vec![
        (
            "cmake/MyModule.cmake",
            r#"function(my_install)
    cmake_parse_arguments(MY_INSTALL "OPTIONAL" "DESTINATION;COMPONENT" "TARGETS;CONFIGURATIONS" ${ARGN})
endfunction()
"#,
        ),
        (
            "CMakeLists.txt",
            r#"include(cmake/MyModule.cmake)
my_install(TARGETS mylib mylib2 DESTINATION lib COMPONENT runtime CONFIGURATIONS Release Debug)
"#,
        ),
    ];

    let mut config = FormatConfig::default();
    config.max_line_length = 40; // Force breaking

    let result1 = setup_and_format(&files, "CMakeLists.txt", &config);

    // Format the result again
    let files_pass2 = vec![
        (
            "cmake/MyModule.cmake",
            r#"function(my_install)
    cmake_parse_arguments(MY_INSTALL "OPTIONAL" "DESTINATION;COMPONENT" "TARGETS;CONFIGURATIONS" ${ARGN})
endfunction()
"#,
        ),
        ("CMakeLists.txt", &result1),
    ];

    let result2 = setup_and_format(&files_pass2, "CMakeLists.txt", &config);

    // format(format(x)) == format(x)
    assert_eq!(result1, result2);
}

#[test]
fn test_variable_resolution_in_project() {
    let files = vec![
        (
            "cmake/MyModule.cmake",
            r#"function(my_build)
    set(_opts VERBOSE FORCE)
    set(_single OUTPUT_DIR)
    set(_multi SOURCES HEADERS)
    cmake_parse_arguments(MY "${_opts}" "${_single}" "${_multi}" ${ARGN})
endfunction()
"#,
        ),
        (
            "CMakeLists.txt",
            r#"include(cmake/MyModule.cmake)
my_build(VERBOSE SOURCES main.cpp util.cpp HEADERS main.h OUTPUT_DIR build)
"#,
        ),
    ];

    let mut config = FormatConfig::default();
    config.max_line_length = 40; // Force breaking

    let result = setup_and_format(&files, "CMakeLists.txt", &config);

    // Expected: VERBOSE is Flag, OUTPUT_DIR is SingleValue (inline), SOURCES/HEADERS are MultiValue
    // Note: HEADERS has only 1 arg, so it stays inline (MultiValue single-arg behavior)
    let expected = "include(cmake/MyModule.cmake)\nmy_build(\n\tVERBOSE\n\tSOURCES\n\t\tmain.cpp\n\t\tutil.cpp\n\tHEADERS main.h\n\tOUTPUT_DIR build\n)\n";

    assert_eq!(result, expected);
}

#[test]
fn test_same_file_cmake_parse_arguments_grammar() {
    let files = vec![
        (
            "CMakeLists.txt",
            r#"function(mevi_ConfigureExample)
    cmake_parse_arguments(PARSED "" "TARGET" "SOURCES;LINK_TARGETS" ${ARGN})

    mevi_ConfigureApplication(
        TARGET ${PARSED_TARGET}
        SOURCES "${PARSED_SOURCES}"
        LINK_TARGETS "mevi-example-base;${PARSED_LINK_TARGETS}"
        RESOURCES_FOLDER "${CMAKE_CURRENT_LIST_DIR}/Resources"
    )
endfunction()

mevi_ConfigureExample(
    TARGET 01_HelloTriangle
    SOURCES "${sourceFiles}"
)
"#,
        ),
    ];

    let mut config = FormatConfig::default();
    config.max_line_length = 40; // Force breaking

    let result = setup_and_format(&files, "CMakeLists.txt", &config);

    // TARGET is a SingleValue keyword -- it should stay on the same line as its value
    assert!(
        result.contains("TARGET 01_HelloTriangle"),
        "TARGET (single-value) should keep its value on the same line.\nGot:\n{}",
        result
    );
    // SOURCES is a MultiValue keyword
    assert!(
        result.contains("SOURCES"),
        "SOURCES keyword should be present.\nGot:\n{}",
        result
    );
}

#[test]
fn test_stdin_cmake_parse_arguments_grammar() {
    let input = r#"function(my_cmd)
    cmake_parse_arguments(MY "" "OUTPUT" "SOURCES" ${ARGN})
endfunction()

my_cmd(OUTPUT build SOURCES main.cpp util.cpp)
"#;

    let mut config = FormatConfig::default();
    config.max_line_length = 30; // Force breaking

    let (result, _warnings) = format_text_with_diagnostics(input, &config);

    // OUTPUT is SingleValue -- should keep its value inline
    assert!(
        result.contains("OUTPUT build"),
        "OUTPUT (single-value) should keep its value on the same line in stdin mode.\nGot:\n{}",
        result
    );
}
