use cmake_fmt::SyntaxNode;
use cmake_fmt::cst::{CommandInvocation, parse_text};
use cmake_fmt::formatter::grammar::KeywordType;
use cmake_fmt::formatter::grammar::argparse_extractor::extract_command_grammars_from_body;

/// Helper to extract function/macro body commands from parsed CMake text
///
/// Returns (function_name, body_commands)
fn extract_function_body(root: &SyntaxNode) -> (String, Vec<CommandInvocation>) {
    let mut function_name = String::new();
    let mut body_commands = Vec::new();
    let mut in_function = false;

    for child in root.children() {
        if let Some(cmd) = CommandInvocation::cast(child)
            && let Some(name) = cmd.name_text()
        {
            let name_lower = name.to_lowercase();

            if name_lower == "function" || name_lower == "macro" {
                // Extract function name (first argument)
                if let Some(arg_list) = cmd.argument_list()
                    && let Some(first_arg) = arg_list.arguments().next()
                {
                    function_name = first_arg.text().to_string();
                }
                in_function = true;
            } else if name_lower == "endfunction" || name_lower == "endmacro" {
                in_function = false;
            } else if in_function {
                body_commands.push(cmd);
            }
        }
    }

    (function_name, body_commands)
}

#[test]
fn test_basic_cmake_parse_arguments_literal() {
    let cmake_text = r#"
function(my_install)
    cmake_parse_arguments(MY_INSTALL "OPTIONAL;VERBOSE" "DESTINATION;COMPONENT" "TARGETS;CONFIGURATIONS" ${ARGN})
endfunction()
"#;

    let cst = parse_text(cmake_text);
    let (func_name, body_commands) = extract_function_body(&cst.root);

    assert_eq!(func_name, "my_install");

    let grammar = extract_command_grammars_from_body(&func_name, &body_commands);
    assert!(grammar.is_some(), "Expected grammar to be extracted");

    let grammar = grammar.unwrap();

    // Verify Flag keywords
    assert_eq!(grammar.keyword_type("OPTIONAL"), Some(KeywordType::Flag));
    assert_eq!(grammar.keyword_type("VERBOSE"), Some(KeywordType::Flag));

    // Verify SingleValue keywords
    assert_eq!(
        grammar.keyword_type("DESTINATION"),
        Some(KeywordType::SingleValue)
    );
    assert_eq!(
        grammar.keyword_type("COMPONENT"),
        Some(KeywordType::SingleValue)
    );

    // Verify MultiValue keywords
    assert_eq!(
        grammar.keyword_type("TARGETS"),
        Some(KeywordType::MultiValue)
    );
    assert_eq!(
        grammar.keyword_type("CONFIGURATIONS"),
        Some(KeywordType::MultiValue)
    );
}

#[test]
fn test_parse_argv_form() {
    let cmake_text = r#"
function(my_func)
    cmake_parse_arguments(PARSE_ARGV 0 MY_FUNC "QUIET" "OUTPUT_DIR" "SOURCES;HEADERS")
endfunction()
"#;

    let cst = parse_text(cmake_text);
    let (func_name, body_commands) = extract_function_body(&cst.root);

    assert_eq!(func_name, "my_func");

    let grammar = extract_command_grammars_from_body(&func_name, &body_commands);
    assert!(grammar.is_some(), "Expected grammar to be extracted");

    let grammar = grammar.unwrap();

    // Verify Flag keywords
    assert_eq!(grammar.keyword_type("QUIET"), Some(KeywordType::Flag));

    // Verify SingleValue keywords
    assert_eq!(
        grammar.keyword_type("OUTPUT_DIR"),
        Some(KeywordType::SingleValue)
    );

    // Verify MultiValue keywords
    assert_eq!(
        grammar.keyword_type("SOURCES"),
        Some(KeywordType::MultiValue)
    );
    assert_eq!(
        grammar.keyword_type("HEADERS"),
        Some(KeywordType::MultiValue)
    );
}

