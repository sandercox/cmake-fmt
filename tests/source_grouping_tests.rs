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

#[test]
fn test_source_grouping_with_leading_comments() {
    let input = r#"set(SOURCES
    foo.h foo.cpp
    bar.h bar.cpp

    # Generators
    Generators/baz.h Generators/baz.cpp
    Generators/qux.h Generators/qux.cpp
)"#;
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // foo and bar should be grouped
    assert!(
        output.contains("foo.h foo.cpp"),
        "Expected 'foo.h foo.cpp' in output:\n{}",
        output
    );
    assert!(
        output.contains("bar.h bar.cpp"),
        "Expected 'bar.h bar.cpp' in output:\n{}",
        output
    );
    // Comment should be preserved
    assert!(output.contains("# Generators"), "Expected comment preserved:\n{}", output);
    // Generators files should be grouped
    assert!(
        output.contains("Generators/baz.h Generators/baz.cpp"),
        "Expected 'Generators/baz.h Generators/baz.cpp' in output:\n{}",
        output
    );
    assert!(
        output.contains("Generators/qux.h Generators/qux.cpp"),
        "Expected 'Generators/qux.h Generators/qux.cpp' in output:\n{}",
        output
    );
    // Blank line should be preserved (comment should come after it)
    let lines: Vec<&str> = output.lines().collect();
    let comment_line = lines.iter().position(|l| l.contains("# Generators")).unwrap();
    let bar_line = lines.iter().position(|l| l.contains("bar.h")).unwrap();
    // There should be at least one line between bar and comment
    assert!(comment_line > bar_line + 1, "Expected blank line before comment");
}

#[test]
fn test_source_grouping_with_multiple_comment_sections() {
    let input = r#"set(SOURCES
    # Generators
    Generators/noise.h Generators/noise.cpp
    Generators/wave.h Generators/wave.cpp

    # Effects
    Effects/reverb.h Effects/reverb.cpp
    Effects/delay.h Effects/delay.cpp

    # Mixers
    Mixers/stereo.h Mixers/stereo.cpp
)"#;
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // All comments should be preserved
    assert!(output.contains("# Generators"), "Expected '# Generators' comment");
    assert!(output.contains("# Effects"), "Expected '# Effects' comment");
    assert!(output.contains("# Mixers"), "Expected '# Mixers' comment");

    // All pairs should be grouped within their sections
    assert!(
        output.contains("Generators/noise.h Generators/noise.cpp"),
        "Expected grouped noise files:\n{}",
        output
    );
    assert!(
        output.contains("Generators/wave.h Generators/wave.cpp"),
        "Expected grouped wave files:\n{}",
        output
    );
    assert!(
        output.contains("Effects/reverb.h Effects/reverb.cpp"),
        "Expected grouped reverb files:\n{}",
        output
    );
    assert!(
        output.contains("Effects/delay.h Effects/delay.cpp"),
        "Expected grouped delay files:\n{}",
        output
    );
    assert!(
        output.contains("Mixers/stereo.h Mixers/stereo.cpp"),
        "Expected grouped stereo files:\n{}",
        output
    );

    // Verify comment ordering (Generators before Effects before Mixers)
    let gen_pos = output.find("# Generators").unwrap();
    let eff_pos = output.find("# Effects").unwrap();
    let mix_pos = output.find("# Mixers").unwrap();
    assert!(gen_pos < eff_pos, "Generators comment should come before Effects");
    assert!(eff_pos < mix_pos, "Effects comment should come before Mixers");
}

#[test]
fn test_source_grouping_comments_idempotent() {
    let input = r#"set(SOURCES
    # Section 1
    foo.h foo.cpp

    # Section 2
    bar.h bar.cpp
)"#;
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };

    let pass1 = format_text(input, &config);
    let pass2 = format_text(&pass1, &config);

    assert_eq!(
        pass1, pass2,
        "Formatting with source grouping and comments is not idempotent.\nPass1:\n{}\n\nPass2:\n{}",
        pass1, pass2
    );
}

