use std::collections::{HashMap, HashSet};

pub mod argparse_extractor;
pub mod builtins;
pub mod export;
pub mod resolver;
pub mod user_scanner;

pub use self::builtins::*;
pub use self::export::{
    detect_grammar_format, export_command_grammars, export_command_grammars_to_toml,
    export_command_grammars_to_yaml, export_grammars, export_grammars_to_toml,
    export_grammars_to_yaml, import_grammar_file, GrammarFile, GrammarFormat,
};
pub use self::resolver::{
    clear_project_grammar_cache, clear_project_scan_cache, config_grammars_to_map,
    get_project_user_commands, get_project_user_grammars, GrammarRegistry,
};

/// Classification of CMake command keywords
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordType {
    /// Flag keyword (no values consumed, e.g., REQUIRED, QUIET)
    Flag,
    /// Single-value keyword (consumes exactly one value, e.g., VERSION, DESTINATION)
    SingleValue,
    /// Multi-value keyword (consumes multiple values, e.g., COMPONENTS, TARGETS)
    MultiValue,
    /// Pair-value keyword: consumes alternating key/value pairs
    /// e.g., PROPERTIES prop1 value1 prop2 value2
    PairValue,
    /// Bin-pack keyword: packs values to fill lines (e.g., COMMAND)
    BinPack,
}

/// Grammar definition for a CMake command
#[derive(Debug, Clone)]
pub enum Grammar {
    /// Single-mode command (e.g., find_package, target_link_libraries)
    Simple(CommandGrammar),
    /// Multi-mode command where first keyword selects the grammar
    /// e.g., install(TARGETS ...) vs install(FILES ...)
    Modes {
        modes: HashMap<String, CommandGrammar>,
    },
}

impl Grammar {
    /// Resolve the grammar based on the first keyword (for multi-mode commands)
    pub fn resolve(&self, first_keyword: Option<&str>) -> Option<&CommandGrammar> {
        match self {
            Grammar::Simple(grammar) => Some(grammar),
            Grammar::Modes { modes } => {
                first_keyword.and_then(|kw| modes.get(kw))
            }
        }
    }

    /// Check if this is a multi-mode command
    pub fn is_multi_mode(&self) -> bool {
        matches!(self, Grammar::Modes { .. })
    }
}

/// Grammar definition for a CMake command
#[derive(Debug, Clone)]
pub struct CommandGrammar {
    /// Map of keyword name (UPPERCASE) to its type
    pub keywords: HashMap<String, KeywordType>,
    /// When true, ALL args go on new lines when multiline (no arg trails the command name)
    pub force_args_on_new_line: bool,
    /// Keywords that should be consumed as regular args when inside a BinPack section
    /// (e.g., DESTINATION inside LIBRARY BinPack section for install(TARGETS))
    pub sub_keywords: HashSet<String>,
    /// MultiValue keywords that should consume sub_keywords as grouped values.
    /// Only these specific MultiValue keywords will absorb sub_keywords;
    /// other MultiValue keywords (like TARGETS) will not.
    /// BinPack keywords always consume sub_keywords regardless.
    pub collection_keywords: HashSet<String>,
}

impl CommandGrammar {
    /// Create a new empty command grammar
    pub fn new() -> Self {
        Self {
            keywords: HashMap::new(),
            force_args_on_new_line: false,
            sub_keywords: HashSet::new(),
            collection_keywords: HashSet::new(),
        }
    }

    /// Create a command grammar from a list of (keyword, type) pairs
    pub fn from_keywords(keywords: &[(&str, KeywordType)]) -> Self {
        let mut map = HashMap::new();
        for (kw, ty) in keywords {
            map.insert(kw.to_string(), *ty);
        }
        Self { keywords: map, force_args_on_new_line: false, sub_keywords: HashSet::new(), collection_keywords: HashSet::new() }
    }

    /// Get the type of a keyword (case-sensitive lookup - CMake keywords are case-sensitive)
    pub fn keyword_type(&self, keyword: &str) -> Option<KeywordType> {
        self.keywords.get(keyword).copied()
    }
}

impl Default for CommandGrammar {
    fn default() -> Self {
        Self::new()
    }
}
