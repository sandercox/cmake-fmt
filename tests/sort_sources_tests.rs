use cmake_fmt::formatter::{
    CommandGrammarConfig, FormatConfig, SortSources, SourceGrouping, format_text,
};
use std::collections::HashMap;

#[test]
fn test_sort_sources_disabled_by_default() {
    let input = "set(SOURCES z.cpp a.cpp m.cpp)\n";
    let config = FormatConfig::default();
    let result = format_text(input, &config);

    // Order should be preserved (z, a, m)
    assert!(result.contains("z.cpp"));
    assert!(result.find("z.cpp").unwrap() < result.find("a.cpp").unwrap());
    assert!(result.find("a.cpp").unwrap() < result.find("m.cpp").unwrap());
}

#[test]
fn test_sort_sources_alphabetical_basic() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\tz.cpp\n\t\ta.cpp\n\t\tm.cpp\n)\n";
    let config = FormatConfig {
        sort_sources: SortSources::Alphabetical,
        ..Default::default()
    };
    let result = format_text(input, &config);

    // Files should be sorted: a, m, z
    let a_pos = result.find("a.cpp").unwrap();
    let m_pos = result.find("m.cpp").unwrap();
    let z_pos = result.find("z.cpp").unwrap();
    assert!(a_pos < m_pos, "a.cpp should come before m.cpp");
    assert!(m_pos < z_pos, "m.cpp should come before z.cpp");
}

#[test]
fn test_sort_sources_case_insensitive() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\tZoo.cpp\n\t\tapple.cpp\n\t\tBanana.cpp\n)\n";
    let config = FormatConfig {
        sort_sources: SortSources::Alphabetical,
        ..Default::default()
    };
    let result = format_text(input, &config);

    // Files should be sorted case-insensitively: apple, Banana, Zoo
    let apple_pos = result.find("apple.cpp").unwrap();
    let banana_pos = result.find("Banana.cpp").unwrap();
    let zoo_pos = result.find("Zoo.cpp").unwrap();
    assert!(
        apple_pos < banana_pos,
        "apple.cpp should come before Banana.cpp"
    );
    assert!(
        banana_pos < zoo_pos,
        "Banana.cpp should come before Zoo.cpp"
    );
}

#[test]
fn test_sort_sources_blank_line_sections() {
    let input = "set(SOURCES\n\tz.cpp\n\ta.cpp\n\n\tc.cpp\n\tb.cpp\n)\n";
    let config = FormatConfig {
        sort_sources: SortSources::Alphabetical,
        ..Default::default()
    };
    let result = format_text(input, &config);

    // First section: a.cpp, z.cpp
    // Second section: b.cpp, c.cpp
    let lines: Vec<&str> = result.lines().collect();
    let source_lines: Vec<&str> = lines
        .iter()
        .filter(|l| l.contains(".cpp"))
        .copied()
        .collect();

    // Check first section (before blank line)
    assert!(source_lines[0].contains("a.cpp"), "First should be a.cpp");
    assert!(source_lines[1].contains("z.cpp"), "Second should be z.cpp");

    // Check second section (after blank line)
    assert!(source_lines[2].contains("b.cpp"), "Third should be b.cpp");
    assert!(source_lines[3].contains("c.cpp"), "Fourth should be c.cpp");
}

#[test]
fn test_sort_sources_variable_is_a_barrier() {
    let input = "set(MY_VAR ${SOME_VAR} z.cpp a.cpp)\n";
    let config = FormatConfig {
        sort_sources: SortSources::Alphabetical,
        ..Default::default()
    };
    let result = format_text(input, &config);

    // The variable name is pinned and ${SOME_VAR} holds its position, because
    // what it expands to is unknown; the files around it still sort.
    assert_eq!(result, "set(MY_VAR ${SOME_VAR} a.cpp z.cpp)\n");
}

#[test]
fn test_sort_sources_with_keyword_sections() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\tz.cpp\n\t\ta.cpp\n\tPRIVATE\n\t\ty.cpp\n\t\tb.cpp\n)\n";
    let config = FormatConfig {
        sort_sources: SortSources::Alphabetical,
        ..Default::default()
    };
    let result = format_text(input, &config);

    // PUBLIC section: a.cpp, z.cpp
    // PRIVATE section: b.cpp, y.cpp
    let public_pos = result.find("PUBLIC").unwrap();
    let private_pos = result.find("PRIVATE").unwrap();

    let a_pos = result.find("a.cpp").unwrap();
    let z_pos = result.find("z.cpp").unwrap();
    let b_pos = result.find("b.cpp").unwrap();
    let y_pos = result.find("y.cpp").unwrap();

    // Check PUBLIC section
    assert!(public_pos < a_pos);
    assert!(a_pos < z_pos);
    assert!(z_pos < private_pos);

    // Check PRIVATE section
    assert!(private_pos < b_pos);
    assert!(b_pos < y_pos);
}

