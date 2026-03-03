use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use super::{CommandGrammar, Grammar, KeywordType};

/// Grammar export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrammarFormat {
    Toml,
    Yaml,
}

/// Detect grammar format from file extension
pub fn detect_grammar_format(path: &Path) -> GrammarFormat {
    match path.extension().and_then(|e| e.to_str()) {
        Some("toml" | "tml") => GrammarFormat::Toml,
        Some("yaml" | "yml") => GrammarFormat::Yaml,
        _ => GrammarFormat::Yaml, // Default to YAML
    }
}

/// A single grammar entry in the TOML interchange format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarEntry {
    pub command: String,
    /// Optional mode name for multi-mode commands (e.g., "TARGETS" for install)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Map of keyword name to keyword type string
    pub keywords: BTreeMap<String, String>,
}

/// Top-level TOML grammar file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarFile {
    #[serde(rename = "grammar")]
    pub grammars: Vec<GrammarEntry>,
}

/// Convert KeywordType to readable string for export
pub fn keyword_type_to_str(kw_type: &KeywordType) -> &'static str {
    match kw_type {
        KeywordType::Flag => "Flag",
        KeywordType::SingleValue => "SingleValue",
        KeywordType::MultiValue => "MultiValue",
        KeywordType::PairValue => "PairValue",
        KeywordType::BinPack => "BinPack",
    }
}

/// Convert string to KeywordType for import
pub fn str_to_keyword_type(s: &str) -> Result<KeywordType, String> {
    match s {
        "Flag" => Ok(KeywordType::Flag),
        "SingleValue" => Ok(KeywordType::SingleValue),
        "MultiValue" => Ok(KeywordType::MultiValue),
        "PairValue" => Ok(KeywordType::PairValue),
        "BinPack" => Ok(KeywordType::BinPack),
        _ => Err(format!("Unknown keyword type: {}", s)),
    }
}

/// Build grammar entries from Grammar HashMap
fn build_grammar_entries(grammars: &HashMap<String, Grammar>) -> Vec<GrammarEntry> {
    let mut entries = Vec::new();

    // Sort by command name for deterministic output
    let mut sorted_commands: Vec<_> = grammars.keys().collect();
    sorted_commands.sort();

    for command_name in sorted_commands {
        let grammar = &grammars[command_name];
        match grammar {
            Grammar::Simple(cg) => {
                // Single-mode command: one entry, no mode field
                let mut keywords = BTreeMap::new();
                for (kw, kw_type) in &cg.keywords {
                    keywords.insert(kw.clone(), keyword_type_to_str(kw_type).to_string());
                }
                entries.push(GrammarEntry {
                    command: command_name.clone(),
                    mode: None,
                    keywords,
                });
            }
            Grammar::Modes { modes } => {
                // Multi-mode command: one entry per mode
                let mut sorted_modes: Vec<_> = modes.keys().collect();
                sorted_modes.sort();
                for mode_name in sorted_modes {
                    let cg = &modes[mode_name];
                    let mut keywords = BTreeMap::new();
                    for (kw, kw_type) in &cg.keywords {
                        keywords.insert(kw.clone(), keyword_type_to_str(kw_type).to_string());
                    }
                    entries.push(GrammarEntry {
                        command: command_name.clone(),
                        mode: Some(mode_name.clone()),
                        keywords,
                    });
                }
            }
        }
    }

    entries
}

/// Build grammar entries from CommandGrammar HashMap (custom grammars only)
fn build_command_grammar_entries(
    grammars: &HashMap<String, CommandGrammar>,
    name_map: Option<&HashMap<String, String>>,
) -> Vec<GrammarEntry> {
    let mut entries = Vec::new();

    // Sort by command name for deterministic output
    let mut sorted_commands: Vec<_> = grammars.keys().collect();
    sorted_commands.sort();

    for command_name in sorted_commands {
        let cg = &grammars[command_name];
        let mut keywords = BTreeMap::new();
        for (kw, kw_type) in &cg.keywords {
            keywords.insert(kw.clone(), keyword_type_to_str(kw_type).to_string());
        }
        // Use original casing from name_map if available
        let display_name = name_map
            .and_then(|m| m.get(command_name))
            .cloned()
            .unwrap_or_else(|| command_name.clone());
        entries.push(GrammarEntry {
            command: display_name,
            mode: None, // User grammars are always simple
            keywords,
        });
    }

    entries
}

