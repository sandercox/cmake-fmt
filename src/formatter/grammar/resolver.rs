use super::{builtin_grammars, CommandGrammar, Grammar, KeywordType};
use std::collections::HashMap;
use std::sync::OnceLock;

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
