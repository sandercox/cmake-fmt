use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

// Helper to get the binary path
fn cmake_fmt_bin() -> String {
    env!("CARGO_BIN_EXE_cmake-fmt").to_string()
}

#[test]
fn test_assume_filename_basic_stdin_formatting() {
    let input = "set(FOO   bar)\n";
    let expected = "set(FOO bar)\n";

    // Write input to command stdin
    let mut child = Command::new(cmake_fmt_bin())
        .arg("--assume-filename")
        .arg("/tmp/fake.cmake")
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

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    assert!(output.status.success(), "Command should succeed");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected,
        "Output should be formatted"
    );
}

#[test]
fn test_assume_filename_config_resolution() {
    let tempdir = TempDir::new().expect("Failed to create tempdir");

    // Create a .cmake-fmt.toml with indent_width = 2 and use_tabs = false
    let config_path = tempdir.path().join(".cmake-fmt.toml");
    std::fs::write(&config_path, "indent_width = 2\nuse_tabs = false\n")
        .expect("Failed to write config");

    // Input that needs indentation
    let input = "if(TRUE)\nset(X y)\nendif()\n";
    let expected = "if(TRUE)\n  set(X y)\nendif()\n"; // 2-space indent

    let assumed_file = tempdir.path().join("CMakeLists.txt");

    let mut child = Command::new(cmake_fmt_bin())
        .arg("--assume-filename")
        .arg(&assumed_file)
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

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    assert!(output.status.success(), "Command should succeed");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected,
        "Should use 2-space indentation from config"
    );
}

#[test]
fn test_assume_filename_matches_file_output() {
    let tempdir = TempDir::new().expect("Failed to create tempdir");

    // Create a config file
    let config_path = tempdir.path().join(".cmake-fmt.toml");
    std::fs::write(&config_path, "indent_width = 2\nmax_line_length = 40\n")
        .expect("Failed to write config");

    // Create a CMakeLists.txt with content needing formatting
    let cmake_file = tempdir.path().join("CMakeLists.txt");
    let content = "if(TRUE)\nset(X y)\nendif()\n";
    std::fs::write(&cmake_file, content).expect("Failed to write CMakeLists.txt");

    // Format the file directly
    let file_output = Command::new(cmake_fmt_bin())
        .arg(&cmake_file)
        .output()
        .expect("Failed to run cmake-fmt on file");

    assert!(
        file_output.status.success(),
        "File formatting should succeed"
    );

    // Format via stdin with --assume-filename
    let mut child = Command::new(cmake_fmt_bin())
        .arg("--assume-filename")
        .arg(&cmake_file)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn cmake-fmt");

    child
        .stdin
        .as_mut()
        .expect("Failed to get stdin")
        .write_all(content.as_bytes())
        .expect("Failed to write to stdin");

    let stdin_output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    assert!(
        stdin_output.status.success(),
        "Stdin formatting should succeed"
    );

    // Both outputs should be identical
    assert_eq!(
        file_output.stdout, stdin_output.stdout,
        "File and stdin+assume-filename outputs should match"
    );
}

#[test]
fn test_assume_filename_relative_path() {
    let tempdir = TempDir::new().expect("Failed to create tempdir");

    // Create a config with use_tabs = true
    let config_path = tempdir.path().join(".cmake-fmt.toml");
    std::fs::write(&config_path, "use_tabs = true\nindent_width = 4\n")
        .expect("Failed to write config");

    let input = "if(TRUE)\nset(X y)\nendif()\n";
    let expected = "if(TRUE)\n\tset(X y)\nendif()\n"; // Tab indent

    // Run from the tempdir with a relative path
    let mut child = Command::new(cmake_fmt_bin())
        .current_dir(tempdir.path())
        .arg("--assume-filename")
        .arg("./CMakeLists.txt")
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

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    assert!(output.status.success(), "Command should succeed");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected,
        "Should use tabs from config (relative path resolution)"
    );
}

#[test]
fn test_assume_filename_with_file_args_errors() {
    let output = Command::new(cmake_fmt_bin())
        .arg("--assume-filename")
        .arg("/tmp/fake.cmake")
        .arg("somefile.cmake")
        .output()
        .expect("Failed to run cmake-fmt");

    assert!(
        !output.status.success(),
        "Command should fail with non-zero exit code"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--assume-filename can only be used with stdin input"),
        "Should error about --assume-filename with file args"
    );
}

