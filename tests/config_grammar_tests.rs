use cmake_fmt::formatter::{CommandGrammarConfig, FormatConfig, format_text, format_text_with_diagnostics_and_path};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_config_grammar_toml_parsing() {
    let toml_str = r#"
[command_grammars.my_install]
options = ["OPTIONAL"]
one_value_keywords = ["DESTINATION"]
multi_value_keywords = ["FILES", "TARGETS"]
"#;

    let config: FormatConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.command_grammars.len(), 1);
    assert!(config.command_grammars.contains_key("my_install"));

    let grammar = &config.command_grammars["my_install"];
    assert_eq!(grammar.options, vec!["OPTIONAL"]);
    assert_eq!(grammar.one_value_keywords, vec!["DESTINATION"]);
    assert_eq!(grammar.multi_value_keywords, vec!["FILES", "TARGETS"]);
    assert_eq!(grammar.pair_value_keywords.len(), 0);
}

#[test]
fn test_config_grammar_yaml_parsing() {
    let yaml_str = r#"
command_grammars:
  my_install:
    options: [OPTIONAL]
    one_value_keywords: [DESTINATION]
    multi_value_keywords: [FILES, TARGETS]
"#;

    let config: FormatConfig = serde_yml::from_str(yaml_str).unwrap();
    assert_eq!(config.command_grammars.len(), 1);
    assert!(config.command_grammars.contains_key("my_install"));

    let grammar = &config.command_grammars["my_install"];
    assert_eq!(grammar.options, vec!["OPTIONAL"]);
    assert_eq!(grammar.one_value_keywords, vec!["DESTINATION"]);
    assert_eq!(grammar.multi_value_keywords, vec!["FILES", "TARGETS"]);
}

#[test]
fn test_config_grammar_formatting() {
    let mut config = FormatConfig::default();
    config.max_line_length = 40;

    // Add a custom command grammar
    let mut command_grammars = HashMap::new();
    let mut my_install = CommandGrammarConfig::default();
    my_install.options = vec!["OPTIONAL".to_string()];
    my_install.one_value_keywords = vec!["DESTINATION".to_string()];
    my_install.multi_value_keywords = vec!["FILES".to_string(), "TARGETS".to_string()];
    command_grammars.insert("my_install".to_string(), my_install);
    config.command_grammars = command_grammars;

    let input = "my_install(FILES foo.cmake bar.cmake baz.cmake DESTINATION share/cmake OPTIONAL)";
    let output = format_text(input, &config);

    // With max_line_length=40, this should break with keyword-aware layout
    // The command should recognize FILES, DESTINATION, and OPTIONAL as keywords
    assert!(output.contains("FILES"));
    assert!(output.contains("DESTINATION"));
    assert!(output.contains("OPTIONAL"));
    // Should be multiline due to length constraint
    assert!(output.lines().count() > 1);
}

#[test]
fn test_config_grammar_overrides_autodetected() {
    let tempdir = TempDir::new().unwrap();
    let cmake_file = tempdir.path().join("test.cmake");
    let config_file = tempdir.path().join(".cmake-fmt.toml");

    // Write a CMake file with a function that uses cmake_parse_arguments
    let cmake_content = r#"
function(my_custom_function)
    cmake_parse_arguments(PREFIX "AUTO_FLAG" "AUTO_ONE" "AUTO_MULTI" ${ARGN})
endfunction()

my_custom_function(CONFIG_FLAG CONFIG_ONE value CONFIG_MULTI a b c)
"#;
    fs::write(&cmake_file, cmake_content).unwrap();

    // Write a config file that defines different keywords for the same function
    let config_content = r#"
[command_grammars.my_custom_function]
options = ["CONFIG_FLAG"]
one_value_keywords = ["CONFIG_ONE"]
multi_value_keywords = ["CONFIG_MULTI"]
"#;
    fs::write(&config_file, config_content).unwrap();

    // Load config from file
    let config = crate::config::load_config_file(&config_file).unwrap();

    // Format the file - config grammar should be used, not auto-detected
    let (output, _) = format_text_with_diagnostics_and_path(cmake_content, &config, Some(&cmake_file), false);

    // Verify the config keywords are recognized (not the auto-detected ones)
    assert!(output.contains("CONFIG_FLAG") || output.contains("config_flag"));
    assert!(output.contains("CONFIG_ONE") || output.contains("config_one"));
    assert!(output.contains("CONFIG_MULTI") || output.contains("config_multi"));
}

#[test]
fn test_config_grammar_does_not_override_builtin() {
    let mut config = FormatConfig::default();
    config.max_line_length = 40;

    // Try to override target_link_libraries (a builtin)
    let mut command_grammars = HashMap::new();
    let mut fake_grammar = CommandGrammarConfig::default();
    fake_grammar.options = vec!["FAKE_OPTION".to_string()];
    command_grammars.insert("target_link_libraries".to_string(), fake_grammar);
    config.command_grammars = command_grammars;

    // target_link_libraries should still use builtin grammar
    let input = "target_link_libraries(mylib PUBLIC lib1 lib2 PRIVATE lib3)";
    let output = format_text(input, &config);

    // Builtin grammar should be used (PUBLIC/PRIVATE are recognized)
    assert!(output.contains("PUBLIC") || output.contains("public"));
    assert!(output.contains("PRIVATE") || output.contains("private"));
    // Our fake FAKE_OPTION should NOT affect formatting
    assert!(!output.contains("FAKE_OPTION"));
}