#[test]
fn test_sort_sources_with_comments() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\t# Widget implementation\n\t\tz_widget.cpp\n\t\ta_main.cpp\n)\n";
    let config = FormatConfig {
        sort_sources: SortSources::Alphabetical,
        ..Default::default()
    };
    let result = format_text(input, &config);

    // a_main.cpp should come first (no comment)
    // "# Widget implementation" should stay before z_widget.cpp
    let a_pos = result.find("a_main.cpp").unwrap();
    let comment_pos = result.find("# Widget implementation").unwrap();
    let z_pos = result.find("z_widget.cpp").unwrap();

    assert!(a_pos < comment_pos, "a_main.cpp should come before comment");
    assert!(
        comment_pos < z_pos,
        "comment should stay before z_widget.cpp"
    );
}

#[test]
fn test_sort_sources_with_source_grouping() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\tz.cpp\n\t\tz.h\n\t\ta.cpp\n\t\ta.h\n)\n";
    let config = FormatConfig {
        sort_sources: SortSources::Alphabetical,
        source_grouping: SourceGrouping::HeadersFirst,
        ..Default::default()
    };
    let result = format_text(input, &config);

    // After sorting: a.cpp, a.h, z.cpp, z.h
    // After grouping (headers first): "a.h a.cpp", "z.h z.cpp"
    assert!(
        result.contains("a.h a.cpp"),
        "Should contain grouped a files"
    );
    assert!(
        result.contains("z.h z.cpp"),
        "Should contain grouped z files"
    );

    let a_pair_pos = result.find("a.h a.cpp").unwrap();
    let z_pair_pos = result.find("z.h z.cpp").unwrap();
    assert!(a_pair_pos < z_pair_pos, "a pair should come before z pair");
}

#[test]
fn test_sort_sources_no_sort_directive() {
    let input = "# cmake-fmt: no-sort\ntarget_sources(mylib\n\tPUBLIC\n\t\tz.cpp\n\t\ta.cpp\n)\ntarget_sources(other\n\tPUBLIC\n\t\tz.cpp\n\t\ta.cpp\n)\n";
    let config = FormatConfig {
        sort_sources: SortSources::Alphabetical,
        ..Default::default()
    };
    let result = format_text(input, &config);

    // First target_sources should keep z, a order (no-sort directive)
    // Second target_sources should sort to a, z
    let lines: Vec<&str> = result.lines().collect();

    // Find the two target_sources sections
    let mut first_section_lines = Vec::new();
    let mut second_section_lines = Vec::new();
    let mut in_first = false;
    let mut in_second = false;

    for line in lines {
        if line.contains("target_sources(mylib") {
            in_first = true;
        } else if line.contains("target_sources(other") {
            in_first = false;
            in_second = true;
        } else if line.contains(")") {
            in_second = false;
        }

        if in_first && line.contains(".cpp") {
            first_section_lines.push(line);
        }
        if in_second && line.contains(".cpp") {
            second_section_lines.push(line);
        }
    }

    // First section: z.cpp before a.cpp (no sorting)
    assert!(
        first_section_lines[0].contains("z.cpp"),
        "First section should have z.cpp first"
    );
    assert!(
        first_section_lines[1].contains("a.cpp"),
        "First section should have a.cpp second"
    );

    // Second section: a.cpp before z.cpp (sorted)
    assert!(
        second_section_lines[0].contains("a.cpp"),
        "Second section should have a.cpp first"
    );
    assert!(
        second_section_lines[1].contains("z.cpp"),
        "Second section should have z.cpp second"
    );
}

#[test]
fn test_sort_sources_add_executable() {
    let input = "add_executable(myapp\n\tz.cpp\n\ta.cpp\n\tm.cpp\n)\n";
    let config = FormatConfig {
        sort_sources: SortSources::Alphabetical,
        ..Default::default()
    };
    let result = format_text(input, &config);

    // Target name myapp stays first, files are sorted: a, m, z
    assert!(result.contains("myapp"));
    let myapp_pos = result.find("myapp").unwrap();
    let a_pos = result.find("a.cpp").unwrap();
    let m_pos = result.find("m.cpp").unwrap();
    let z_pos = result.find("z.cpp").unwrap();

    assert!(myapp_pos < a_pos, "myapp should stay first");
    assert!(a_pos < m_pos, "a.cpp before m.cpp");
    assert!(m_pos < z_pos, "m.cpp before z.cpp");
}

#[test]
fn test_sort_sources_add_library() {
    let input = "add_library(mylib\n\tz.cpp\n\ta.cpp\n)\n";
    let config = FormatConfig {
        sort_sources: SortSources::Alphabetical,
        ..Default::default()
    };
    let result = format_text(input, &config);

    // Library name mylib stays first, files are sorted: a, z
    assert!(result.contains("mylib"));
    let mylib_pos = result.find("mylib").unwrap();
    let a_pos = result.find("a.cpp").unwrap();
    let z_pos = result.find("z.cpp").unwrap();

    assert!(mylib_pos < a_pos, "mylib should stay first");
    assert!(a_pos < z_pos, "a.cpp before z.cpp");
}

