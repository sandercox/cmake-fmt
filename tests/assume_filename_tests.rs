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

    // Positive control: an absent substring and exit 0 are also what a run that
    // walks nothing produces. Turning the walk off entirely left the assertions
    // above green, so a file that must be reported goes through the same
    // symlink.
    let outside = root.join("proj").join("outside");
    std::fs::create_dir_all(&outside).expect("dirs");
    std::fs::write(outside.join("o.cmake"), unformatted).expect("write");
    std::os::unix::fs::symlink(&outside, root.join("link_out")).expect("symlink");
    let output = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", "link_out"])
        .current_dir(root)
        .output()
        .expect("Failed to run walk");
    let report = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(
        report.contains("Would reformat"),
        "the control file was not reached, so the walk never ran:\n{}",
        report
    );
    assert_eq!(output.status.code(), Some(1), "{}", report);
}

#[test]
fn test_an_unreadable_ignore_file_fails_the_run() {
    // A typo used to be ignored outright — the walk dropped the error — and then
    // format everything the user meant to exclude, which is the outcome this
    // machinery exists to prevent and which looked like a clean run. `is_file()`
    // is not the question to ask, either: it is true for a file whose mode
    // forbids reading it, which left that same path intact.
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

#[test]
#[cfg(unix)]
fn test_a_walk_roots_spelling_does_not_change_the_verdict() {
    // The ignore crate matches an --ignore-file pattern against the path as
    // spelled, while canonicalizing for the ignore files it reads above an
    // entry. So an anchored pattern excluded `-r sub` and not `-r link-to-sub`
    // or `-r sub/../sub` — three verdicts for one directory, and the stdin path
    // (which resolves) could not agree with all three.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::create_dir_all(root.join("sub").join("deep")).expect("dirs");
    std::fs::write(root.join("ignorefile"), "sub/deep/**\n").expect("write");
    let unformatted = "set(A   b)\n";
    std::fs::write(root.join("sub").join("deep").join("d.cmake"), unformatted).expect("write");
    std::os::unix::fs::symlink("sub", root.join("link_sub")).expect("symlink");

    for spelling in ["sub", "link_sub", "sub/../sub", "./sub"] {
        let output = Command::new(cmake_fmt_bin())
            .args(["-r", "--check", spelling, "--ignore-file", "ignorefile"])
            .current_dir(root)
            .output()
            .expect("run");
        let report = String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr);
        assert!(
            !report.contains("Would reformat"),
            "walk root spelled {:?} disagreed:\n{}",
            spelling,
            report
        );
    }

    // Positive control: every assertion above is the absence of a substring,
    // which a run that walks nothing satisfies too — turning the walk off left
    // them green. A file outside the excluded pattern must be reported under
    // each spelling.
    std::fs::write(root.join("sub").join("kept.cmake"), unformatted).expect("write");
    for spelling in ["sub", "link_sub", "sub/../sub", "./sub"] {
        let output = Command::new(cmake_fmt_bin())
            .args(["-r", "--check", spelling, "--ignore-file", "ignorefile"])
            .current_dir(root)
            .output()
            .expect("run");
        let report = String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr);
        assert!(
            report.contains("kept.cmake"),
            "walk root spelled {:?} reached nothing at all:\n{}",
            spelling,
            report
        );
        assert!(
            !report.contains("d.cmake"),
            "walk root spelled {:?} formatted the excluded file:\n{}",
            spelling,
            report
        );
    }

    // And the stdin path agrees, through either spelling
    for target in ["sub/deep/d.cmake", "link_sub/deep/d.cmake"] {
        let (_, stdout) = run_with_stdin_in(
            Some(root),
            &[
                "-",
                "--assume-filename",
                target,
                "--ignore-file",
                "ignorefile",
            ],
            unformatted,
        );
        assert_eq!(stdout, unformatted, "stdin formatted {}", target);
    }
}

