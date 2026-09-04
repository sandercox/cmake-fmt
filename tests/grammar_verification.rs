use cmake_fmt::formatter::{FormatConfig, GrammarRegistry, KeywordType};

#[test]
fn test_target_link_libraries_grammar() {
    let registry = GrammarRegistry::global();
    let grammar = registry
        .resolve_grammar("target_link_libraries", None)
        .expect("target_link_libraries should have grammar");

    assert_eq!(
        grammar.keyword_type("PUBLIC"),
        Some(KeywordType::MultiValue)
    );
    assert_eq!(
        grammar.keyword_type("PRIVATE"),
        Some(KeywordType::MultiValue)
    );
    assert_eq!(
        grammar.keyword_type("INTERFACE"),
        Some(KeywordType::MultiValue)
    );
}

#[test]
fn test_find_package_grammar() {
    let registry = GrammarRegistry::global();
    let grammar = registry
        .resolve_grammar("find_package", None)
        .expect("find_package should have grammar");

    assert_eq!(grammar.keyword_type("REQUIRED"), Some(KeywordType::Flag));
    assert_eq!(grammar.keyword_type("QUIET"), Some(KeywordType::Flag));
    assert_eq!(
        grammar.keyword_type("COMPONENTS"),
        Some(KeywordType::MultiValue)
    );
}

#[test]
fn test_unknown_command_returns_none() {
    let registry = GrammarRegistry::global();
    assert!(registry.get("unknown_command").is_none());
}

#[test]
fn test_force_break_keywords_default() {
    let config = FormatConfig::default();
    assert!(!config.force_break_keywords);
}

/// The reordering allowlist, whole, as a literal.
///
/// `mark_sortable_lists` is this branch's entire contract: nothing is reordered
/// unless it appears here. Every other test checks the allowlist by its
/// *effects*, which cannot see a widening — adding `add_custom_target SOURCES`
/// or `file(GLOB)` positional passed the suite silently, and glob order decides
/// the order of what a glob matches.
///
/// Sorted by (command, mode); keywords sorted within each row.
const ALLOWLIST: &[(&str, Option<&str>, &[&str], bool)] = &[
    (
        "add_executable",
        None,
        &["EXCLUDE_FROM_ALL", "MACOSX_BUNDLE", "WIN32"],
        true,
    ),
    (
        "add_library",
        None,
        &[
            "EXCLUDE_FROM_ALL",
            "INTERFACE",
            "MODULE",
            "OBJECT",
            "SHARED",
            "STATIC",
        ],
        true,
    ),
    ("install", Some("FILES"), &["FILES"], false),
    ("install", Some("PROGRAMS"), &["PROGRAMS"], false),
    ("list", Some("APPEND"), &[], true),
    ("list", Some("PREPEND"), &[], true),
    ("list", Some("REMOVE_ITEM"), &[], true),
    ("set", None, &[], true),
    ("source_group", None, &["FILES"], false),
    (
        "target_sources",
        None,
        &["FILES", "INTERFACE", "PRIVATE", "PUBLIC"],
        false,
    ),
];

#[test]
fn test_the_reordering_allowlist_is_exactly_this() {
    let mut actual: Vec<(String, Option<String>, Vec<String>, bool)> = Vec::new();
    for (command, mode, grammar) in
        cmake_fmt::formatter::grammar::GrammarRegistry::global().entries()
    {
        if grammar.sortable_keywords.is_empty() && !grammar.sortable_positional {
            continue;
        }
        let mut keywords: Vec<String> = grammar.sortable_keywords.iter().cloned().collect();
        keywords.sort();
        actual.push((
            command.to_string(),
            mode.map(str::to_string),
            keywords,
            grammar.sortable_positional,
        ));
    }

    let expected: Vec<(String, Option<String>, Vec<String>, bool)> = ALLOWLIST
        .iter()
        .map(|(command, mode, keywords, positional)| {
            (
                command.to_string(),
                mode.map(str::to_string),
                keywords.iter().map(|k| k.to_string()).collect(),
                *positional,
            )
        })
        .collect();

    assert_eq!(
        actual, expected,
        "the reordering allowlist changed. Every entry here is a promise that \
         reordering that list cannot change what the file says — if you are \
         adding one, say why in the commit, and check the command's own \
         documentation for an argument whose position carries meaning."
    );
}
