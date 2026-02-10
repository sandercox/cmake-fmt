use cmake_fmt::interactive::{apply_decisions, generate_hunks, UserChoice};
use std::process::Command;
use tempfile::TempDir;

// Helper to get the binary path
fn cmake_fmt_bin() -> String {
    env!("CARGO_BIN_EXE_cmake-fmt").to_string()
}

// ============================================================================
// DIFF GENERATION TESTS (INT-01, INT-02)
// ============================================================================

#[test]
fn test_generate_hunks_no_changes() {
    let text = "set(FOO bar)\nset(BAR baz)\n";
    let hunks = generate_hunks(text, text, 3);
    assert_eq!(hunks.len(), 0, "No changes should produce empty hunk list");
}

#[test]
fn test_generate_hunks_single_change() {
    let original = "set(FOO   bar)\n";
    let formatted = "set(FOO bar)\n";

    let hunks = generate_hunks(original, formatted, 3);
    assert_eq!(hunks.len(), 1, "Single change should produce one hunk");

    let hunk = &hunks[0];
    assert_eq!(hunk.old_start, 0, "Change starts at line 0");
    assert_eq!(hunk.old_count, 1, "One line removed");
    assert_eq!(hunk.new_start, 0, "Replacement starts at line 0");
    assert_eq!(hunk.new_count, 1, "One line added");
}

#[test]
fn test_generate_hunks_multiple_changes() {
    let original = "set(FOO   bar)\nset(BAR baz)\nset(QUX   quux)\n";
    let formatted = "set(FOO bar)\nset(BAR baz)\nset(QUX quux)\n";

    let hunks = generate_hunks(original, formatted, 0); // No context to keep them separate

    // With context=0, changes far apart should be separate hunks
    // Line 0 changes, line 1 stays, line 2 changes
    // Depending on grouping, could be 1 or 2 hunks
    // The key is that both changes are captured
    assert!(!hunks.is_empty(), "Multiple changes should produce hunks");

    // Verify total changes are captured
    let total_deletes: usize = hunks.iter().map(|h| h.old_count).sum();
    let total_inserts: usize = hunks.iter().map(|h| h.new_count).sum();
    assert_eq!(total_deletes, 2, "Should have 2 deletions");
    assert_eq!(total_inserts, 2, "Should have 2 insertions");
}

#[test]
fn test_generate_hunks_adjacent_changes() {
    let original = "set(FOO   bar)\nset(BAR   baz)\n";
    let formatted = "set(FOO bar)\nset(BAR baz)\n";

    let hunks = generate_hunks(original, formatted, 3);

    // Adjacent changes should merge into single hunk
    assert_eq!(hunks.len(), 1, "Adjacent changes should merge into one hunk");
    assert_eq!(hunks[0].old_count, 2, "Both lines changed");
}

#[test]
fn test_generate_hunks_context_lines() {
    let original = "# Comment\nset(FOO   bar)\n# Footer\n";
    let formatted = "# Comment\nset(FOO bar)\n# Footer\n";

    let hunks = generate_hunks(original, formatted, 1);
    assert_eq!(hunks.len(), 1);

    // Hunk should have Equal changes for context
    let hunk = &hunks[0];
    assert!(hunk.changes.len() >= 3, "Should have context + change");
}

// ============================================================================
// APPLY DECISIONS TESTS (INT-03, INT-04)
// ============================================================================

#[test]
fn test_apply_accept() {
    let original = "set(FOO   bar)\n";
    let formatted = "set(FOO bar)\n";

    let hunks = generate_hunks(original, formatted, 3);
    assert_eq!(hunks.len(), 1);

    let decisions = vec![(hunks[0].clone(), UserChoice::Accept)];
    let result = apply_decisions(original, &decisions);

    assert_eq!(result, formatted, "Accept should apply formatted version");
}

#[test]
fn test_apply_reject() {
    let original = "set(FOO   bar)\n";
    let formatted = "set(FOO bar)\n";

    let hunks = generate_hunks(original, formatted, 3);
    assert_eq!(hunks.len(), 1);

    let decisions = vec![(hunks[0].clone(), UserChoice::Reject)];
    let result = apply_decisions(original, &decisions);

    assert_eq!(result, original, "Reject should keep original version");
}

#[test]
fn test_apply_suppress() {
    let original = "set(FOO   bar)\n";
    let formatted = "set(FOO bar)\n";

    let hunks = generate_hunks(original, formatted, 3);
    assert_eq!(hunks.len(), 1);

    let decisions = vec![(hunks[0].clone(), UserChoice::Suppress)];
    let result = apply_decisions(original, &decisions);

    // Single-line hunk uses skip directive instead of off/on pair
    assert!(result.contains("# cmake-fmt: skip"), "Should contain skip marker");
    assert!(result.contains("set(FOO   bar)"), "Should preserve original line");
}