/// Export grammars to TOML format
pub fn export_grammars_to_toml(grammars: &HashMap<String, Grammar>) -> String {
    let entries = build_grammar_entries(grammars);
    let grammar_file = GrammarFile { grammars: entries };
    let toml_string = toml::to_string_pretty(&grammar_file).unwrap();

    // Add header comment
    format!(
        "# cmake-fmt grammar export\n# Generated by cmake-fmt\n\n{}",
        toml_string
    )
}

/// Export grammars to YAML format
pub fn export_grammars_to_yaml(grammars: &HashMap<String, Grammar>) -> String {
    let entries = build_grammar_entries(grammars);
    let grammar_file = GrammarFile { grammars: entries };
    let yaml_string = serde_yml::to_string(&grammar_file).unwrap();

    // Add header comment
    format!(
        "# cmake-fmt grammar export\n# Generated by cmake-fmt\n\n{}",
        yaml_string
    )
}

/// Export command grammars to TOML format (custom grammars only)
pub fn export_command_grammars_to_toml(
    grammars: &HashMap<String, CommandGrammar>,
    name_map: Option<&HashMap<String, String>>,
) -> String {
    let entries = build_command_grammar_entries(grammars, name_map);
    let grammar_file = GrammarFile { grammars: entries };
    let toml_string = toml::to_string_pretty(&grammar_file).unwrap();

    // Add header comment
    format!(
        "# cmake-fmt grammar export\n# Generated by cmake-fmt\n\n{}",
        toml_string
    )
}

/// Export command grammars to YAML format (custom grammars only)
pub fn export_command_grammars_to_yaml(
    grammars: &HashMap<String, CommandGrammar>,
    name_map: Option<&HashMap<String, String>>,
) -> String {
    let entries = build_command_grammar_entries(grammars, name_map);
    let grammar_file = GrammarFile { grammars: entries };
    let yaml_string = serde_yml::to_string(&grammar_file).unwrap();

    // Add header comment
    format!(
        "# cmake-fmt grammar export\n# Generated by cmake-fmt\n\n{}",
        yaml_string
    )
}

/// Export grammars with format dispatch
pub fn export_grammars(grammars: &HashMap<String, Grammar>, format: &GrammarFormat) -> String {
    match format {
        GrammarFormat::Toml => export_grammars_to_toml(grammars),
        GrammarFormat::Yaml => export_grammars_to_yaml(grammars),
    }
}

/// Export command grammars with format dispatch
pub fn export_command_grammars(
    grammars: &HashMap<String, CommandGrammar>,
    format: &GrammarFormat,
    name_map: Option<&HashMap<String, String>>,
) -> String {
    match format {
        GrammarFormat::Toml => export_command_grammars_to_toml(grammars, name_map),
        GrammarFormat::Yaml => export_command_grammars_to_yaml(grammars, name_map),
    }
}

/// Import grammar file from TOML/YAML content
pub fn import_grammar_file(content: &str) -> Result<HashMap<String, CommandGrammar>, String> {
    // Try TOML first
    let grammar_file: GrammarFile = if let Ok(parsed) = toml::from_str(content) {
        parsed
    } else {
        // Try YAML as fallback
        serde_yml::from_str(content)
            .map_err(|e| format!("Failed to parse grammar file as TOML or YAML: {}", e))?
    };

    let mut result = HashMap::new();

    for entry in grammar_file.grammars {
        // Skip multi-mode entries for now (only simple commands supported in import)
        if let Some(mode) = &entry.mode {
            eprintln!(
                "Warning: Skipping multi-mode grammar entry for command '{}' (mode: {})",
                entry.command, mode
            );
            continue;
        }

        // Convert keyword type strings back to KeywordType
        let mut keywords = HashMap::new();
        for (kw, type_str) in entry.keywords {
            let kw_type = str_to_keyword_type(&type_str)
                .map_err(|e| format!("In command '{}': {}", entry.command, e))?;
            keywords.insert(kw, kw_type);
        }

        // Store with lowercase command name
        result.insert(
            entry.command.to_lowercase(),
            CommandGrammar {
                keywords,
                force_args_on_new_line: false,
                sub_keywords: HashSet::new(),
                collection_keywords: HashSet::new(),
            },
        );
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_type_roundtrip() {
        let types = [
            KeywordType::Flag,
            KeywordType::SingleValue,
            KeywordType::MultiValue,
            KeywordType::PairValue,
            KeywordType::BinPack,
        ];
        for kw_type in types {
            let s = keyword_type_to_str(&kw_type);
            let parsed = str_to_keyword_type(s).unwrap();
            assert_eq!(kw_type, parsed);
        }
    }

    #[test]
    fn test_invalid_keyword_type_string() {
        assert!(str_to_keyword_type("InvalidType").is_err());
    }
}