#[test]
fn test_variable_resolution() {
    let cmake_text = r#"
function(my_cmd)
    set(_options VERBOSE FORCE)
    set(_single DESTINATION)
    set(_multi SOURCES HEADERS)
    cmake_parse_arguments(MY_CMD "${_options}" "${_single}" "${_multi}" ${ARGN})
endfunction()
"#;

    let cst = parse_text(cmake_text);
    let (func_name, body_commands) = extract_function_body(&cst.root);

    assert_eq!(func_name, "my_cmd");

    let grammar = extract_command_grammars_from_body(&func_name, &body_commands);
    assert!(grammar.is_some(), "Expected grammar to be extracted");

    let grammar = grammar.unwrap();

    // Verify Flag keywords (from _options)
    assert_eq!(grammar.keyword_type("VERBOSE"), Some(KeywordType::Flag));
    assert_eq!(grammar.keyword_type("FORCE"), Some(KeywordType::Flag));

    // Verify SingleValue keywords (from _single)
    assert_eq!(
        grammar.keyword_type("DESTINATION"),
        Some(KeywordType::SingleValue)
    );

    // Verify MultiValue keywords (from _multi)
    assert_eq!(
        grammar.keyword_type("SOURCES"),
        Some(KeywordType::MultiValue)
    );
    assert_eq!(
        grammar.keyword_type("HEADERS"),
        Some(KeywordType::MultiValue)
    );
}

#[test]
fn test_empty_keyword_list() {
    let cmake_text = r#"
function(simple_cmd)
    cmake_parse_arguments(SIMPLE "" "NAME" "VALUES" ${ARGN})
endfunction()
"#;

    let cst = parse_text(cmake_text);
    let (func_name, body_commands) = extract_function_body(&cst.root);

    assert_eq!(func_name, "simple_cmd");

    let grammar = extract_command_grammars_from_body(&func_name, &body_commands);
    assert!(grammar.is_some(), "Expected grammar to be extracted");

    let grammar = grammar.unwrap();

    // No Flag keywords (empty string)
    assert_eq!(grammar.keyword_type("OPTIONAL"), None);
    assert_eq!(grammar.keyword_type("VERBOSE"), None);

    // Verify SingleValue keywords
    assert_eq!(grammar.keyword_type("NAME"), Some(KeywordType::SingleValue));

    // Verify MultiValue keywords
    assert_eq!(
        grammar.keyword_type("VALUES"),
        Some(KeywordType::MultiValue)
    );
}

#[test]
fn test_no_cmake_parse_arguments() {
    let cmake_text = r#"
function(helper_func ARG1)
    message(STATUS "Hello ${ARG1}")
endfunction()
"#;

    let cst = parse_text(cmake_text);
    let (func_name, body_commands) = extract_function_body(&cst.root);

    assert_eq!(func_name, "helper_func");

    let grammar = extract_command_grammars_from_body(&func_name, &body_commands);
    assert!(
        grammar.is_none(),
        "Expected no grammar when cmake_parse_arguments is absent"
    );
}

#[test]
fn test_single_keyword_per_category() {
    let cmake_text = r#"
function(my_func)
    cmake_parse_arguments(MY "ENABLE" "PATH" "FILES" ${ARGN})
endfunction()
"#;

    let cst = parse_text(cmake_text);
    let (func_name, body_commands) = extract_function_body(&cst.root);

    assert_eq!(func_name, "my_func");

    let grammar = extract_command_grammars_from_body(&func_name, &body_commands);
    assert!(grammar.is_some(), "Expected grammar to be extracted");

    let grammar = grammar.unwrap();

    // Verify Flag keywords
    assert_eq!(grammar.keyword_type("ENABLE"), Some(KeywordType::Flag));

    // Verify SingleValue keywords
    assert_eq!(grammar.keyword_type("PATH"), Some(KeywordType::SingleValue));

    // Verify MultiValue keywords
    assert_eq!(grammar.keyword_type("FILES"), Some(KeywordType::MultiValue));
}
