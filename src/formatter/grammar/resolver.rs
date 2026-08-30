use super::{CommandGrammar, Grammar, KeywordType, builtin_grammars, user_scanner};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Registry for command grammar lookup
pub struct GrammarRegistry {
    grammars: HashMap<String, Grammar>,
}

impl GrammarRegistry {
    /// Every builtin grammar, as `(command, mode, grammar)`.
    ///
    /// Exposed so a test can assert the *whole* reordering allowlist rather than
    /// its effects: `mark_sortable_lists` is this branch's entire contract, and
    /// adding a command to it — `file(GLOB)`, say, where glob order decides
    /// match order — passed the suite silently because nothing enumerated it.
    pub fn entries(&self) -> Vec<(&str, Option<&str>, &CommandGrammar)> {
        let mut out = Vec::new();
        for (command, grammar) in &self.grammars {
            match grammar {
                Grammar::Simple(g) => out.push((command.as_str(), None, g)),
                Grammar::Modes { modes } => {
                    for (mode, g) in modes {
                        out.push((command.as_str(), Some(mode.as_str()), g));
                    }
                }
            }
        }
        out.sort_by_key(|(command, mode, _)| (*command, *mode));
        out
    }

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
    pub fn resolve_grammar(
        &self,
        command_name: &str,
        first_keyword: Option<&str>,
    ) -> Option<&CommandGrammar> {
        self.get(command_name)?.resolve(first_keyword)
    }
}

/// Cache for combined project-wide scan results (commands + grammars)
static PROJECT_SCAN_CACHE: OnceLock<Mutex<HashMap<PathBuf, user_scanner::ProjectScanResult>>> =
    OnceLock::new();

/// Get combined project scan results (commands and grammars) with caching
///
/// Determines the project root from the file's parent directory,
/// scans all CMake files in the project tree (following CMake dependency graph),
/// and caches the results per project root.
fn get_project_scan(file_path: &Path, verbose: bool) -> user_scanner::ProjectScanResult {
    // Determine project root from the file's parent directory
    let start_dir = file_path.parent().unwrap_or(file_path);
    let project_root = user_scanner::find_project_root(start_dir, verbose);

    // Get or init cache
    let cache = PROJECT_SCAN_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache_lock = cache.lock().unwrap();

    // Return cached if available
    if let Some(cached) = cache_lock.get(&project_root) {
        if verbose {
            eprintln!(
                "verbose: using cached scan results for {}",
                project_root.display()
            );
        }
        return cached.clone();
    }

    // Scan and cache
    let scan_result = user_scanner::scan_project(&project_root, verbose);
    cache_lock.insert(project_root, scan_result.clone());
    scan_result
}

/// Get project-wide user command definitions with caching
///
/// Determines the project root from the file's parent directory,
/// scans all CMake files in the project tree (following CMake dependency graph),
/// and caches the results per project root.
pub fn get_project_user_commands(file_path: &Path, verbose: bool) -> HashMap<String, String> {
    get_project_scan(file_path, verbose).commands
}

/// Get project-wide user command grammars extracted from cmake_parse_arguments
///
/// Determines the project root from the file's parent directory,
/// scans all CMake files in the project tree for cmake_parse_arguments calls,
/// and caches the results per project root.
pub fn get_project_user_grammars(
    file_path: &Path,
    verbose: bool,
) -> HashMap<String, CommandGrammar> {
    get_project_scan(file_path, verbose).grammars
}

/// Clear the project scan cache (for testing purposes)
pub fn clear_project_scan_cache() {
    if let Some(cache) = PROJECT_SCAN_CACHE.get()
        && let Ok(mut cache_lock) = cache.lock()
    {
        cache_lock.clear();
    }
}

/// Clear the project grammar cache (for testing purposes)
///
/// Same cache as clear_project_scan_cache - kept for backward compatibility
pub fn clear_project_grammar_cache() {
    clear_project_scan_cache();
}

/// Convert config grammar definitions to CommandGrammar map
pub fn config_grammars_to_map(
    config_grammars: &HashMap<String, crate::formatter::config::CommandGrammarConfig>,
) -> HashMap<String, CommandGrammar> {
    config_grammars
        .iter()
        .map(|(name, cfg)| {
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
            for kw in &cfg.bin_pack_keywords {
                keywords.insert(kw.clone(), KeywordType::BinPack);
            }
            // A config entry replaces the grammar auto-detected from
            // `cmake_parse_arguments` wholesale, so a user who declared one for
            // wrapping reasons silently lost the sorting the auto-detected
            // grammar gave them, with no diagnostic. The conventional file-list
            // names are a default here for the same reason they are one there —
            // but only a *default*: naming any sortable keyword is the user
            // saying what is unordered, and "reordering is opt-in, keywords not
            // listed here are left alone" is what the config docs, the schema
            // and `--help-grammar` all promise. Overriding that left no way to
            // say "not this one".
            //
            // The default is drawn from the multi-value keywords, not from every
            // declared keyword: a `FILES` declared as a flag takes no values, so
            // marking it sortable only reorders whatever positional arguments
            // happen to follow it.
            let sortable_keywords: HashSet<String> = match &cfg.sortable_keywords {
                // Named, however short the list: that list is the whole list
                Some(declared) => declared.iter().cloned().collect(),
                // Not named at all: the same default an auto-detected grammar
                // would have carried
                None => cfg
                    .multi_value_keywords
                    .iter()
                    .filter(|kw| super::argparse_extractor::is_conventional_file_list(kw))
                    .cloned()
                    .collect(),
            };

            (
                name.to_lowercase(),
                CommandGrammar {
                    keywords,
                    force_args_on_new_line: false,
                    sub_keywords: HashSet::new(),
                    collection_keywords: HashSet::new(),
                    sortable_keywords,
                    sortable_positional: cfg.sortable_positional,
                },
            )
        })
        .collect()
}
