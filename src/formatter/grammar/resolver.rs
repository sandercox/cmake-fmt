use super::{builtin_grammars, user_scanner, CommandGrammar, Grammar, KeywordType};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Registry for command grammar lookup
pub struct GrammarRegistry {
    grammars: HashMap<String, Grammar>,
}

impl GrammarRegistry {
    /// Get the global singleton instance
    pub fn global() -> &'static GrammarRegistry {
        static REGISTRY: OnceLock<GrammarRegistry> = OnceLock::new();
        REGISTRY.get_or_init(|| {
            let grammars = builtin_grammars();
            GrammarRegistry { grammars }
        })
    }

    /// Get the grammar for a command by name (case-insensitive)
    pub fn get(&self, command_name: &str) -> Option<&Grammar> {
        self.grammars.get(&command_name.to_lowercase())
    }

    /// Resolve grammar to CommandGrammar based on first keyword (for multi-mode commands)
    pub fn resolve_grammar(&self, command_name: &str, first_keyword: Option<&str>) -> Option<&CommandGrammar> {
        self.get(command_name)?.resolve(first_keyword)
    }
}

/// Cache for project-wide user command scans
static PROJECT_SCAN_CACHE: OnceLock<Mutex<HashMap<PathBuf, HashMap<String, String>>>> = OnceLock::new();

/// Get project-wide user command definitions with caching
///
/// Determines the project root from the file's parent directory,
/// scans all CMake files in the project tree (respecting .gitignore),
/// and caches the results per project root.
pub fn get_project_user_commands(file_path: &Path) -> HashMap<String, String> {
    // Determine project root from the file's parent directory
    let start_dir = file_path.parent().unwrap_or(file_path);
    let project_root = user_scanner::find_project_root(start_dir);

    // Get or init cache
    let cache = PROJECT_SCAN_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache_lock = cache.lock().unwrap();

    // Return cached if available
    if let Some(cached) = cache_lock.get(&project_root) {
        return cached.clone();
    }

    // Scan and cache
    let user_defs = user_scanner::scan_project_commands(&project_root);
    cache_lock.insert(project_root, user_defs.clone());
    user_defs
}

/// Clear the project scan cache (for testing purposes)
pub fn clear_project_scan_cache() {
    if let Some(cache) = PROJECT_SCAN_CACHE.get() {
        if let Ok(mut cache_lock) = cache.lock() {
            cache_lock.clear();
        }
    }
}

/// Cache for project-wide grammar extraction
static PROJECT_GRAMMAR_CACHE: OnceLock<Mutex<HashMap<PathBuf, HashMap<String, CommandGrammar>>>> = OnceLock::new();

/// Get project-wide user command grammars extracted from cmake_parse_arguments
///
/// Determines the project root from the file's parent directory,
/// scans all CMake files in the project tree for cmake_parse_arguments calls,
/// and caches the results per project root.
pub fn get_project_user_grammars(file_path: &Path) -> HashMap<String, CommandGrammar> {
    // Determine project root from the file's parent directory
    let start_dir = file_path.parent().unwrap_or(file_path);
    let project_root = user_scanner::find_project_root(start_dir);

    // Get or init cache
    let cache = PROJECT_GRAMMAR_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache_lock = cache.lock().unwrap();

    // Return cached if available
    if let Some(cached) = cache_lock.get(&project_root) {
        return cached.clone();
    }

    // Scan and cache
    let grammars = user_scanner::scan_project_grammars(&project_root);
    cache_lock.insert(project_root, grammars.clone());
    grammars
}

/// Clear the project grammar cache (for testing purposes)
pub fn clear_project_grammar_cache() {
    if let Some(cache) = PROJECT_GRAMMAR_CACHE.get() {
        if let Ok(mut cache_lock) = cache.lock() {
            cache_lock.clear();
        }
    }
}

/// Convert config grammar definitions to CommandGrammar map
pub fn config_grammars_to_map(
    config_grammars: &HashMap<String, crate::formatter::config::CommandGrammarConfig>
) -> HashMap<String, CommandGrammar> {
    config_grammars.iter().map(|(name, cfg)| {
        let mut keywords = HashMap::new();
        for kw in &cfg.options {
            keywords.insert(kw.clone(), KeywordType::Flag);
        }
        for kw in &cfg.one_value_keywords {
            keywords.insert(kw.clone(), KeywordType::SingleValue);
        }
        for kw in &cfg.multi_value_keywords {
            keywords.insert(kw.clone(), KeywordType::MultiValue);
        }
        for kw in &cfg.pair_value_keywords {
            keywords.insert(kw.clone(), KeywordType::PairValue);
        }
        for kw in &cfg.command_line_keywords {
            keywords.insert(kw.clone(), KeywordType::CommandLine);
        }
        (name.to_lowercase(), CommandGrammar { keywords })
    }).collect()
}