#[test]
#[cfg(unix)]
fn test_an_endless_ignore_file_is_refused_rather_than_read() {
    // `GitignoreBuilder::add` reads the whole file, and a line-less device like
    // /dev/zero never ends, so the process aborted trying to hold it. The probe
    // is bounded now. /dev/null is the same kind of file and must still work.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::write(root.join("a.cmake"), "set(A   b)\n").expect("write");

    let output = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", ".", "--ignore-file", "/dev/zero"])
        .current_dir(root)
        .output()
        .expect("run");
    assert_eq!(
        output.status.code(),
        Some(1),
        "an endless ignore file should be refused"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("not an ignore file"),
        "no diagnostic: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new(cmake_fmt_bin())
        .args(["-r", "--check", ".", "--ignore-file", "/dev/null"])
        .current_dir(root)
        .output()
        .expect("run");
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("error:"),
        "/dev/null was refused: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_an_ignored_file_is_passed_through_as_bytes() {
    // The passthrough decoded the buffer it only needed to copy, so a file the
    // ignore rules said to leave alone was rejected for not being UTF-8 — an
    // error about a file nobody asked us to look at, on every save.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::write(root.join(".cmake-fmt-ignore"), "a.cmake\n").expect("write");

    let mut child = Command::new(cmake_fmt_bin())
        .args(["-", "--assume-filename", "a.cmake"])
        .current_dir(root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");
    let invalid = [0xffu8, 0xfe, 0x00, b'b', b'a', b'd'];
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(&invalid)
        .expect("write");
    let output = child.wait_with_output().expect("wait");

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, invalid, "the bytes were not passed through");
}

#[test]
fn test_diff_mode_on_an_ignored_file_reports_nothing() {
    // The mode split inside the passthrough had no test: --diff must emit no
    // diff for a file the ignore rules exclude, and still succeed.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::write(root.join(".cmake-fmt-ignore"), "a.cmake\n").expect("write");

    let (ok, stdout) = run_with_stdin_in(
        Some(root),
        &["-", "--assume-filename", "a.cmake", "--diff"],
        "set(A   b)\n",
    );
    assert!(ok, "--diff on an ignored file should succeed");
    assert!(stdout.is_empty(), "unexpected diff output: {:?}", stdout);
}

#[test]
fn test_interactive_validates_the_ignore_file() {
    // The check sits before the interactive early return, so every mode
    // validates the argument. Nothing covered that ordering.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::write(root.join("a.cmake"), "set(A   b)\n").expect("write");

    let output = Command::new(cmake_fmt_bin())
        .args(["--interactive", "a.cmake", "--ignore-file", "typo.txt"])
        .current_dir(root)
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--ignore-file"),
        "no diagnostic: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[cfg(unix)]
fn test_reported_paths_use_the_spelling_the_user_gave() {
    // The walk runs on resolved roots so its verdicts do not depend on how a
    // directory was named, but every message has to name the directory the user
    // typed. Nothing asserted that, and the `..`-through-a-symlink case is also
    // what catches a resolver that collapses `..` as text: `link/..` is the
    // parent of link's *target*, not of the directory link sits in.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::create_dir_all(root.join("outer").join("inner")).expect("dirs");
    std::fs::create_dir(root.join("outer").join("other")).expect("dirs");
    let unformatted = "set(A   b)\n";
    std::fs::write(
        root.join("outer").join("inner").join("f.cmake"),
        unformatted,
    )
    .expect("write");
    std::os::unix::fs::symlink("outer/other", root.join("link")).expect("symlink");

    for spelling in ["outer/inner", "./outer/inner", "link/../inner"] {
        let output = Command::new(cmake_fmt_bin())
            .args(["-r", "--check", spelling])
            .current_dir(root)
            .output()
            .expect("run");
        let report = String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr);
        assert!(
            report.contains(&format!("{}/f.cmake", spelling)),
            "the report should name the path as {:?} was spelled:\n{}",
            spelling,
            report
        );
        assert_eq!(
            output.status.code(),
            Some(1),
            "the file needs formatting and should be found:\n{}",
            report
        );
    }
}

/// Run a command to completion, or give up on it.
///
/// A regression in the one-shot `--ignore-file` handling makes the tool reopen a
/// source that is already spent and block there for ever, and `cargo test` has
/// no per-test timeout — so a test that only asserts on the output turns a red
/// build into a hung one. Both regressions this file exists to catch behave that
/// way, so every such run gets a deadline and `None` means it blew it.
///
/// Output goes to files rather than pipes: a killed child leaves its pipe half
/// written, and reading one after the fact can block as well.
#[cfg(unix)]
fn run_with_deadline(command: &mut Command, seconds: u64) -> Option<String> {
    let logs = TempDir::new().expect("log tempdir");
    let out = logs.path().join("stdout");
    let err = logs.path().join("stderr");
    let mut child = command
        .stdout(std::fs::File::create(&out).expect("stdout"))
        .stderr(std::fs::File::create(&err).expect("stderr"))
        .spawn()
        .expect("spawn");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    loop {
        match child.try_wait().expect("try_wait") {
            Some(_) => break,
            None if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(std::time::Duration::from_millis(25)),
        }
    }

    Some(
        std::fs::read_to_string(&out).unwrap_or_default()
            + &std::fs::read_to_string(&err).unwrap_or_default(),
    )
}

#[test]
#[cfg(unix)]
fn test_a_terminal_ignore_file_is_read_once() {
    // A character device was probed by the readability check — drained into
    // `sink()` to measure it — and then reopened by the copy, which found
    // nothing. `/dev/null` and `/dev/zero` survive being read twice, which is
    // why every test passed; a terminal does not, so `--ignore-file /dev/stdin`
    // from an editor or a shell lost every pattern and reformatted the files the
    // author had excluded, with no diagnostic. The one-shot source is now taken
    // once and the check runs on the copy.
    //
    // `script` gives the child a pty, which is the character device anyone
    // actually passes here.
    if !Command::new("sh")
        .args(["-c", "command -v script >/dev/null"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return; // no `script` on this platform
    }

    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::create_dir(root.join("build")).expect("dirs");
    let unformatted = "set(A   b)\n";
    std::fs::write(root.join("kept.cmake"), unformatted).expect("write");
    std::fs::write(root.join("build").join("b.cmake"), unformatted).expect("write");

    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(format!(
            "printf 'build/\\n' | script -qec '{} --ignore-file /dev/stdin --check -r .' /dev/null",
            cmake_fmt_bin()
        ))
        .current_dir(root);
    // Before the fix this reopened the spent pty and blocked, so a deadline is
    // what makes the regression a failure rather than a hung suite.
    let report = run_with_deadline(&mut command, 30).expect("the run blocked on a spent stream");

    assert!(
        report.contains("kept.cmake"),
        "the walk reached nothing at all:\n{}",
        report
    );
    assert!(
        !report.contains("b.cmake"),
        "the pattern from the terminal was lost:\n{}",
        report
    );
}

#[test]
#[cfg(unix)]
fn test_an_over_bound_one_shot_ignore_file_is_refused() {
    // A regular file over the bound is refused. A one-shot source is never
    // probed, so it used to be truncated at the bound instead — silently, and
    // the cut could land mid-line and turn the last pattern into a different
    // one. The copy now takes one byte more than the bound so the two cases can
    // be told apart, and refuses.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::write(root.join("a.cmake"), "set(A   b)\n").expect("write");

    let fifo = root.join("patterns");
    let made = Command::new("sh")
        .arg("-c")
        .arg(format!("mkfifo {}", fifo.display()))
        .status()
        .expect("mkfifo");
    if !made.success() {
        return; // no mkfifo on this platform
    }

    // 1 MiB of comment, then a pattern past the bound
    let feeder = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "{{ for i in $(seq 1 20000); do printf '# padpadpadpadpadpadpadpadpadpadpadpadpadpadpadpadpad\\n'; done; printf 'a.cmake\\n'; }} > {} 2>/dev/null",
            fifo.display()
        ))
        .spawn()
        .expect("spawn writer");

    let fifo_str = fifo.to_str().unwrap().to_string();
    let mut command = Command::new(cmake_fmt_bin());
    command
        .args(["--check", "-r", ".", "--ignore-file", &fifo_str])
        .current_dir(root);
    let report = run_with_deadline(&mut command, 60).expect("the run blocked");
    let mut feeder = feeder;
    let _ = feeder.kill();
    let _ = feeder.wait();

    assert!(
        report.contains("is larger than 1024 KiB"),
        "an over-bound one-shot source should be refused, not truncated:\n{}",
        report
    );
    assert!(
        report.contains(&fifo_str),
        "the refusal should name the argument, not the temporary copy:\n{}",
        report
    );
}

