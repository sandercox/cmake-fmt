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

#[test]
fn test_the_guard_never_fires_on_its_own_output() {
    // Convergence over every real file: once the guard has accepted a file, the
    // formatted result must be acceptable too. A guard that refuses its own
    // output turns one bad pass into a permanently unformattable file.
    let mut files = Vec::new();
    for dir in ["tests/corpus", "tests/fixtures", "tests/format_fixtures"] {
        collect_cmake_files(std::path::Path::new(dir), &mut files);
    }
    assert!(files.len() > 30, "expected a corpus, found {}", files.len());

    let configs = [
        FormatConfig::default(),
        FormatConfig {
            closing_style: ClosingStyle::Force,
            sort_sources: SortSources::Alphabetical,
            source_grouping: SourceGrouping::HeadersFirst,
            ..Default::default()
        },
        FormatConfig {
            closing_style: ClosingStyle::Preserve,
            command_case: CommandCase::Preserve,
            comment_style: CommentStyle::Preserve,
            ..Default::default()
        },
    ];

    for path in &files {
        let source = std::fs::read_to_string(path).expect("read");
        for config in &configs {
            let (once, first) = format_text_with_diagnostics(&source, config);
            if first
                .iter()
                .any(|w| matches!(w, FormatWarning::ContentChanged { .. }))
            {
                // Genuinely unformattable; that it is refused is asserted elsewhere
                continue;
            }
            let (_, second) = format_text_with_diagnostics(&once, config);
            assert!(
                !second
                    .iter()
                    .any(|w| matches!(w, FormatWarning::ContentChanged { .. })),
                "the guard refused the formatter's own output for {}",
                path.display()
            );
        }
    }
}

fn collect_cmake_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_cmake_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "cmake")
                || path.file_name().is_some_and(|n| n == "CMakeLists.txt")
            {
                out.push(path);
            }
        }
    }
}

#[test]
fn test_a_suppressed_region_is_not_held_to_the_closing_style() {
    // A `# cmake-fmt: off` region is emitted verbatim, so `closing_style` never
    // applies inside it — but the guard normalised the input as if it did, and
    // refused the whole file at default settings. The rest of the file was left
    // unformatted as a result.
    let tempdir = TempDir::new().expect("tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    std::fs::write(
        &path,
        "# cmake-fmt: off\nif(A)\nmessage(hi)\nendif(A)\n# cmake-fmt: on\nset(X    1)\n",
    )
    .expect("write");

    let output = Command::new(cmake_fmt_bin())
        .args(["-i", path.to_str().unwrap()])
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(0),
        "the guard refused a suppressed region: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let formatted = std::fs::read_to_string(&path).expect("read back");
    assert!(
        formatted.contains("endif(A)") && formatted.contains("set(X 1)"),
        "the suppressed region or the rest of the file was mangled:\n{}",
        formatted
    );
}

