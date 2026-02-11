use cmake_fmt::diff::{generate_diff, print_colored_diff};
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

// Helper to get the binary path
fn cmake_format_bin() -> String {
    env!("CARGO_BIN_EXE_cmake-fmt").to_string()
}

// ============================================================================
// UNIT TESTS: generate_diff function
// ============================================================================

#[test]
fn test_generate_diff_with_changes() {
    let original = "set(FOO   bar)\n";
    let formatted = "set(FOO bar)\n";

    let diff = generate_diff(original, formatted, "test.cmake");
    assert!(diff.is_some(), "Expected Some(diff) when content differs");

    let diff_text = diff.unwrap();

    // Verify headers
    assert!(diff_text.contains("--- a/test.cmake"), "Missing '---' header");
    assert!(diff_text.contains("+++ b/test.cmake"), "Missing '+++' header");

    // Verify hunk header
    assert!(diff_text.contains("@@"), "Missing hunk header");

    // Verify removed line
    assert!(diff_text.contains("-set(FOO   bar)"), "Missing removed line");

    // Verify added line
    assert!(diff_text.contains("+set(FOO bar)"), "Missing added line");
}

#[test]
fn test_generate_diff_no_changes() {
    let original = "set(FOO bar)\n";
    let formatted = "set(FOO bar)\n";

    let diff = generate_diff(original, formatted, "test.cmake");
    assert!(diff.is_none(), "Expected None when content is identical");
}

#[test]
fn test_generate_diff_header_format() {
    // Test that backslashes in paths are converted to forward slashes
    let original = "set(A b)\n";
    let formatted = "set(A c)\n";

    // Use Windows-style path with backslashes
    let diff = generate_diff(original, formatted, "dir\\subdir\\file.cmake");
    assert!(diff.is_some());

    let diff_text = diff.unwrap();

    // Verify forward slashes are used (not backslashes)
    assert!(diff_text.contains("--- a/dir/subdir/file.cmake"),
            "Path should use forward slashes, got:\n{}", diff_text);
    assert!(diff_text.contains("+++ b/dir/subdir/file.cmake"),
            "Path should use forward slashes, got:\n{}", diff_text);

    // Verify no backslashes in headers
    let first_line = diff_text.lines().next().unwrap();
    assert!(!first_line.contains('\\'), "Header should not contain backslashes");
}

#[test]
fn test_generate_diff_stdin_path() {
    let original = "set(A b)\n";
    let formatted = "set(A c)\n";

    let diff = generate_diff(original, formatted, "stdin");
    assert!(diff.is_some());

    let diff_text = diff.unwrap();

    // Verify stdin is used in headers
    assert!(diff_text.contains("--- a/stdin"), "Missing stdin in --- header");
    assert!(diff_text.contains("+++ b/stdin"), "Missing stdin in +++ header");
}

#[test]
fn test_generate_diff_patch_compatible() {
    let original = "# Line 1\n# Line 2\n# Line 3\nset(OLD value)\n# Line 5\n# Line 6\n# Line 7\n";
    let formatted = "# Line 1\n# Line 2\n# Line 3\nset(NEW value)\n# Line 5\n# Line 6\n# Line 7\n";

    let diff = generate_diff(original, formatted, "test.cmake");
    assert!(diff.is_some());

    let diff_text = diff.unwrap();

    // Verify standard unified diff format
    assert!(diff_text.starts_with("---"), "Should start with ---");
    assert!(diff_text.contains("+++"), "Should contain +++");
    assert!(diff_text.contains("@@ -"), "Should contain @@ hunk headers");

    // Verify context radius of 3 lines (should include surrounding lines)
    assert!(diff_text.contains("# Line 1") || diff_text.contains("# Line 2") || diff_text.contains("# Line 3"),
            "Should include context lines before change");
    assert!(diff_text.contains("# Line 5") || diff_text.contains("# Line 6") || diff_text.contains("# Line 7"),
            "Should include context lines after change");
}

#[test]
fn test_print_colored_diff_runs_without_panic() {
    // This test just verifies print_colored_diff doesn't panic
    // We can't test actual color output in CI, but we can test it runs
    let original = "old\n";
    let formatted = "new\n";

    // Redirect stdout to suppress output during test
    print_colored_diff(original, formatted, "test.cmake");

    // If we got here without panic, test passes
}

// ============================================================================
// CLI INTEGRATION TESTS
// ============================================================================

#[test]
fn test_diff_exit_code_no_changes() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.cmake");

    // Write already-formatted content
    std::fs::write(&file_path, "set(FOO bar)\n").unwrap();

    let output = Command::new(cmake_format_bin())
        .arg("--diff")
        .arg(&file_path)
        .output()
        .expect("Failed to execute cmake-format");

    // Should exit with code 0 (no changes needed)
    assert!(output.status.success(),
            "Exit code should be 0 for no changes. stderr: {}",
            String::from_utf8_lossy(&output.stderr));

    // Stdout should be empty (no diff to show)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.is_empty() || stdout.trim().is_empty(),
            "Stdout should be empty when no changes needed");
}

