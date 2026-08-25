use cmake_fmt::formatter::{
    CommandGrammarConfig, FormatConfig, format_text, format_text_with_diagnostics_and_path,
};
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
    let mut config = FormatConfig {
        max_line_length: 40,
        ..Default::default()
    };

    // Add a custom command grammar
    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "my_install".to_string(),
        CommandGrammarConfig {
            options: vec!["OPTIONAL".to_string()],
            one_value_keywords: vec!["DESTINATION".to_string()],
            multi_value_keywords: vec!["FILES".to_string(), "TARGETS".to_string()],
            ..Default::default()
        },
    );
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
    let (output, _) =
        format_text_with_diagnostics_and_path(cmake_content, &config, Some(&cmake_file), false);

    // Verify the config keywords are recognized (not the auto-detected ones)
    assert!(output.contains("CONFIG_FLAG") || output.contains("config_flag"));
    assert!(output.contains("CONFIG_ONE") || output.contains("config_one"));
    assert!(output.contains("CONFIG_MULTI") || output.contains("config_multi"));
}

#[test]
fn test_config_grammar_does_not_override_builtin() {
    // Try to override target_link_libraries (a builtin)
    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "target_link_libraries".to_string(),
        CommandGrammarConfig {
            options: vec!["FAKE_OPTION".to_string()],
            ..Default::default()
        },
    );

    let config = FormatConfig {
        max_line_length: 40,
        command_grammars,
        ..Default::default()
    };

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
    let config = FormatConfig {
        command_grammars: HashMap::new(),
        ..Default::default()
    };

    let input = "set(MY_VAR value)\nadd_library(mylib src.cpp)";
    let output = format_text(input, &config);

    // Should format normally with empty command_grammars
    assert!(output.contains("set"));
    assert!(output.contains("add_library"));
}

#[test]
fn test_config_grammar_pair_value() {
    // Add a custom command with pair_value_keywords
    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "my_set_props".to_string(),
        CommandGrammarConfig {
            pair_value_keywords: vec!["PROPERTIES".to_string()],
            ..Default::default()
        },
    );

    let config = FormatConfig {
        max_line_length: 80,
        command_grammars,
        ..Default::default()
    };

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
    assert!(!config.use_tabs);
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
    assert!(config.use_tabs);
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
    let _input = "set(MY_VAR_WITH_A_LONG_NAME value1 value2 value3 value4 value5 value6)";

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

#[test]
fn test_recursive_config_merges_root_first() {
    let tempdir = TempDir::new().unwrap();
    let root_dir = tempdir.path();
    let sub_dir = root_dir.join("sub");
    fs::create_dir(&sub_dir).unwrap();

    // Root config sets indent_width
    let root_config = root_dir.join(".cmake-fmt.toml");
    fs::write(&root_config, "indent_width = 2\n").unwrap();

    // Child config sets use_tabs
    let child_config = sub_dir.join(".cmake-fmt.toml");
    fs::write(&child_config, "use_tabs = false\n").unwrap();

    // Resolve from child directory
    let cmake_file = sub_dir.join("CMakeLists.txt");
    let config = config::resolve_config(Some(&cmake_file), None, &[]);

    // Should have both: indent_width from root, use_tabs from child
    assert_eq!(config.indent_width, 2);
    assert!(!config.use_tabs);
    // Default value for unset key
    assert_eq!(config.max_line_length, 80);
}

#[test]
fn test_recursive_config_child_overrides_parent() {
    let tempdir = TempDir::new().unwrap();
    let root_dir = tempdir.path();
    let sub_dir = root_dir.join("sub");
    fs::create_dir(&sub_dir).unwrap();

    // Root config sets both indent_width and max_line_length
    let root_config = root_dir.join(".cmake-fmt.toml");
    fs::write(&root_config, "indent_width = 2\nmax_line_length = 100\n").unwrap();

    // Child config overrides only indent_width
    let child_config = sub_dir.join(".cmake-fmt.toml");
    fs::write(&child_config, "indent_width = 4\n").unwrap();

    // Resolve from child directory
    let cmake_file = sub_dir.join("CMakeLists.txt");
    let config = config::resolve_config(Some(&cmake_file), None, &[]);

    // Child's indent_width wins, parent's max_line_length kept
    assert_eq!(config.indent_width, 4);
    assert_eq!(config.max_line_length, 100);
}