#[test]
fn test_a_directory_only_pattern_does_not_match_a_file() {
    // The stdin target is matched as a file, and only that keeps a
    // directory-only pattern — a trailing `/`, which is how `build/` is written
    // — from excluding a file of the same name. Matching it as a directory
    // instead survived every test.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::write(root.join("ig.txt"), "x.cmake/\n").expect("write");
    let unformatted = "set(FOO   bar)\n";

    let (_, stdout) = run_with_stdin_in(
        Some(root),
        &[
            "-",
            "--assume-filename",
            "x.cmake",
            "--ignore-file",
            "ig.txt",
        ],
        unformatted,
    );
    assert_eq!(
        stdout, "set(FOO bar)\n",
        "a directory-only pattern excluded a file"
    );

    // And the same name as a directory *is* matched, so the pattern works
    std::fs::create_dir(root.join("x.cmake")).expect("dirs");
    std::fs::write(root.join("x.cmake").join("a.cmake"), unformatted).expect("write");
    let output = Command::new(cmake_fmt_bin())
        .args(["--check", "-r", ".", "--ignore-file", "ig.txt"])
        .current_dir(root)
        .output()
        .expect("run");
    let report = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(
        !report.contains("a.cmake"),
        "the directory the pattern names was walked anyway:\n{}",
        report
    );
}