#[test]
fn test_diff_exit_code_with_changes() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.cmake");

    // Write unformatted content (extra spaces)
    std::fs::write(&file_path, "set(FOO   bar)\n").unwrap();

    let output = Command::new(cmake_format_bin())
        .arg("--diff")
        .arg(&file_path)
        .output()
        .expect("Failed to execute cmake-format");

    // Should exit with code 1 (changes needed)
    assert_eq!(output.status.code(), Some(1),
               "Exit code should be 1 for changes needed. stderr: {}",
               String::from_utf8_lossy(&output.stderr));

    // Stdout should contain diff output
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("---"), "Diff should contain --- header");
    assert!(stdout.contains("+++"), "Diff should contain +++ header");
}

#[test]
fn test_diff_conflicts_with_in_place() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.cmake");

    std::fs::write(&file_path, "set(FOO bar)\n").unwrap();

    let output = Command::new(cmake_format_bin())
        .arg("--diff")
        .arg("-i")
        .arg(&file_path)
        .output()
        .expect("Failed to execute cmake-format");

    // Should fail (non-zero exit code)
    assert!(!output.status.success(),
            "Should fail when --diff and -i are used together");

    // Stderr should mention the conflict
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.to_lowercase().contains("conflict") ||
            stderr.to_lowercase().contains("cannot be used"),
            "Error message should mention conflict: {}", stderr);
}

#[test]
fn test_diff_compatible_with_check() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.cmake");

    // Write unformatted content
    std::fs::write(&file_path, "set(FOO   bar)\n").unwrap();

    let output = Command::new(cmake_format_bin())
        .arg("--diff")
        .arg("--check")
        .arg(&file_path)
        .output()
        .expect("Failed to execute cmake-format");

    // Should exit with code 1 (changes needed)
    assert_eq!(output.status.code(), Some(1),
               "Exit code should be 1 when both --diff and --check are used");

    // Stdout should contain diff output
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("---"), "Diff should be shown");
    assert!(stdout.contains("+++"), "Diff should be shown");
}

#[test]
fn test_diff_stdin() {
    // Write unformatted content to stdin
    let input = "set(FOO   bar)\n";

    let mut child = Command::new(cmake_format_bin())
        .arg("--diff")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn cmake-format");

    // Write to stdin
    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(input.as_bytes()).expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read output");

    // Should exit with code 1 (changes needed)
    assert_eq!(output.status.code(), Some(1),
               "Exit code should be 1 for stdin with changes");

    // Stdout should contain diff with "stdin" in headers
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("stdin"), "Diff headers should reference 'stdin'");
    assert!(stdout.contains("---"), "Should contain --- header");
    assert!(stdout.contains("+++"), "Should contain +++ header");
}

#[test]
fn test_diff_multiple_files() {
    let temp_dir = TempDir::new().unwrap();
    let file1_path = temp_dir.path().join("file1.cmake");
    let file2_path = temp_dir.path().join("file2.cmake");

    // Write unformatted content to both files
    std::fs::write(&file1_path, "set(FOO   bar)\n").unwrap();
    std::fs::write(&file2_path, "set(BAZ   qux)\n").unwrap();

    let output = Command::new(cmake_format_bin())
        .arg("--diff")
        .arg(&file1_path)
        .arg(&file2_path)
        .output()
        .expect("Failed to execute cmake-format");

    // Should exit with code 1 (changes needed)
    assert_eq!(output.status.code(), Some(1),
               "Exit code should be 1 when multiple files need changes");

    // Stdout should contain diffs for both files
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("file1.cmake"), "Should show diff for file1.cmake");
    assert!(stdout.contains("file2.cmake"), "Should show diff for file2.cmake");

    // Count occurrences of diff headers (should be 2 pairs)
    let minus_count = stdout.matches("---").count();
    let plus_count = stdout.matches("+++").count();
    assert_eq!(minus_count, 2, "Should have 2 --- headers (one per file)");
    assert_eq!(plus_count, 2, "Should have 2 +++ headers (one per file)");
}

#[test]
fn test_diff_file_not_modified_in_diff_mode() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.cmake");

    // Write unformatted content
    let original_content = "set(FOO   bar)\n";
    std::fs::write(&file_path, original_content).unwrap();

    // Run diff mode
    Command::new(cmake_format_bin())
        .arg("--diff")
        .arg(&file_path)
        .output()
        .expect("Failed to execute cmake-format");

    // Verify file was NOT modified
    let after_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(original_content, after_content,
               "File should not be modified in diff mode");
}

#[test]
fn test_diff_multiline_changes() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.cmake");

    // Write unformatted content with multiple issues
    let unformatted = "set(FOO   bar)\nset(BAZ   qux)\nset(X   y)\n";
    std::fs::write(&file_path, unformatted).unwrap();

    let output = Command::new(cmake_format_bin())
        .arg("--diff")
        .arg(&file_path)
        .output()
        .expect("Failed to execute cmake-format");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show all changes
    assert!(stdout.contains("-set(FOO   bar)"), "Should show old FOO line");
    assert!(stdout.contains("+set(FOO bar)"), "Should show new FOO line");
    assert!(stdout.contains("-set(BAZ   qux)"), "Should show old BAZ line");
    assert!(stdout.contains("+set(BAZ qux)"), "Should show new BAZ line");
}
