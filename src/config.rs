use anyhow::{Context, Result};
use cmake_fmt::formatter::FormatConfig;
use std::fs;
use std::path::{Path, PathBuf};

/// Supported config file names in priority order
///
/// Explicit extensions first, then extensionless (YAML, like clang-format)
const CONFIG_FILENAMES: &[&str] = &[
    ".cmake-fmt.toml",
    ".cmake-fmt.tml",
    ".cmake-fmt.yaml",
    ".cmake-fmt.yml",
    ".cmake-fmt",
];

/// Find a config file by walking up the directory tree
///
/// Searches for config files in this priority order:
/// 1. `.cmake-fmt.toml` (TOML)
/// 2. `.cmake-fmt.tml` (TOML shorthand)
/// 3. `.cmake-fmt.yaml` (YAML)
/// 4. `.cmake-fmt.yml` (YAML shorthand)
/// 5. `.cmake-fmt` (extensionless, parsed as YAML)
///
/// Returns the first config file found, or None if no config file exists
pub fn find_config_file(start_dir: &Path) -> Option<PathBuf> {
    for ancestor in start_dir.ancestors() {
        for filename in CONFIG_FILENAMES {
            let config_path = ancestor.join(filename);
            if config_path.exists() && config_path.is_file() {
                return Some(config_path);
            }
        }
    }
    None
}

/// Load a config file from disk
///
/// Supports TOML (.toml, .tml) and YAML (.yaml, .yml, extensionless) formats
pub fn load_config_file(path: &Path) -> Result<FormatConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    // Determine format from extension
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match extension {
        "toml" | "tml" => {
            toml::from_str(&content)
                .with_context(|| format!("Failed to parse TOML config: {}", path.display()))
        }
        "yaml" | "yml" => {
            serde_yml::from_str(&content)
                .with_context(|| format!("Failed to parse YAML config: {}", path.display()))
        }
        _ => {
            // Extensionless config files (like .cmake-fmt) default to YAML (like clang-format)
            serde_yml::from_str(&content)
                .with_context(|| format!("Failed to parse config as YAML: {}", path.display()))
        }
    }
}

/// Resolve the final configuration
///
/// Applies configuration in this order (later overrides earlier):
/// 1. Default values
/// 2. Config file (if found)
/// 3. CLI style overrides
/// 4. CLI grammar files
///
/// # Arguments
/// * `file_path` - Path to the file being formatted (used to find config), or None for stdin
/// * `style_override` - CLI style override string (e.g., "indent_width=4,max_line_length=120")
/// * `grammar_files` - CLI grammar files to import
pub fn resolve_config(
    file_path: Option<&Path>,
    style_override: Option<&str>,
    grammar_files: &[PathBuf],
) -> FormatConfig {
    let mut config = FormatConfig::default();

    // Determine search directory
    let search_dir = if let Some(path) = file_path {
        // For file arguments, search from the file's parent directory
        path.parent().unwrap_or_else(|| Path::new("."))
    } else {
        // For stdin, search from current working directory
        Path::new(".")
    };

    // Try to find and load config file
    let config_path = find_config_file(search_dir);
    if let Some(ref config_file_path) = config_path {
        match load_config_file(config_file_path) {
            Ok(file_config) => {
                config = file_config;
            }
            Err(e) => {
                eprintln!("Warning: Failed to load config file: {:#}", e);
                // Continue with defaults
            }
        }
    }

    // Resolve grammar_files paths relative to config file location
    if let Some(ref config_file_path) = config_path {
        let config_dir = config_file_path.parent().unwrap_or_else(|| Path::new("."));
        config.grammar_files = config.grammar_files.iter().map(|p| {
            if p.is_relative() {
                config_dir.join(p)
            } else {
                p.clone()
            }
        }).collect();
    }

    // Apply CLI style overrides
    if let Some(style) = style_override {
        apply_style_overrides(&mut config, style);
    }

    // Add CLI grammar files
    config.grammar_files.extend(grammar_files.iter().cloned());

    config
}

/// Apply CLI style overrides to a config
///
/// Parses comma-separated key=value pairs and applies them to the config
/// Example: "indent_width=4,max_line_length=120"
fn apply_style_overrides(config: &mut FormatConfig, style: &str) {
    for pair in style.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }

        let parts: Vec<&str> = pair.splitn(2, '=').collect();
        if parts.len() != 2 {
            eprintln!("Warning: Invalid style override (expected key=value): {}", pair);
            continue;
        }

        let key = parts[0].trim();
        let value = parts[1].trim();

        if let Err(msg) = config.apply_override(key, value) {
            eprintln!("Warning: {}", msg);
        }
    }
}