#[test]
fn test_an_excluded_named_root_says_so() {
    // The only signal that a run deliberately did nothing. Deleting the line
    // left every test green, and a silent "0 files" reads as "nothing to do"
    // rather than "everything you asked about is excluded".
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::create_dir(root.join("build")).expect("dirs");
    std::fs::write(root.join("build").join("b.cmake"), "set(A   b)\n").expect("write");
    std::fs::write(root.join("ig.txt"), "build/\n").expect("write");

    let output = Command::new(cmake_fmt_bin())
        .args(["--check", "-r", "build", "--ignore-file", "ig.txt"])
        .current_dir(root)
        .output()
        .expect("run");
    let report = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(
        report.contains("Skipping build: excluded by an ignore file"),
        "an excluded root should say so:\n{}",
        report
    );
    assert!(!report.contains("b.cmake"), "{}", report);
}

#[test]
fn test_verbose_names_an_ignored_stdin_target() {
    // The `--verbose` line for an ignored target had no test at all, and it is
    // the only way to find out why a buffer came back untouched.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::write(root.join(".cmake-fmt-ignore"), "x.cmake\n").expect("write");

    let mut command = Command::new(cmake_fmt_bin());
    command
        .args(["-", "--assume-filename", "x.cmake", "--verbose"])
        .current_dir(root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"set(FOO   bar)\n")
        .expect("write");
    let output = child.wait_with_output().expect("wait");
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        stderr.contains("is ignored, skipping"),
        "verbose said nothing about an ignored target:\n{}",
        stderr
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "set(FOO   bar)\n",
        "an ignored target should come back untouched"
    );
}