#[test]
fn test_sort_sources_idempotent() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\tz.cpp\n\t\ta.cpp\n\t\tm.cpp\n)\n";
    let config = FormatConfig {
        sort_sources: SortSources::Alphabetical,
        ..Default::default()
    };

    let result1 = format_text(input, &config);
    let result2 = format_text(&result1, &config);

    assert_eq!(result1, result2, "Formatting should be idempotent");
}

#[test]
fn test_sort_sources_toml_config() {
    // Test that config parses from TOML
    let toml_content = r#"
sort_sources = "alphabetical"
"#;

    let config: FormatConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(config.sort_sources, SortSources::Alphabetical);
}

#[test]
fn test_sort_sources_with_paths() {
    let input =
        "target_sources(mylib\n\tPUBLIC\n\t\tsrc/z.cpp\n\t\tinclude/a.h\n\t\tsrc/b.cpp\n)\n";
    let config = FormatConfig {
        sort_sources: SortSources::Alphabetical,
        ..Default::default()
    };
    let result = format_text(input, &config);

    // Files with directory prefixes should sort by full path
    let a_pos = result.find("include/a.h").unwrap();
    let b_pos = result.find("src/b.cpp").unwrap();
    let z_pos = result.find("src/z.cpp").unwrap();

    assert!(a_pos < b_pos, "include/a.h before src/b.cpp");
    assert!(b_pos < z_pos, "src/b.cpp before src/z.cpp");
}

#[test]
fn test_sort_sources_paired_lines_sort_as_unit() {
    // When source_grouping is already applied (re-format of paired output),
    // the "foo.h foo.cpp" paired lines should sort as units
    let input = "target_sources(mylib\n\tPUBLIC\n\t\tz.h z.cpp\n\t\ta.h a.cpp\n)\n";
    let config = FormatConfig {
        sort_sources: SortSources::Alphabetical,
        source_grouping: SourceGrouping::HeadersFirst,
        ..Default::default()
    };
    let result = format_text(input, &config);

    // Paired lines should be sorted as units by their first component
    let a_pair = result.find("a.h a.cpp").unwrap();
    let z_pair = result.find("z.h z.cpp").unwrap();

    assert!(
        a_pair < z_pair,
        "a.h a.cpp pair should come before z.h z.cpp pair"
    );
}

// ============================================================================
// ALLOWLIST: reordering only happens where a grammar says a list is unordered
//
// Every test below runs with BOTH reordering passes on, because sort_sources
// and source_grouping are gated by the same allowlist.
// ============================================================================

/// Both reordering passes enabled.
fn reordering_config() -> FormatConfig {
    FormatConfig {
        sort_sources: SortSources::Alphabetical,
        source_grouping: SourceGrouping::HeadersFirst,
        ..Default::default()
    }
}

/// Assert the command is left exactly as written.
fn assert_unchanged(input: &str) {
    let result = format_text(input, &reordering_config());
    assert_eq!(result, input, "arguments were reordered");
}

#[test]
fn test_set_cache_type_and_docstring_hold() {
    // Regression: https://github.com/sandercox/cmake-fmt/issues/3
    // CACHE holds a positional `<type> "<docstring>"` pair.
    assert_unchanged("set(V x CACHE PATH \"docs (etc/xdg)\")\n");
    assert_unchanged("set(V 0 CACHE INTERNAL \"Enables debug (DLOG_F etc).\")\n");
}

#[test]
fn test_command_line_holds() {
    // Regression: https://github.com/sandercox/cmake-fmt/issues/6
    assert_unchanged("add_custom_target(g COMMAND dot -Tpng in.dot -o out.png)\n");
    assert_unchanged("execute_process(COMMAND cp src/a.txt dst/b.txt)\n");
    assert_unchanged("add_test(NAME t COMMAND runner.sh b.txt a.txt)\n");
}

#[test]
fn test_install_directory_pattern_pairs_hold() {
    // FILES_MATCHING holds PATTERN/glob pairs; sorting tore the keywords off
    // their globs and piled them up at the end. Seen in
    // tests/corpus/llvm/CMakeLists.txt. This command wraps, so assert the
    // pairing rather than byte-identity.
    let input = "install(DIRECTORY inc/ DESTINATION inc FILES_MATCHING PATTERN \"*.h\" PATTERN \"*.inc\")\n";
    let result = format_text(input, &reordering_config());

    assert!(
        result.contains("PATTERN \"*.h\"") && result.contains("PATTERN \"*.inc\""),
        "PATTERN lost its glob:\n{}",
        result
    );
    let h = result.find("\"*.h\"").expect("*.h missing");
    let inc = result.find("\"*.inc\"").expect("*.inc missing");
    assert!(h < inc, "PATTERN order changed:\n{}", result);
}