#[test]
fn test_assume_filename_check_mode() {
    let input = "set(FOO   bar)\n";

    let mut child = Command::new(cmake_fmt_bin())
        .arg("--assume-filename")
        .arg("/tmp/fake.cmake")
        .arg("--check")
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

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    assert_eq!(
        output.status.code(),
        Some(1),
        "Check mode should exit with code 1 for unformatted input"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Would reformat"),
        "Should output 'Would reformat' message"
    );
}

#[test]
fn test_assume_filename_diff_mode() {
    let input = "set(FOO   bar)\n";

    let mut child = Command::new(cmake_fmt_bin())
        .arg("--assume-filename")
        .arg("/tmp/test.cmake")
        .arg("--diff")
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

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    assert_eq!(
        output.status.code(),
        Some(1),
        "Diff mode should exit with code 1 for differences"
    );

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout_str.contains("--- a/tmp/test.cmake") || stdout_str.contains("--- a//tmp/test.cmake"),
        "Diff should contain the assumed filename in header, got: {}",
        stdout_str
    );
    assert!(
        stdout_str.contains("+++ b/tmp/test.cmake") || stdout_str.contains("+++ b//tmp/test.cmake"),
        "Diff should contain the assumed filename in header, got: {}",
        stdout_str
    );
    assert!(
        !stdout_str.contains("stdin"),
        "Diff should NOT contain 'stdin' when filename is assumed"
    );
}

#[test]
fn test_assume_filename_grammar_detection() {
    let tempdir = TempDir::new().expect("Failed to create tempdir");

    // Create a CMakeLists.txt with a custom function definition
    let cmake_file = tempdir.path().join("CMakeLists.txt");
    let function_def = r#"function(my_custom_func)
    cmake_parse_arguments(MY_CUSTOM_FUNC "OPT" "SINGLE" "MULTI" ${ARGN})
    message("OPT: ${MY_CUSTOM_FUNC_OPT}")
endfunction()
"#;
    std::fs::write(&cmake_file, function_def).expect("Failed to write CMakeLists.txt");

    // Create content that uses the custom function
    let user_file_content = "my_custom_func(OPT SINGLE val MULTI a b c)\n";

    // Format the content via stdin with assumed path
    let user_file_path = tempdir.path().join("user.cmake");

    let mut child = Command::new(cmake_fmt_bin())
        .arg("--assume-filename")
        .arg(&user_file_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn cmake-fmt");

    child
        .stdin
        .as_mut()
        .expect("Failed to get stdin")
        .write_all(user_file_content.as_bytes())
        .expect("Failed to write to stdin");

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    assert!(output.status.success(), "Command should succeed");

    // The output should be formatted with keyword awareness
    // At minimum, it should not error and should produce valid output
    let formatted = String::from_utf8_lossy(&output.stdout);
    assert!(
        formatted.contains("my_custom_func"),
        "Should contain the function call"
    );

    // Now create the user.cmake file and format it directly to compare
    std::fs::write(&user_file_path, user_file_content).expect("Failed to write user.cmake");

    let file_output = Command::new(cmake_fmt_bin())
        .arg(&user_file_path)
        .output()
        .expect("Failed to format file directly");

    assert!(
        file_output.status.success(),
        "File formatting should succeed"
    );

    // The outputs should be identical (both should use project grammar scanning)
    assert_eq!(
        output.stdout, file_output.stdout,
        "Stdin+assume-filename should match direct file formatting for grammar detection"
    );
}

/// Run cmake-fmt with the given args, piping `input` to stdin.
/// Returns (exit code success, stdout).
fn run_with_stdin(args: &[&str], input: &str) -> (bool, String) {
    let mut child = Command::new(cmake_fmt_bin())
        .args(args)
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

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
    )
}

