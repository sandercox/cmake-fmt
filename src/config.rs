use anyhow::{Context, Result};
use cmake_formatter::formatter::FormatConfig;
use std::fs;
use std::path::{Path, PathBuf};

/// Find a config file by walking up the directory tree
///
/// Searches for `.cmake-format.toml` first, then `.cmake-format.yaml`
/// Returns the first config file found, or None if no config file exists
pub fn find_config_file(start_dir: &Path) -> Option<PathBuf> {
    for ancestor in start_dir.ancestors() {
        // Check for TOML config first
        let toml_path = ancestor.join(".cmake-format.toml");
        if toml_path.exists() && toml_path.is_file() {
            return Some(toml_path);
        }

        // Check for YAML config
        let yaml_path = ancestor.join(".cmake-format.yaml");
        if yaml_path.exists() && yaml_path.is_file() {
            return Some(yaml_path);
        }
    }
    None
}

/// Load a config file from disk
///
/// Supports both TOML (.toml) and YAML (.yaml, .yml) formats
pub fn load_config_file(path: &Path) -> Result<FormatConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    // Determine format from extension
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match extension {
        "toml" => {
            toml::from_str(&content)
                .with_context(|| format!("Failed to parse TOML config: {}", path.display()))
        }
        "yaml" | "yml" => {
            serde_yml::from_str(&content)
                .with_context(|| format!("Failed to parse YAML config: {}", path.display()))
        }
        _ => {
            anyhow::bail!("Unsupported config file extension: {}", extension)
        }
    }
}

/// Resolve the final configuration
///
/// Applies configuration in this order (later overrides earlier):
/// 1. Default values
/// 2. Config file (if found)
/// 3. CLI style overrides
///
/// # Arguments
/// * `file_path` - Path to the file being formatted (used to find config), or None for stdin
/// * `style_override` - CLI style override string (e.g., "indent_width=4,max_line_length=120")
pub fn resolve_config(file_path: Option<&Path>, style_override: Option<&str>) -> FormatConfig {
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
    if let Some(config_path) = find_config_file(search_dir) {
        match load_config_file(&config_path) {
            Ok(file_config) => {
                config = file_config;
            }
            Err(e) => {
                eprintln!("Warning: Failed to load config file: {:#}", e);
                // Continue with defaults
            }
        }
    }

    // Apply CLI style overrides
    if let Some(style) = style_override {
        apply_style_overrides(&mut config, style);
    }

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

        match key {
            "indent_width" => {
                match value.parse::<usize>() {
                    Ok(v) => config.indent_width = v,
                    Err(_) => eprintln!("Warning: Invalid value for indent_width: {}", value),
                }
            }
            "max_line_length" => {
                match value.parse::<usize>() {
                    Ok(v) => config.max_line_length = v,
                    Err(_) => eprintln!("Warning: Invalid value for max_line_length: {}", value),
                }
            }
            "use_tabs" => {
                match value.parse::<bool>() {
                    Ok(v) => config.use_tabs = v,
                    Err(_) => eprintln!("Warning: Invalid value for use_tabs: {}", value),
                }
            }
            "command_case" => {
                match value {
                    "lowercase" => config.command_case = cmake_formatter::formatter::CommandCase::Lowercase,
                    "uppercase" => config.command_case = cmake_formatter::formatter::CommandCase::Uppercase,
                    "preserve" => config.command_case = cmake_formatter::formatter::CommandCase::Preserve,
                    _ => eprintln!("Warning: Invalid value for command_case (expected lowercase, uppercase, or preserve): {}", value),
                }
            }
            "max_blank_lines" => {
                match value.parse::<usize>() {
                    Ok(v) => config.max_blank_lines = v,
                    Err(_) => eprintln!("Warning: Invalid value for max_blank_lines: {}", value),
                }
            }
            _ => {
                eprintln!("Warning: Unknown config key (ignored): {}", key);
            }
        }
    }
}