#[test]
fn test_source_grouping_blank_line_after_comments_preserved() {
    let input = r#"set(SOURCES
    AlleyMenu.h AlleyMenu.cpp
    stCircles.h
    # ChristMas.cpp
    # ChristMas.h

    AboutComponent.h AboutComponent.cpp
    AlleyColours.h
)"#;
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // Files should be grouped
    assert!(
        output.contains("AlleyMenu.h AlleyMenu.cpp"),
        "Expected 'AlleyMenu.h AlleyMenu.cpp' grouped:\n{}",
        output
    );
    assert!(
        output.contains("AboutComponent.h AboutComponent.cpp"),
        "Expected 'AboutComponent.h AboutComponent.cpp' grouped:\n{}",
        output
    );
    assert!(output.contains("stCircles.h"), "Expected stCircles.h");
    assert!(output.contains("AlleyColours.h"), "Expected AlleyColours.h");

    // Comments should be preserved
    assert!(output.contains("# ChristMas.cpp"), "Expected '# ChristMas.cpp' comment");
    assert!(output.contains("# ChristMas.h"), "Expected '# ChristMas.h' comment");

    // Critical: The blank line should appear AFTER the comments, not before them
    // In the source, the order is: comments, then blank line, then AboutComponent
    // This ordering must be preserved after source_grouping remapping
    let lines: Vec<&str> = output.lines().collect();
    let christmas_cpp_line = lines.iter().position(|l| l.contains("# ChristMas.cpp")).unwrap();
    let christmas_h_line = lines.iter().position(|l| l.contains("# ChristMas.h")).unwrap();
    let about_line = lines.iter().position(|l| l.contains("AboutComponent")).unwrap();

    // Comments should come before AboutComponent
    assert!(christmas_cpp_line < about_line, "Comments should come before AboutComponent");
    assert!(christmas_h_line < about_line, "Comments should come before AboutComponent");

    // There should be a blank line between the last comment and AboutComponent
    // (The blank line appears AFTER the comments, not before them)
    let last_comment_line = christmas_cpp_line.max(christmas_h_line);
    assert!(
        about_line > last_comment_line + 1,
        "Expected blank line AFTER comments (before AboutComponent). Last comment at {}, About at {}",
        last_comment_line,
        about_line
    );

    // Verify idempotency
    let pass2 = format_text(&output, &config);
    assert_eq!(
        output, pass2,
        "Formatting is not idempotent with blank line after comments.\nPass1:\n{}\n\nPass2:\n{}",
        output, pass2
    );
}

#[test]
fn test_source_grouping_blank_line_before_comment_preserved() {
    let input = r#"set(SOURCES
    foo.h foo.cpp

    # Section header
    bar.h bar.cpp
)"#;
    let config = FormatConfig {
        source_grouping: SourceGrouping::HeadersFirst,
        max_line_length: 100,
        ..Default::default()
    };
    let output = format_text(input, &config);

    // Files should be grouped
    assert!(
        output.contains("foo.h foo.cpp"),
        "Expected 'foo.h foo.cpp' grouped:\n{}",
        output
    );
    assert!(
        output.contains("bar.h bar.cpp"),
        "Expected 'bar.h bar.cpp' grouped:\n{}",
        output
    );

    // Comment should be preserved
    assert!(output.contains("# Section header"), "Expected '# Section header' comment");

    // Critical: The blank line should appear BEFORE the comment (the default case)
    // In the source, the order is: foo, blank line, comment, bar
    // This ordering must be preserved (no regression from our fix)
    let lines: Vec<&str> = output.lines().collect();
    let foo_line = lines.iter().position(|l| l.contains("foo.h")).unwrap();
    let comment_line = lines.iter().position(|l| l.contains("# Section header")).unwrap();
    let bar_line = lines.iter().position(|l| l.contains("bar.h")).unwrap();

    // Comment should come after foo and before bar
    assert!(foo_line < comment_line, "foo should come before comment");
    assert!(comment_line < bar_line, "Comment should come before bar");

    // There should be a blank line between foo and the comment
    assert!(
        comment_line > foo_line + 1,
        "Expected blank line BEFORE comment (after foo). Foo at {}, Comment at {}",
        foo_line,
        comment_line
    );

    // Verify idempotency
    let pass2 = format_text(&output, &config);
    assert_eq!(
        output, pass2,
        "Formatting is not idempotent with blank line before comment.\nPass1:\n{}\n\nPass2:\n{}",
        output, pass2
    );
}