#[test]
fn test_positional_pairs_hold() {
    assert_unchanged("file(RENAME z.txt a.txt)\n");
    assert_unchanged("configure_file(z.h.in a.h)\n");
}

#[test]
fn test_property_lists_hold() {
    assert_unchanged("set_property(TARGET t PROPERTY SOURCES b.cpp a.cpp)\n");
    assert_unchanged("set_target_properties(t PROPERTIES A_PROP z.exe B_PROP a.exe)\n");
}

#[test]
fn test_link_libraries_and_flags_hold() {
    // Static archive link order and compile-flag order are both significant
    assert_unchanged("target_link_libraries(app z/libz.a a/liba.a)\n");
    assert_unchanged("target_compile_options(app PRIVATE -include p.h -Wall)\n");
    assert_unchanged("target_include_directories(app PRIVATE z/inc a/inc)\n");
    assert_unchanged("add_compile_options(/O2 /Oi /Ot /GL)\n");
}

#[test]
fn test_dotted_target_name_stays_first() {
    // The target name used to be sorted into the source list because it looked
    // like a filename. Index 0 of a leading positional run is now always pinned.
    let result = format_text("add_library(zz.lib b.cpp a.cpp)\n", &reordering_config());
    assert_eq!(result, "add_library(zz.lib a.cpp b.cpp)\n");
}

#[test]
fn test_add_library_sorts_after_type_keyword() {
    // A type flag ends the positional run and collects the sources itself
    let config = reordering_config();
    assert_eq!(
        format_text("add_library(lib STATIC z.cpp a.cpp)\n", &config),
        "add_library(lib STATIC a.cpp z.cpp)\n"
    );
    assert_eq!(
        format_text("add_executable(app WIN32 z.cpp a.cpp)\n", &config),
        "add_executable(app WIN32 a.cpp z.cpp)\n"
    );
    // ALIAS/IMPORTED forms carry a target name, not sources
    assert_unchanged("add_library(foo ALIAS bar)\n");
}

#[test]
fn test_unlisted_list_modes_hold() {
    // POP_BACK out-vars are positionally bound to the popped elements, and
    // TRANSFORM REPLACE holds regex then replacement.
    assert_unchanged("list(POP_BACK l z.var a.var)\n");
    assert_unchanged("list(TRANSFORM l REPLACE \"b.x\" \"a.y\")\n");
    assert_unchanged("list(GET l 0 z.out)\n");
}

#[test]
fn test_allowed_list_modes_sort() {
    let config = reordering_config();
    assert_eq!(
        format_text("list(APPEND SRCS z.cpp a.cpp)\n", &config),
        "list(APPEND SRCS a.cpp z.cpp)\n"
    );
    assert_eq!(
        format_text("list(PREPEND SRCS z.cpp a.cpp)\n", &config),
        "list(PREPEND SRCS a.cpp z.cpp)\n"
    );
    assert_eq!(
        format_text("list(REMOVE_ITEM SRCS z.cpp a.cpp)\n", &config),
        "list(REMOVE_ITEM SRCS a.cpp z.cpp)\n"
    );
}

#[test]
fn test_search_path_variables_hold() {
    // Element order in a search path is precedence: first match wins, so
    // sorting silently resolves a different module.
    assert_unchanged("list(APPEND CMAKE_MODULE_PATH cmake/overrides cmake/defaults)\n");
    assert_unchanged("list(PREPEND CMAKE_PREFIX_PATH z/root a/root)\n");
    assert_unchanged("set(MY_INCLUDE_DIRS z/inc a/inc)\n");
    assert_unchanged("set(BASE_WARNING_FLAGS /W4 /permissive-)\n");
}

#[test]
fn test_target_sources_file_set_files_sort_base_dirs_hold() {
    let input =
        "target_sources(t PUBLIC FILE_SET api TYPE HEADERS BASE_DIRS zinc ainc FILES z.h a.h)\n";
    let result = format_text(input, &reordering_config());

    assert!(
        result.contains("BASE_DIRS\n\t\tzinc\n\t\tainc"),
        "BASE_DIRS is a search path and must hold its order:\n{}",
        result
    );
    let a = result.find("a.h").expect("a.h missing");
    let z = result.find("z.h").expect("z.h missing");
    assert!(a < z, "FILE_SET FILES should sort:\n{}", result);
}

#[test]
fn test_install_files_and_source_group_sort() {
    let config = reordering_config();
    assert_eq!(
        format_text("install(FILES z.h a.h DESTINATION inc)\n", &config),
        "install(FILES a.h z.h DESTINATION inc)\n"
    );
    assert_eq!(
        format_text("source_group(grp FILES z.cpp a.cpp)\n", &config),
        "source_group(grp FILES a.cpp z.cpp)\n"
    );
    // Extension-less names sort like anything else inside an allowed list,
    // case-insensitively as everywhere else
    assert_eq!(
        format_text(
            "install(FILES README LICENSE zoo.txt apple.txt DESTINATION s)\n",
            &config
        ),
        "install(FILES apple.txt LICENSE README zoo.txt DESTINATION s)\n"
    );
}

