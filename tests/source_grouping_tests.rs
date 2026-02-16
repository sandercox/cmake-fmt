use cmake_fmt::formatter::{format_text, FormatConfig, SourceGrouping};

#[test]
fn test_source_grouping_disabled_by_default() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\ta.cpp\n\t\ta.h\n\t\tb.cpp\n)";
    let config = FormatConfig::default();
    let output = format_text(input, &config);

    // With default config (None), files should appear one per line
    assert!(output.contains("a.cpp"));
    assert!(output.contains("a.h"));
    assert!(output.contains("b.cpp"));
    // Should NOT be grouped (each on separate line)
    assert!(!output.contains("a.h a.cpp"));
}

#[test]
fn test_source_grouping_headers_first() {
    let input =
        "target_sources(mylib\n\tPUBLIC\n\t\ta.cpp\n\t\ta.h\n\t\tb.cpp\n\t\tb.h\n\t\tc.cpp\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // Headers should be listed first in pairs
    assert!(
        output.contains("a.h a.cpp"),
        "Expected 'a.h a.cpp' in output:\n{}",
        output
    );
    assert!(
        output.contains("b.h b.cpp"),
        "Expected 'b.h b.cpp' in output:\n{}",
        output
    );
    // c.cpp has no matching header, should be alone
    assert!(output.contains("c.cpp"));
    // Should NOT have reverse order
    assert!(!output.contains("a.cpp a.h"));
}

#[test]
fn test_source_grouping_headers_first_on_set() {
    let input = "set(SOURCES\n\ta.cpp\n\ta.h\n\tb.cpp\n\tb.h\n\tc.cpp\n\n\nd.h\nd.cpp\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // Headers should be listed first in pairs
    assert!(
        output.contains("a.h a.cpp"),
        "Expected 'a.h a.cpp' in output:\n{}",
        output
    );
    assert!(
        output.contains("b.h b.cpp"),
        "Expected 'b.h b.cpp' in output:\n{}",
        output
    );

    assert!(
        output.contains("d.h d.cpp"),
        "Expected 'd.h d.cpp' in output:\n{}",
        output
    );
    // c.cpp has no matching header, should be alone
    assert!(output.contains("c.cpp"));
}

#[test]
fn test_source_grouping_sources_first() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\ta.h\n\t\ta.cpp\n\t\tb.h\n\t\tb.cpp\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::SourcesFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // Sources should be listed first in pairs
    assert!(
        output.contains("a.cpp a.h"),
        "Expected 'a.cpp a.h' in output:\n{}",
        output
    );
    assert!(
        output.contains("b.cpp b.h"),
        "Expected 'b.cpp b.h' in output:\n{}",
        output
    );
    // Should NOT have reverse order
    assert!(!output.contains("a.h a.cpp"));
}

#[test]
fn test_source_grouping_hpp_cpp_pairs() {
    let input =
        "target_sources(mylib\n\tPUBLIC\n\t\twidget.cpp\n\t\twidget.hpp\n\t\tutil.c\n\t\tutil.h\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // Both .hpp/.cpp and .h/.c pairs should be grouped
    assert!(
        output.contains("widget.hpp widget.cpp"),
        "Expected 'widget.hpp widget.cpp' in output:\n{}",
        output
    );
    assert!(
        output.contains("util.h util.c"),
        "Expected 'util.h util.c' in output:\n{}",
        output
    );
}

#[test]
fn test_source_grouping_no_match_passthrough() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\tmain.cpp\n\t\thelper.h\n\t\tother.cpp\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // Files without matching pairs pass through unchanged on their own lines
    assert!(output.contains("main.cpp"));
    assert!(output.contains("helper.h"));
    assert!(output.contains("other.cpp"));
    // Should NOT be incorrectly paired
    assert!(!output.contains("main.cpp helper.h"));
    assert!(!output.contains("helper.h other.cpp"));
}

#[test]
fn test_source_grouping_with_paths() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\tsrc/foo.h\n\t\tsrc/foo.cpp\n\t\tinclude/bar.hpp\n\t\tsrc/bar.cpp\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // Files with directory paths should still match by base name
    assert!(
        output.contains("src/foo.h src/foo.cpp"),
        "Expected 'src/foo.h src/foo.cpp' in output:\n{}",
        output
    );
    assert!(
        output.contains("include/bar.hpp src/bar.cpp"),
        "Expected 'include/bar.hpp src/bar.cpp' in output:\n{}",
        output
    );
}

#[test]
fn test_source_grouping_preserves_non_source_files() {
    let input = "add_library(mylib\n\tfile.txt\n\tfile.cmake\n\ta.h\n\ta.cpp\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // Non-source files should NOT be grouped even if they share base names
    assert!(output.contains("file.txt"));
    assert!(output.contains("file.cmake"));
    // Should NOT be grouped
    assert!(!output.contains("file.txt file.cmake"));
    // Source files should still be grouped
    assert!(
        output.contains("a.h a.cpp"),
        "Expected 'a.h a.cpp' in output:\n{}",
        output
    );
}

