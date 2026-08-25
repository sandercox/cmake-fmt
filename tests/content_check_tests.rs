//! The formatter must only move characters around. These tests pin both halves
//! of that: the transformations it is allowed to make must not trip the guard,
//! and a real content change must stop the file being written.

use cmake_fmt::formatter::{
    ClosingStyle, CommandCase, CommentStyle, FormatConfig, FormatWarning, SortSources,
    SourceGrouping, format_text, format_text_with_diagnostics,
};
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

fn cmake_fmt_bin() -> String {
    env!("CARGO_BIN_EXE_cmake-fmt").to_string()
}

fn content_changed(input: &str, config: &FormatConfig) -> bool {
    let (_, warnings) = format_text_with_diagnostics(input, config);
    warnings
        .iter()
        .any(|w| matches!(w, FormatWarning::ContentChanged { .. }))
}

/// Every transformation the formatter is allowed to make, against the guard.
#[test]
fn test_permitted_transformations_do_not_trip_the_guard() {
    let cases: &[(&str, FormatConfig)] = &[
        // Re-casing a command changes letters
        (
            "SET(A b)\nMESSAGE(STATUS hi)\n",
            FormatConfig {
                command_case: CommandCase::Lowercase,
                ..Default::default()
            },
        ),
        (
            "set(A b)\n",
            FormatConfig {
                command_case: CommandCase::Uppercase,
                ..Default::default()
            },
        ),
        // Dropping a closer's arguments removes characters
        (
            "if(A)\n\tmessage(x)\nendif(A)\n",
            FormatConfig {
                closing_style: ClosingStyle::Remove,
                ..Default::default()
            },
        ),
        // Reconstructing them adds characters
        (
            "if(A)\n\tmessage(x)\nendif()\n",
            FormatConfig {
                closing_style: ClosingStyle::Force,
                ..Default::default()
            },
        ),
        // Moving the space after `#` changes a comment token's text
        (
            "#foo\nset(A b) #bar\n",
            FormatConfig {
                comment_style: CommentStyle::HashSpace,
                ..Default::default()
            },
        ),
        (
            "# foo\n",
            FormatConfig {
                comment_style: CommentStyle::HashNoSpace,
                ..Default::default()
            },
        ),
        // Reordering changes argument order
        (
            "set(SOURCES z.cpp a.cpp)\n",
            FormatConfig {
                sort_sources: SortSources::Alphabetical,
                ..Default::default()
            },
        ),
        (
            "target_sources(t PRIVATE z.cpp z.h a.cpp a.h)\n",
            FormatConfig {
                sort_sources: SortSources::Alphabetical,
                source_grouping: SourceGrouping::HeadersFirst,
                ..Default::default()
            },
        ),
        // Re-indenting, re-wrapping and blank-line collapsing are pure whitespace
        (
            "if(A)\n\n\n\n\tmessage(x)\nendif()\n",
            FormatConfig {
                use_tabs: false,
                indent_width: 2,
                max_line_length: 40,
                ..Default::default()
            },
        ),
    ];

    for (input, config) in cases {
        assert!(
            !content_changed(input, config),
            "the guard rejected a permitted transformation on {:?}",
            input
        );
    }
}

#[test]
fn test_guard_leaves_the_input_untouched_when_it_fires() {
    // A group running off the end of the file: formatting it would have to
    // invent the closer, so nothing is written.
    let input = "f((A # c";
    let config = FormatConfig::default();

    assert!(content_changed(input, &config));
    assert_eq!(
        format_text(input, &config),
        input,
        "the input should come back byte-identical"
    );
}

#[test]
fn test_guard_reports_what_changed() {
    let (_, warnings) = format_text_with_diagnostics("f((A # c", &FormatConfig::default());
    let detail = warnings
        .iter()
        .find_map(|w| match w {
            FormatWarning::ContentChanged { detail } => Some(detail.clone()),
            _ => None,
        })
        .expect("expected a ContentChanged warning");

    // Enough to point at the command, and on one line so it reads in a log
    assert!(detail.contains("f("), "unhelpful detail: {}", detail);
    assert!(!detail.contains('\n'), "detail spans lines: {}", detail);
}

#[test]
fn test_guard_fails_the_run_in_place_and_leaves_the_file() {
    // The bool that process_file already returned only failed check and diff
    // mode; a file the formatter declines to touch has to fail an -i run too,
    // or CI never learns about it.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    let input = "f((A # c";
    std::fs::write(&path, input).expect("Failed to write file");

    let output = Command::new(cmake_fmt_bin())
        .args(["-i", path.to_str().unwrap()])
        .output()
        .expect("Failed to run cmake-fmt");

    assert_eq!(
        output.status.code(),
        Some(1),
        "-i should fail when a file is left unformatted"
    );
    assert_eq!(
        std::fs::read_to_string(&path).expect("Failed to read back"),
        input,
        "the file should be byte-identical"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("left unchanged"),
        "no warning printed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_guard_fails_the_run_on_stdin_but_still_echoes_the_buffer() {
    // An editor piping a buffer through must get its text back unchanged rather
    // than nothing, even though the run reports failure.
    let input = "f((A # c";
    let mut child = Command::new(cmake_fmt_bin())
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn cmake-fmt");
    child
        .stdin
        .as_mut()
        .expect("Failed to get stdin")
        .write_all(input.as_bytes())
        .expect("Failed to write to stdin");
    let output = child.wait_with_output().expect("Failed to wait");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), input);
}

#[test]
fn test_ordinary_files_are_unaffected() {
    // The guard must be invisible in normal use: a file that needs formatting
    // is still formatted, and the run succeeds.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    std::fs::write(&path, "SET(A   b)\nif(X)\nmessage(hi)\nendif(X)\n").expect("write");

    let output = Command::new(cmake_fmt_bin())
        .args(["-i", path.to_str().unwrap()])
        .output()
        .expect("Failed to run cmake-fmt");

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read back"),
        "set(A b)\nif(X)\n\tmessage(hi)\nendif()\n"
    );
}
