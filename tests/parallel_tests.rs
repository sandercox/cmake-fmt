use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the cmake-fmt binary
fn cmake_fmt_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cmake-fmt"))
}

/// Create an unformatted CMake file with known content
fn create_unformatted_cmake_file(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    let content = r#"# Test file
SET(  MY_VAR   value1   value2   value3)
IF(SOME_CONDITION)
SET(  INNER_VAR   inner_value)
ENDIF()
"#;
    fs::write(&path, content).expect("Failed to write test file");
    path
}

/// Create a formatted CMake file
fn create_formatted_cmake_file(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    let content = "# Test file\nset(MY_VAR value1 value2 value3)\nif(SOME_CONDITION)\n\tset(INNER_VAR inner_value)\nendif()\n";
    fs::write(&path, content).expect("Failed to write test file");
    path
}

/// Expected formatted output (uses tabs by default)
fn expected_formatted_content() -> &'static str {
    "# Test file\nset(MY_VAR value1 value2 value3)\nif(SOME_CONDITION)\n\tset(INNER_VAR inner_value)\nendif()\n"
}

#[test]
fn test_parallel_inplace_formats_multiple_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create 10 unformatted files
    let mut files = Vec::new();
    for i in 1..=10 {
        let file = create_unformatted_cmake_file(temp_dir.path(), &format!("file{}.cmake", i));
        files.push(file);
    }

    // Run cmake-fmt -i on all files
    let output = Command::new(cmake_fmt_bin())
        .arg("-i")
        .args(&files)
        .output()
        .expect("Failed to execute cmake-fmt");

    // Check exit code
    assert!(output.status.success(), "cmake-fmt should succeed");

    // Verify all files are formatted correctly
    for file in &files {
        let content = fs::read_to_string(file).expect("Failed to read formatted file");
        assert_eq!(
            content,
            expected_formatted_content(),
            "File {} should be formatted",
            file.display()
        );
    }
}

#[test]
fn test_parallel_check_detects_changes() {
    let temp_dir = TempDir::new().unwrap();

    // Create 10 unformatted files
    let mut files = Vec::new();
    for i in 1..=10 {
        let file = create_unformatted_cmake_file(temp_dir.path(), &format!("file{}.cmake", i));
        files.push(file);
    }

    // Run cmake-fmt --check on unformatted files
    let output = Command::new(cmake_fmt_bin())
        .arg("--check")
        .args(&files)
        .output()
        .expect("Failed to execute cmake-fmt");

    // Should return exit code 1 (changes needed)
    assert_eq!(
        output.status.code(),
        Some(1),
        "cmake-fmt --check should return 1 for unformatted files"
    );

    // Format them
    let format_output = Command::new(cmake_fmt_bin())
        .arg("-i")
        .args(&files)
        .output()
        .expect("Failed to execute cmake-fmt");

    assert!(format_output.status.success());

    // Run check again - should pass
    let check_output = Command::new(cmake_fmt_bin())
        .arg("--check")
        .args(&files)
        .output()
        .expect("Failed to execute cmake-fmt");

    assert_eq!(
        check_output.status.code(),
        Some(0),
        "cmake-fmt --check should return 0 for formatted files"
    );
}

#[test]
fn test_parallel_check_no_changes() {
    let temp_dir = TempDir::new().unwrap();

    // Create 10 already-formatted files
    let mut files = Vec::new();
    for i in 1..=10 {
        let file = create_formatted_cmake_file(temp_dir.path(), &format!("file{}.cmake", i));
        files.push(file);
    }

    // Run cmake-fmt --check on formatted files
    let output = Command::new(cmake_fmt_bin())
        .arg("--check")
        .args(&files)
        .output()
        .expect("Failed to execute cmake-fmt");

    // Should return exit code 0 (no changes needed)
    assert_eq!(
        output.status.code(),
        Some(0),
        "cmake-fmt --check should return 0 for already-formatted files"
    );
}

#[test]
fn test_parallel_progress_on_stderr() {
    let temp_dir = TempDir::new().unwrap();

    // Create 10 unformatted files (enough to trigger progress display)
    let mut files = Vec::new();
    for i in 1..=10 {
        let file = create_unformatted_cmake_file(temp_dir.path(), &format!("file{}.cmake", i));
        files.push(file);
    }

    // Run cmake-fmt -i and capture stderr
    let output = Command::new(cmake_fmt_bin())
        .arg("-i")
        .args(&files)
        .output()
        .expect("Failed to execute cmake-fmt");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check that stderr contains progress information
    // Should contain "Formatted" and file count
    assert!(
        stderr.contains("Formatted"),
        "stderr should contain progress message: {}",
        stderr
    );
    assert!(
        stderr.contains("10"),
        "stderr should contain total file count: {}",
        stderr
    );
    assert!(
        stderr.contains("files"),
        "stderr should contain 'files': {}",
        stderr
    );
}

