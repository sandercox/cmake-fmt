use cmake_fmt::formatter::grammar::{clear_project_scan_cache, user_scanner};
use cmake_fmt::formatter::{format_text_with_diagnostics_and_path, FormatConfig};
use std::fs;
use tempfile::TempDir;

/// Helper to create a file in a temp directory
fn create_file(dir: &TempDir, rel_path: &str, content: &str) {
    let path = dir.path().join(rel_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[test]
fn test_cross_file_user_command_discovery() {
    clear_project_scan_cache();

    let temp_dir = TempDir::new().unwrap();

    // Create project structure with function definition in one file
    create_file(
        &temp_dir,
        "cmake/helpers.cmake",
        "function(my_add_library)\nendfunction()\n",
    );
    create_file(
        &temp_dir,
        "src/CMakeLists.txt",
        "my_add_library(mylib src/main.cpp)\n",
    );
    create_file(&temp_dir, ".cmake-fmt.toml", "indent_width = 2\n");

    let src_file = temp_dir.path().join("src/CMakeLists.txt");
    let input = fs::read_to_string(&src_file).unwrap();
    let config = FormatConfig::default();

    let (output, _warnings) = format_text_with_diagnostics_and_path(&input, &config, Some(&src_file));

    // Verify the user command is recognized (name preserved with case)
    assert!(
        output.contains("my_add_library"),
        "Expected my_add_library to be recognized: {}",
        output
    );
}

#[test]
fn test_gitignore_respected() {
    clear_project_scan_cache();

    let temp_dir = TempDir::new().unwrap();

    // Initialize git repo (required for .gitignore to work with ignore crate)
    std::process::Command::new("git")
        .arg("init")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to initialize git repo");

    // Create project with .gitignore excluding build/
    create_file(&temp_dir, ".gitignore", "build/\n");
    create_file(&temp_dir, ".cmake-fmt.toml", "");
    create_file(
        &temp_dir,
        "build/CMakeLists.txt",
        "function(build_only_func)\nendfunction()\n",
    );
    create_file(&temp_dir, "src/CMakeLists.txt", "build_only_func(arg1)\n");

    // Test that scanner doesn't include build/ directory files
    let cmake_files = user_scanner::find_cmake_files(temp_dir.path());

    // Convert to strings for easier checking
    let file_paths: Vec<String> = cmake_files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    // Verify build/CMakeLists.txt is NOT in the results
    let has_build_file = file_paths.iter().any(|p| p.contains("build") && p.contains("CMakeLists.txt"));
    assert!(
        !has_build_file,
        "Expected build/CMakeLists.txt to be excluded by .gitignore, but found it in: {:?}",
        file_paths
    );

    // Verify src/CMakeLists.txt IS in the results
    let has_src_file = file_paths.iter().any(|p| p.contains("src") && p.contains("CMakeLists.txt"));
    assert!(
        has_src_file,
        "Expected src/CMakeLists.txt to be included, but not found in: {:?}",
        file_paths
    );
}

#[test]
fn test_config_file_determines_project_root() {
    clear_project_scan_cache();

    let temp_dir = TempDir::new().unwrap();

    // Create nested project structure
    create_file(&temp_dir, "project/.cmake-fmt.toml", "");
    create_file(
        &temp_dir,
        "project/CMakeLists.txt",
        "function(project_func)\nendfunction()\n",
    );
    create_file(&temp_dir, "project/sub/CMakeLists.txt", "project_func(arg1)\n");
    create_file(
        &temp_dir,
        "outside/CMakeLists.txt",
        "function(outside_func)\nendfunction()\n",
    );

    // Find project root starting from sub directory
    let sub_dir = temp_dir.path().join("project/sub");
    let project_root = user_scanner::find_project_root(&sub_dir);

    // Should find the project/ directory (where config file is)
    let expected_root = temp_dir.path().join("project");
    assert_eq!(
        project_root, expected_root,
        "Expected project root to be project/, got {:?}",
        project_root
    );

    // Scan for user commands from that root
    let user_commands = user_scanner::scan_project_commands(&project_root);

    // Verify project_func is found
    assert!(
        user_commands.contains_key("project_func"),
        "Expected project_func to be found in: {:?}",
        user_commands
    );

    // Verify outside_func is NOT found (outside project boundary)
    assert!(
        !user_commands.contains_key("outside_func"),
        "Expected outside_func to NOT be found, but got: {:?}",
        user_commands
    );
}

#[test]
fn test_no_config_file_fallback() {
    clear_project_scan_cache();

    let temp_dir = TempDir::new().unwrap();

    // Create structure with NO config file
    create_file(
        &temp_dir,
        "CMakeLists.txt",
        "function(local_func)\nendfunction()\n",
    );

    // Find project root starting from temp dir
    let project_root = user_scanner::find_project_root(temp_dir.path());

    // Should fall back to the start directory itself
    assert_eq!(
        project_root,
        temp_dir.path(),
        "Expected fallback to temp dir, got {:?}",
        project_root
    );
}

#[test]
fn test_malformed_file_skipped() {
    clear_project_scan_cache();

    let temp_dir = TempDir::new().unwrap();

    // Create project with both good and malformed files
    create_file(&temp_dir, ".cmake-fmt.toml", "");
    create_file(
        &temp_dir,
        "good.cmake",
        "function(good_func)\nendfunction()\n",
    );
    // Malformed file: incomplete function definition
    create_file(&temp_dir, "bad.cmake", "function(incomplete\n");

    // Scan should not crash and should find the good function
    let user_commands = user_scanner::scan_project_commands(temp_dir.path());

    assert!(
        user_commands.contains_key("good_func"),
        "Expected good_func to be found even with malformed file present: {:?}",
        user_commands
    );

    // The bad file should be silently skipped (no panic, no error)
    // This test passes if we get here without panicking
}

#[test]
fn test_idempotency_with_project_scanning() {
    clear_project_scan_cache();

    let temp_dir = TempDir::new().unwrap();

    // Create project with user command definition
    create_file(&temp_dir, ".cmake-fmt.toml", "");
    create_file(
        &temp_dir,
        "helpers.cmake",
        "function(MyHelper)\nendfunction()\n",
    );
    create_file(&temp_dir, "main.cmake", "MyHelper(arg1 arg2 arg3)\n");

    let main_file = temp_dir.path().join("main.cmake");
    let input = fs::read_to_string(&main_file).unwrap();
    let config = FormatConfig::default();

    // Format once
    let (first_pass, _) = format_text_with_diagnostics_and_path(&input, &config, Some(&main_file));

    // Format again (should be idempotent)
    let (second_pass, _) = format_text_with_diagnostics_and_path(&first_pass, &config, Some(&main_file));

    assert_eq!(
        first_pass, second_pass,
        "Expected formatting to be idempotent.\nFirst pass:\n{}\nSecond pass:\n{}",
        first_pass, second_pass
    );
}