#[test]
fn test_assume_filename_respects_cmake_fmt_ignore() {
    // Regression: https://github.com/sandercox/cmake-fmt/issues/4
    // Editors format on save via --assume-filename, which used to bypass
    // .cmake-fmt-ignore entirely.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    std::fs::create_dir(root.join("vendor")).expect("Failed to create vendor dir");
    std::fs::write(root.join(".cmake-fmt-ignore"), "vendor/\n").expect("Failed to write ignore");

    let input = "set(FOO   bar)\n";

    let ignored = root.join("vendor").join("CMakeLists.txt");
    let (ok, stdout) = run_with_stdin(
        &["-", "--assume-filename", ignored.to_str().unwrap()],
        input,
    );
    assert!(ok, "Command should succeed for an ignored file");
    assert_eq!(stdout, input, "Ignored file should pass through unchanged");

    let formatted = root.join("src.cmake");
    let (ok, stdout) = run_with_stdin(
        &["-", "--assume-filename", formatted.to_str().unwrap()],
        input,
    );
    assert!(ok, "Command should succeed for a non-ignored file");
    assert_eq!(stdout, "set(FOO bar)\n", "Non-ignored file should format");
}

#[test]
fn test_assume_filename_ignore_negation_wins() {
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    std::fs::create_dir(root.join("vendor")).expect("Failed to create vendor dir");
    std::fs::write(
        root.join(".cmake-fmt-ignore"),
        "vendor/*\n!vendor/keep.cmake\n",
    )
    .expect("Failed to write ignore");

    let input = "set(FOO   bar)\n";
    let kept = root.join("vendor").join("keep.cmake");
    let (ok, stdout) = run_with_stdin(&["-", "--assume-filename", kept.to_str().unwrap()], input);

    assert!(ok, "Command should succeed");
    assert_eq!(
        stdout, "set(FOO bar)\n",
        "Negated pattern should be formatted"
    );
}

#[test]
fn test_assume_filename_nearest_ignore_file_wins() {
    // A deeper .cmake-fmt-ignore can re-include what a shallower one excludes
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    let sub = root.join("sub");
    std::fs::create_dir(&sub).expect("Failed to create sub dir");
    std::fs::write(root.join(".cmake-fmt-ignore"), "*.cmake\n").expect("Failed to write ignore");
    std::fs::write(sub.join(".cmake-fmt-ignore"), "!keep.cmake\n").expect("Failed to write ignore");

    let input = "set(FOO   bar)\n";

    let kept = sub.join("keep.cmake");
    let (_, stdout) = run_with_stdin(&["-", "--assume-filename", kept.to_str().unwrap()], input);
    assert_eq!(stdout, "set(FOO bar)\n", "Deeper negation should win");

    let ignored = sub.join("other.cmake");
    let (_, stdout) = run_with_stdin(
        &["-", "--assume-filename", ignored.to_str().unwrap()],
        input,
    );
    assert_eq!(stdout, input, "Parent ignore still applies to other files");
}

#[test]
fn test_assume_filename_respects_extra_ignore_file() {
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    let ignore_file = root.join("my-ignore");
    std::fs::write(&ignore_file, "generated_*.cmake\n").expect("Failed to write ignore");

    let input = "set(FOO   bar)\n";
    let target = root.join("generated_api.cmake");
    let (ok, stdout) = run_with_stdin(
        &[
            "-",
            "--assume-filename",
            target.to_str().unwrap(),
            "--ignore-file",
            ignore_file.to_str().unwrap(),
        ],
        input,
    );

    assert!(ok, "Command should succeed");
    assert_eq!(stdout, input, "--ignore-file should apply to stdin too");
}

#[test]
fn test_assume_filename_ignored_file_passes_check_mode() {
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    std::fs::write(root.join(".cmake-fmt-ignore"), "*.cmake\n").expect("Failed to write ignore");

    let target = root.join("thing.cmake");
    let (ok, stdout) = run_with_stdin(
        &[
            "--check",
            "-",
            "--assume-filename",
            target.to_str().unwrap(),
        ],
        "set(FOO   bar)\n",
    );

    assert!(ok, "--check should succeed for an ignored file");
    assert!(stdout.is_empty(), "--check should not echo the buffer");
}

