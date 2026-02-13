use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

// Helper to get the binary path
fn cmake_fmt_bin() -> String {
    env!("CARGO_BIN_EXE_cmake-fmt").to_string()
}

#[test]
fn test_line_ranges_single_range_stdin() {
    let input = "set(  FOO   bar)\nmessage(hello)\nset(  BAZ   qux)\n";

    let mut child = Command::new(cmake_fmt_bin())
        .arg("--line-ranges=1:1")
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

    let output = child.wait_with_output().expect("Failed to wait for command");

    assert!(output.status.success(), "Command should succeed");
    let result = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = result.lines().collect();

    // Line 1 should be formatted
    assert_eq!(lines[0], "set(FOO bar)", "Line 1 should be formatted");
    // Lines 2-3 should be unchanged
    assert_eq!(lines[1], "message(hello)", "Line 2 should be unchanged");
    assert_eq!(lines[2], "set(  BAZ   qux)", "Line 3 should be unchanged");
}

#[test]
fn test_line_ranges_multiple_ranges_stdin() {
    let input = "set(  A   1)\nset(  B   2)\nset(  C   3)\nset(  D   4)\nset(  E   5)\n";

    let mut child = Command::new(cmake_fmt_bin())
        .arg("--line-ranges=1:2,4:5")
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

    let output = child.wait_with_output().expect("Failed to wait for command");

    assert!(output.status.success(), "Command should succeed");
    let result = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = result.lines().collect();

    // Lines 1-2 should be formatted
    assert_eq!(lines[0], "set(A 1)", "Line 1 should be formatted");
    assert_eq!(lines[1], "set(B 2)", "Line 2 should be formatted");
    // Line 3 should be unchanged
    assert_eq!(lines[2], "set(  C   3)", "Line 3 should be unchanged");
    // Lines 4-5 should be formatted
    assert_eq!(lines[3], "set(D 4)", "Line 4 should be formatted");
    assert_eq!(lines[4], "set(E 5)", "Line 5 should be formatted");
}

#[test]
fn test_line_ranges_preserves_unselected_lines() {
    // Intentionally bad formatting on lines outside range
    let input = "set(  FOO   bar)\nmessage(   very    bad     formatting   )\nset(  BAZ   qux)\n";

    let mut child = Command::new(cmake_fmt_bin())
        .arg("--line-ranges=1:1,3:3")
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

    let output = child.wait_with_output().expect("Failed to wait for command");

    assert!(output.status.success(), "Command should succeed");
    let result = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = result.lines().collect();

    // Line 1 should be formatted
    assert_eq!(lines[0], "set(FOO bar)");
    // Line 2 should be UNCHANGED (preserve bad formatting)
    assert_eq!(lines[1], "message(   very    bad     formatting   )");
    // Line 3 should be formatted
    assert_eq!(lines[2], "set(BAZ qux)");
}

#[test]
fn test_line_ranges_with_file_input() {
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let cmake_file = tempdir.path().join("test.cmake");

    let content = "set(  FOO   bar)\nmessage(hello)\nset(  BAZ   qux)\n";
    std::fs::write(&cmake_file, content).expect("Failed to write test file");

    let output = Command::new(cmake_fmt_bin())
        .arg("--line-ranges=1:1")
        .arg("--check")
        .arg(&cmake_file)
        .output()
        .expect("Failed to run cmake-fmt");

    // File has unformatted content in range, so should exit 1
    assert_eq!(
        output.status.code(),
        Some(1),
        "Should exit with code 1 when changes needed"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Would reformat"),
        "Should show reformat message"
    );
}

#[test]
fn test_line_ranges_with_in_place() {
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let cmake_file = tempdir.path().join("test.cmake");

    let content = "set(  FOO   bar)\nmessage(hello)\nset(  BAZ   qux)\n";
    std::fs::write(&cmake_file, content).expect("Failed to write test file");

    let output = Command::new(cmake_fmt_bin())
        .arg("--line-ranges=1:1")
        .arg("-i")
        .arg(&cmake_file)
        .output()
        .expect("Failed to run cmake-fmt");

    assert!(output.status.success(), "Command should succeed");

    // Read file back
    let result = std::fs::read_to_string(&cmake_file).expect("Failed to read file");
    let lines: Vec<&str> = result.lines().collect();

    // Line 1 should be formatted
    assert_eq!(lines[0], "set(FOO bar)");
    // Lines 2-3 should be unchanged
    assert_eq!(lines[1], "message(hello)");
    assert_eq!(lines[2], "set(  BAZ   qux)");
}

