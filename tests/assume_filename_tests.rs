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
    run_with_stdin_in(None, args, input)
}

/// The same, from a given working directory. `--ignore-file` is resolved
/// relative to it, so comparing the stdin path against a walk means running
/// both from the same place.
fn run_with_stdin_in(dir: Option<&std::path::Path>, args: &[&str], input: &str) -> (bool, String) {
    let mut command = Command::new(cmake_fmt_bin());
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    let mut child = command
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
    // reach the same verdict for every file in a tree. Both halves are checked
    // with an extra --ignore-file in play, and from two walk roots — the
    // project root and a subdirectory — because the root itself is the one
    // entry a walk never tests, so it is checked by separate code.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();

    for dir in ["vendor", "gen", "gen2", "sub", "build"] {
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
    // The extra ignore file the walk gets via add_ignore and the stdin path
    // gets via --ignore-file
    std::fs::write(root.join("ig.txt"), "gen2/\n*.tmpl.cmake\n").expect("Failed to write ignore");

    // Every file needs reformatting, so --check names exactly the files the
    // walk considers.
    let unformatted = "set(FOO   bar)\n";
    let files = [
        "top.cmake",
        "top.tmpl.cmake",
        "vendor/a.cmake",
        "vendor/keep.cmake",
        "gen/a.cmake",
        "gen/keep.cmake",
        "gen2/a.cmake",
        "sub/local.cmake",
        "sub/other.cmake",
        "sub/x.tmpl.cmake",
        "build/keep.cmake",
    ];
    for file in files {
        std::fs::write(root.join(file), unformatted).expect("Failed to write file");
    }

    // (walk root, the files it is responsible for)
    let scopes: [(&str, &[&str]); 2] = [
        (".", &files),
        (
            "sub",
            &["sub/local.cmake", "sub/other.cmake", "sub/x.tmpl.cmake"],
        ),
    ];

    for (walk_root, scope) in scopes {
        let walk = Command::new(cmake_fmt_bin())
            .args(["-r", "--check", walk_root, "--ignore-file", "ig.txt"])
            .current_dir(root)
            .output()
            .expect("Failed to run walk");
        let walk_report = String::from_utf8_lossy(&walk.stderr).to_string()
            + &String::from_utf8_lossy(&walk.stdout);

        for file in scope {
            let walk_sees = walk_report.contains(file);
            let target = root.join(file);
            let (_, stdout) = run_with_stdin_in(
                Some(root),
                &[
                    "-",
                    "--assume-filename",
                    target.to_str().unwrap(),
                    "--ignore-file",
                    "ig.txt",
                ],
                unformatted,
            );
            let stdin_formats = stdout != unformatted;

            assert_eq!(
                walk_sees, stdin_formats,
                "walk from {} and stdin disagree about {}: walk_sees={}, stdin_formats={}\nwalk report:\n{}",
                walk_root, file, walk_sees, stdin_formats, walk_report
            );
        }
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
    // Asserting only the absence of a substring passes on a crash, an abort or
    // an argument error too — the silent-no-op that reads as success, which is
    // the failure this whole branch is about
    assert_eq!(
        inside.status.code(),
        Some(0),
        "the run did not succeed:\n{}",
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
    assert_eq!(named.status.code(), Some(0), "the run did not succeed");

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
    assert_eq!(
        normal.status.code(),
        Some(1),
        "an unformatted file was found"
    );
}

#[test]
fn test_ignore_file_pattern_matching_an_ancestor_does_not_disable_the_walk() {
    // Gitignore matches a pattern with no slash against a path's basename, so
    // an ordinary pattern like `build/` matches any `build` component of an
    // absolute path — including one above the walk root, which nothing in the
    // project asked to exclude. Consulting --ignore-file for every ancestor
    // therefore excluded the walk root itself and the whole run silently found
    // nothing, and `--check` exited 0 having checked nothing, which in CI reads
    // as pass.
    //
    // The tree is deliberately nested under a directory named `build` so the
    // ancestor match is there on every platform, rather than relying on the
    // system temp directory happening to be called `/tmp`.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let work = tempdir.path().join("build").join("proj");
    std::fs::create_dir_all(work.join("src")).expect("Failed to create dirs");

    // Patterns from an ordinary .gitignore, naming nothing inside this project
    std::fs::write(work.join("ig.txt"), "build/\n*.o\n").expect("Failed to write ignore");

    let unformatted = "set(FOO   bar)\n";
    std::fs::write(work.join("CMakeLists.txt"), unformatted).expect("Failed to write file");
    std::fs::write(work.join("src").join("a.cmake"), unformatted).expect("Failed to write file");

    let output = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", ".", "--ignore-file", "ig.txt"])
        .current_dir(&work)
        .output()
        .expect("Failed to run walk");
    let report = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);

    assert!(
        report.contains("CMakeLists.txt") && report.contains("a.cmake"),
        "an --ignore-file pattern matching an ancestor disabled the walk:\n{}",
        report
    );
    // The exit code is the half CI reads: a run that checks nothing passes
    assert_eq!(
        output.status.code(),
        Some(1),
        "--check found unformatted files but did not report failure:\n{}",
        report
    );

    // The same file still excludes what it actually names
    std::fs::write(work.join("ig.txt"), "src/\n").expect("Failed to write ignore");
    let output = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", ".", "--ignore-file", "ig.txt"])
        .current_dir(&work)
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

#[test]
fn test_ignore_file_excludes_a_directory_named_as_the_walk_root() {
    // The walk never tests its own root as an entry, so a root named on the
    // command line has to be checked up front — with --ignore-file included, or
    // `cmake-fmt -r build` formats the very files that `--assume-filename` under
    // `build/` refuses to touch.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    std::fs::create_dir(root.join("build")).expect("Failed to create build dir");
    std::fs::write(root.join("ig.txt"), "build/\n").expect("Failed to write ignore");

    let unformatted = "set(FOO   bar)\n";
    std::fs::write(root.join("build").join("a.cmake"), unformatted).expect("Failed to write file");
    std::fs::write(root.join("keep.cmake"), unformatted).expect("Failed to write file");

    let named = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", "build", "--ignore-file", "ig.txt"])
        .current_dir(root)
        .output()
        .expect("Failed to run walk");
    let report = String::from_utf8_lossy(&named.stdout).to_string()
        + &String::from_utf8_lossy(&named.stderr);
    assert!(
        !report.contains("a.cmake"),
        "naming an --ignore-file'd directory as the walk root should skip it:\n{}",
        report
    );
    assert_eq!(
        named.status.code(),
        Some(0),
        "nothing to check:\n{}",
        report
    );

    // The stdin path already reached that verdict; this is the agreement.
    // Both run from `root`, because --ignore-file is rooted at the working
    // directory — see test_extra_ignore_file_is_rooted_at_the_working_directory.
    let (_, stdout) = run_with_stdin_in(
        Some(root),
        &[
            "-",
            "--assume-filename",
            root.join("build").join("a.cmake").to_str().unwrap(),
            "--ignore-file",
            "ig.txt",
        ],
        unformatted,
    );
    assert_eq!(
        stdout, unformatted,
        "stdin should pass an excluded file through"
    );

    // And the rest of the tree is untouched by the new root check
    let whole = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", ".", "--ignore-file", "ig.txt"])
        .current_dir(root)
        .output()
        .expect("Failed to run walk");
    let whole_report = String::from_utf8_lossy(&whole.stdout).to_string()
        + &String::from_utf8_lossy(&whole.stderr);
    assert!(
        whole_report.contains("keep.cmake") && !whole_report.contains("a.cmake"),
        "the walk from the project root should still see the rest:\n{}",
        whole_report
    );
}

#[test]
fn test_ignore_file_allowlist_idiom_survives_the_root_check() {
    // The allowlist idiom (`*` plus a re-inclusion) matches the working
    // directory itself with `*`. Since the root check now consults
    // --ignore-file, a matcher that decided about its own root would drop the
    // whole tree before any re-inclusion could apply.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    std::fs::write(root.join("ig.txt"), "*\n!*.cmake\n").expect("Failed to write ignore");

    let unformatted = "set(FOO   bar)\n";
    std::fs::write(root.join("a.cmake"), unformatted).expect("Failed to write file");
    std::fs::write(root.join("notes.txt"), "whatever\n").expect("Failed to write file");

    let output = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", ".", "--ignore-file", "ig.txt"])
        .current_dir(root)
        .output()
        .expect("Failed to run walk");
    let report = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);

    assert!(
        report.contains("a.cmake"),
        "the allowlist idiom's re-inclusion never got a chance:\n{}",
        report
    );

    // The stdin path has to agree, for the same reason
    let (_, stdout) = run_with_stdin_in(
        Some(root),
        &[
            "-",
            "--assume-filename",
            root.join("a.cmake").to_str().unwrap(),
            "--ignore-file",
            "ig.txt",
        ],
        unformatted,
    );
    assert_eq!(stdout, "set(FOO bar)\n", "stdin dropped a re-included file");
}

#[test]
fn test_cmake_fmt_ignore_allowlist_idiom_survives_the_root_check() {
    // The mirror of test_ignore_file_allowlist_idiom_survives_the_root_check,
    // for the `.cmake-fmt-ignore` chain. Both halves of the rule that protects
    // it — an ignore file has no say about its own directory, and it governs
    // only what lies beneath it — could be deleted with the whole suite still
    // green, and each one alone makes the stdin path skip a file the walk
    // formats.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    std::fs::write(root.join(".cmake-fmt-ignore"), "*\n!*.cmake\n").expect("Failed to write");

    let unformatted = "set(FOO   bar)\n";
    std::fs::write(root.join("a.cmake"), unformatted).expect("Failed to write file");
    std::fs::write(root.join("notes.txt"), "whatever\n").expect("Failed to write file");

    let walk = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", "."])
        .current_dir(root)
        .output()
        .expect("Failed to run walk");
    let report =
        String::from_utf8_lossy(&walk.stdout).to_string() + &String::from_utf8_lossy(&walk.stderr);
    assert!(
        report.contains("a.cmake"),
        "the walk lost the re-included file:\n{}",
        report
    );

    let (_, stdout) = run_with_stdin_in(
        Some(root),
        &[
            "-",
            "--assume-filename",
            root.join("a.cmake").to_str().unwrap(),
        ],
        unformatted,
    );
    assert_eq!(stdout, "set(FOO bar)\n", "stdin dropped a re-included file");
}

#[test]
#[cfg(unix)]
fn test_a_symlinked_walk_root_is_still_excluded() {
    // The walk canonicalizes when it reads the ignore files above an entry, so
    // a root reached through a symlink has to be resolved before it is checked
    // — otherwise `cmake-fmt -r link` formats files that both the same walk
    // from the project root and the stdin path skip.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    let inner = root.join("proj").join("vendor").join("inner");
    std::fs::create_dir_all(&inner).expect("Failed to create dirs");
    std::fs::write(root.join("proj").join(".cmake-fmt-ignore"), "vendor/\n")
        .expect("Failed to write ignore");
    let unformatted = "set(FOO   bar)\n";
    std::fs::write(inner.join("a.cmake"), unformatted).expect("Failed to write file");

    std::os::unix::fs::symlink(&inner, root.join("link")).expect("Failed to symlink");

    let output = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", "link"])
        .current_dir(root)
        .output()
        .expect("Failed to run walk");
    let report = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);

    assert!(
        !report.contains("Would reformat"),
        "a symlink walked into an excluded directory:\n{}",
        report
    );
    assert_eq!(output.status.code(), Some(0), "{}", report);
}

#[test]
fn test_an_unreadable_ignore_file_fails_the_run() {
    // A typo used to warn and then format everything the user meant to exclude,
    // exiting 0 — the outcome this machinery exists to prevent, and it looked
    // like a clean run. `is_file()` is not the question to ask, either: it is
    // true for a file whose mode forbids reading it, which left that same path
    // intact.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    std::fs::create_dir(root.join("src")).expect("Failed to create src dir");
    std::fs::write(root.join("src").join("a.cmake"), "set(FOO   bar)\n").expect("write");

    let refused = |args: &[&str]| -> (Option<i32>, String) {
        let output = Command::new(cmake_fmt_bin())
            .args(args)
            .current_dir(root)
            .output()
            .expect("Failed to run cmake-fmt");
        (
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        )
    };

    for arg in ["typo.txt", "."] {
        let (code, stderr) = refused(&["-r", "--check", ".", "--ignore-file", arg]);
        assert_eq!(code, Some(1), "{:?} should fail the run: {}", arg, stderr);
        assert!(
            stderr.contains("--ignore-file"),
            "no diagnostic for {:?}: {}",
            arg,
            stderr
        );
    }

    // A file that exists but cannot be read is the case `is_file()` misses
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let unreadable = root.join("ig.txt");
        std::fs::write(&unreadable, "src/\n").expect("write");
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000))
            .expect("chmod");
        // Root ignores the mode, so only assert when the probe really fails
        if std::fs::File::open(&unreadable).is_err() {
            let (code, stderr) = refused(&["-r", "--check", ".", "--ignore-file", "ig.txt"]);
            assert_eq!(
                code,
                Some(1),
                "an unreadable ignore file should fail the run"
            );
            assert!(
                stderr.contains("cannot be read"),
                "no diagnostic: {}",
                stderr
            );
        }
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o644))
            .expect("chmod back");
    }

    // And the shapes that must keep working: `--ignore-file /dev/null` is the
    // standard way to disable one in a script, and the check must not reject it
    // for not being a regular file
    #[cfg(unix)]
    {
        let (_, stderr) = refused(&["-r", "--check", ".", "--ignore-file", "/dev/null"]);
        assert!(
            !stderr.contains("error:"),
            "/dev/null was rejected: {}",
            stderr
        );
    }

    // The stdin path agrees about the typo
    let (ok, _) = run_with_stdin_in(
        Some(root),
        &[
            "-",
            "--assume-filename",
            root.join("src").join("a.cmake").to_str().unwrap(),
            "--ignore-file",
            "typo.txt",
        ],
        "set(FOO   bar)\n",
    );
    assert!(!ok, "stdin should fail on an unreadable --ignore-file");
}