#[test]
fn test_apply_mixed_decisions() {
    let original = "set(FOO   bar)\nset(BAR   baz)\nset(QUX   quux)\n";
    let formatted = "set(FOO bar)\nset(BAR baz)\nset(QUX quux)\n";

    let hunks = generate_hunks(original, formatted, 0);

    // If we get one merged hunk, split manually for testing
    // Or use separate decisions
    if hunks.len() >= 2 {
        let decisions = vec![
            (hunks[0].clone(), UserChoice::Accept),
            (hunks[1].clone(), UserChoice::Reject),
        ];
        let result = apply_decisions(original, &decisions);

        // First hunk accepted, second rejected
        assert!(result.contains("set(FOO bar)"), "First change should be accepted");
        // The exact result depends on hunk boundaries
    } else if hunks.len() == 1 {
        // All changes in one hunk - just test it doesn't crash
        let decisions = vec![(hunks[0].clone(), UserChoice::Accept)];
        let result = apply_decisions(original, &decisions);
        assert_eq!(result, formatted);
    }
}

#[test]
fn test_apply_suppress_preserves_indent() {
    let original = "    set(FOO   bar)\n";
    let formatted = "    set(FOO bar)\n";

    let hunks = generate_hunks(original, formatted, 3);
    assert_eq!(hunks.len(), 1);

    let decisions = vec![(hunks[0].clone(), UserChoice::Suppress)];
    let result = apply_decisions(original, &decisions);

    // Single-line hunk: skip marker should match indentation
    assert!(result.contains("    # cmake-fmt: skip"), "Skip marker should be indented");
}

#[test]
fn test_apply_empty_decisions() {
    let original = "set(FOO bar)\n";
    let decisions = vec![];

    let result = apply_decisions(original, &decisions);
    assert_eq!(result, original, "Empty decisions should return original");
}

#[test]
fn test_apply_multiple_accepts_with_offset() {
    // Test that line offset tracking works correctly across multiple hunks
    let original = "line1\nline2\nline3\n";
    let formatted = "line1\nNEW2a\nNEW2b\nline3\n";

    let hunks = generate_hunks(original, formatted, 0);

    if hunks.len() == 1 {
        let decisions = vec![(hunks[0].clone(), UserChoice::Accept)];
        let result = apply_decisions(original, &decisions);
        assert_eq!(result, formatted, "Accept should handle line count changes");
    }
}

// ============================================================================
// CLI INTEGRATION TESTS (INT-06)
// ============================================================================

#[test]
fn test_interactive_no_file_arg() {
    let output = Command::new(cmake_fmt_bin())
        .arg("--interactive")
        .output()
        .expect("Failed to execute cmake-fmt");

    assert!(!output.status.success(), "Should exit with error");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("requires a file"),
            "Should require file argument. stderr: {}", stderr);
}

#[test]
fn test_interactive_conflicts_with_in_place() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.cmake");
    std::fs::write(&file_path, "set(FOO bar)\n").unwrap();

    let output = Command::new(cmake_fmt_bin())
        .arg("--interactive")
        .arg("-i")
        .arg(&file_path)
        .output()
        .expect("Failed to execute cmake-fmt");

    assert!(!output.status.success(), "Should exit with error");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("conflict") || stderr.contains("cannot be used with"),
            "Should indicate flag conflict. stderr: {}", stderr);
}

#[test]
fn test_interactive_conflicts_with_check() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.cmake");
    std::fs::write(&file_path, "set(FOO bar)\n").unwrap();

    let output = Command::new(cmake_fmt_bin())
        .arg("--interactive")
        .arg("--check")
        .arg(&file_path)
        .output()
        .expect("Failed to execute cmake-fmt");

    assert!(!output.status.success(), "Should exit with error");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("conflict") || stderr.contains("cannot be used with"),
            "Should indicate flag conflict. stderr: {}", stderr);
}

#[test]
fn test_interactive_conflicts_with_diff() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.cmake");
    std::fs::write(&file_path, "set(FOO bar)\n").unwrap();

    let output = Command::new(cmake_fmt_bin())
        .arg("--interactive")
        .arg("--diff")
        .arg(&file_path)
        .output()
        .expect("Failed to execute cmake-fmt");

    assert!(!output.status.success(), "Should exit with error");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("conflict") || stderr.contains("cannot be used with"),
            "Should indicate flag conflict. stderr: {}", stderr);
}

#[test]
fn test_interactive_multiple_files_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("test1.cmake");
    let file2 = temp_dir.path().join("test2.cmake");
    std::fs::write(&file1, "set(FOO bar)\n").unwrap();
    std::fs::write(&file2, "set(BAR baz)\n").unwrap();

    let output = Command::new(cmake_fmt_bin())
        .arg("--interactive")
        .arg(&file1)
        .arg(&file2)
        .output()
        .expect("Failed to execute cmake-fmt");

    assert!(!output.status.success(), "Should exit with error");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("one file at a time"),
            "Should reject multiple files. stderr: {}", stderr);
}
