use cmake_fmt::formatter::{GrammarRegistry, KeywordType, FormatConfig};

#[test]
fn test_target_link_libraries_grammar() {
    let registry = GrammarRegistry::global();
    let grammar = registry.resolve_grammar("target_link_libraries", None)
        .expect("target_link_libraries should have grammar");

    assert_eq!(grammar.keyword_type("PUBLIC"), Some(KeywordType::MultiValue));
    assert_eq!(grammar.keyword_type("PRIVATE"), Some(KeywordType::MultiValue));
    assert_eq!(grammar.keyword_type("INTERFACE"), Some(KeywordType::MultiValue));
}

#[test]
fn test_find_package_grammar() {
    let registry = GrammarRegistry::global();
    let grammar = registry.resolve_grammar("find_package", None)
        .expect("find_package should have grammar");

    assert_eq!(grammar.keyword_type("REQUIRED"), Some(KeywordType::Flag));
    assert_eq!(grammar.keyword_type("QUIET"), Some(KeywordType::Flag));
    assert_eq!(grammar.keyword_type("COMPONENTS"), Some(KeywordType::MultiValue));
}

#[test]
fn test_unknown_command_returns_none() {
    let registry = GrammarRegistry::global();
    assert!(registry.get("unknown_command").is_none());
}

#[test]
fn test_force_break_keywords_default() {
    let config = FormatConfig::default();
    assert_eq!(config.force_break_keywords, false);
}
