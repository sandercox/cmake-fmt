use anyhow::{Context, Result};
use cmake_fmt::formatter::FormatConfig;
use std::fs;
use std::path::{Path, PathBuf};

/// Supported config file names in priority order
///
/// Extensionless first (like clang-format), then explicit extensions
const CONFIG_FILENAMES: &[&str] = &[
    ".cmake-fmt",
    ".cmake-fmt.yml",
    ".cmake-fmt.yaml",
    ".cmake-fmt.tml",
    ".cmake-fmt.toml",
];

/// Find all config files by walking up the directory tree
///
/// Searches for config files in this priority order per directory:
/// 1. `.cmake-fmt` (extensionless, parsed as YAML)
/// 2. `.cmake-fmt.yml` (YAML shorthand)
/// 3. `.cmake-fmt.yaml` (YAML)
/// 4. `.cmake-fmt.tml` (TOML shorthand)
/// 5. `.cmake-fmt.toml` (TOML)
///
/// Returns all config files found (nearest-first order), stopping at filesystem root
pub fn find_config_files(start_dir: &Path) -> Vec<PathBuf> {
    let mut config_files = Vec::new();

    for ancestor in start_dir.ancestors() {
        // Find first matching config file in this directory
        for filename in CONFIG_FILENAMES {
            let config_path = ancestor.join(filename);
            if config_path.exists() && config_path.is_file() {
                config_files.push(config_path);
                break; // First match in this directory wins
            }
        }
    }

    config_files
}

/// Load a config file as a raw TOML table for merging
///
/// Supports TOML and YAML formats, converting YAML to TOML table representation
fn load_config_as_table(path: &Path) -> Result<toml::Table> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match extension {
        "toml" | "tml" => toml::from_str::<toml::Table>(&content)
            .with_context(|| format!("Failed to parse TOML config: {}", path.display())),
        _ => {
            // YAML or extensionless - parse and convert to TOML table
            let yaml_value: serde_yml::Value = serde_yml::from_str(&content)
                .with_context(|| format!("Failed to parse YAML config: {}", path.display()))?;

            yml_value_to_toml_table(yaml_value)
                .ok_or_else(|| anyhow::anyhow!("YAML config must be a mapping at top level"))
        }
    }
}

/// Convert a serde_yml::Value to a toml::Value
fn yml_value_to_toml_value(v: serde_yml::Value) -> Option<toml::Value> {
    match v {
        serde_yml::Value::String(s) => Some(toml::Value::String(s)),
        serde_yml::Value::Bool(b) => Some(toml::Value::Boolean(b)),
        serde_yml::Value::Number(n) => {
            if n.is_i64() {
                Some(toml::Value::Integer(n.as_i64()?))
            } else if n.is_f64() {
                Some(toml::Value::Float(n.as_f64()?))
            } else {
                None
            }
        }
        serde_yml::Value::Sequence(seq) => {
            let mut arr = Vec::new();
            for item in seq {
                arr.push(yml_value_to_toml_value(item)?);
            }
            Some(toml::Value::Array(arr))
        }
        serde_yml::Value::Mapping(map) => {
            let mut table = toml::Table::new();
            for (k, v) in map {
                if let serde_yml::Value::String(key) = k
                    && let Some(value) = yml_value_to_toml_value(v)
                {
                    table.insert(key, value);
                }
            }
            Some(toml::Value::Table(table))
        }
        serde_yml::Value::Null | serde_yml::Value::Tagged(_) => None,
    }
}

/// Helper to extract TOML table from Value::Mapping conversion
fn yml_value_to_toml_table(v: serde_yml::Value) -> Option<toml::Table> {
    match yml_value_to_toml_value(v)? {
        toml::Value::Table(t) => Some(t),
        _ => None,
    }
}