#[test]
fn test_an_unreadable_ignore_file_reaching_the_walk_is_reported() {
    // A `--ignore-file` that opens and reads but is not text passes the
    // readability check and then fails inside the matcher. Discarding that error
    // is how a broken ignore file used to reach the walk with no diagnostic at
    // all, so the walk says so loudly. Nothing pinned the line.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::write(root.join("a.cmake"), "set(A   b)\n").expect("write");
    std::fs::write(root.join("ig.bin"), b"# fine\n\xff\xfe not text\n").expect("write");

    let output = Command::new(cmake_fmt_bin())
        .args(["--check", "-r", ".", "--ignore-file", "ig.bin"])
        .current_dir(root)
        .output()
        .expect("run");
    let report = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(
        report.contains("did not contain valid UTF-8"),
        "a matcher that refused the ignore file said nothing:\n{}",
        report
    );
    // Once, naming the file the user typed. It used to be printed once per
    // named root plus once more by the walk, with the path repeated inside the
    // sentence.
    assert_eq!(
        report.matches("did not contain valid UTF-8").count(),
        1,
        "the same complaint was printed more than once:\n{}",
        report
    );
    assert!(
        report.contains("Warning: ig.bin: line 2:"),
        "the complaint should name the file as it was given:\n{}",
        report
    );

    // And still once when several roots each read it
    std::fs::create_dir_all(root.join("x")).expect("dirs");
    std::fs::create_dir_all(root.join("y")).expect("dirs");
    std::fs::write(root.join("x").join("c.cmake"), "set(A   b)\n").expect("write");
    std::fs::write(root.join("y").join("d.cmake"), "set(A   b)\n").expect("write");
    let output = Command::new(cmake_fmt_bin())
        .args(["--check", "-r", "x", "y", ".", "--ignore-file", "ig.bin"])
        .current_dir(root)
        .output()
        .expect("run");
    let report = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        report.matches("did not contain valid UTF-8").count(),
        1,
        "one complaint however many roots read it:\n{}",
        report
    );
}

#[test]
fn test_a_broken_cmake_fmt_ignore_is_reported_once() {
    // The only diagnostic for a `.cmake-fmt-ignore` whose patterns the matcher
    // will not take. Deleting it left the suite green, and it was printed once
    // per named root — the same duplication `--ignore-file` was cured of.
    //
    // It is reached only for an *ancestor* of what is being checked, because an
    // ignore file has no say about its own directory. So the file in the walk
    // root itself is validated by the walk, which keeps no error to print; that
    // gap is documented at `build_ignore_matcher` and asserted here so a change
    // to it is deliberate.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    for dir in ["sub", "sub/deep", "other"] {
        std::fs::create_dir_all(root.join(dir)).expect("dirs");
    }
    for f in [
        "y.cmake",
        "sub/y.cmake",
        "sub/deep/y.cmake",
        "other/y.cmake",
    ] {
        std::fs::write(root.join(f), "set(A   b)\n").expect("write");
    }
    // Line 2 is not UTF-8, so the matcher refuses the file part-way
    std::fs::write(
        root.join(".cmake-fmt-ignore"),
        b"x.cmake\n\xff\xfebad\ny.cmake\n",
    )
    .expect("write");

    let complaints = |args: &[&str]| -> usize {
        let output = Command::new(cmake_fmt_bin())
            .args(args)
            .current_dir(root)
            .output()
            .expect("run");
        let report = String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr);
        report.matches("did not contain valid UTF-8").count()
    };

    // One root above the ignore file's directory: reported, once
    assert_eq!(complaints(&["--check", "-r", "sub"]), 1);
    // Three of them: still once
    assert_eq!(
        complaints(&["--check", "-r", "sub", "other", "sub/deep"]),
        1,
        "the same complaint was printed per root"
    );
    // The walk root itself: the walk validates it and keeps no error, so nothing
    // is printed. Documented, not desirable.
    assert_eq!(complaints(&["--check", "-r", "."]), 0);
}