#[test]
fn test_line_ranges_with_diff_mode() {
    let input = "set(  FOO   bar)\nmessage(hello)\nset(  BAZ   qux)\n";

    let mut child = Command::new(cmake_fmt_bin())
        .arg("--line-ranges=1:1")
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

    let output = child.wait_with_output().expect("Failed to wait for command");

    assert_eq!(
        output.status.code(),
        Some(1),
        "Should exit with code 1 for differences"
    );

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    // Diff should show change in line 1
    assert!(
        stdout_str.contains("-set(  FOO   bar)") || stdout_str.contains("FOO"),
        "Diff should show line 1 change"
    );
}

#[test]
fn test_line_ranges_with_assume_filename() {
    let tempdir = TempDir::new().expect("Failed to create tempdir");

    // Create a config file with indent_width = 2
    let config_path = tempdir.path().join(".cmake-fmt.toml");
    std::fs::write(&config_path, "indent_width = 2\nuse_tabs = false\n")
        .expect("Failed to write config");

    let input = "if(TRUE)\nset(  X   y)\nendif()\n";
    let assumed_file = tempdir.path().join("CMakeLists.txt");

    let mut child = Command::new(cmake_fmt_bin())
        .arg("--assume-filename")
        .arg(&assumed_file)
        .arg("--line-ranges=2:2")
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

    let output = child.wait_with_output().expect("Failed to wait for command");

    assert!(output.status.success(), "Command should succeed");
    let result = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = result.lines().collect();

    // Line 1 unchanged
    assert_eq!(lines[0], "if(TRUE)");
    // Line 2 formatted with 2-space indent (config respected)
    assert_eq!(lines[1], "  set(X y)");
    // Line 3 unchanged
    assert_eq!(lines[2], "endif()");
}

#[test]
fn test_line_ranges_invalid_format_error() {
    let output = Command::new(cmake_fmt_bin())
        .arg("--line-ranges=abc")
        .stdin(std::process::Stdio::piped())
        .output()
        .expect("Failed to run cmake-fmt");

    assert!(
        !output.status.success(),
        "Should exit with non-zero code"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Invalid range format"),
        "Should show format error"
    );
}

#[test]
fn test_line_ranges_inverted_range_error() {
    let output = Command::new(cmake_fmt_bin())
        .arg("--line-ranges=10:5")
        .stdin(std::process::Stdio::piped())
        .output()
        .expect("Failed to run cmake-fmt");

    assert!(
        !output.status.success(),
        "Should exit with non-zero code"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("start > end"),
        "Should show inverted range error"
    );
}

#[test]
fn test_line_ranges_out_of_bounds_clamp() {
    let input = "set(  A   1)\nset(  B   2)\nset(  C   3)\n";

    let mut child = Command::new(cmake_fmt_bin())
        .arg("--line-ranges=1:100")
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

    let output = child.wait_with_output().expect("Failed to wait for command");

    assert!(output.status.success(), "Should not crash on out-of-bounds range");
    let result = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = result.lines().collect();

    // All lines should be formatted (range clamped to file length)
    assert_eq!(lines[0], "set(A 1)");
    assert_eq!(lines[1], "set(B 2)");
    assert_eq!(lines[2], "set(C 3)");
}

#[test]
fn test_line_ranges_multiple_files_error() {
    let tempdir = TempDir::new().expect("Failed to create tempdir");
    let file1 = tempdir.path().join("file1.cmake");
    let file2 = tempdir.path().join("file2.cmake");

    std::fs::write(&file1, "set(A 1)\n").expect("Failed to write file1");
    std::fs::write(&file2, "set(B 2)\n").expect("Failed to write file2");

    let output = Command::new(cmake_fmt_bin())
        .arg("--line-ranges=1:1")
        .arg(&file1)
        .arg(&file2)
        .output()
        .expect("Failed to run cmake-fmt");

    assert!(
        !output.status.success(),
        "Should exit with non-zero code"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("single file"),
        "Should error about single file only"
    );
}

#[test]
fn test_line_ranges_single_line_range() {
    let input = "set(  A   1)\nset(  B   2)\nset(  C   3)\n";

    let mut child = Command::new(cmake_fmt_bin())
        .arg("--line-ranges=2:2")
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

    let output = child.wait_with_output().expect("Failed to wait for command");

    assert!(output.status.success(), "Command should succeed");
    let result = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = result.lines().collect();

    // Line 1 unchanged
    assert_eq!(lines[0], "set(  A   1)");
    // Line 2 formatted
    assert_eq!(lines[1], "set(B 2)");
    // Line 3 unchanged
    assert_eq!(lines[2], "set(  C   3)");
}
