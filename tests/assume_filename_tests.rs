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