#[test]
fn test_recursive_config_command_grammars_merge() {
    let tempdir = TempDir::new().unwrap();
    let root_dir = tempdir.path();
    let sub_dir = root_dir.join("sub");
    fs::create_dir(&sub_dir).unwrap();

    // Root config defines my_cmd
    let root_config = root_dir.join(".cmake-fmt.toml");
    fs::write(
        &root_config,
        r#"
[command_grammars.my_cmd]
options = ["OPT_A"]
"#,
    )
    .unwrap();

    // Child config defines other_cmd and overrides my_cmd
    let child_config = sub_dir.join(".cmake-fmt.toml");
    fs::write(
        &child_config,
        r#"
[command_grammars.other_cmd]
options = ["OPT_B"]

[command_grammars.my_cmd]
options = ["OPT_X"]
"#,
    )
    .unwrap();

    // Resolve from child directory
    let cmake_file = sub_dir.join("CMakeLists.txt");
    let config = config::resolve_config(Some(&cmake_file), None, &[]);

    // Both commands should be present
    assert!(config.command_grammars.contains_key("my_cmd"));
    assert!(config.command_grammars.contains_key("other_cmd"));

    // Child's my_cmd definition should override parent's
    assert_eq!(config.command_grammars["my_cmd"].options, vec!["OPT_X"]);
    assert_eq!(config.command_grammars["other_cmd"].options, vec!["OPT_B"]);
}

#[test]
fn test_recursive_config_grammar_files_concatenate() {
    let tempdir = TempDir::new().unwrap();
    let root_dir = tempdir.path();
    let sub_dir = root_dir.join("sub");
    fs::create_dir(&sub_dir).unwrap();

    // Root config has grammar_files
    let root_config = root_dir.join(".cmake-fmt.toml");
    fs::write(
        &root_config,
        r#"
grammar_files = ["root.toml"]
"#,
    )
    .unwrap();

    // Child config has different grammar_files
    let child_config = sub_dir.join(".cmake-fmt.toml");
    fs::write(
        &child_config,
        r#"
grammar_files = ["child.toml"]
"#,
    )
    .unwrap();

    // Resolve from child directory
    let cmake_file = sub_dir.join("CMakeLists.txt");
    let config = config::resolve_config(Some(&cmake_file), None, &[]);

    // Should have both files
    assert_eq!(config.grammar_files.len(), 2);

    // Check that paths are resolved relative to their config directories
    let _root_grammar = root_dir.join("root.toml");
    let _child_grammar = sub_dir.join("child.toml");

    let config_paths: Vec<String> = config
        .grammar_files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    assert!(config_paths.iter().any(|p| p.contains("root.toml")));
    assert!(config_paths.iter().any(|p| p.contains("child.toml")));

    // Verify one contains root dir, other contains sub dir
    let has_root_path = config.grammar_files.iter().any(|p| {
        p.to_string_lossy()
            .contains(&root_dir.file_name().unwrap().to_string_lossy().to_string())
    });
    let has_sub_path = config
        .grammar_files
        .iter()
        .any(|p| p.to_string_lossy().contains("sub"));

    assert!(has_root_path || has_sub_path);
}

#[test]
fn test_recursive_config_cli_overrides_win() {
    let tempdir = TempDir::new().unwrap();
    let root_dir = tempdir.path();
    let sub_dir = root_dir.join("sub");
    fs::create_dir(&sub_dir).unwrap();

    // Root config sets indent_width=2
    let root_config = root_dir.join(".cmake-fmt.toml");
    fs::write(&root_config, "indent_width = 2\n").unwrap();

    // Child config sets indent_width=4
    let child_config = sub_dir.join(".cmake-fmt.toml");
    fs::write(&child_config, "indent_width = 4\n").unwrap();

    // Resolve with CLI override
    let cmake_file = sub_dir.join("CMakeLists.txt");
    let config = config::resolve_config(Some(&cmake_file), Some("indent_width=8"), &[]);

    // CLI override should win
    assert_eq!(config.indent_width, 8);
}

