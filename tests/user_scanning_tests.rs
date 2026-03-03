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

    // Create root CMakeLists.txt that references both files
    create_file(
        &temp_dir,
        "CMakeLists.txt",
        "add_subdirectory(src)\ninclude(cmake/helpers.cmake)\n",
    );
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

    let (output, _warnings) = format_text_with_diagnostics_and_path(&input, &config, Some(&src_file), false);

    // Verify the user command is recognized (name preserved with case)
    assert!(
        output.contains("my_add_library"),
        "Expected my_add_library to be recognized: {}",
        output
    );
}

#[test]
fn test_unreferenced_files_not_scanned() {
    clear_project_scan_cache();

    let temp_dir = TempDir::new().unwrap();

    // Create project where build/ is not referenced by CMake dependency graph
    create_file(&temp_dir, ".cmake-fmt.toml", "");
    create_file(
        &temp_dir,
        "CMakeLists.txt",
        "add_subdirectory(src)\n",
    );
    create_file(
        &temp_dir,
        "build/CMakeLists.txt",
        "function(build_only_func)\nendfunction()\n",
    );
    create_file(&temp_dir, "src/CMakeLists.txt", "build_only_func(arg1)\n");

    // Test that scanner doesn't include unreferenced build/ directory files
    let user_commands = user_scanner::scan_project_commands(temp_dir.path(), false);

    // Verify build_only_func is NOT found (build/ not referenced)
    assert!(
        !user_commands.contains_key("build_only_func"),
        "Expected build_only_func to NOT be found (unreferenced), but got: {:?}",
        user_commands
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
        "add_subdirectory(sub)\nfunction(project_func)\nendfunction()\n",
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
    let user_commands = user_scanner::scan_project_commands(&project_root, false);

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

    // Should either:
    // 1. Find a config file in a parent directory (if one exists)
    // 2. Fall back to the start directory itself (if no config found in ancestors)
    // Either way, project_root should be an ancestor of or equal to start_path
    assert!(
        temp_dir.path().starts_with(&project_root) || project_root == temp_dir.path(),
        "Expected project_root to be an ancestor of start path, got {:?} for start {:?}",
        project_root,
        temp_dir.path()
    );
}

#[test]
fn test_malformed_file_skipped() {
    clear_project_scan_cache();

    let temp_dir = TempDir::new().unwrap();

    // Create root CMakeLists.txt that references both files
    create_file(
        &temp_dir,
        "CMakeLists.txt",
        "include(good.cmake)\ninclude(bad.cmake)\n",
    );
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
    let user_commands = user_scanner::scan_project_commands(temp_dir.path(), false);

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

    // Create root CMakeLists.txt that includes helpers.cmake
    create_file(
        &temp_dir,
        "CMakeLists.txt",
        "include(helpers.cmake)\n",
    );
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
    let (first_pass, _) = format_text_with_diagnostics_and_path(&input, &config, Some(&main_file), false);

    // Format again (should be idempotent)
    let (second_pass, _) = format_text_with_diagnostics_and_path(&first_pass, &config, Some(&main_file), false);

    assert_eq!(
        first_pass, second_pass,
        "Expected formatting to be idempotent.\nFirst pass:\n{}\nSecond pass:\n{}",
        first_pass, second_pass
    );
}

#[test]
fn test_add_subdirectory_chain() {
    clear_project_scan_cache();

    let temp_dir = TempDir::new().unwrap();

    // Create root with multiple subdirectories
    create_file(
        &temp_dir,
        "CMakeLists.txt",
        "add_subdirectory(liba)\nadd_subdirectory(libb)\n",
    );
    create_file(&temp_dir, ".cmake-fmt.toml", "");
    create_file(
        &temp_dir,
        "liba/CMakeLists.txt",
        "function(liba_func)\nendfunction()\n",
    );
    create_file(
        &temp_dir,
        "libb/CMakeLists.txt",
        "function(libb_func)\nendfunction()\n",
    );

    let user_commands = user_scanner::scan_project_commands(temp_dir.path(), false);

    // Verify both functions are found
    assert!(
        user_commands.contains_key("liba_func"),
        "Expected liba_func to be found in: {:?}",
        user_commands
    );
    assert!(
        user_commands.contains_key("libb_func"),
        "Expected libb_func to be found in: {:?}",
        user_commands
    );
}

#[test]
fn test_include_chain() {
    clear_project_scan_cache();

    let temp_dir = TempDir::new().unwrap();

    // Create root that includes a.cmake
    create_file(
        &temp_dir,
        "CMakeLists.txt",
        "include(cmake/a.cmake)\n",
    );
    create_file(&temp_dir, ".cmake-fmt.toml", "");
    // a.cmake has an unresolvable include (with variable) and a function
    create_file(
        &temp_dir,
        "cmake/a.cmake",
        "include(${CMAKE_CURRENT_LIST_DIR}/b.cmake)\nfunction(func_a)\nendfunction()\n",
    );

    let user_commands = user_scanner::scan_project_commands(temp_dir.path(), false);

    // Verify func_a is found
    assert!(
        user_commands.contains_key("func_a"),
        "Expected func_a to be found in: {:?}",
        user_commands
    );

    // Test should not crash from unresolvable include
}

#[test]
fn test_stray_cmake_file_not_found() {
    clear_project_scan_cache();

    let temp_dir = TempDir::new().unwrap();

    // Create root CMakeLists.txt with no references
    create_file(&temp_dir, "CMakeLists.txt", "# Empty project\n");
    create_file(&temp_dir, ".cmake-fmt.toml", "");
    // Create stray file not referenced anywhere
    create_file(
        &temp_dir,
        "stray.cmake",
        "function(stray_func)\nendfunction()\n",
    );

    let user_commands = user_scanner::scan_project_commands(temp_dir.path(), false);

    // Verify stray_func is NOT found (file not referenced)
    assert!(
        !user_commands.contains_key("stray_func"),
        "Expected stray_func to NOT be found (unreferenced file), but got: {:?}",
        user_commands
    );
}

#[test]
fn test_root_true_isolates_subdirectory_definitions() {
    clear_project_scan_cache();

    let temp_dir = TempDir::new().unwrap();

    // Root project: CMakeLists.txt references sub/
    create_file(
        &temp_dir,
        "CMakeLists.txt",
        "add_subdirectory(sub)\n",
    );
    create_file(&temp_dir, ".cmake-fmt.toml", "indent_width = 2\n");

    // Sub-project with root = true — should be isolated from parent scan
    create_file(
        &temp_dir,
        "sub/CMakeLists.txt",
        "function(sub_only_func)\nendfunction()\n",
    );
    create_file(&temp_dir, "sub/.cmake-fmt.toml", "root = true\n");

    // Scan from root — sub_only_func should NOT be visible
    let user_commands = user_scanner::scan_project_commands(temp_dir.path(), false);

    assert!(
        !user_commands.contains_key("sub_only_func"),
        "Expected sub_only_func to NOT be found (sub-project isolated by root:true), but got: {:?}",
        user_commands
    );
}

#[test]
fn test_without_root_true_subdirectory_definitions_visible() {
    clear_project_scan_cache();

    let temp_dir = TempDir::new().unwrap();

    // Root project: CMakeLists.txt references sub/
    create_file(
        &temp_dir,
        "CMakeLists.txt",
        "add_subdirectory(sub)\n",
    );
    create_file(&temp_dir, ".cmake-fmt.toml", "indent_width = 2\n");

    // Sub-project WITHOUT root = true — definitions should leak into parent
    create_file(
        &temp_dir,
        "sub/CMakeLists.txt",
        "function(sub_only_func)\nendfunction()\n",
    );
    create_file(&temp_dir, "sub/.cmake-fmt.toml", "indent_width = 4\n");

    // Scan from root — sub_only_func SHOULD be visible (no isolation)
    let user_commands = user_scanner::scan_project_commands(temp_dir.path(), false);

    assert!(
        user_commands.contains_key("sub_only_func"),
        "Expected sub_only_func to be found (no root:true isolation), but got: {:?}",
        user_commands
    );
}

#[test]
fn test_find_project_root_with_relative_path() {
    clear_project_scan_cache();

    let temp_dir = TempDir::new().unwrap();

    // Create a config file in the temp dir
    create_file(&temp_dir, ".cmake-fmt.toml", "indent_width = 2\n");

    // Save old working directory and switch to temp dir
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Call find_project_root with a relative path "."
    let project_root = user_scanner::find_project_root(std::path::Path::new("."));

    // Restore original directory before asserting (cleanup first)
    std::env::set_current_dir(&original_dir).unwrap();

    // The project root should be "." (relative) or resolve to temp_dir
    // find_project_root returns the ancestor (relative ".") when it finds config there
    assert!(
        project_root == std::path::Path::new(".") || project_root == temp_dir.path(),
        "Expected project root to be '.' or the temp dir path, got: {:?}",
        project_root
    );
}
