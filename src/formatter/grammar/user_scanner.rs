use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Find the project root by walking up the directory tree looking for a config file
///
/// Searches for `.cmake-fmt.toml` first, then `.cmake-fmt.yaml`
/// Returns the directory containing the config file, or the start_path if none found
pub fn find_project_root(start_path: &Path) -> PathBuf {
    // Walk up the directory tree
    for ancestor in start_path.ancestors() {
        // Check for TOML config first
        let toml_path = ancestor.join(".cmake-fmt.toml");
        if toml_path.exists() && toml_path.is_file() {
            return ancestor.to_path_buf();
        }

        // Check for YAML config
        let yaml_path = ancestor.join(".cmake-fmt.yaml");
        if yaml_path.exists() && yaml_path.is_file() {
            return ancestor.to_path_buf();
        }
    }

    // No config file found, use start_path as project root
    start_path.to_path_buf()
}

/// Check if a path is a CMake file (CMakeLists.txt or *.cmake)
fn is_cmake_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    if let Some(filename) = path.file_name() {
        if filename == "CMakeLists.txt" {
            return true;
        }
    }

    if let Some(ext) = path.extension() {
        if ext == "cmake" {
            return true;
        }
    }

    false
}

/// Find all CMake files in a project tree, respecting .gitignore
///
/// Uses the `ignore` crate to traverse directories while respecting .gitignore patterns
/// Returns a list of all CMakeLists.txt and *.cmake files found
pub fn find_cmake_files(project_root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    let walker = ignore::WalkBuilder::new(project_root)
        .git_ignore(true)      // Respect .gitignore
        .git_exclude(true)     // Respect .git/info/exclude
        .hidden(false)         // Include hidden files
        .follow_links(false)   // Don't follow symlinks (prevent infinite loops)
        .build();

    for entry_result in walker {
        // Skip entries with errors (permission denied, etc.)
        let Ok(entry) = entry_result else {
            continue;
        };

        let path = entry.path();

        if is_cmake_file(path) {
            files.push(path.to_path_buf());
        }
    }

    files
}

/// Scan all CMake files in a project and extract user command definitions
///
/// Returns a map of lowercase command names to their original casing as defined.
/// Later definitions win (matching CMake's last-definition-wins behavior).
pub fn scan_project_commands(project_root: &Path) -> HashMap<String, String> {
    let mut all_defs = HashMap::new();

    let cmake_files = find_cmake_files(project_root);

    for file_path in cmake_files {
        // Read file content
        let Ok(content) = fs::read_to_string(&file_path) else {
            // Skip files we can't read
            continue;
        };

        // Parse the file
        let cst = crate::cst::parse_text(&content);

        // Scan for user command definitions
        let file_defs = crate::formatter::user_commands::scan_user_command_definitions(&cst.root);

        // Merge into master map (later definitions win)
        all_defs.extend(file_defs);
    }

    all_defs
}