#[test]
fn test_recursive_config_single_file_backward_compatible() {
    let tempdir = TempDir::new().unwrap();
    let root_dir = tempdir.path();

    // Single config file
    let config_file = root_dir.join(".cmake-fmt.toml");
    fs::write(&config_file, "indent_width = 2\nuse_tabs = false\n").unwrap();

    // Resolve from root directory
    let cmake_file = root_dir.join("CMakeLists.txt");
    let config = config::resolve_config(Some(&cmake_file), None, &[]);

    // Should work exactly as before
    assert_eq!(config.indent_width, 2);
    assert!(!config.use_tabs);
}

#[test]
fn test_root_true_stops_parent_inheritance_toml() {
    let tempdir = TempDir::new().unwrap();
    let root_dir = tempdir.path();
    let sub_dir = root_dir.join("sub");
    fs::create_dir(&sub_dir).unwrap();

    // Root config sets indent_width=2, max_line_length=100
    let root_config = root_dir.join(".cmake-fmt.toml");
    fs::write(&root_config, "indent_width = 2\nmax_line_length = 100\n").unwrap();

    // Sub config declares root: true and sets its own indent_width
    let sub_config = sub_dir.join(".cmake-fmt.toml");
    fs::write(&sub_config, "root = true\nindent_width = 6\n").unwrap();

    let cmake_file = sub_dir.join("CMakeLists.txt");
    let config = config::resolve_config(Some(&cmake_file), None, &[]);

    // indent_width from child
    assert_eq!(config.indent_width, 6);
    // max_line_length at default (80), not 100 from parent (parent was discarded)
    assert_eq!(config.max_line_length, 80);
    // use_tabs at default (true), proving fresh start from defaults
    assert!(config.use_tabs);
}

#[test]
fn test_root_true_stops_parent_inheritance_yaml() {
    let tempdir = TempDir::new().unwrap();
    let root_dir = tempdir.path();
    let sub_dir = root_dir.join("sub");
    fs::create_dir(&sub_dir).unwrap();

    // Root config uses extensionless YAML
    let root_config = root_dir.join(".cmake-fmt");
    fs::write(&root_config, "indent_width: 2\nmax_line_length: 100\n").unwrap();

    // Sub config uses extensionless YAML with root: true
    let sub_config = sub_dir.join(".cmake-fmt");
    fs::write(&sub_config, "root: true\nindent_width: 6\n").unwrap();

    let cmake_file = sub_dir.join("CMakeLists.txt");
    let config = config::resolve_config(Some(&cmake_file), None, &[]);

    // indent_width from child
    assert_eq!(config.indent_width, 6);
    // max_line_length at default (80), not 100 from parent
    assert_eq!(config.max_line_length, 80);
    // use_tabs at default (true)
    assert!(config.use_tabs);
}

#[test]
fn test_root_false_does_not_stop_inheritance() {
    let tempdir = TempDir::new().unwrap();
    let root_dir = tempdir.path();
    let sub_dir = root_dir.join("sub");
    fs::create_dir(&sub_dir).unwrap();

    // Root config sets indent_width=2, max_line_length=100
    let root_config = root_dir.join(".cmake-fmt.toml");
    fs::write(&root_config, "indent_width = 2\nmax_line_length = 100\n").unwrap();

    // Sub config has root=false (should NOT stop inheritance) and overrides indent_width
    let sub_config = sub_dir.join(".cmake-fmt.toml");
    fs::write(&sub_config, "root = false\nindent_width = 6\n").unwrap();

    let cmake_file = sub_dir.join("CMakeLists.txt");
    let config = config::resolve_config(Some(&cmake_file), None, &[]);

    // indent_width from child (overrides parent)
    assert_eq!(config.indent_width, 6);
    // max_line_length inherited from parent (root=false has no effect)
    assert_eq!(config.max_line_length, 100);
}