#[test]
fn test_assume_filename_excluded_directory_is_final() {
    // Git cannot re-include a file whose parent directory is excluded, and the
    // walk never descends into it, so `!vendor/keep.cmake` must not win here.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    std::fs::create_dir(root.join("vendor")).expect("Failed to create vendor dir");
    std::fs::write(
        root.join(".cmake-fmt-ignore"),
        "vendor/\n!vendor/keep.cmake\n",
    )
    .expect("Failed to write ignore");

    let input = "set(FOO   bar)\n";
    let target = root.join("vendor").join("keep.cmake");
    let (ok, stdout) = run_with_stdin(&["-", "--assume-filename", target.to_str().unwrap()], input);

    assert!(ok, "Command should succeed");
    assert_eq!(
        stdout, input,
        "negation inside an excluded directory must not re-include the file"
    );
}

#[test]
fn test_assume_filename_ignore_file_inside_excluded_directory_is_not_read() {
    // The walk never descends into build/, so it never reads the ignore file
    // sitting there; the stdin path must not honour it either.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    let build = root.join("build");
    std::fs::create_dir(&build).expect("Failed to create build dir");
    std::fs::write(root.join(".cmake-fmt-ignore"), "build/\n").expect("Failed to write ignore");
    std::fs::write(build.join(".cmake-fmt-ignore"), "!keep.cmake\n").expect("Failed to write");

    let input = "set(FOO   bar)\n";
    let target = build.join("keep.cmake");
    let (_, stdout) = run_with_stdin(&["-", "--assume-filename", target.to_str().unwrap()], input);

    assert_eq!(
        stdout, input,
        "file under an excluded directory must be skipped"
    );
}

#[test]
fn test_assume_filename_star_negation_is_honoured() {
    // `vendor/*` excludes the contents rather than the directory, so git — and
    // therefore this — does honour a negation under it.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    std::fs::create_dir(root.join("vendor")).expect("Failed to create vendor dir");
    std::fs::write(
        root.join(".cmake-fmt-ignore"),
        "vendor/*\n!vendor/keep.cmake\n",
    )
    .expect("Failed to write ignore");

    let input = "set(FOO   bar)\n";
    let kept = root.join("vendor").join("keep.cmake");
    let (_, stdout) = run_with_stdin(&["-", "--assume-filename", kept.to_str().unwrap()], input);
    assert_eq!(
        stdout, "set(FOO bar)\n",
        "negation under vendor/* should apply"
    );

    let ignored = root.join("vendor").join("other.cmake");
    let (_, stdout) = run_with_stdin(
        &["-", "--assume-filename", ignored.to_str().unwrap()],
        input,
    );
    assert_eq!(stdout, input, "other files under vendor/* stay excluded");
}

#[test]
fn test_assume_filename_relative_ignore_file_does_not_panic() {
    // `Gitignore::matched_path_or_any_parents` asserts the path is under the
    // matcher root, which aborted the process (exit 101, empty stdout) for an
    // --ignore-file whose directory is not a prefix of the assumed path.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    let work = root.join("work");
    std::fs::create_dir(&work).expect("Failed to create work dir");
    std::fs::write(work.join("ig.txt"), "a.cmake\n").expect("Failed to write ignore");
    std::fs::write(root.join("ig.txt"), "a.cmake\n").expect("Failed to write ignore");

    let input = "set(FOO   bar)\n";
    let target = work.join("a.cmake");

    for form in ["ig.txt", "./ig.txt", "../ig.txt"] {
        let mut child = Command::new(cmake_fmt_bin())
            .args([
                "-",
                "--assume-filename",
                target.to_str().unwrap(),
                "--ignore-file",
                form,
            ])
            .current_dir(&work)
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

        assert!(
            output.status.success(),
            "--ignore-file {} must not abort the process (exit {:?}): {}",
            form,
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            input,
            "--ignore-file {} should exclude a.cmake",
            form
        );
    }
}