#[test]
fn test_unknown_command_uses_conventional_keyword_names() {
    // cmake_parse_arguments reports arity, not meaning, so only keywords named
    // after a file list are treated as unordered on an auto-detected command.
    let input = concat!(
        "function(my_runner)\n",
        "\tcmake_parse_arguments(ARG \"\" \"NAME\" \"SOURCES;COMMAND\" ${ARGN})\n",
        "endfunction()\n",
        "my_runner(NAME hello SOURCES z.cpp a.cpp COMMAND dot -Tpng in.dot -o out.png)\n"
    );
    let result = format_text(input, &reordering_config());

    assert!(
        result.contains("SOURCES a.cpp z.cpp"),
        "conventional SOURCES keyword should sort:\n{}",
        result
    );
    assert!(
        result.contains("COMMAND dot -Tpng in.dot -o out.png"),
        "auto-detected COMMAND must hold its argv:\n{}",
        result
    );
}

#[test]
fn test_no_sort_directive_also_stops_grouping() {
    // source_grouping reorders too — it hoists a file next to its pair — so the
    // directive has to cover both passes to mean anything.
    let input = "# cmake-fmt: no-sort\nset(SRC z.cpp a.cpp z.h)\nset(SRC2 z.cpp a.cpp z.h)\n";
    let result = format_text(input, &reordering_config());

    assert!(
        result.contains("set(SRC z.cpp a.cpp z.h)"),
        "no-sort must suppress grouping as well:\n{}",
        result
    );
    assert!(
        !result.contains("set(SRC2 z.cpp a.cpp z.h)"),
        "the directive applies to the next command only:\n{}",
        result
    );
}

#[test]
fn test_flag_lists_hold_regardless_of_variable_case() {
    // A positional run has no keyword to vouch for it, so its values must look
    // like source files. GCC processes -W flags in order and the last wins, so
    // reordering `-Wno-unused -Wall` re-enables a warning the author disabled.
    // Seen in tests/corpus/llvm/HandleLLVMOptions.cmake.
    assert_unchanged("set(warning_flags -Wno-unused -Wall)\n");
    assert_unchanged("set(WARNING_FLAGS -Wno-unused -Wall)\n");
    assert_unchanged("set(msvc_warning_flags /wd4141 /wd4100)\n");
    // CMAKE_<LANG>_FLAGS_<CONFIG> ends in the config name, not _FLAGS
    assert_unchanged("set(CMAKE_CXX_FLAGS_RELEASE /O2 /Ob2 /DNDEBUG)\n");
    // pkg-config output has no underscore before FLAGS
    assert_unchanged("list(APPEND GTK_CFLAGS -I/z -DA)\n");
    assert_unchanged("list(APPEND MY_LDFLAGS -Lz -la)\n");
}

#[test]
fn test_argv_and_library_lists_hold() {
    // `list(APPEND ARGS -o out.png)` then `COMMAND tool ${ARGS}` is an argv one
    // level of indirection away, and static archive link order is significant.
    assert_unchanged("set(RUN_ARGS --output z.txt --input a.txt)\n");
    assert_unchanged("list(APPEND MY_ARGUMENTS -o in.dot out.png)\n");
    assert_unchanged("list(APPEND LIBS libz.a liba.a)\n");
    assert_unchanged("set(MY_LIBRARIES z.so a.so)\n");
    // Even without a telling variable name, a linkable extension holds
    assert_unchanged("set(BLOBS libz.a liba.a)\n");
    assert_unchanged("set(OBJECTS z.o a.o)\n");
}

#[test]
fn test_positional_run_requires_file_like_values() {
    // Extension-less names sort where a keyword vouches for them, but not in a
    // positional run, where the same shape could be a flag or a target name.
    assert_unchanged("set(DOCS README LICENSE)\n");
    assert_unchanged("set(DEFS A=1 B=2)\n");
    // A source list still sorts
    let config = reordering_config();
    assert_eq!(
        format_text("set(PROTO_FILES b.proto a.proto)\n", &config),
        "set(PROTO_FILES a.proto b.proto)\n"
    );
}

#[test]
fn test_quoted_variable_reference_is_a_barrier() {
    // A leading quote used to defeat the barrier check, and `"` (0x22) sorts
    // below every letter, so the reference jumped to the front of the list.
    let config = reordering_config();
    assert_eq!(
        format_text("set(S z.cpp a.cpp \"${GEN}\" y.cpp b.cpp)\n", &config),
        "set(S a.cpp z.cpp \"${GEN}\" b.cpp y.cpp)\n"
    );
    assert_eq!(
        format_text("set(S2 z.cpp \"$<TARGET_OBJECTS:x>\" a.cpp)\n", &config),
        "set(S2 z.cpp \"$<TARGET_OBJECTS:x>\" a.cpp)\n"
    );
}