#[test]
#[cfg(unix)]
fn test_a_symlinked_path_reaches_the_same_verdict_either_way() {
    // The walk resolves symlinks for every entry whose ignore files it reads, so
    // a path handed to the stdin path has to be resolved the same way. Round one
    // canonicalized the walk-root check only, which made the two disagree in
    // both directions — and format-on-save is exactly where an unresolved
    // spelling arrives, because the editor hands over the path the user opened.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path().join("proj");
    std::fs::create_dir_all(root.join("excluded").join("inner")).expect("dirs");
    std::fs::create_dir(root.join("src")).expect("dirs");
    std::fs::write(root.join(".cmake-fmt-ignore"), "excluded/\n").expect("write");

    let unformatted = "set(FOO   bar)\n";
    std::fs::write(
        root.join("excluded").join("inner").join("b.cmake"),
        unformatted,
    )
    .expect("write");
    std::fs::write(root.join("src").join("a.cmake"), unformatted).expect("write");
    // Into the excluded tree, and back out of it
    std::os::unix::fs::symlink("excluded/inner", root.join("link-to-inner")).expect("symlink");
    std::os::unix::fs::symlink("../src", root.join("excluded").join("link-to-src"))
        .expect("symlink");

    for (target, should_format) in [
        ("link-to-inner/b.cmake", false),
        ("excluded/link-to-src/a.cmake", true),
    ] {
        let (_, stdout) = run_with_stdin_in(
            Some(&root),
            &["-", "--assume-filename", target],
            unformatted,
        );
        assert_eq!(
            stdout != unformatted,
            should_format,
            "stdin reached the wrong verdict for {}",
            target
        );

        let walk = Command::new(cmake_fmt_bin())
            .args([
                "-r",
                "--check",
                target.rsplit_once('/').expect("has a directory").0,
            ])
            .current_dir(&root)
            .output()
            .expect("Failed to run walk");
        let report = String::from_utf8_lossy(&walk.stdout).to_string()
            + &String::from_utf8_lossy(&walk.stderr);
        assert_eq!(
            report.contains("Would reformat"),
            should_format,
            "walk and stdin disagree about {}:\n{}",
            target,
            report
        );
    }
}

