use super::{CommandGrammar, Grammar, KeywordType};
use std::collections::HashMap;

use KeywordType::*;

/// Returns grammar definitions for all keyword-aware CMake builtin commands
pub fn builtin_grammars() -> HashMap<String, Grammar> {
    let mut grammars = HashMap::new();

    // Helper macro to reduce boilerplate
    macro_rules! grammar {
        ($cmd:expr, $($kw:expr => $ty:expr),* $(,)?) => {{
            let keywords = vec![
                $(($kw, $ty),)*
            ];
            grammars.insert($cmd.to_string(), Grammar::Simple(CommandGrammar::from_keywords(&keywords)));
        }};
    }

    // target_link_libraries
    grammar!("target_link_libraries",
        "PUBLIC" => MultiValue,
        "PRIVATE" => MultiValue,
        "INTERFACE" => MultiValue,
    );

    // target_sources
    grammar!("target_sources",
        "PUBLIC" => MultiValue,
        "PRIVATE" => MultiValue,
        "INTERFACE" => MultiValue,
        "FILE_SET" => SingleValue,
    );

    // target_compile_options
    grammar!("target_compile_options",
        "PUBLIC" => MultiValue,
        "PRIVATE" => MultiValue,
        "INTERFACE" => MultiValue,
        "BEFORE" => Flag,
    );

    // target_include_directories
    grammar!("target_include_directories",
        "PUBLIC" => MultiValue,
        "PRIVATE" => MultiValue,
        "INTERFACE" => MultiValue,
        "BEFORE" => Flag,
        "AFTER" => Flag,
        "SYSTEM" => Flag,
    );

    // target_compile_definitions
    grammar!("target_compile_definitions",
        "PUBLIC" => MultiValue,
        "PRIVATE" => MultiValue,
        "INTERFACE" => MultiValue,
    );

    // target_compile_features
    grammar!("target_compile_features",
        "PUBLIC" => MultiValue,
        "PRIVATE" => MultiValue,
        "INTERFACE" => MultiValue,
    );

    // target_link_directories
    grammar!("target_link_directories",
        "PUBLIC" => MultiValue,
        "PRIVATE" => MultiValue,
        "INTERFACE" => MultiValue,
        "BEFORE" => Flag,
        "AFTER" => Flag,
    );

    // target_link_options
    grammar!("target_link_options",
        "PUBLIC" => MultiValue,
        "PRIVATE" => MultiValue,
        "INTERFACE" => MultiValue,
        "BEFORE" => Flag,
    );

    // target_precompile_headers
    grammar!("target_precompile_headers",
        "PUBLIC" => MultiValue,
        "PRIVATE" => MultiValue,
        "INTERFACE" => MultiValue,
        "REUSE_FROM" => SingleValue,
    );

    // add_library - REMOVED: type keywords (STATIC/SHARED/MODULE) are positional, not section keywords
    // Falls back to simple formatting which was working correctly

    // add_executable - REMOVED: type keywords (WIN32/MACOSX_BUNDLE) are positional, not section keywords
    // Falls back to simple formatting which was working correctly

    // find_package
    grammar!("find_package",
        "REQUIRED" => Flag,
        "QUIET" => Flag,
        "CONFIG" => Flag,
        "MODULE" => Flag,
        "NO_MODULE" => Flag,
        "NO_POLICY_SCOPE" => Flag,
        "GLOBAL" => Flag,
        "COMPONENTS" => MultiValue,
        "OPTIONAL_COMPONENTS" => MultiValue,
        "NAMES" => MultiValue,
        "CONFIGS" => MultiValue,
        "HINTS" => MultiValue,
        "PATHS" => MultiValue,
        "REGISTRY_VIEW" => SingleValue,
        "PATH_SUFFIXES" => MultiValue,
        "NO_DEFAULT_PATH" => Flag,
        "NO_PACKAGE_ROOT_PATH" => Flag,
        "NO_CMAKE_PATH" => Flag,
        "NO_CMAKE_ENVIRONMENT_PATH" => Flag,
        "NO_SYSTEM_ENVIRONMENT_PATH" => Flag,
        "NO_CMAKE_PACKAGE_REGISTRY" => Flag,
        "NO_CMAKE_SYSTEM_PATH" => Flag,
        "NO_CMAKE_SYSTEM_PACKAGE_REGISTRY" => Flag,
        "NO_CMAKE_INSTALL_PREFIX" => Flag,
        "CMAKE_FIND_ROOT_PATH_BOTH" => Flag,
        "ONLY_CMAKE_FIND_ROOT_PATH" => Flag,
        "NO_CMAKE_FIND_ROOT_PATH" => Flag,
    );

    // install - multi-mode command (Phase 14)
    {
        let mut modes = HashMap::new();

        // TARGETS mode
        modes.insert("TARGETS".to_string(), CommandGrammar::from_keywords(&[
            ("TARGETS", MultiValue),
            ("DESTINATION", SingleValue),
            ("PERMISSIONS", MultiValue),
            ("CONFIGURATIONS", MultiValue),
            ("COMPONENT", SingleValue),
            ("NAMELINK_COMPONENT", SingleValue),
            ("OPTIONAL", Flag),
            ("EXCLUDE_FROM_ALL", Flag),
            ("NAMELINK_ONLY", Flag),
            ("NAMELINK_SKIP", Flag),
            // Artifact type selectors
            ("ARCHIVE", Flag),
            ("LIBRARY", Flag),
            ("RUNTIME", Flag),
            ("OBJECTS", Flag),
            ("FRAMEWORK", Flag),
            ("BUNDLE", Flag),
            ("PUBLIC_HEADER", Flag),
            ("PRIVATE_HEADER", Flag),
            ("RESOURCE", Flag),
            // FILE_SET support
            ("FILE_SET", SingleValue),
            ("TYPE", SingleValue),
            ("INCLUDES", MultiValue),
        ]));

        // FILES mode
        modes.insert("FILES".to_string(), CommandGrammar::from_keywords(&[
            ("FILES", MultiValue),
            ("DESTINATION", SingleValue),
            ("PERMISSIONS", MultiValue),
            ("CONFIGURATIONS", MultiValue),
            ("COMPONENT", SingleValue),
            ("RENAME", SingleValue),
            ("OPTIONAL", Flag),
            ("EXCLUDE_FROM_ALL", Flag),
            ("TYPE", SingleValue),
        ]));

        // PROGRAMS mode
        modes.insert("PROGRAMS".to_string(), CommandGrammar::from_keywords(&[
            ("PROGRAMS", MultiValue),
            ("DESTINATION", SingleValue),
            ("PERMISSIONS", MultiValue),
            ("CONFIGURATIONS", MultiValue),
            ("COMPONENT", SingleValue),
            ("RENAME", SingleValue),
            ("OPTIONAL", Flag),
            ("EXCLUDE_FROM_ALL", Flag),
            ("TYPE", SingleValue),
        ]));

        // DIRECTORY mode
        modes.insert("DIRECTORY".to_string(), CommandGrammar::from_keywords(&[
            ("DIRECTORY", MultiValue),
            ("DESTINATION", SingleValue),
            ("FILE_PERMISSIONS", MultiValue),
            ("DIRECTORY_PERMISSIONS", MultiValue),
            ("USE_SOURCE_PERMISSIONS", Flag),
            ("OPTIONAL", Flag),
            ("MESSAGE_NEVER", Flag),
            ("CONFIGURATIONS", MultiValue),
            ("COMPONENT", SingleValue),
            ("EXCLUDE_FROM_ALL", Flag),
            ("FILES_MATCHING", Flag),
            ("PATTERN", SingleValue),
            ("REGEX", SingleValue),
            ("EXCLUDE", Flag),
            ("TYPE", SingleValue),
        ]));

        // SCRIPT mode
        modes.insert("SCRIPT".to_string(), CommandGrammar::from_keywords(&[
            ("SCRIPT", SingleValue),
        ]));

        // CODE mode
        modes.insert("CODE".to_string(), CommandGrammar::from_keywords(&[
            ("CODE", SingleValue),
        ]));

        // EXPORT mode
        modes.insert("EXPORT".to_string(), CommandGrammar::from_keywords(&[
            ("EXPORT", SingleValue),
            ("DESTINATION", SingleValue),
            ("NAMESPACE", SingleValue),
            ("FILE", SingleValue),
            ("PERMISSIONS", MultiValue),
            ("CONFIGURATIONS", MultiValue),
            ("COMPONENT", SingleValue),
            ("EXPORT_LINK_INTERFACE_LIBRARIES", Flag),
        ]));

        // RUNTIME_DEPENDENCY_SET mode (CMake 3.21+)
        modes.insert("RUNTIME_DEPENDENCY_SET".to_string(), CommandGrammar::from_keywords(&[
            ("RUNTIME_DEPENDENCY_SET", SingleValue),
            ("DESTINATION", SingleValue),
            ("COMPONENT", SingleValue),
            ("NAMELINK_COMPONENT", SingleValue),
            ("OPTIONAL", Flag),
            ("EXCLUDE_FROM_ALL", Flag),
        ]));

        grammars.insert("install".to_string(), Grammar::Modes { modes });
    }

    // add_custom_command
    grammar!("add_custom_command",
        "OUTPUT" => MultiValue,
        "COMMAND" => MultiValue,
        "DEPENDS" => MultiValue,
        "WORKING_DIRECTORY" => SingleValue,
        "COMMENT" => SingleValue,
        "MAIN_DEPENDENCY" => SingleValue,
        "APPEND" => Flag,
        "VERBATIM" => Flag,
        "USES_TERMINAL" => Flag,
        "COMMAND_EXPAND_LISTS" => Flag,
        "DEPFILE" => SingleValue,
        "TARGET" => SingleValue,
        "PRE_BUILD" => Flag,
        "PRE_LINK" => Flag,
        "POST_BUILD" => Flag,
        "BYPRODUCTS" => MultiValue,
        "JOB_POOL" => SingleValue,
    );

    // add_custom_target
    grammar!("add_custom_target",
        "COMMAND" => MultiValue,
        "DEPENDS" => MultiValue,
        "WORKING_DIRECTORY" => SingleValue,
        "COMMENT" => SingleValue,
        "VERBATIM" => Flag,
        "USES_TERMINAL" => Flag,
        "COMMAND_EXPAND_LISTS" => Flag,
        "SOURCES" => MultiValue,
        "JOB_POOL" => SingleValue,
        "ALL" => Flag,
        "BYPRODUCTS" => MultiValue,
    );

    // set_property
    grammar!("set_property",
        "GLOBAL" => Flag,
        "DIRECTORY" => MultiValue,
        "TARGET" => MultiValue,
        "SOURCE" => MultiValue,
        "INSTALL" => MultiValue,
        "TEST" => MultiValue,
        "CACHE" => MultiValue,
        "PROPERTY" => MultiValue,
        "APPEND" => Flag,
        "APPEND_STRING" => Flag,
    );

    // get_property
    grammar!("get_property",
        "GLOBAL" => Flag,
        "DIRECTORY" => SingleValue,
        "TARGET" => SingleValue,
        "SOURCE" => SingleValue,
        "INSTALL" => SingleValue,
        "TEST" => SingleValue,
        "CACHE" => SingleValue,
        "PROPERTY" => SingleValue,
        "VARIABLE" => SingleValue,
        "SET" => Flag,
        "DEFINED" => Flag,
        "BRIEF_DOCS" => Flag,
        "FULL_DOCS" => Flag,
    );

    // set_target_properties
    grammar!("set_target_properties",
        "PROPERTIES" => MultiValue,
    );

    // add_test
    grammar!("add_test",
        "NAME" => SingleValue,
        "COMMAND" => MultiValue,
        "CONFIGURATIONS" => MultiValue,
        "WORKING_DIRECTORY" => SingleValue,
        "COMMAND_EXPAND_LISTS" => Flag,
    );

    // project
    grammar!("project",
        "VERSION" => SingleValue,
        "DESCRIPTION" => SingleValue,
        "HOMEPAGE_URL" => SingleValue,
        "LANGUAGES" => MultiValue,
    );

    // cmake_minimum_required
    grammar!("cmake_minimum_required",
        "VERSION" => SingleValue,
        "FATAL_ERROR" => Flag,
    );

    // export
    grammar!("export",
        "TARGETS" => MultiValue,
        "NAMESPACE" => SingleValue,
        "FILE" => SingleValue,
        "APPEND" => Flag,
        "EXPORT_LINK_INTERFACE_LIBRARIES" => Flag,
        "ANDROID_MK" => SingleValue,
    );

    // Commands with positional-only arguments (empty grammar, but recognized)
    grammars.insert("add_dependencies".to_string(), Grammar::Simple(CommandGrammar::new()));
    grammars.insert("add_compile_definitions".to_string(), Grammar::Simple(CommandGrammar::new()));
    grammars.insert("add_compile_options".to_string(), Grammar::Simple(CommandGrammar::new()));
    grammars.insert("add_link_options".to_string(), Grammar::Simple(CommandGrammar::new()));

    grammars
}