#[test]
fn test_dynamic_governing_variable_holds() {
    // The variable name is unreadable, so it cannot be vetted against the
    // search-path list — the same conservatism applied to values.
    assert_unchanged("list(APPEND ${DYNAMIC_VAR} z.cmake a.cmake)\n");
}

#[test]
fn test_stray_positional_run_after_a_later_keyword_holds() {
    // Only a leading mode keyword that consumed the list variable opens an
    // unordered run. A run after some later single-value keyword is a stray
    // positional argument, not the command's argument list.
    //
    // `list(APPEND SRCS a.cpp SORT z.cpp b.cpp)` also holds, but for a
    // different reason — `SORT` is not a keyword in that mode and has no
    // extension, so the value check rejects the whole run — and deleting this
    // guard left it green. Reaching the guard needs a command whose grammar
    // declares the later keyword, so the run is genuinely a positional overflow
    // of a sortable command.
    assert_unchanged("list(APPEND SRCS a.cpp SORT z.cpp b.cpp)\n");

    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "my_set".to_string(),
        CommandGrammarConfig {
            one_value_keywords: vec!["UNUSED".to_string()],
            sortable_positional: true,
            ..Default::default()
        },
    );
    let config = FormatConfig {
        command_grammars,
        ..reordering_config()
    };

    assert_eq!(
        format_text("my_set(VAR z.cpp a.cpp UNUSED x q.cpp b.cpp)\n", &config),
        "my_set(VAR a.cpp z.cpp UNUSED x q.cpp b.cpp)\n",
        "the leading run should sort and the overflow run should hold"
    );
}

#[test]
fn test_dynamic_target_name_does_not_block_sorting() {
    // A dynamic *target* name says nothing about whether its sources are
    // ordered — only a dynamic *list variable* does. `add_library(${PROJECT_NAME}
    // ...)` is about as idiomatic as CMake gets.
    let config = reordering_config();
    for input in [
        "add_library(${PROJECT_NAME} z.cpp a.cpp)\n",
        "add_executable(${TARGET_NAME} z.cpp a.cpp)\n",
    ] {
        let result = format_text(input, &config);
        assert!(
            result.contains("a.cpp z.cpp"),
            "dynamic target name blocked sorting:\n{}",
            result
        );
    }

    // A readable suffix is still vetted, dynamic prefix or not
    assert_eq!(
        format_text("set(${PROJECT_NAME}_SOURCES z.cpp a.cpp)\n", &config),
        "set(${PROJECT_NAME}_SOURCES a.cpp z.cpp)\n"
    );
    assert_unchanged("set(${PROJECT_NAME}_FLAGS -Wall -Wno-unused)\n");
}

#[test]
fn test_quoted_flags_are_not_file_like() {
    // The flag test ran on the raw argument and the extension test on the
    // unquoted one, so a quoted flag containing a dot passed both.
    assert_unchanged("set(a \"z.cpp\" \"-I/usr/inc/a.h\")\n");
    assert_unchanged("set(b \"z.cpp\" \"-include a.h\")\n");
    assert_unchanged("set(c z.cpp \"/DWIN32.x\")\n");
}

#[test]
fn test_version_lists_hold() {
    // `3.9` reads as extension "9"; version lists are precedence lists, and
    // Python_ADDITIONAL_VERSIONS is a documented first-found-wins hint.
    assert_unchanged("set(PYTHON_VERSIONS 3.9 3.12 3.11)\n");
    assert_unchanged("set(SUPPORTED 1.10 1.9)\n");
}

/// Assert the command is left exactly as written by `source_grouping` alone.
///
/// `source_grouping` pairs by base name and hoists the header to its pair's
/// index, so it reorders whether or not `sort_sources` is on. Every
/// `assert_unchanged` case above uses distinct base names, which is precisely
/// what grouping needs to do nothing — so none of them notices if the guard on
/// the grouping pass disappears.
fn assert_grouping_leaves_alone(input: &str) {
    let config = FormatConfig {
        sort_sources: SortSources::None,
        source_grouping: SourceGrouping::HeadersFirst,
        ..Default::default()
    };
    assert_eq!(
        format_text(input, &config),
        input,
        "source_grouping reordered a list it does not own"
    );
    // And with both passes on, for good measure
    assert_unchanged(input);
}

#[test]
fn test_grouping_does_not_pair_across_an_unowned_list() {
    // Each of these holds a colliding base name, so the grouping pass has
    // something to do — and doing it swaps a source with its destination, a
    // link order, or a compiler flag with its argument.
    assert_grouping_leaves_alone("file(RENAME z.cpp z.h)\n");
    assert_grouping_leaves_alone("configure_file(cfg.cpp cfg.h)\n");
    assert_grouping_leaves_alone("target_link_libraries(app q.cpp q.h)\n");
    assert_grouping_leaves_alone("add_custom_command(OUTPUT w.cpp w.h COMMAND touch w.cpp)\n");
    assert_grouping_leaves_alone(
        "target_compile_options(app PRIVATE -include p.cpp -include p.h)\n",
    );
}

