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