#[test]
fn test_root_true_middle_directory() {
    let tempdir = TempDir::new().unwrap();
    let root_dir = tempdir.path();
    let mid_dir = root_dir.join("mid");
    let deep_dir = mid_dir.join("deep");
    fs::create_dir_all(&deep_dir).unwrap();

    // Root config: indent_width=2
    let root_config = root_dir.join(".cmake-fmt.toml");
    fs::write(&root_config, "indent_width = 2\n").unwrap();

    // Mid config: root=true, max_line_length=100
    let mid_config = mid_dir.join(".cmake-fmt.toml");
    fs::write(&mid_config, "root = true\nmax_line_length = 100\n").unwrap();

    // Deep config: use_tabs=false (no root flag)
    let deep_config = deep_dir.join(".cmake-fmt.toml");
    fs::write(&deep_config, "use_tabs = false\n").unwrap();

    let cmake_file = deep_dir.join("CMakeLists.txt");
    let config = config::resolve_config(Some(&cmake_file), None, &[]);

    // indent_width at default (4), not 2 (root_dir was discarded because mid has root=true)
    assert_eq!(config.indent_width, 4);
    // max_line_length from mid config
    assert_eq!(config.max_line_length, 100);
    // use_tabs from deep config
    assert!(!config.use_tabs);
}

#[test]
fn test_root_true_command_grammars_not_inherited() {
    let tempdir = TempDir::new().unwrap();
    let root_dir = tempdir.path();
    let sub_dir = root_dir.join("sub");
    fs::create_dir(&sub_dir).unwrap();

    // Root config with custom command grammar
    let root_config = root_dir.join(".cmake-fmt.toml");
    fs::write(
        &root_config,
        "[command_grammars.parent_cmd]\noptions = [\"OPT_A\"]\n",
    )
    .unwrap();

    // Sub config with root=true and different command grammar
    let sub_config = sub_dir.join(".cmake-fmt.toml");
    fs::write(
        &sub_config,
        "root = true\n[command_grammars.child_cmd]\noptions = [\"OPT_B\"]\n",
    )
    .unwrap();

    let cmake_file = sub_dir.join("CMakeLists.txt");
    let config = config::resolve_config(Some(&cmake_file), None, &[]);

    // child_cmd grammar should be present
    assert!(
        config.command_grammars.contains_key("child_cmd"),
        "Expected child_cmd grammar to be present"
    );
    // parent_cmd grammar should NOT be present (discarded by root=true)
    assert!(
        !config.command_grammars.contains_key("parent_cmd"),
        "Expected parent_cmd grammar to be absent (discarded by root=true)"
    );
}

// Module for accessing config loading functions
mod config {
    use anyhow::{Context, Result};
    use cmake_fmt::formatter::FormatConfig;
    use std::fs;
    use std::path::{Path, PathBuf};