#[test]
fn test_grouping_respects_the_barriers_sorting_respects() {
    // A variable reference holds its index and nothing moves across it — the
    // README promises this, and `sort_source_args` implements it. The grouping
    // pass did not: it hoisted `b.h` past two expansions whose contents nobody
    // can read.
    let config = FormatConfig {
        sort_sources: SortSources::None,
        source_grouping: SourceGrouping::HeadersFirst,
        ..Default::default()
    };
    let input = "set(SRCS b.cpp ${GENERATED} ${OTHER} b.h)\n";
    assert_eq!(
        format_text(input, &config),
        input,
        "grouping crossed a barrier"
    );
    assert_unchanged(input);

    // Within one run it still groups
    assert_eq!(
        format_text("set(SRCS b.cpp b.h ${GENERATED})\n", &config),
        "set(SRCS b.h b.cpp ${GENERATED})\n"
    );
}

#[test]
fn test_grouping_pins_the_target_name() {
    // Index 0 of an add_library is the target, not part of the list, so a
    // header must not pair with it and hoist it out of first place.
    let config = FormatConfig {
        sort_sources: SortSources::None,
        source_grouping: SourceGrouping::HeadersFirst,
        ..Default::default()
    };
    let input = "add_library(foo.cpp bar.cpp foo.h)\n";
    assert_eq!(format_text(input, &config), input, "the target name moved");
}

#[test]
fn test_a_target_name_is_not_read_as_a_list_variable() {
    // The search-path/flag blocklist is about the *variable* that holds the
    // list. Applying it to an add_library target name silently switched sorting
    // off for ordinary targets whose names happen to end in `_LIBS`, `_DIRS` or
    // `_OPTIONS`, or to contain `FLAGS`.
    let config = reordering_config();
    for name in [
        "my_libs",
        "plugin_dirs",
        "render_options",
        "cxx_flags_helper",
    ] {
        let input = format!("add_library({} z.cpp a.cpp)\n", name);
        assert_eq!(
            format_text(&input, &config),
            format!("add_library({} a.cpp z.cpp)\n", name),
            "sorting was disabled by the target name {}",
            name
        );
    }
}

#[test]
fn test_the_search_path_blocklist_holds_file_like_values() {
    // Every earlier case for this blocklist was over-determined: the values were
    // extension-less or flag-shaped, so the value check rejected them anyway and
    // deleting the whole blocklist left the suite green. These values are
    // file-like, so only the variable's name can hold them.
    let config = reordering_config();
    for input in [
        "list(APPEND CMAKE_MODULE_PATH cmake/z.cmake cmake/a.cmake)\n",
        "list(APPEND CMAKE_PREFIX_PATH z.cmake a.cmake)\n",
        "set(MY_INCLUDE_DIRS z.cmake a.cmake)\n",
        "set(BUILD_DIRS z.cmake a.cmake)\n",
        "list(APPEND GTK_CFLAGS z.cmake a.cmake)\n",
        "set(MY_PATTERNS z.txt a.txt)\n",
    ] {
        assert_eq!(
            format_text(input, &config),
            input,
            "a search-path or flag list was sorted"
        );
    }

    // The same values under an ordinary name do sort, so the inputs above are
    // held by the name and nothing else
    assert_eq!(
        format_text("list(APPEND MY_SOURCES z.cmake a.cmake)\n", &config),
        "list(APPEND MY_SOURCES a.cmake z.cmake)\n"
    );
}

#[test]
fn test_a_config_grammar_keeps_the_conventional_file_list_default() {
    // A config entry replaces the grammar auto-detected from
    // `cmake_parse_arguments` wholesale. Without a default here, a user who
    // declared one to fix wrapping silently lost the sorting they already had,
    // with no diagnostic — and the README's "keywords named SOURCES, SRCS or
    // FILES on your own commands" was true only for auto-detection.
    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "my_runner".to_string(),
        CommandGrammarConfig {
            one_value_keywords: vec!["NAME".to_string()],
            multi_value_keywords: vec!["SOURCES".to_string(), "DEPENDS".to_string()],
            ..Default::default()
        },
    );
    let config = FormatConfig {
        command_grammars,
        ..reordering_config()
    };

    assert_eq!(
        format_text("my_runner(NAME hello SOURCES z.cpp a.cpp)\n", &config),
        "my_runner(NAME hello SOURCES a.cpp z.cpp)\n",
        "a conventionally named file list should still sort"
    );
    // And only the conventional names: DEPENDS is order-significant
    assert_eq!(
        format_text("my_runner(NAME hello DEPENDS z.cpp a.cpp)\n", &config),
        "my_runner(NAME hello DEPENDS z.cpp a.cpp)\n",
        "a keyword nobody marked sortable was reordered"
    );
}