#[test]
fn test_a_complaint_names_the_ignore_file_as_it_was_given() {
    // A one-shot source is read from a temporary copy, so the matcher's own
    // message names a path nobody typed. `named_error` swaps it back, and two
    // separate mutations of that — returning the raw text, and passing the copy's
    // path as the name — both survived the whole suite.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::write(root.join("a.cmake"), "set(A   b)\n").expect("write");
    std::fs::write(root.join("pat.bin"), b"# ok\n\xff\xfebad\na.cmake\n").expect("write");

    let fifo = root.join("patterns");
    let made = Command::new("sh")
        .arg("-c")
        .arg(format!("mkfifo {}", fifo.display()))
        .status()
        .expect("mkfifo");
    if !made.success() {
        return; // no mkfifo on this platform
    }
    let feeder = Command::new("sh")
        .arg("-c")
        .arg(format!("cat pat.bin > {}", fifo.display()))
        .current_dir(root)
        .spawn()
        .expect("spawn writer");

    let fifo_str = fifo.to_str().unwrap().to_string();
    let mut command = Command::new(cmake_fmt_bin());
    command
        .args(["--check", "-r", ".", "--ignore-file", &fifo_str])
        .current_dir(root);
    let report = run_with_deadline(&mut command, 30).expect("the run blocked");
    let mut feeder = feeder;
    let _ = feeder.wait();

    assert!(
        report.contains("did not contain valid UTF-8"),
        "the complaint went missing:\n{}",
        report
    );
    assert!(
        report.contains(&format!("Warning: {}: line 2", fifo_str)),
        "the complaint should name the fifo the caller passed:\n{}",
        report
    );
    // The copy is a random `NamedTempFile` name, so naming the fifo — which the
    // assertion above pins exactly — is what rules the copy out. Asserting the
    // absence of a `/tmp/` prefix would not: the fifo itself lives in one.
}

#[test]
fn test_a_whitelisted_ancestor_is_not_an_excluded_one() {
    // An excluded ancestor is final — git cannot re-include a file whose parent
    // directory is excluded — but an ancestor a pattern explicitly *re-includes*
    // is not excluded, and treating any decision as exclusion skipped it.
    // Nothing pinned the difference between "matched" and "matched as ignored".
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::create_dir_all(root.join("sub")).expect("dirs");
    std::fs::write(root.join("sub").join("b.cmake"), "set(A   b)\n").expect("write");
    std::fs::write(root.join(".cmake-fmt-ignore"), "!sub/\n").expect("write");

    for args in [&["--check", "-r", "."][..], &["--check", "-r", "sub"][..]] {
        let output = Command::new(cmake_fmt_bin())
            .args(args)
            .current_dir(root)
            .output()
            .expect("run");
        let report = String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr);
        assert!(
            report.contains("b.cmake"),
            "a re-included directory was skipped for {:?}:\n{}",
            args,
            report
        );
    }

    // And the stdin path agrees
    let (_, stdout) = run_with_stdin_in(
        Some(root),
        &["-", "--assume-filename", "sub/b.cmake"],
        "set(A   b)\n",
    );
    assert_eq!(
        stdout, "set(A b)\n",
        "the stdin path skipped a re-included directory"
    );
}

#[test]
fn test_the_ignore_file_bound_is_a_mebibyte() {
    // Only the message was ever asserted, so the bound itself could be any
    // number. A file just under it is read and a file just over it is refused.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::write(root.join("a.cmake"), "set(A   b)\n").expect("write");

    // Each line is 8 bytes, so the pattern lands well inside the bound
    let line = "#123456\n";
    let under = line.repeat((1 << 20) / line.len() - 2) + "a.cmake\n";
    assert!(under.len() < (1 << 20));
    std::fs::write(root.join("under.txt"), &under).expect("write");
    let output = Command::new(cmake_fmt_bin())
        .args(["--check", "-r", ".", "--ignore-file", "under.txt"])
        .current_dir(root)
        .output()
        .expect("run");
    let report = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(
        !report.contains("a.cmake") && !report.contains("larger than"),
        "a file just under the bound should be read:\n{}",
        report
    );

    let over = line.repeat((1 << 20) / line.len() + 1);
    assert!(over.len() > (1 << 20));
    std::fs::write(root.join("over.txt"), &over).expect("write");
    let output = Command::new(cmake_fmt_bin())
        .args(["--check", "-r", ".", "--ignore-file", "over.txt"])
        .current_dir(root)
        .output()
        .expect("run");
    let report = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(
        report.contains("is larger than 1024 KiB"),
        "a file just over the bound should be refused:\n{}",
        report
    );
    assert_eq!(output.status.code(), Some(1), "{}", report);
}