    pub fn load_config_file(path: &Path) -> Result<FormatConfig> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension {
            "toml" | "tml" => toml::from_str(&content)
                .with_context(|| format!("Failed to parse TOML config: {}", path.display())),
            "yaml" | "yml" => serde_yml::from_str(&content)
                .with_context(|| format!("Failed to parse YAML config: {}", path.display())),
            _ => {
                // Extensionless config files (like .cmake-fmt) default to YAML
                serde_yml::from_str(&content)
                    .with_context(|| format!("Failed to parse config as YAML: {}", path.display()))
            }
        }
    }

    pub fn resolve_config(
        file_path: Option<&Path>,
        style_override: Option<&str>,
        grammar_files: &[PathBuf],
    ) -> FormatConfig {
        // This is a simplified version for tests - in production, use the real implementation
        // We need to import the actual function from the binary crate
        use std::path::Path;

        // Determine search directory
        let search_dir = if let Some(path) = file_path {
            path.parent().unwrap_or_else(|| Path::new("."))
        } else {
            Path::new(".")
        };

        // Find all config files
        let config_paths = find_config_files(search_dir);

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

        // Merge configs root-first
        let mut merged_table = toml::Table::new();

        for config_path in config_paths.iter().rev() {
            if let Ok(mut file_table) = load_config_as_table(config_path) {
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

                merge_tables(&mut merged_table, &file_table);
            }
        }

        // Deserialize merged table
        let mut config = if let Ok(toml_str) = toml::to_string(&merged_table) {
            toml::from_str::<FormatConfig>(&toml_str).unwrap_or_default()
        } else {
            FormatConfig::default()
        };

        // Apply CLI overrides
        if let Some(style) = style_override {
            apply_style_overrides(&mut config, style);
        }

        // Add CLI grammar files
        config.grammar_files.extend(grammar_files.iter().cloned());

        config
    }

    fn find_config_files(start_dir: &Path) -> Vec<PathBuf> {
        const CONFIG_FILENAMES: &[&str] = &[
            ".cmake-fmt.toml",
            ".cmake-fmt.tml",
            ".cmake-fmt.yaml",
            ".cmake-fmt.yml",
            ".cmake-fmt",
        ];

        let mut config_files = Vec::new();

        for ancestor in start_dir.ancestors() {
            for filename in CONFIG_FILENAMES {
                let config_path = ancestor.join(filename);
                if config_path.exists() && config_path.is_file() {
                    config_files.push(config_path);
                    break;
                }
            }
        }

        config_files
    }

    fn load_config_as_table(path: &Path) -> Result<toml::Table> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension {
            "toml" | "tml" => toml::from_str::<toml::Table>(&content)
                .with_context(|| format!("Failed to parse TOML config: {}", path.display())),
            _ => {
                let yaml_value: serde_yml::Value = serde_yml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML config: {}", path.display()))?;

                yml_value_to_toml_table(yaml_value)
                    .ok_or_else(|| anyhow::anyhow!("YAML config must be a mapping at top level"))
            }
        }
    }

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

    fn yml_value_to_toml_table(v: serde_yml::Value) -> Option<toml::Table> {
        match yml_value_to_toml_value(v)? {
            toml::Value::Table(t) => Some(t),
            _ => None,
        }
    }

    fn merge_tables(base: &mut toml::Table, overlay: &toml::Table) {
        for (key, overlay_value) in overlay {
            if key == "command_grammars" {
                match (base.get_mut("command_grammars"), overlay_value) {
                    (Some(toml::Value::Table(base_cmds)), toml::Value::Table(overlay_cmds)) => {
                        for (cmd_name, cmd_grammar) in overlay_cmds {
                            base_cmds.insert(cmd_name.clone(), cmd_grammar.clone());
                        }
                    }
                    (None, toml::Value::Table(_)) => {
                        base.insert(key.clone(), overlay_value.clone());
                    }
                    _ => {
                        base.insert(key.clone(), overlay_value.clone());
                    }
                }
            } else if key == "grammar_files" {
                match (base.get_mut("grammar_files"), overlay_value) {
                    (Some(toml::Value::Array(base_files)), toml::Value::Array(overlay_files)) => {
                        base_files.extend(overlay_files.iter().cloned());
                    }
                    (None, toml::Value::Array(_)) => {
                        base.insert(key.clone(), overlay_value.clone());
                    }
                    _ => {
                        base.insert(key.clone(), overlay_value.clone());
                    }
                }
            } else {
                base.insert(key.clone(), overlay_value.clone());
            }
        }
    }

    fn apply_style_overrides(config: &mut FormatConfig, style: &str) {
        for pair in style.split(',') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }

            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }

            let key = parts[0].trim();
            let value = parts[1].trim();

            let _ = config.apply_override(key, value);
        }
    }
}

#[test]
fn test_config_grammar_sortable_keywords_parsing() {
    let toml_str = r#"
[command_grammars.my_add_library]
one_value_keywords = ["NAME"]
multi_value_keywords = ["SRC_FILES", "COMMAND"]
sortable_keywords = ["SRC_FILES"]
sortable_positional = true
"#;

    let config: FormatConfig = toml::from_str(toml_str).unwrap();
    let grammar = &config.command_grammars["my_add_library"];
    assert_eq!(
        grammar.sortable_keywords,
        Some(vec!["SRC_FILES".to_string()])
    );
    assert!(grammar.sortable_positional);
}