#[test]
fn test_a_config_grammar_can_say_what_is_not_sortable() {
    // The conventional file-list names are a default, not an override. The
    // config docs, the JSON schema and `--help-grammar` all promise that
    // reordering is opt-in and a keyword left out of `sortable_keywords` is left
    // alone; applying the default on top of an explicit list broke that promise
    // with no way to express "not this one".
    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "my_concat".to_string(),
        CommandGrammarConfig {
            one_value_keywords: vec!["OUT".to_string()],
            multi_value_keywords: vec!["FILES".to_string(), "PARTS".to_string()],
            sortable_keywords: vec!["PARTS".to_string()],
            ..Default::default()
        },
    );
    let config = FormatConfig {
        command_grammars,
        ..reordering_config()
    };

    assert_eq!(
        format_text(
            "my_concat(OUT bundle.js FILES prelude.js main.js PARTS z.txt a.txt)\n",
            &config
        ),
        "my_concat(OUT bundle.js FILES prelude.js main.js PARTS a.txt z.txt)\n",
        "an explicit sortable_keywords list should be the whole list"
    );

    // And the default comes from the multi-value keywords: a `FILES` declared as
    // a flag takes no values, so marking it sortable only reorders whatever
    // positional arguments follow it
    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "my_flagcmd".to_string(),
        CommandGrammarConfig {
            options: vec!["FILES".to_string()],
            ..Default::default()
        },
    );
    let config = FormatConfig {
        command_grammars,
        ..reordering_config()
    };
    assert_eq!(
        format_text("my_flagcmd(FILES z.txt a.txt)\n", &config),
        "my_flagcmd(FILES z.txt a.txt)\n",
        "a flag's trailing positionals are not its values"
    );
}

#[test]
fn test_grouping_reaches_a_run_after_a_flag() {
    // `add_library(l STATIC a.cpp a.h)` keeps its sources in a section led by a
    // Flag keyword. `sort_sources` reorders that run, so `source_grouping` has
    // to as well — the two share one allowlist, which is the whole point of it.
    let grouping_only = FormatConfig {
        sort_sources: SortSources::None,
        source_grouping: SourceGrouping::HeadersFirst,
        ..Default::default()
    };
    for (input, expected) in [
        (
            "add_library(l1 STATIC a.cpp a.h b.cpp)\n",
            "add_library(l1 STATIC a.h a.cpp b.cpp)\n",
        ),
        (
            "add_executable(e1 WIN32 a.cpp a.h)\n",
            "add_executable(e1 WIN32 a.h a.cpp)\n",
        ),
        // and the form with no flag at all, which always worked
        ("add_library(l2 a.cpp a.h)\n", "add_library(l2 a.h a.cpp)\n"),
    ] {
        assert_eq!(format_text(input, &grouping_only), expected);
    }
}

#[test]
fn test_grouping_holds_a_run_that_is_not_the_commands_list() {
    // Three of the five places that decided whether to group had no test, and
    // each one reachable only through a different rendering path. A pre-keyword
    // positional run in a command whose list is not sortable, and a keyword
    // section in the single-keyword inline path, are the two the suite missed.
    let grouping_only = FormatConfig {
        sort_sources: SortSources::None,
        source_grouping: SourceGrouping::HeadersFirst,
        ..Default::default()
    };
    for input in [
        // pre-keyword run, keyword after it
        "add_custom_command(w.cpp w.h OUTPUT out.txt)\n",
        // single keyword section, inline path
        "target_link_libraries(app PRIVATE q.cpp q.h)\n",
    ] {
        assert_eq!(
            format_text(input, &grouping_only),
            input,
            "grouping reordered a list it does not own"
        );
    }
}

#[test]
fn test_grouping_keeps_a_comment_with_its_argument_across_a_barrier() {
    // Grouping runs per sortable run, and each run's index remap has to be
    // offset by where that run starts. Without the offset a comment attached to
    // an argument in a later run was remapped to the head of the list.
    let grouping_only = FormatConfig {
        sort_sources: SortSources::None,
        source_grouping: SourceGrouping::HeadersFirst,
        ..Default::default()
    };
    let result = format_text(
        "set(SRCS\n\ta.cpp\n\ta.h\n\t${GEN}\n\t# comment for b\n\tb.cpp\n\tb.h\n)\n",
        &grouping_only,
    );

    let comment_line = result
        .lines()
        .position(|l| l.contains("# comment for b"))
        .expect("the comment survived");
    let b_line = result
        .lines()
        .position(|l| l.contains("b.h"))
        .expect("b.h is present");
    assert_eq!(
        comment_line + 1,
        b_line,
        "the comment left its argument:\n{}",
        result
    );
    assert!(
        comment_line > 1,
        "the comment was hoisted to the head of the list:\n{}",
        result
    );
}