/// Merge two TOML tables with custom merge rules
///
/// - command_grammars: merge at command-name level (overlay adds/overrides per-command)
/// - grammar_files: concatenate arrays (parent + child)
/// - Other keys: overlay replaces base
fn merge_tables(base: &mut toml::Table, overlay: &toml::Table) {
    for (key, overlay_value) in overlay {
        if key == "command_grammars" {
            // Merge command_grammars at command-name level
            match (base.get_mut("command_grammars"), overlay_value) {
                (Some(toml::Value::Table(base_cmds)), toml::Value::Table(overlay_cmds)) => {
                    for (cmd_name, cmd_grammar) in overlay_cmds {
                        base_cmds.insert(cmd_name.clone(), cmd_grammar.clone());
                    }
                }
                (None, toml::Value::Table(_)) => {
                    // Base doesn't have command_grammars, insert entire overlay
                    base.insert(key.clone(), overlay_value.clone());
                }
                _ => {
                    // Mismatched types, overlay wins
                    base.insert(key.clone(), overlay_value.clone());
                }
            }
        } else if key == "grammar_files" {
            // Concatenate grammar_files arrays
            match (base.get_mut("grammar_files"), overlay_value) {
                (Some(toml::Value::Array(base_files)), toml::Value::Array(overlay_files)) => {
                    base_files.extend(overlay_files.iter().cloned());
                }
                (None, toml::Value::Array(_)) => {
                    // Base doesn't have grammar_files, insert entire overlay
                    base.insert(key.clone(), overlay_value.clone());
                }
                _ => {
                    // Mismatched types, overlay wins
                    base.insert(key.clone(), overlay_value.clone());
                }
            }
        } else {
            // All other keys: simple override
            base.insert(key.clone(), overlay_value.clone());
        }
    }
}

/// Resolve the final configuration
///
/// Applies configuration in this order (later overrides earlier):
/// 1. Default values
/// 2. Config files (root-most to nearest, merged)
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
    // Determine search directory (must be absolute so ancestors() walks the real tree)
    let search_dir = if let Some(path) = file_path {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        if parent.is_absolute() {
            parent.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(parent))
                .unwrap_or_else(|_| parent.to_path_buf())
        }
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    // Find all config files (nearest-first)
    let config_paths = find_config_files(&search_dir);

    // If no config files found, use defaults
    if config_paths.is_empty() {
        let mut config = FormatConfig::default();

        // Apply CLI overrides
        if let Some(style) = style_override {
            apply_style_overrides(&mut config, style);
        }

        // Add CLI grammar files
        config.grammar_files.extend(grammar_files.iter().cloned());

        return config;
    }

    // Merge configs root-first (reverse the nearest-first order)
    let mut merged_table = toml::Table::new();

    for config_path in config_paths.iter().rev() {
        match load_config_as_table(config_path) {
            Ok(mut file_table) => {
                // Resolve grammar_files paths BEFORE merging
                if let Some(toml::Value::Array(grammar_files_array)) =
                    file_table.get_mut("grammar_files")
                {
                    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));

                    for item in grammar_files_array.iter_mut() {
                        if let toml::Value::String(path_str) = item {
                            let path = PathBuf::from(&*path_str);
                            if path.is_relative() {
                                let resolved = config_dir.join(path);
                                *path_str = resolved.to_string_lossy().to_string();
                            }
                        }
                    }
                }

                // Check for root flag - if true, discard all previously merged ancestor configs
                if let Some(toml::Value::Boolean(true)) = file_table.get("root") {
                    merged_table.clear();
                }
                // Remove root key before merging (meta-option, not a format setting)
                file_table.remove("root");

                // Merge this config into the accumulated config
                merge_tables(&mut merged_table, &file_table);
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to load config file {}: {:#}",
                    config_path.display(),
                    e
                );
                // Skip this config and continue
            }
        }
    }

    // Deserialize merged table to FormatConfig
    // This applies #[serde(default)] for any unspecified fields
    let config = match toml::to_string(&merged_table) {
        Ok(toml_str) => match toml::from_str::<FormatConfig>(&toml_str) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Warning: Failed to deserialize merged config: {:#}", e);
                FormatConfig::default()
            }
        },
        Err(e) => {
            eprintln!("Warning: Failed to serialize merged config: {:#}", e);
            FormatConfig::default()
        }
    };

    let mut config = config;

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
            eprintln!(
                "Warning: Invalid style override (expected key=value): {}",
                pair
            );
            continue;
        }

        let key = parts[0].trim();
        let value = parts[1].trim();

        if let Err(msg) = config.apply_override(key, value) {
            eprintln!("Warning: {}", msg);
        }
    }
}