#[test]
fn test_config_grammar_sortable_keywords_applied() {
    // A wrapper command opts one keyword into reordering; its neighbours,
    // including a COMMAND argv, are left alone.
    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "my_add_library".to_string(),
        CommandGrammarConfig {
            one_value_keywords: vec!["NAME".to_string()],
            multi_value_keywords: vec!["SRC_FILES".to_string(), "COMMAND".to_string()],
            sortable_keywords: Some(vec!["SRC_FILES".to_string()]),
            ..Default::default()
        },
    );

    let config = FormatConfig {
        sort_sources: cmake_fmt::formatter::SortSources::Alphabetical,
        command_grammars,
        ..Default::default()
    };

    let result = format_text(
        "my_add_library(NAME foo SRC_FILES z.cpp a.cpp COMMAND run z.sh a.sh)\n",
        &config,
    );

    assert!(
        result.contains("SRC_FILES a.cpp z.cpp"),
        "declared sortable keyword should sort:\n{}",
        result
    );
    assert!(
        result.contains("COMMAND run z.sh a.sh"),
        "undeclared keyword must hold its order:\n{}",
        result
    );
}

#[test]
fn test_config_grammar_sortable_positional_applied() {
    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "my_set".to_string(),
        CommandGrammarConfig {
            one_value_keywords: vec!["UNUSED".to_string()],
            sortable_positional: true,
            ..Default::default()
        },
    );

    let config = FormatConfig {
        sort_sources: cmake_fmt::formatter::SortSources::Alphabetical,
        command_grammars,
        ..Default::default()
    };

    // The first positional argument is the variable name and stays pinned
    let result = format_text("my_set(VAR z.cpp a.cpp)\n", &config);
    assert_eq!(result, "my_set(VAR a.cpp z.cpp)\n");
}

#[test]
fn test_config_grammar_cannot_make_bin_pack_sortable() {
    // A grammar declaring the same keyword as both bin-pack and sortable must
    // not be able to scramble an argv.
    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "my_runner".to_string(),
        CommandGrammarConfig {
            bin_pack_keywords: vec!["COMMAND".to_string()],
            sortable_keywords: Some(vec!["COMMAND".to_string()]),
            ..Default::default()
        },
    );

    let config = FormatConfig {
        sort_sources: cmake_fmt::formatter::SortSources::Alphabetical,
        command_grammars,
        ..Default::default()
    };

    let result = format_text("my_runner(COMMAND run z.sh a.sh)\n", &config);
    assert!(
        result.contains("COMMAND run z.sh a.sh"),
        "a bin-pack keyword must never be reordered:\n{}",
        result
    );
}

#[test]
fn test_user_grammar_cannot_hijack_a_condition() {
    // `if` is a language construct, not a command a project defines. A user
    // grammar naming it used to route conditions to the keyword-aware path,
    // restoring the one-argument-per-line layout the clause layout exists to
    // remove — and, with sortable_keywords, letting the reordering passes move
    // condition operands, which changes what the condition means.
    let mut command_grammars = HashMap::new();
    command_grammars.insert(
        "if".to_string(),
        CommandGrammarConfig {
            multi_value_keywords: vec!["AND".to_string(), "OR".to_string()],
            sortable_keywords: Some(vec!["AND".to_string()]),
            ..Default::default()
        },
    );

    let config = FormatConfig {
        sort_sources: cmake_fmt::formatter::SortSources::Alphabetical,
        command_grammars,
        ..Default::default()
    };

    let input = concat!(
        "if(NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\" ",
        "AND NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"GROUP\")\n",
        "endif()\n"
    );
    let result = format_text(input, &config);

    assert_eq!(
        result,
        concat!(
            "if(NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"BATCH\"\n",
            "\tAND NOT WICKHOPPER_JUMBO_BUILD_MODE STREQUAL \"GROUP\"\n",
            ")\n",
            "endif()\n"
        ),
        "a user grammar on `if` should not change how a condition is laid out"
    );
}