#[test]
fn test_stdout_mode_sequential_no_interleaving() {
    let temp_dir = TempDir::new().unwrap();

    // Create 5 files with distinct content
    let mut files = Vec::new();
    for i in 1..=5 {
        let path = temp_dir.path().join(format!("file{}.cmake", i));
        let content = format!("# File {}\nSET(  VAR_{}   value_{})\n", i, i, i);
        fs::write(&path, content).expect("Failed to write test file");
        files.push(path);
    }

    // Run cmake-fmt in stdout mode (no flags)
    let output = Command::new(cmake_fmt_bin())
        .args(&files)
        .output()
        .expect("Failed to execute cmake-fmt");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify sequential ordering - file 1 content should appear before file 2, etc.
    // Each file should contain its formatted SET command
    for i in 1..=5 {
        let expected_line = format!("set(VAR_{} value_{})", i, i);
        assert!(
            stdout.contains(&expected_line),
            "stdout should contain formatted content for file {}",
            i
        );
    }

    // Verify file 1's content appears before file 2's content
    let var1_pos = stdout.find("VAR_1").expect("Should find VAR_1");
    let var2_pos = stdout.find("VAR_2").expect("Should find VAR_2");
    let var3_pos = stdout.find("VAR_3").expect("Should find VAR_3");

    assert!(
        var1_pos < var2_pos,
        "File 1 content should appear before File 2"
    );
    assert!(
        var2_pos < var3_pos,
        "File 2 content should appear before File 3"
    );
}

#[test]
fn test_exit_code_aggregation_mixed_files() {
    let temp_dir = TempDir::new().unwrap();

    let mut files = Vec::new();

    // Create 5 formatted files
    for i in 1..=5 {
        let file = create_formatted_cmake_file(temp_dir.path(), &format!("formatted{}.cmake", i));
        files.push(file);
    }

    // Create 5 unformatted files
    for i in 1..=5 {
        let file =
            create_unformatted_cmake_file(temp_dir.path(), &format!("unformatted{}.cmake", i));
        files.push(file);
    }

    // Run cmake-fmt --check on mixed files
    let output = Command::new(cmake_fmt_bin())
        .arg("--check")
        .args(&files)
        .output()
        .expect("Failed to execute cmake-fmt");

    // Should return exit code 1 (at least one needs formatting)
    assert_eq!(
        output.status.code(),
        Some(1),
        "cmake-fmt --check should return 1 when any file needs formatting"
    );
}

#[test]
fn test_small_file_set_still_works() {
    let temp_dir = TempDir::new().unwrap();

    // Create only 2 unformatted files (below parallel threshold)
    let file1 = create_unformatted_cmake_file(temp_dir.path(), "file1.cmake");
    let file2 = create_unformatted_cmake_file(temp_dir.path(), "file2.cmake");

    // Run cmake-fmt -i
    let output = Command::new(cmake_fmt_bin())
        .arg("-i")
        .arg(&file1)
        .arg(&file2)
        .output()
        .expect("Failed to execute cmake-fmt");

    assert!(
        output.status.success(),
        "cmake-fmt should succeed on small file sets"
    );

    // Verify both files are formatted correctly
    let content1 = fs::read_to_string(&file1).expect("Failed to read file1");
    let content2 = fs::read_to_string(&file2).expect("Failed to read file2");

    assert_eq!(content1, expected_formatted_content());
    assert_eq!(content2, expected_formatted_content());
}

#[test]
fn test_diff_mode_multiple_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create 5 unformatted files
    let mut files = Vec::new();
    for i in 1..=5 {
        let file = create_unformatted_cmake_file(temp_dir.path(), &format!("file{}.cmake", i));
        files.push(file);
    }

    // Run cmake-fmt --diff
    let output = Command::new(cmake_fmt_bin())
        .arg("--diff")
        .args(&files)
        .output()
        .expect("Failed to execute cmake-fmt");

    // Should return exit code 1 (changes needed)
    assert_eq!(
        output.status.code(),
        Some(1),
        "cmake-fmt --diff should return 1 for unformatted files"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify diff output contains markers for multiple files
    // Diff output should contain "---" markers
    let marker_count = stdout.matches("---").count();
    assert!(
        marker_count >= 5,
        "Diff output should contain markers for multiple files, found {} markers",
        marker_count
    );
}