#[test]
fn test_cmake_fmt_ignore_outranks_the_extra_ignore_file() {
    // `--ignore-file` ranks below the `.cmake-fmt-ignore` chain, matching
    // WalkBuilder::add_ignore (ignore's `matched_ignore` consults the custom
    // ignore first). Nothing pinned the ordering, so consulting the extra file
    // first left the whole suite green.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    std::fs::write(root.join(".cmake-fmt-ignore"), "!keep.cmake\n").expect("write");
    std::fs::write(root.join("xignore"), "keep.cmake\n").expect("write");

    let unformatted = "set(FOO   bar)\n";
    std::fs::write(root.join("keep.cmake"), unformatted).expect("write");

    // The nearer file re-includes it, so it is formatted despite --ignore-file
    let (_, stdout) = run_with_stdin_in(
        Some(root),
        &[
            "-",
            "--assume-filename",
            root.join("keep.cmake").to_str().unwrap(),
            "--ignore-file",
            "xignore",
        ],
        unformatted,
    );
    assert_eq!(
        stdout, "set(FOO bar)\n",
        "--ignore-file outranked the .cmake-fmt-ignore chain"
    );

    let walk = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", ".", "--ignore-file", "xignore"])
        .current_dir(root)
        .output()
        .expect("Failed to run walk");
    let report =
        String::from_utf8_lossy(&walk.stdout).to_string() + &String::from_utf8_lossy(&walk.stderr);
    assert!(
        report.contains("keep.cmake"),
        "the walk disagrees about precedence:\n{}",
        report
    );
}