#[test]
fn test_source_grouping_toml_config() {
    let toml_content = r#"
source_grouping = "headers_first"
max_line_length = 100
"#;
    let config: FormatConfig = toml::from_str(toml_content).expect("Failed to parse TOML config");

    assert_eq!(config.source_grouping, SourceGrouping::HeadersFirst);

    let input = "target_sources(mylib PUBLIC a.cpp a.h)";
    let output = format_text(input, &config);
    assert!(
        output.contains("a.h a.cpp"),
        "Expected 'a.h a.cpp' in output:\n{}",
        output
    );
}

#[test]
fn test_source_grouping_vert_frag_pairs() {
    let input =
        "target_sources(mylib\n\tPUBLIC\n\t\tshader.frag\n\t\tshader.vert\n\t\teffect.frag\n\t\teffect.vert\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // .vert is header, .frag is source — headers_first means vert before frag
    assert!(
        output.contains("shader.vert shader.frag"),
        "Expected 'shader.vert shader.frag' in output:\n{}",
        output
    );
    assert!(
        output.contains("effect.vert effect.frag"),
        "Expected 'effect.vert effect.frag' in output:\n{}",
        output
    );
}

#[test]
fn test_source_grouping_idempotent() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\ta.cpp\n\t\ta.h\n\t\tb.cpp\n\t\tb.h\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };

    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);

    assert_eq!(
        pass1, pass2,
        "Formatting with source grouping is not idempotent"
    );
}

#[test]
fn test_source_grouping_triplet_h_hpp_cpp() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\twidget.cpp\n\t\twidget.h\n\t\twidget.hpp\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // All three should be grouped on one line in extension priority order
    assert!(
        output.contains("widget.h widget.hpp widget.cpp"),
        "Expected 'widget.h widget.hpp widget.cpp' in output:\n{}",
        output
    );
}

#[test]
fn test_source_grouping_quad_h_hpp_cpp_ipp() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\tcore.cpp\n\t\tcore.h\n\t\tcore.hpp\n\t\tcore.ipp\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // All four should be grouped on one line with .ipp before .cpp
    assert!(
        output.contains("core.h core.hpp core.ipp core.cpp"),
        "Expected 'core.h core.hpp core.ipp core.cpp' in output:\n{}",
        output
    );
}

#[test]
fn test_source_grouping_ipp_recognized_as_header() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\ttmpl.cpp\n\t\ttmpl.ipp\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // .ipp is a header extension, should come before .cpp
    assert!(
        output.contains("tmpl.ipp tmpl.cpp"),
        "Expected 'tmpl.ipp tmpl.cpp' in output:\n{}",
        output
    );
}

#[test]
fn test_source_grouping_mixed_triplets_and_singles() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\ta.h\n\t\ta.cpp\n\t\ta.ipp\n\t\tb.cpp\n\t\tc.h\n\t\tc.cpp\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // a-files should be grouped as triplet with correct ordering
    assert!(
        output.contains("a.h a.ipp a.cpp"),
        "Expected 'a.h a.ipp a.cpp' in output:\n{}",
        output
    );
    // b.cpp has no match, should be alone
    assert!(output.contains("b.cpp"));
    assert!(!output.contains("b.cpp a.") && !output.contains("b.cpp c."));
    // c-files should be grouped as pair
    assert!(
        output.contains("c.h c.cpp"),
        "Expected 'c.h c.cpp' in output:\n{}",
        output
    );
}

#[test]
fn test_source_grouping_triplet_idempotent() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\twidget.cpp\n\t\twidget.h\n\t\twidget.hpp\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };

    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);

    assert_eq!(
        pass1, pass2,
        "Formatting triplet with source grouping is not idempotent"
    );
}

#[test]
fn test_source_grouping_triplet_with_sort() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\tz.cpp\n\t\tz.h\n\t\tz.hpp\n\t\ta.cpp\n\t\ta.h\n\t\ta.hpp\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        sort_sources: cmake_fmt::formatter::SortSources::Alphabetical,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // Both should be grouped as triplets
    assert!(
        output.contains("a.h a.hpp a.cpp"),
        "Expected 'a.h a.hpp a.cpp' in output:\n{}",
        output
    );
    assert!(
        output.contains("z.h z.hpp z.cpp"),
        "Expected 'z.h z.hpp z.cpp' in output:\n{}",
        output
    );

    // a-triplet should come before z-triplet (alphabetical sort)
    let a_pos = output.find("a.h a.hpp a.cpp").unwrap();
    let z_pos = output.find("z.h z.hpp z.cpp").unwrap();
    assert!(
        a_pos < z_pos,
        "Expected a-triplet before z-triplet in sorted output"
    );
}