#[test]
fn test_assume_filename_malformed_ignore_line_keeps_valid_patterns() {
    // `GitignoreBuilder::add` is a partial failure: it still took the valid
    // lines, so one bad glob must not disable the whole file.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    std::fs::create_dir(root.join("vendor")).expect("Failed to create vendor dir");
    std::fs::write(root.join(".cmake-fmt-ignore"), "vendor/\n{a,b\n")
        .expect("Failed to write ignore");

    let input = "set(FOO   bar)\n";
    let target = root.join("vendor").join("a.cmake");
    let (ok, stdout) = run_with_stdin(&["-", "--assume-filename", target.to_str().unwrap()], input);

    assert!(ok, "Command should succeed");
    assert_eq!(
        stdout, input,
        "a malformed line must not disable the valid patterns"
    );
}

#[test]
fn test_assume_filename_normalizes_dot_segments() {
    // An un-normalized path makes ancestors() yield directories that are not
    // ancestors at all, applying the wrong ignore file.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    let cwd = root.join("cwd");
    let foo = root.join("foo");
    std::fs::create_dir(&cwd).expect("Failed to create cwd");
    std::fs::create_dir(&foo).expect("Failed to create foo");
    std::fs::write(cwd.join(".cmake-fmt-ignore"), "CMakeLists.txt\n")
        .expect("Failed to write ignore");

    let input = "set(FOO   bar)\n";

    // cwd/.cmake-fmt-ignore governs cwd/, not the sibling foo/
    let mut child = Command::new(cmake_fmt_bin())
        .args(["-", "--assume-filename", "../foo/CMakeLists.txt"])
        .current_dir(&cwd)
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

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "set(FOO bar)\n",
        "a sibling directory's ignore file must not apply"
    );
}

#[test]
fn test_assume_filename_agrees_with_directory_walk() {
    // The invariant the README promises: the stdin path and the directory walk
    // reach the same verdict for every file in a tree.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();

    for dir in ["vendor", "gen", "sub", "build"] {
        std::fs::create_dir(root.join(dir)).expect("Failed to create dir");
    }
    std::fs::write(
        root.join(".cmake-fmt-ignore"),
        "vendor/\n!vendor/keep.cmake\ngen/*\n!gen/keep.cmake\nbuild/\n",
    )
    .expect("Failed to write ignore");
    std::fs::write(root.join("sub").join(".cmake-fmt-ignore"), "local.cmake\n")
        .expect("Failed to write ignore");
    std::fs::write(
        root.join("build").join(".cmake-fmt-ignore"),
        "!keep.cmake\n",
    )
    .expect("Failed to write ignore");

    // Every file needs reformatting, so --check names exactly the files the
    // walk considers.
    let unformatted = "set(FOO   bar)\n";
    let files = [
        "top.cmake",
        "vendor/a.cmake",
        "vendor/keep.cmake",
        "gen/a.cmake",
        "gen/keep.cmake",
        "sub/local.cmake",
        "sub/other.cmake",
        "build/keep.cmake",
    ];
    for file in files {
        std::fs::write(root.join(file), unformatted).expect("Failed to write file");
    }

    let walk = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", "."])
        .current_dir(root)
        .output()
        .expect("Failed to run walk");
    let walk_report =
        String::from_utf8_lossy(&walk.stderr).to_string() + &String::from_utf8_lossy(&walk.stdout);

    for file in files {
        let walk_sees = walk_report.contains(file);
        let target = root.join(file);
        let (_, stdout) = run_with_stdin(
            &["-", "--assume-filename", target.to_str().unwrap()],
            unformatted,
        );
        let stdin_formats = stdout != unformatted;

        assert_eq!(
            walk_sees, stdin_formats,
            "walk and stdin disagree about {}: walk_sees={}, stdin_formats={}\nwalk report:\n{}",
            file, walk_sees, stdin_formats, walk_report
        );
    }
}

#[test]
fn test_extra_ignore_file_is_rooted_at_the_working_directory() {
    // --ignore-file is rooted at the cwd, matching WalkBuilder::add_ignore, so
    // an anchored pattern resolves against the cwd rather than against the
    // ignore file's own directory. Nothing covered this, and mutating the
    // rooting to an empty path left every other test green.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    std::fs::create_dir(root.join("build")).expect("Failed to create build dir");
    std::fs::write(root.join("ig.txt"), "/build/\n").expect("Failed to write ignore");

    let input = "set(FOO   bar)\n";
    let target = root.join("build").join("a.cmake");

    let mut child = Command::new(cmake_fmt_bin())
        .args([
            "-",
            "--assume-filename",
            target.to_str().unwrap(),
            "--ignore-file",
            "ig.txt",
        ])
        .current_dir(root)
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

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        input,
        "an anchored --ignore-file pattern should resolve against the cwd"
    );
}

