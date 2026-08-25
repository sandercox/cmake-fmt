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

#[test]
fn test_line_ranges_output_is_checked_too() {
    // --line-ranges splices lines from the formatted text into the original by
    // index, so as soon as formatting changes the line count the result says
    // something neither text said. The guard sat one level below that splice,
    // so this destroyed two thirds of the file and exited 0.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    let input = "target_sources(t PRIVATE aaaaaaaaaa.cpp bbbbbbbbbb.cpp cccccccccc.cpp dddddddddd.cpp)\nmessage(hi)\n";
    std::fs::write(&path, input).expect("Failed to write file");

    let output = Command::new(cmake_fmt_bin())
        .args(["-i", "--line-ranges", "1:1", path.to_str().unwrap()])
        .output()
        .expect("Failed to run cmake-fmt");

    assert_eq!(
        std::fs::read_to_string(&path).expect("Failed to read back"),
        input,
        "the spliced output was written to the file"
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "the run should fail: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_line_ranges_still_formats_what_it_can() {
    // The guard must not turn --line-ranges off: a range whose formatting does
    // not change the line count still applies.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    std::fs::write(&path, "set(A    b)\nset(C    d)\n").expect("Failed to write file");

    let output = Command::new(cmake_fmt_bin())
        .args(["-i", "--line-ranges", "1:1", path.to_str().unwrap()])
        .output()
        .expect("Failed to run cmake-fmt");

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read back"),
        "set(A b)\nset(C    d)\n"
    );
}

#[test]
fn test_stdout_mode_fails_when_a_file_is_left_alone() {
    // Printing to stdout is the mode a person runs by hand; it returned success
    // unconditionally, so it was the one mode that said nothing was wrong.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    let input = "f((A # c";
    std::fs::write(&path, input).expect("Failed to write file");

    let output = Command::new(cmake_fmt_bin())
        .arg(path.to_str().unwrap())
        .output()
        .expect("Failed to run cmake-fmt");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), input);
}

#[test]
fn test_recasing_a_user_command_is_not_a_content_change() {
    // command_case and user_command_case are independent settings. Only the
    // first was consulted, so `command_case = preserve` — an ordinary choice —
    // reported every re-cased user command as a content change and quietly
    // stopped formatting the file.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    std::fs::write(
        &path,
        "function(my_helper arg)\n\tmessage(${arg})\nendfunction()\n\nMY_HELPER(hello)\nset(X    y)\n",
    )
    .expect("Failed to write file");

    let output = Command::new(cmake_fmt_bin())
        .args([
            "-i",
            "--style",
            "command_case=preserve,user_command_case=lowercase",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run cmake-fmt");

    assert_eq!(
        output.status.code(),
        Some(0),
        "guard fired on a permitted re-casing: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let formatted = std::fs::read_to_string(&path).expect("read back");
    assert!(
        formatted.contains("my_helper(hello)") && formatted.contains("set(X y)"),
        "the file was not formatted:\n{}",
        formatted
    );
}

#[test]
fn test_an_invented_closing_paren_is_refused() {
    // The parser hangs a command's own parens off the invocation rather than the
    // argument list, so a model built only from the argument list could not see
    // the formatter supply a missing `)` — one of the bugs this guard exists
    // for. A real corpus file (llvm/HandleLLVMOptions.cmake) ends this way,
    // with its closing paren inside a trailing comment.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    let input = "set(flags\n\t/wd4324  # padded due to alignment specifier)\n";
    std::fs::write(&path, input).expect("Failed to write file");

    let output = Command::new(cmake_fmt_bin())
        .args(["-i", path.to_str().unwrap()])
        .output()
        .expect("Failed to run cmake-fmt");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read back"),
        input,
        "the file should be byte-identical"
    );
}
