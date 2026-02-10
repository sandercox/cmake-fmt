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

    // file - multi-mode command (Phase 14)
    {
        let mut modes = HashMap::new();
        let empty = CommandGrammar::new();

        // Reading modes
        modes.insert("READ".to_string(), CommandGrammar::from_keywords(&[
            ("READ", SingleValue),
            ("OFFSET", SingleValue),
            ("LIMIT", SingleValue),
            ("HEX", Flag),
        ]));

        modes.insert("STRINGS".to_string(), CommandGrammar::from_keywords(&[
            ("STRINGS", SingleValue),
            ("LENGTH_MAXIMUM", SingleValue),
            ("LENGTH_MINIMUM", SingleValue),
            ("LIMIT_COUNT", SingleValue),
            ("LIMIT_INPUT", SingleValue),
            ("LIMIT_OUTPUT", SingleValue),
            ("NEWLINE_CONSUME", Flag),
            ("NO_HEX_CONVERSION", Flag),
            ("REGEX", SingleValue),
            ("ENCODING", SingleValue),
        ]));

        // Hash modes (empty grammar - mode keyword + positional args only)
        for hash in &["MD5", "SHA1", "SHA224", "SHA256", "SHA384", "SHA512", "SHA3_224", "SHA3_256", "SHA3_384", "SHA3_512"] {
            modes.insert(hash.to_string(), empty.clone());
        }

        // Writing modes
        modes.insert("WRITE".to_string(), CommandGrammar::from_keywords(&[
            ("WRITE", SingleValue),
            ("NEWLINE_STYLE", SingleValue),
            ("NO_SOURCE_PERMISSIONS", Flag),
            ("FILE_PERMISSIONS", MultiValue),
        ]));

        modes.insert("APPEND".to_string(), CommandGrammar::from_keywords(&[
            ("APPEND", SingleValue),
            ("NEWLINE_STYLE", SingleValue),
            ("NO_SOURCE_PERMISSIONS", Flag),
            ("FILE_PERMISSIONS", MultiValue),
        ]));

        modes.insert("TOUCH".to_string(), empty.clone());
        modes.insert("TOUCH_NOCREATE".to_string(), empty.clone());

        // Filesystem modes
        modes.insert("GLOB".to_string(), CommandGrammar::from_keywords(&[
            ("GLOB", SingleValue),
            ("LIST_DIRECTORIES", SingleValue),
            ("RELATIVE", SingleValue),
            ("CONFIGURE_DEPENDS", Flag),
        ]));

        modes.insert("GLOB_RECURSE".to_string(), CommandGrammar::from_keywords(&[
            ("GLOB_RECURSE", SingleValue),
            ("LIST_DIRECTORIES", SingleValue),
            ("RELATIVE", SingleValue),
            ("FOLLOW_SYMLINKS", Flag),
            ("CONFIGURE_DEPENDS", Flag),
        ]));

        modes.insert("RENAME".to_string(), empty.clone());
        modes.insert("REMOVE".to_string(), empty.clone());
        modes.insert("REMOVE_RECURSE".to_string(), empty.clone());
        modes.insert("MAKE_DIRECTORY".to_string(), empty.clone());
        modes.insert("RELATIVE_PATH".to_string(), empty.clone());
        modes.insert("TO_CMAKE_PATH".to_string(), empty.clone());
        modes.insert("TO_NATIVE_PATH".to_string(), empty.clone());

        modes.insert("REAL_PATH".to_string(), CommandGrammar::from_keywords(&[
            ("REAL_PATH", SingleValue),
            ("BASE_DIRECTORY", SingleValue),
            ("EXPAND_TILDE", Flag),
        ]));

        // Transfer modes
        modes.insert("DOWNLOAD".to_string(), CommandGrammar::from_keywords(&[
            ("DOWNLOAD", SingleValue),
            ("INACTIVITY_TIMEOUT", SingleValue),
            ("LOG", SingleValue),
            ("STATUS", SingleValue),
            ("TIMEOUT", SingleValue),
            ("USERPWD", SingleValue),
            ("HTTPHEADER", SingleValue),
            ("NETRC", SingleValue),
            ("NETRC_FILE", SingleValue),
            ("EXPECTED_HASH", SingleValue),
            ("EXPECTED_MD5", SingleValue),
            ("TLS_VERIFY", SingleValue),
            ("TLS_CAINFO", SingleValue),
            ("SHOW_PROGRESS", Flag),
            ("RANGE_START", SingleValue),
            ("RANGE_END", SingleValue),
        ]));

        modes.insert("UPLOAD".to_string(), CommandGrammar::from_keywords(&[
            ("UPLOAD", SingleValue),
            ("INACTIVITY_TIMEOUT", SingleValue),
            ("LOG", SingleValue),
            ("STATUS", SingleValue),
            ("TIMEOUT", SingleValue),
            ("USERPWD", SingleValue),
            ("HTTPHEADER", SingleValue),
            ("NETRC", SingleValue),
            ("NETRC_FILE", SingleValue),
            ("TLS_VERIFY", SingleValue),
            ("TLS_CAINFO", SingleValue),
            ("SHOW_PROGRESS", Flag),
        ]));

        // Locking mode
        modes.insert("LOCK".to_string(), CommandGrammar::from_keywords(&[
            ("LOCK", SingleValue),
            ("DIRECTORY", Flag),
            ("RELEASE", Flag),
            ("GUARD", SingleValue),
            ("RESULT_VARIABLE", SingleValue),
            ("TIMEOUT", SingleValue),
        ]));

        // Path modes
        modes.insert("COPY".to_string(), CommandGrammar::from_keywords(&[
            ("COPY", SingleValue),
            ("DESTINATION", SingleValue),
            ("PATTERN", SingleValue),
            ("REGEX", SingleValue),
            ("EXCLUDE", Flag),
            ("FILES_MATCHING", Flag),
            ("PERMISSIONS", MultiValue),
            ("FILE_PERMISSIONS", MultiValue),
            ("DIRECTORY_PERMISSIONS", MultiValue),
            ("NO_SOURCE_PERMISSIONS", Flag),
            ("USE_SOURCE_PERMISSIONS", Flag),
            ("FOLLOW_SYMLINK_CHAIN", Flag),
        ]));

        modes.insert("INSTALL".to_string(), CommandGrammar::from_keywords(&[
            ("INSTALL", SingleValue),
            ("DESTINATION", SingleValue),
            ("PATTERN", SingleValue),
            ("REGEX", SingleValue),
            ("EXCLUDE", Flag),
            ("FILES_MATCHING", Flag),
            ("PERMISSIONS", MultiValue),
            ("FILE_PERMISSIONS", MultiValue),
            ("DIRECTORY_PERMISSIONS", MultiValue),
            ("NO_SOURCE_PERMISSIONS", Flag),
            ("USE_SOURCE_PERMISSIONS", Flag),
            ("FOLLOW_SYMLINK_CHAIN", Flag),
        ]));

        // Archive modes
        modes.insert("ARCHIVE_CREATE".to_string(), CommandGrammar::from_keywords(&[
            ("ARCHIVE_CREATE", Flag),
            ("DESTINATION", SingleValue),
            ("PATHS", MultiValue),
            ("FORMAT", SingleValue),
            ("COMPRESSION", SingleValue),
            ("COMPRESSION_LEVEL", SingleValue),
            ("MTIME", SingleValue),
            ("VERBOSE", Flag),
        ]));

        modes.insert("ARCHIVE_EXTRACT".to_string(), CommandGrammar::from_keywords(&[
            ("ARCHIVE_EXTRACT", Flag),
            ("INPUT", SingleValue),
            ("DESTINATION", SingleValue),
            ("PATTERNS", MultiValue),
            ("LIST_ONLY", Flag),
            ("VERBOSE", Flag),
            ("TOUCH", Flag),
        ]));

        // Misc modes
        modes.insert("SIZE".to_string(), CommandGrammar::from_keywords(&[
            ("SIZE", SingleValue),
        ]));

        modes.insert("READ_SYMLINK".to_string(), empty.clone());

        modes.insert("CREATE_LINK".to_string(), CommandGrammar::from_keywords(&[
            ("CREATE_LINK", SingleValue),
            ("RESULT", SingleValue),
            ("COPY_ON_ERROR", Flag),
            ("SYMBOLIC", Flag),
        ]));

        modes.insert("CHMOD".to_string(), CommandGrammar::from_keywords(&[
            ("CHMOD", MultiValue),
            ("PERMISSIONS", MultiValue),
            ("FILE_PERMISSIONS", MultiValue),
            ("DIRECTORY_PERMISSIONS", MultiValue),
        ]));

        modes.insert("CHMOD_RECURSE".to_string(), CommandGrammar::from_keywords(&[
            ("CHMOD_RECURSE", MultiValue),
            ("PERMISSIONS", MultiValue),
            ("FILE_PERMISSIONS", MultiValue),
            ("DIRECTORY_PERMISSIONS", MultiValue),
        ]));

        modes.insert("GET_RUNTIME_DEPENDENCIES".to_string(), CommandGrammar::from_keywords(&[
            ("GET_RUNTIME_DEPENDENCIES", Flag),
            ("RESOLVED_DEPENDENCIES_VAR", SingleValue),
            ("UNRESOLVED_DEPENDENCIES_VAR", SingleValue),
            ("CONFLICTING_DEPENDENCIES_PREFIX", SingleValue),
            ("EXECUTABLES", MultiValue),
            ("LIBRARIES", MultiValue),
            ("DIRECTORIES", MultiValue),
            ("BUNDLE_EXECUTABLE", SingleValue),
            ("MODULES", MultiValue),
            ("PRE_INCLUDE_REGEXES", MultiValue),
            ("PRE_EXCLUDE_REGEXES", MultiValue),
            ("POST_INCLUDE_REGEXES", MultiValue),
            ("POST_EXCLUDE_REGEXES", MultiValue),
            ("POST_INCLUDE_FILES", MultiValue),
            ("POST_EXCLUDE_FILES", MultiValue),
        ]));

        modes.insert("CONFIGURE".to_string(), CommandGrammar::from_keywords(&[
            ("CONFIGURE", Flag),
            ("OUTPUT", SingleValue),
            ("CONTENT", SingleValue),
            ("NEWLINE_STYLE", SingleValue),
            ("NO_SOURCE_PERMISSIONS", Flag),
            ("FILE_PERMISSIONS", MultiValue),
            ("ESCAPE_QUOTES", Flag),
        ]));

        grammars.insert("file".to_string(), Grammar::Modes { modes });
    }

    // string - multi-mode command (Phase 14)
    {
        let mut modes = HashMap::new();
        let empty = CommandGrammar::new();

        // Most string modes have no section keywords (purely positional)
        modes.insert("FIND".to_string(), CommandGrammar::from_keywords(&[
            ("FIND", Flag),
            ("REVERSE", Flag),
        ]));

        modes.insert("REPLACE".to_string(), empty.clone());
        modes.insert("REGEX".to_string(), empty.clone());
        modes.insert("APPEND".to_string(), empty.clone());
        modes.insert("PREPEND".to_string(), empty.clone());
        modes.insert("CONCAT".to_string(), empty.clone());
        modes.insert("JOIN".to_string(), empty.clone());
        modes.insert("TOLOWER".to_string(), empty.clone());
        modes.insert("TOUPPER".to_string(), empty.clone());
        modes.insert("LENGTH".to_string(), empty.clone());
        modes.insert("SUBSTRING".to_string(), empty.clone());
        modes.insert("STRIP".to_string(), empty.clone());
        modes.insert("GENEX_STRIP".to_string(), empty.clone());
        modes.insert("REPEAT".to_string(), empty.clone());
        modes.insert("COMPARE".to_string(), empty.clone());
        modes.insert("ASCII".to_string(), empty.clone());
        modes.insert("HEX".to_string(), empty.clone());
        modes.insert("MAKE_C_IDENTIFIER".to_string(), empty.clone());

        modes.insert("CONFIGURE".to_string(), CommandGrammar::from_keywords(&[
            ("CONFIGURE", Flag),
            ("ESCAPE_QUOTES", Flag),
            ("@ONLY", Flag),
        ]));

        modes.insert("RANDOM".to_string(), CommandGrammar::from_keywords(&[
            ("RANDOM", Flag),
            ("LENGTH", SingleValue),
            ("ALPHABET", SingleValue),
            ("RANDOM_SEED", SingleValue),
        ]));

        modes.insert("TIMESTAMP".to_string(), CommandGrammar::from_keywords(&[
            ("TIMESTAMP", Flag),
            ("UTC", Flag),
        ]));

        modes.insert("UUID".to_string(), CommandGrammar::from_keywords(&[
            ("UUID", Flag),
            ("NAMESPACE", SingleValue),
            ("NAME", SingleValue),
            ("TYPE", SingleValue),
            ("UPPER", Flag),
        ]));

        modes.insert("JSON".to_string(), CommandGrammar::from_keywords(&[
            ("JSON", Flag),
            ("ERROR_VARIABLE", SingleValue),
            ("MEMBER", SingleValue),
            ("GET", Flag),
            ("TYPE", Flag),
            ("LENGTH", Flag),
            ("REMOVE", Flag),
            ("SET", Flag),
            ("EQUAL", Flag),
        ]));

        grammars.insert("string".to_string(), Grammar::Modes { modes });
    }

    // list - multi-mode command (Phase 14)
    {
        let mut modes = HashMap::new();
        let empty = CommandGrammar::new();

        // Most list modes have no section keywords
        modes.insert("LENGTH".to_string(), empty.clone());
        modes.insert("GET".to_string(), empty.clone());
        modes.insert("APPEND".to_string(), empty.clone());
        modes.insert("PREPEND".to_string(), empty.clone());
        modes.insert("INSERT".to_string(), empty.clone());
        modes.insert("REMOVE_ITEM".to_string(), empty.clone());
        modes.insert("REMOVE_AT".to_string(), empty.clone());
        modes.insert("REMOVE_DUPLICATES".to_string(), empty.clone());
        modes.insert("REVERSE".to_string(), empty.clone());
        modes.insert("POP_BACK".to_string(), empty.clone());
        modes.insert("POP_FRONT".to_string(), empty.clone());
        modes.insert("JOIN".to_string(), empty.clone());
        modes.insert("SUBLIST".to_string(), empty.clone());

        modes.insert("SORT".to_string(), CommandGrammar::from_keywords(&[
            ("SORT", Flag),
            ("COMPARE", SingleValue),
            ("CASE", SingleValue),
            ("ORDER", SingleValue),
        ]));

        modes.insert("FILTER".to_string(), CommandGrammar::from_keywords(&[
            ("FILTER", Flag),
            ("INCLUDE", Flag),
            ("EXCLUDE", Flag),
            ("REGEX", SingleValue),
        ]));

        modes.insert("TRANSFORM".to_string(), CommandGrammar::from_keywords(&[
            ("TRANSFORM", Flag),
            ("OUTPUT_VARIABLE", SingleValue),
            ("FOR", SingleValue),
            ("REGEX", SingleValue),
            ("AT", MultiValue),
        ]));

        grammars.insert("list".to_string(), Grammar::Modes { modes });
    }

    // Commands with positional-only arguments (empty grammar, but recognized)
    grammars.insert("add_dependencies".to_string(), Grammar::Simple(CommandGrammar::new()));
    grammars.insert("add_compile_definitions".to_string(), Grammar::Simple(CommandGrammar::new()));
    grammars.insert("add_compile_options".to_string(), Grammar::Simple(CommandGrammar::new()));
    grammars.insert("add_link_options".to_string(), Grammar::Simple(CommandGrammar::new()));

    grammars
}
