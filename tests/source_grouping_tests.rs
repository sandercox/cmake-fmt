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
    let input = "target_sources(mylib\n\tPUBLIC\n\t\ta.cpp\n\t\ta.h\n\t\tb.cpp\n\t\tb.h\n\t\tc.cpp\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // Headers should be listed first in pairs
    assert!(output.contains("a.h a.cpp"), "Expected 'a.h a.cpp' in output:\n{}", output);
    assert!(output.contains("b.h b.cpp"), "Expected 'b.h b.cpp' in output:\n{}", output);
    // c.cpp has no matching header, should be alone
    assert!(output.contains("c.cpp"));
    // Should NOT have reverse order
    assert!(!output.contains("a.cpp a.h"));
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
    assert!(output.contains("a.cpp a.h"), "Expected 'a.cpp a.h' in output:\n{}", output);
    assert!(output.contains("b.cpp b.h"), "Expected 'b.cpp b.h' in output:\n{}", output);
    // Should NOT have reverse order
    assert!(!output.contains("a.h a.cpp"));
}

#[test]
fn test_source_grouping_hpp_cpp_pairs() {
    let input = "target_sources(mylib\n\tPUBLIC\n\t\twidget.cpp\n\t\twidget.hpp\n\t\tutil.c\n\t\tutil.h\n)";
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // Both .hpp/.cpp and .h/.c pairs should be grouped
    assert!(output.contains("widget.hpp widget.cpp"), "Expected 'widget.hpp widget.cpp' in output:\n{}", output);
    assert!(output.contains("util.h util.c"), "Expected 'util.h util.c' in output:\n{}", output);
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
    assert!(output.contains("src/foo.h src/foo.cpp"), "Expected 'src/foo.h src/foo.cpp' in output:\n{}", output);
    assert!(output.contains("include/bar.hpp src/bar.cpp"), "Expected 'include/bar.hpp src/bar.cpp' in output:\n{}", output);
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
    assert!(output.contains("a.h a.cpp"), "Expected 'a.h a.cpp' in output:\n{}", output);
}

#[test]
fn test_source_grouping_toml_config() {
    let toml_content = r#"
source_grouping = "headers_first"
max_line_length = 100
"#;
    let config: FormatConfig = toml::from_str(toml_content)
        .expect("Failed to parse TOML config");

    assert_eq!(config.source_grouping, SourceGrouping::HeadersFirst);

    let input = "target_sources(mylib PUBLIC a.cpp a.h)";
    let output = format_text(input, &config);
    assert!(output.contains("a.h a.cpp"), "Expected 'a.h a.cpp' in output:\n{}", output);
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

    assert_eq!(pass1, pass2, "Formatting with source grouping is not idempotent");
}
