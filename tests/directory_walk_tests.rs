use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the cmake-fmt binary
fn cmake_fmt_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cmake-fmt"))
}

/// Write a formatted CMake file (already canonical)
fn write_formatted(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("Failed to create directories");
    }
    fs::write(&path, "project(test)\n").expect("Failed to write file");
    path
}

/// Write an unformatted CMake file (will need reformatting)
fn write_unformatted(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("Failed to create directories");
    }
    fs::write(&path, "project(  test  )\n").expect("Failed to write file");
    path
}

// ---------------------------------------------------------------------------
// Test 1: directory collects .cmake and CMakeLists.txt, not unrelated files
// ---------------------------------------------------------------------------
#[test]
fn test_directory_collects_cmake_files() {
    let tmp = TempDir::new().unwrap();

    write_formatted(tmp.path(), "CMakeLists.txt");
    write_formatted(tmp.path(), "foo.cmake");
    fs::write(tmp.path().join("bar.txt"), "hello").unwrap();

    // --check should succeed (all cmake files are already formatted)
    let output = Command::new(cmake_fmt_bin())
        .arg(tmp.path())
        .arg("--check")
        .output()
        .expect("Failed to run cmake-fmt");

    assert!(
        output.status.success(),
        "Expected exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// Test 2: non-recursive by default (does NOT descend into subdirectories)
// ---------------------------------------------------------------------------
#[test]
fn test_directory_non_recursive_by_default() {
    let tmp = TempDir::new().unwrap();

    write_formatted(tmp.path(), "CMakeLists.txt");
    // Unformatted file in subdirectory — should NOT be found without -r
    write_unformatted(tmp.path(), "subdir/CMakeLists.txt");

    // Without -r the subdir file should be ignored, so check should pass
    let output = Command::new(cmake_fmt_bin())
        .arg(tmp.path())
        .arg("--check")
        .output()
        .expect("Failed to run cmake-fmt");

    assert!(
        output.status.success(),
        "Expected exit 0 (subdir not visited without -r), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// Test 3: -r walks subdirectories
// ---------------------------------------------------------------------------
#[test]
fn test_directory_recursive_flag() {
    let tmp = TempDir::new().unwrap();

    write_formatted(tmp.path(), "CMakeLists.txt");
    // Unformatted file in subdirectory — should be found with -r
    write_unformatted(tmp.path(), "subdir/CMakeLists.txt");

    let output = Command::new(cmake_fmt_bin())
        .arg(tmp.path())
        .arg("--check")
        .arg("-r")
        .output()
        .expect("Failed to run cmake-fmt");

    // Should exit 1 because subdir/CMakeLists.txt needs reformatting
    assert!(
        !output.status.success(),
        "Expected exit 1 (subdir file needs reformatting with -r), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("CMakeLists.txt"),
        "Expected subdir file mentioned in output, got: {}",
        stderr
    );
}

// ---------------------------------------------------------------------------
// Test 4: .cmake-fmt-ignore in the directory excludes matched paths
// ---------------------------------------------------------------------------
#[test]
fn test_cmake_fmt_ignore_respected() {
    let tmp = TempDir::new().unwrap();

    write_formatted(tmp.path(), "CMakeLists.txt");
    // Unformatted file that would fail check — but it's in the ignored path
    write_unformatted(tmp.path(), "sub/CMakeLists.txt");

    // Write .cmake-fmt-ignore excluding "sub/"
    fs::write(tmp.path().join(".cmake-fmt-ignore"), "sub/\n").unwrap();

    // With -r, but sub/ is ignored — check should pass
    let output = Command::new(cmake_fmt_bin())
        .arg(tmp.path())
        .arg("--check")
        .arg("-r")
        .output()
        .expect("Failed to run cmake-fmt");

    assert!(
        output.status.success(),
        "Expected exit 0 (sub/ excluded by .cmake-fmt-ignore), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// Test 5: .gitignore is respected
// ---------------------------------------------------------------------------
#[test]
fn test_gitignore_respected() {
    let tmp = TempDir::new().unwrap();

    // Initialize a git repo so .gitignore is picked up
    Command::new("git")
        .arg("init")
        .current_dir(tmp.path())
        .output()
        .expect("Failed to git init");

    // Configure git identity to avoid warnings
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(tmp.path())
        .output()
        .ok();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(tmp.path())
        .output()
        .ok();

    // .gitignore excludes build/
    fs::write(tmp.path().join(".gitignore"), "build/\n").unwrap();

    write_formatted(tmp.path(), "src/CMakeLists.txt");
    // Unformatted file in build/ — should be ignored
    write_unformatted(tmp.path(), "build/CMakeLists.txt");

    let output = Command::new(cmake_fmt_bin())
        .arg(tmp.path())
        .arg("--check")
        .arg("-r")
        .output()
        .expect("Failed to run cmake-fmt");

    assert!(
        output.status.success(),
        "Expected exit 0 (build/ excluded by .gitignore), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// Test 6: --ignore-file specifies a custom ignore file
// ---------------------------------------------------------------------------
#[test]
fn test_custom_ignore_file() {
    let tmp = TempDir::new().unwrap();

    write_formatted(tmp.path(), "CMakeLists.txt");
    // Unformatted file in vendor/ — should be excluded by custom ignore file
    write_unformatted(tmp.path(), "vendor/CMakeLists.txt");

    // Write a custom ignore file (NOT named .cmake-fmt-ignore)
    let ignore_path = tmp.path().join("my-ignore");
    fs::write(&ignore_path, "vendor/\n").unwrap();

    let output = Command::new(cmake_fmt_bin())
        .arg(tmp.path())
        .arg("--check")
        .arg("-r")
        .arg("--ignore-file")
        .arg(&ignore_path)
        .output()
        .expect("Failed to run cmake-fmt");

    assert!(
        output.status.success(),
        "Expected exit 0 (vendor/ excluded by --ignore-file), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// Test 7: passing "." as directory path processes current directory
// ---------------------------------------------------------------------------
#[test]
fn test_dot_arg_processes_current_dir() {
    let tmp = TempDir::new().unwrap();

    // Write an unformatted CMakeLists.txt in the temp dir
    write_unformatted(tmp.path(), "CMakeLists.txt");

    // Run cmake-fmt . --check with current_dir set to tmp
    let output = Command::new(cmake_fmt_bin())
        .arg(".")
        .arg("--check")
        .current_dir(tmp.path())
        .output()
        .expect("Failed to run cmake-fmt");

    // Should exit 1 because CMakeLists.txt needs reformatting
    assert!(
        !output.status.success(),
        "Expected exit 1 (CMakeLists.txt needs reformatting), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("CMakeLists.txt"),
        "Expected CMakeLists.txt mentioned in output, got: {}",
        stderr
    );
}

// ---------------------------------------------------------------------------
// Test 8: --diff mode shows diff output for multiple files
// ---------------------------------------------------------------------------
#[test]
fn test_diff_mode_multiple_files() {
    let tmp = TempDir::new().unwrap();

    // Two unformatted files
    write_unformatted(tmp.path(), "a.cmake");
    write_unformatted(tmp.path(), "b.cmake");

    // Use explicit --diff so we don't rely on terminal detection
    let output = Command::new(cmake_fmt_bin())
        .arg(tmp.path())
        .arg("--diff")
        .output()
        .expect("Failed to run cmake-fmt");

    // Should exit 1 because files need reformatting
    assert!(
        !output.status.success(),
        "Expected exit 1 (files need reformatting), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Diff output should contain diff markers
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("---") || combined.contains("+++") || combined.contains("@@"),
        "Expected diff markers in output, got stdout: '{}', stderr: '{}'",
        stdout,
        stderr
    );
}

// ---------------------------------------------------------------------------
// Test 9: extension filtering — only .cmake and CMakeLists.txt are collected
// ---------------------------------------------------------------------------
#[test]
fn test_only_cmake_extensions_collected() {
    let tmp = TempDir::new().unwrap();

    write_formatted(tmp.path(), "CMakeLists.txt");
    write_formatted(tmp.path(), "module.cmake");
    // Unformatted file with non-cmake extension — should be ignored
    fs::write(tmp.path().join("readme.txt"), "project(  bad  )\n").unwrap();
    fs::write(tmp.path().join("script.sh"), "project(  bad  )\n").unwrap();

    let output = Command::new(cmake_fmt_bin())
        .arg(tmp.path())
        .arg("--check")
        .output()
        .expect("Failed to run cmake-fmt");

    assert!(
        output.status.success(),
        "Expected exit 0 (non-cmake files not collected), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
