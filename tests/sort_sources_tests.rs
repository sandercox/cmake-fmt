use cmake_fmt::formatter::{FormatConfig, SortSources, SourceGrouping, format_text};

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
    let mut config = FormatConfig::default();
    config.sort_sources = SortSources::Alphabetical;
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
    let mut config = FormatConfig::default();
    config.sort_sources = SortSources::Alphabetical;
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
    let mut config = FormatConfig::default();
    config.sort_sources = SortSources::Alphabetical;
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
fn test_sort_sources_preserves_non_filenames() {
    let input = "set(MY_VAR ${SOME_VAR} z.cpp a.cpp)\n";
    let mut config = FormatConfig::default();
    config.sort_sources = SortSources::Alphabetical;
    let result = format_text(input, &config);

    // MY_VAR and ${SOME_VAR} should stay first (pre-keyword section not all filenames)
    // This section should NOT be sorted because it contains non-filenames
    assert!(result.contains("MY_VAR"));
    assert!(result.find("MY_VAR").unwrap() < result.find("z.cpp").unwrap());
    assert!(result.find("z.cpp").unwrap() < result.find("a.cpp").unwrap());
}

#[test]
fn test_sort_sources_with_keyword_sections() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\tz.cpp\n\t\ta.cpp\n\tPRIVATE\n\t\ty.cpp\n\t\tb.cpp\n)\n";
    let mut config = FormatConfig::default();
    config.sort_sources = SortSources::Alphabetical;
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
    let mut config = FormatConfig::default();
    config.sort_sources = SortSources::Alphabetical;
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
    let mut config = FormatConfig::default();
    config.sort_sources = SortSources::Alphabetical;
    config.source_grouping = SourceGrouping::HeadersFirst;
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
    let mut config = FormatConfig::default();
    config.sort_sources = SortSources::Alphabetical;
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
    let mut config = FormatConfig::default();
    config.sort_sources = SortSources::Alphabetical;
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
    let mut config = FormatConfig::default();
    config.sort_sources = SortSources::Alphabetical;
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
    let mut config = FormatConfig::default();
    config.sort_sources = SortSources::Alphabetical;

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
    let mut config = FormatConfig::default();
    config.sort_sources = SortSources::Alphabetical;
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
    let mut config = FormatConfig::default();
    config.sort_sources = SortSources::Alphabetical;
    config.source_grouping = SourceGrouping::HeadersFirst;
    let result = format_text(input, &config);

    // Paired lines should be sorted as units by their first component
    let a_pair = result.find("a.h a.cpp").unwrap();
    let z_pair = result.find("z.h z.cpp").unwrap();

    assert!(
        a_pair < z_pair,
        "a.h a.cpp pair should come before z.h z.cpp pair"
    );
}