#[test]
#[cfg(unix)]
fn test_a_one_shot_ignore_file_is_read_once() {
    // `--ignore-file` is read by the walk-root check, once per root, and again
    // by the walk. A fifo — which is what `<(...)` is — is empty by the second
    // read, so the first root examined ate the patterns and everything after it
    // was formatted, with the verdict depending on argument order.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::create_dir(root.join("a")).expect("dirs");
    std::fs::create_dir(root.join("b")).expect("dirs");
    let unformatted = "set(A   b)\n";
    std::fs::write(root.join("a").join("f.cmake"), unformatted).expect("write");
    std::fs::write(root.join("b").join("f.cmake"), unformatted).expect("write");
    // Positive control: the assertion below is the absence of a substring, which
    // a run that walks nothing satisfies too. `kept.cmake` is not excluded, so
    // it must be reported every time.
    std::fs::write(root.join("a").join("kept.cmake"), unformatted).expect("write");
    std::fs::write(root.join("b").join("kept.cmake"), unformatted).expect("write");

    // A fifo with a writer, which is what process substitution gives the tool
    let fifo = root.join("patterns");
    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("mkfifo {}", fifo.display()))
        .status()
        .expect("mkfifo");
    if !status.success() {
        return; // no mkfifo on this platform
    }

    for args in [
        vec!["a", "b", "--check"],
        vec!["b", "a", "--check"],
        vec!["-r", ".", "--check"],
    ] {
        // A writer that closes after one line; each run gets its own
        let feeder = Command::new("sh")
            .arg("-c")
            .arg(format!("printf 'f.cmake\\n' > {}", fifo.display()))
            .spawn()
            .expect("spawn writer");

        let mut full = args.clone();
        full.push("--ignore-file");
        let fifo_str = fifo.to_str().unwrap().to_string();
        full.push(&fifo_str);
        let mut command = Command::new(cmake_fmt_bin());
        command.args(&full).current_dir(root);
        // A second read of a fifo whose writer is gone blocks for ever, so the
        // regression this test exists to catch does not fail it — it hangs it,
        // and `cargo test` has no per-test timeout. The deadline is what turns
        // that back into a failure.
        let report = run_with_deadline(&mut command, 30).unwrap_or_else(|| {
            panic!(
                "{:?} blocked: the ignore file was read more than once",
                args
            )
        });
        let mut feeder = feeder;
        let _ = feeder.wait();

        assert!(
            !report.contains("f.cmake"),
            "{:?} formatted a file the ignore file excludes:\n{}",
            args,
            report
        );
        assert!(
            report.contains("kept.cmake"),
            "{:?} reached nothing at all, so the exclusion proves nothing:\n{}",
            args,
            report
        );
    }
}

#[test]
fn test_a_large_ignored_buffer_is_drained() {
    // check and diff modes emit nothing for an ignored file, but the pipe still
    // has to be drained or a writer larger than the pipe buffer dies of SIGPIPE.
    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path();
    std::fs::write(root.join(".cmake-fmt-ignore"), "a.cmake\n").expect("write");

    let mut child = Command::new(cmake_fmt_bin())
        .args(["-", "--assume-filename", "a.cmake", "--check"])
        .current_dir(root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");

    // Comfortably larger than a pipe buffer
    let big = "set(A   b)\n".repeat(200_000);
    let wrote = child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(big.as_bytes());
    assert!(wrote.is_ok(), "the writer was killed: {:?}", wrote);
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("wait");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty(), "--check should emit nothing");
}