#[test]
fn test_a_line_range_leaves_an_untouched_closer_alone() {
    // The spliced buffer is only partly formatted, so a closer on a line the
    // range did not select keeps its old arguments. Normalising the input as if
    // the setting had applied refused hundreds of buffers byte-identical to
    // their input.
    let tempdir = TempDir::new().expect("tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    std::fs::write(&path, "if(A)\nmessage(hi)\nendif(A)\n").expect("write");

    let output = Command::new(cmake_fmt_bin())
        .args(["-i", "--line-ranges", "1:1", path.to_str().unwrap()])
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(0),
        "an untouched closer was read as a content change: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[cfg(unix)]
fn test_an_unreadable_file_fails_the_run() {
    // The aggregation now treats a file it could not process as a failed run; a
    // report nobody reads is how an unformatted file reaches a release.
    //
    // The file has to *exist* and be unreadable: a path that does not exist is
    // reported by an earlier check that returns success, so naming one tested
    // nothing.
    use std::os::unix::fs::PermissionsExt;

    let tempdir = TempDir::new().expect("tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    std::fs::write(&path, "set(A   b)\n").expect("write");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).expect("chmod");

    // Root ignores the mode, so only assert when the file really is unreadable
    if std::fs::read_to_string(&path).is_ok() {
        return;
    }

    let output = Command::new(cmake_fmt_bin())
        .args(["-i", path.to_str().unwrap()])
        .output()
        .expect("run");

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("chmod back");

    assert_ne!(
        output.status.code(),
        Some(0),
        "a file that could not be read should fail the run: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_the_warning_says_where_and_what() {
    // The diagnostic truncated both sides at 48 characters independently, so on
    // both files that fire in practice it rendered them identically and told the
    // reader nothing. It now carries a line number and keeps the divergence.
    let tempdir = TempDir::new().expect("tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    std::fs::write(
        &path,
        "set(A b)\nset(B c)\nset(flags\n\t/wd1  # padded due to alignment specifier)\n",
    )
    .expect("write");

    let output = Command::new(cmake_fmt_bin())
        .args(["-i", path.to_str().unwrap()])
        .output()
        .expect("run");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("line 3"), "no line number: {}", stderr);
    let (before, after) = stderr
        .split_once(" became ")
        .expect("the warning names both sides");
    assert_ne!(
        before.rsplit('(').next(),
        after.split(')').next(),
        "both sides rendered identically: {}",
        stderr
    );
}

#[test]
fn test_a_forced_closer_may_drop_the_openers_comment() {
    // The formatter builds a forced closer from the opener's *values*, dropping
    // comment tokens, so `if(A # why)` closes as `endif(A)`. The guard kept the
    // comment in the opener's arguments and so demanded a closer the formatter
    // will never emit — refusing the file.
    let tempdir = TempDir::new().expect("tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    std::fs::write(&path, "if(A # why\n)\nmessage(hi)\nendif()\n").expect("write");

    let output = Command::new(cmake_fmt_bin())
        .args([
            "-i",
            "--style",
            "closing_style=force",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(0),
        "the guard refused a legitimate forced closer: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let formatted = std::fs::read_to_string(&path).expect("read back");
    assert!(
        formatted.contains("endif(A)") && formatted.contains("# why"),
        "the comment or the closer is wrong:\n{}",
        formatted
    );
}

#[test]
fn test_a_command_the_formatter_does_not_treat_as_a_block_is_left_alone() {
    // `block`/`endblock` are not in the formatter's block lists, so it never
    // rewrites an `endblock`'s arguments whatever `closing_style` says. The
    // guard listed them and so demanded a rewrite that never came.
    let tempdir = TempDir::new().expect("tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    std::fs::write(&path, "block(PROPAGATE x)\nset(x    1)\nendblock()\n").expect("write");

    for style in ["force", "remove", "preserve"] {
        std::fs::write(&path, "block(PROPAGATE x)\nset(x    1)\nendblock()\n").expect("write");
        let output = Command::new(cmake_fmt_bin())
            .args([
                "-i",
                "--style",
                &format!("closing_style={}", style),
                path.to_str().unwrap(),
            ])
            .output()
            .expect("run");
        assert_eq!(
            output.status.code(),
            Some(0),
            "closing_style={} refused a block: {}",
            style,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn test_a_file_that_sets_its_own_style_is_still_formatted() {
    // End to end, because the unit tests drive the check with a hand-built
    // output and cannot see whether the formatter and the check agree about what
    // a directive turned on. Two ways they did not: `comment_style` was missing
    // from the union entirely, and the scan took the first `#` on the line —
    // which can be inside a quoted argument.
    let tempdir = TempDir::new().expect("tempdir");
    let cases: &[(&str, &str, &str)] = &[
        (
            "comment_style=hash_space",
            "comment_style=preserve",
            "# cmake-fmt: comment_style=hash_space\nset(A b)\n#foo\n",
        ),
        (
            "sort_sources behind a quoted hash",
            "",
            "set(X \"#h\") # cmake-fmt: sort_sources=alphabetical\nset(SRCS b.cpp a.cpp)\n",
        ),
        (
            "sort_sources after a bracket comment",
            "",
            "#[[c]] # cmake-fmt: sort_sources=alphabetical\nset(SRCS b.cpp a.cpp)\n",
        ),
        (
            "closing_style=force",
            "closing_style=preserve",
            "# cmake-fmt: closing_style=force\nif(A)\n\tmessage(x)\nendif()\n",
        ),
        (
            "command_case=uppercase",
            "command_case=preserve,user_command_case=preserve",
            "# cmake-fmt: command_case=uppercase\nset(C d)\n",
        ),
        (
            "source_grouping=headers_first",
            "",
            "# cmake-fmt: source_grouping=headers_first\nset(SRCS a.cpp a.h)\n",
        ),
    ];

    for (name, style, source) in cases {
        let path = tempdir.path().join("CMakeLists.txt");
        std::fs::write(&path, source).expect("write");

        let mut args = vec!["-i".to_string()];
        if !style.is_empty() {
            args.push("--style".to_string());
            args.push((*style).to_string());
        }
        args.push(path.to_str().unwrap().to_string());

        let output = Command::new(cmake_fmt_bin())
            .args(&args)
            .output()
            .expect("run");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} was refused: {}",
            name,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn test_an_invalid_directive_value_does_not_open_an_exemption() {
    // The formatter rejects the value and carries on with the old setting, so
    // the check must not widen for it. Comparing raw strings accepted anything
    // that was not the literal default.
    let tempdir = TempDir::new().expect("tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    std::fs::write(&path, "# cmake-fmt: command_case=garbage\nset(A   b)\n").expect("write");

    let output = Command::new(cmake_fmt_bin())
        .args([
            "-i",
            "--style",
            "command_case=preserve,user_command_case=preserve",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("run");

    // The file still formats — the invalid value changes nothing — and the
    // command's case is preserved
    assert_eq!(output.status.code(), Some(0));
    let formatted = std::fs::read_to_string(&path).expect("read back");
    assert!(
        formatted.contains("set(A b)"),
        "the file should still be formatted:\n{}",
        formatted
    );
}