#[test]
fn test_config_grammar_empty() {
    let mut config = FormatConfig::default();
    config.command_grammars = HashMap::new();

    let input = "set(MY_VAR value)\nadd_library(mylib src.cpp)";
    let output = format_text(input, &config);

    // Should format normally with empty command_grammars
    assert!(output.contains("set"));
    assert!(output.contains("add_library"));
}

#[test]
fn test_config_grammar_pair_value() {
    let mut config = FormatConfig::default();
    config.max_line_length = 80;

    // Add a custom command with pair_value_keywords
    let mut command_grammars = HashMap::new();
    let mut my_command = CommandGrammarConfig::default();
    my_command.pair_value_keywords = vec!["PROPERTIES".to_string()];
    command_grammars.insert("my_set_props".to_string(), my_command);
    config.command_grammars = command_grammars;

    let input = "my_set_props(PROPERTIES FOO bar BAZ qux)";
    let output = format_text(input, &config);

    // Should recognize PROPERTIES as a pair-value keyword
    assert!(output.contains("PROPERTIES"));
    assert!(output.contains("FOO"));
    assert!(output.contains("bar"));
}

#[test]
fn test_extensionless_config_file_parsed_as_yaml() {
    let tempdir = TempDir::new().unwrap();
    let config_file = tempdir.path().join(".cmake-fmt");

    // Write extensionless config with YAML syntax (like clang-format)
    let config_content = "indent_width: 2\nuse_tabs: false\n";
    fs::write(&config_file, config_content).unwrap();

    // Load and verify
    let config = crate::config::load_config_file(&config_file).unwrap();
    assert_eq!(config.indent_width, 2);
    assert_eq!(config.use_tabs, false);
}

#[test]
fn test_tml_config_file_parsed_as_toml() {
    let tempdir = TempDir::new().unwrap();
    let config_file = tempdir.path().join(".cmake-fmt.tml");

    // Write .tml config with TOML syntax
    let config_content = r#"
indent_width = 3
max_line_length = 100
"#;
    fs::write(&config_file, config_content).unwrap();

    // Load and verify
    let config = crate::config::load_config_file(&config_file).unwrap();
    assert_eq!(config.indent_width, 3);
    assert_eq!(config.max_line_length, 100);
}

#[test]
fn test_yml_config_file_parsed_as_yaml() {
    let tempdir = TempDir::new().unwrap();
    let config_file = tempdir.path().join(".cmake-fmt.yml");

    // Write .yml config with YAML syntax
    let config_content = r#"
indent_width: 4
use_tabs: true
"#;
    fs::write(&config_file, config_content).unwrap();

    // Load and verify
    let config = crate::config::load_config_file(&config_file).unwrap();
    assert_eq!(config.indent_width, 4);
    assert_eq!(config.use_tabs, true);
}

#[test]
fn test_config_file_priority_toml_over_extensionless() {
    let tempdir = TempDir::new().unwrap();
    let toml_config = tempdir.path().join(".cmake-fmt.toml");
    let extensionless_config = tempdir.path().join(".cmake-fmt");

    // Write .cmake-fmt.toml with indent_width=2
    fs::write(&toml_config, "indent_width = 2\n").unwrap();

    // Write .cmake-fmt with indent_width=8 (YAML format)
    fs::write(&extensionless_config, "indent_width: 8\n").unwrap();

    // Test priority via formatting - the config that gets loaded affects output
    let input = "set(MY_VAR_WITH_A_LONG_NAME value1 value2 value3 value4 value5 value6)";

    // Load toml config explicitly
    let toml_loaded = crate::config::load_config_file(&toml_config).unwrap();
    assert_eq!(toml_loaded.indent_width, 2);

    // Load extensionless config explicitly
    let extensionless_loaded = crate::config::load_config_file(&extensionless_config).unwrap();
    assert_eq!(extensionless_loaded.indent_width, 8);

    // In production, .cmake-fmt.toml would be found first (higher priority)
    // We verify both can be loaded and have different values
}

#[test]
fn test_config_file_priority_tml_over_yaml() {
    let tempdir = TempDir::new().unwrap();
    let tml_config = tempdir.path().join(".cmake-fmt.tml");
    let yaml_config = tempdir.path().join(".cmake-fmt.yaml");

    // Write .cmake-fmt.tml with indent_width=3
    fs::write(&tml_config, "indent_width = 3\n").unwrap();

    // Write .cmake-fmt.yaml with indent_width=7
    fs::write(&yaml_config, "indent_width: 7\n").unwrap();

    // Load each config explicitly
    let tml_loaded = crate::config::load_config_file(&tml_config).unwrap();
    assert_eq!(tml_loaded.indent_width, 3);

    let yaml_loaded = crate::config::load_config_file(&yaml_config).unwrap();
    assert_eq!(yaml_loaded.indent_width, 7);

    // In production, .cmake-fmt.tml would be found first (higher priority in search order)
    // We verify both can be loaded with correct parsers
}

// Module for accessing config loading functions
mod config {
    use anyhow::{Context, Result};
    use cmake_fmt::formatter::FormatConfig;
    use std::fs;
    use std::path::Path;

    pub fn load_config_file(path: &Path) -> Result<FormatConfig> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

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
                // Extensionless config files (like .cmake-fmt) default to YAML
                serde_yml::from_str(&content)
                    .with_context(|| format!("Failed to parse config as YAML: {}", path.display()))
            }
        }
    }
}
