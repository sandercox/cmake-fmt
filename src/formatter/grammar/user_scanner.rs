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

/// Scan all CMake files in a project and extract command grammars from cmake_parse_arguments
///
/// Returns a map of lowercase command names to their extracted grammars.
/// Later definitions win (matching CMake's last-definition-wins behavior).
pub fn scan_project_grammars(project_root: &Path) -> HashMap<String, super::CommandGrammar> {
    let mut all_grammars = HashMap::new();

    let cmake_files = find_cmake_files(project_root);

    for file_path in cmake_files {
        // Read file content
        let Ok(content) = fs::read_to_string(&file_path) else {
            // Skip files we can't read
            continue;
        };

        // Parse the file
        let cst = crate::cst::parse_text(&content);

        // Extract grammars from this file
        let file_grammars = extract_grammars_from_file(&cst.root);

        // Merge into master map (later definitions win)
        all_grammars.extend(file_grammars);
    }

    all_grammars
}

/// Extract command grammars from a single CMake file
///
/// Finds function()/macro() definitions and extracts grammars from their bodies
fn extract_grammars_from_file(root: &crate::SyntaxNode) -> HashMap<String, super::CommandGrammar> {
    use crate::cst::CommandInvocation;

    let mut grammars = HashMap::new();
    let mut children_iter = root.children().peekable();

    while let Some(child) = children_iter.next() {
        // Cast to CommandInvocation
        let Some(cmd) = CommandInvocation::cast(child) else {
            continue;
        };

        let Some(cmd_name) = cmd.name_text() else {
            continue;
        };

        let name_lower = cmd_name.to_lowercase();

        // Check if this is a function() or macro() definition
        if name_lower == "function" || name_lower == "macro" {
            // Extract the function/macro name from the first argument
            let func_name = if let Some(arg_list) = cmd.argument_list() {
                arg_list.arguments()
                    .next()
                    .map(|t| t.text().to_string())
            } else {
                None
            };

            let Some(func_name) = func_name else {
                continue;
            };

            // Collect all commands until the matching endfunction()/endmacro()
            let _end_keyword = if name_lower == "function" {
                "endfunction"
            } else {
                "endmacro"
            };

            let mut body_commands = Vec::new();
            let mut nesting_depth = 1; // Start at 1 for the current function/macro

            // Collect body commands
            while let Some(body_child) = children_iter.next() {
                let Some(body_cmd) = CommandInvocation::cast(body_child) else {
                    continue;
                };

                let Some(body_cmd_name) = body_cmd.name_text() else {
                    continue;
                };

                let body_name_lower = body_cmd_name.to_lowercase();

                // Track nesting
                if body_name_lower == "function" || body_name_lower == "macro" {
                    nesting_depth += 1;
                } else if body_name_lower == "endfunction" || body_name_lower == "endmacro" {
                    nesting_depth -= 1;
                    if nesting_depth == 0 {
                        // Reached the end of this function/macro
                        break;
                    }
                }

                // Only collect commands at the top level (nesting_depth == 1)
                if nesting_depth == 1 {
                    body_commands.push(body_cmd);
                }
            }

            // Extract grammar from the body
            if let Some(grammar) = super::argparse_extractor::extract_command_grammars_from_body(
                &func_name,
                &body_commands
            ) {
                grammars.insert(func_name.to_lowercase(), grammar);
            }
        }
    }

    grammars
}
