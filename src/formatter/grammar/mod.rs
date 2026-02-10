use std::collections::HashMap;

pub mod builtins;
pub mod resolver;

pub use self::builtins::*;
pub use self::resolver::*;

/// Classification of CMake command keywords
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordType {
    /// Flag keyword (no values consumed, e.g., REQUIRED, QUIET)
    Flag,
    /// Single-value keyword (consumes exactly one value, e.g., VERSION, DESTINATION)
    SingleValue,
    /// Multi-value keyword (consumes multiple values, e.g., COMPONENTS, TARGETS)
    MultiValue,
}

/// Grammar definition for a CMake command
#[derive(Debug, Clone)]
pub struct CommandGrammar {
    /// Map of keyword name (UPPERCASE) to its type
    pub keywords: HashMap<String, KeywordType>,
}

impl CommandGrammar {
    /// Create a new empty command grammar
    pub fn new() -> Self {
        Self {
            keywords: HashMap::new(),
        }
    }

    /// Create a command grammar from a list of (keyword, type) pairs
    pub fn from_keywords(keywords: &[(&str, KeywordType)]) -> Self {
        let mut map = HashMap::new();
        for (kw, ty) in keywords {
            map.insert(kw.to_string(), *ty);
        }
        Self { keywords: map }
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