#[test]
fn test_walk_from_inside_an_excluded_directory_skips_too() {
    // An excluded path stays excluded however the tool is reached. A walk only
    // tests entries at or below its own root, so without an explicit check
    // `cmake-fmt -r .` from inside an excluded directory formatted files that
    // the same command from the project root — and the stdin path — skip.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    let inner = root.join("proj").join("third_party").join("foo");
    std::fs::create_dir_all(&inner).expect("Failed to create dirs");
    std::fs::write(root.join(".cmake-fmt-ignore"), "third_party/\n")
        .expect("Failed to write ignore");

    let unformatted = "set(FOO   bar)\n";
    std::fs::write(inner.join("CMakeLists.txt"), unformatted).expect("Failed to write file");
    std::fs::write(root.join("proj").join("top.cmake"), unformatted).expect("Failed to write file");

    // From inside the excluded tree
    let inside = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", "."])
        .current_dir(&inner)
        .output()
        .expect("Failed to run walk");
    let report = String::from_utf8_lossy(&inside.stdout).to_string()
        + &String::from_utf8_lossy(&inside.stderr);
    assert!(
        !report.contains("Would reformat"),
        "walk from inside an excluded directory should skip:\n{}",
        report
    );

    // Naming it explicitly is the same answer
    let named = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", "proj/third_party"])
        .current_dir(root)
        .output()
        .expect("Failed to run walk");
    let named_report = String::from_utf8_lossy(&named.stdout).to_string()
        + &String::from_utf8_lossy(&named.stderr);
    assert!(
        !named_report.contains("Would reformat"),
        "explicitly naming an excluded directory should skip:\n{}",
        named_report
    );

    // A non-excluded sibling is still walked
    let normal = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", "."])
        .current_dir(root)
        .output()
        .expect("Failed to run walk");
    let normal_report = String::from_utf8_lossy(&normal.stdout).to_string()
        + &String::from_utf8_lossy(&normal.stderr);
    assert!(
        normal_report.contains("top.cmake"),
        "the rest of the tree must still be walked:\n{}",
        normal_report
    );
}

#[test]
fn test_ignore_file_pattern_matching_an_ancestor_does_not_disable_the_walk() {
    // Gitignore matches a pattern with no slash against a path's basename, so
    // an ordinary pattern like `tmp/` matches the `/tmp` component of an
    // absolute path. Consulting --ignore-file for every ancestor therefore
    // excluded the walk root itself and the whole run silently found nothing —
    // and `--check` exited 0 having checked nothing, which in CI reads as pass.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    std::fs::create_dir(root.join("src")).expect("Failed to create src dir");

    // Patterns from an ordinary .gitignore. `tmp/` matches the system temp
    // directory this tree lives under; nothing here is meant to be excluded.
    std::fs::write(root.join("ig.txt"), "build/\ntmp/\n*.o\n").expect("Failed to write ignore");

    let unformatted = "set(FOO   bar)\n";
    std::fs::write(root.join("CMakeLists.txt"), unformatted).expect("Failed to write file");
    std::fs::write(root.join("src").join("a.cmake"), unformatted).expect("Failed to write file");

    let output = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", ".", "--ignore-file", "ig.txt"])
        .current_dir(root)
        .output()
        .expect("Failed to run walk");
    let report = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);

    assert!(
        report.contains("CMakeLists.txt") && report.contains("a.cmake"),
        "an --ignore-file pattern matching an ancestor disabled the walk:\n{}",
        report
    );

    // The same file still excludes what it actually names
    std::fs::write(root.join("ig.txt"), "src/\n").expect("Failed to write ignore");
    let output = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", ".", "--ignore-file", "ig.txt"])
        .current_dir(root)
        .output()
        .expect("Failed to run walk");
    let report = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);

    assert!(
        report.contains("CMakeLists.txt") && !report.contains("a.cmake"),
        "--ignore-file should still exclude what it names:\n{}",
        report
    );
}