#[test]
fn test_ignore_file_ancestors_are_bounded_by_the_working_directory() {
    // Pins the approximation `is_ignored` makes, so it is a decision rather
    // than an accident. The right boundary for --ignore-file is the walk root;
    // the stdin path has none, so the working directory stands in for it. A
    // target outside the working directory therefore has its ancestors left
    // unconsulted, and this run formats a file that a walk rooted next to it
    // would skip. Erring this way keeps a stray pattern from disabling a whole
    // run, which is the failure that reads as success.
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let root = tempdir.path();
    let elsewhere = root.join("elsewhere");
    std::fs::create_dir_all(elsewhere.join("vendor")).expect("Failed to create dirs");
    std::fs::create_dir(root.join("work")).expect("Failed to create work dir");
    std::fs::write(root.join("work").join("ig.txt"), "vendor/\n").expect("Failed to write");

    let unformatted = "set(FOO   bar)\n";
    let target = elsewhere.join("vendor").join("a.cmake");
    std::fs::write(&target, unformatted).expect("Failed to write file");

    // From `work`, `elsewhere/vendor` is not under the working directory
    let (_, stdout) = run_with_stdin_in(
        Some(&root.join("work")),
        &[
            "-",
            "--assume-filename",
            target.to_str().unwrap(),
            "--ignore-file",
            "ig.txt",
        ],
        unformatted,
    );
    assert_eq!(
        stdout, "set(FOO bar)\n",
        "documented approximation changed: an ancestor outside the working \
         directory is not consulted for --ignore-file"
    );

    // From the directory that contains it, the same pattern does apply
    let ignore_from_root = root.join("work").join("ig.txt");
    std::fs::copy(&ignore_from_root, root.join("ig.txt")).expect("copy");
    let (_, stdout) = run_with_stdin_in(
        Some(root),
        &[
            "-",
            "--assume-filename",
            target.to_str().unwrap(),
            "--ignore-file",
            "ig.txt",
        ],
        unformatted,
    );
    assert_eq!(
        stdout, unformatted,
        "an ancestor under the cwd must be consulted"
    );
}
