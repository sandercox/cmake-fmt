use super::{builtin_grammars, CommandGrammar, KeywordType};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Registry for command grammar lookup
pub struct GrammarRegistry {
    grammars: HashMap<String, CommandGrammar>,
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
    pub fn get(&self, command_name: &str) -> Option<&CommandGrammar> {
        self.grammars.get(&command_name.to_lowercase())
    }

    /// Get the type of a keyword for a command (convenience method)
    ///
    /// Returns None if the command has no grammar or the keyword is not recognized
    pub fn keyword_type(&self, command_name: &str, keyword: &str) -> Option<KeywordType> {
        self.get(command_name)
            .and_then(|grammar| grammar.keyword_type(keyword))
    }
}
