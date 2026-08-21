use super::{CommandGrammar, KeywordType};
use crate::cst::CommandInvocation;
use std::collections::{HashMap, HashSet};

/// Keyword names that conventionally hold an unordered list of files.
///
/// Used only for auto-detected wrapper commands, where the grammar carries no
/// semantics: `cmake_parse_arguments` reports a keyword's arity, not its
/// meaning, so an auto-detected `COMMAND` looks like any other multi-value
/// list. A project can mark any other keyword explicitly through
/// `command_grammars` in `.cmake-fmt`.
fn is_conventional_file_list(keyword: &str) -> bool {
    matches!(keyword, "SOURCES" | "SRCS" | "FILES")
}

/// Extract command grammar from function/macro body by analyzing cmake_parse_arguments calls
///
/// Analyzes the body of a function/macro definition to find cmake_parse_arguments() calls
/// and extract keyword classifications. Resolves variable references via set() calls.
///
/// # Arguments
/// * `function_name` - The name of the function/macro being analyzed
/// * `body_commands` - All commands between function()/endfunction() or macro()/endmacro()
///
/// # Returns
/// Some(CommandGrammar) if cmake_parse_arguments found, None otherwise
pub fn extract_command_grammars_from_body(
    _function_name: &str,
    body_commands: &[CommandInvocation],
) -> Option<CommandGrammar> {
    // Step 1: Collect set() calls into variable map
    let set_vars = collect_set_variables(body_commands);

    // Step 2: Find cmake_parse_arguments() call
    for cmd in body_commands {
        if let Some(cmd_name) = cmd.name_text()
            && cmd_name.eq_ignore_ascii_case("cmake_parse_arguments")
        {
            // Step 3: Extract keyword lists
            if let Some(arg_list) = cmd.argument_list() {
                let args: Vec<_> = arg_list.arguments().collect();

                // Determine which form: standard or PARSE_ARGV
                let (options_idx, single_idx, multi_idx, prefix_idx) =
                    if args.len() > 1 && args[0].text() == "PARSE_ARGV" {
                        // PARSE_ARGV form: positions 3, 4, 5 are keyword lists
                        // Position 2 is prefix
                        (3, 4, 5, Some(2))
                    } else {
                        // Standard form: positions 1, 2, 3 are keyword lists
                        // Position 0 is prefix
                        (1, 2, 3, Some(0))
                    };

                // Extract prefix for variable resolution fallback
                let prefix = if let Some(idx) = prefix_idx {
                    args.get(idx).map(|t| {
                        let text = t.text();
                        strip_quotes(text)
                    })
                } else {
                    None
                };

                // Extract keyword lists
                let options = if let Some(arg) = args.get(options_idx) {
                    resolve_keyword_list(arg.text(), &set_vars, prefix.as_deref())
                } else {
                    vec![]
                };

                let single_value = if let Some(arg) = args.get(single_idx) {
                    resolve_keyword_list(arg.text(), &set_vars, prefix.as_deref())
                } else {
                    vec![]
                };

                let multi_value = if let Some(arg) = args.get(multi_idx) {
                    resolve_keyword_list(arg.text(), &set_vars, prefix.as_deref())
                } else {
                    vec![]
                };

                // Step 4: Build CommandGrammar
                let mut keywords = HashMap::new();

                for kw in options {
                    keywords.insert(kw, KeywordType::Flag);
                }

                for kw in single_value {
                    keywords.insert(kw, KeywordType::SingleValue);
                }

                for kw in multi_value {
                    keywords.insert(kw, KeywordType::MultiValue);
                }

                if !keywords.is_empty() {
                    let sortable_keywords = keywords
                        .keys()
                        .filter(|kw| is_conventional_file_list(kw))
                        .cloned()
                        .collect();

                    return Some(CommandGrammar {
                        keywords,
                        force_args_on_new_line: false,
                        sub_keywords: HashSet::new(),
                        collection_keywords: HashSet::new(),
                        sortable_keywords,
                        sortable_positional: false,
                    });
                }
            }
        }
    }

    None
}

/// Collect set() calls into a variable name -> value list map
fn collect_set_variables(commands: &[CommandInvocation]) -> HashMap<String, Vec<String>> {
    let mut vars = HashMap::new();

    for cmd in commands {
        if let Some(cmd_name) = cmd.name_text()
            && cmd_name.eq_ignore_ascii_case("set")
            && let Some(arg_list) = cmd.argument_list()
        {
            let args: Vec<_> = arg_list.arguments().collect();

            // set(VAR_NAME val1 val2 val3 ...)
            // First arg must be an unquoted identifier (no variable refs)
            if let Some(first_arg) = args.first() {
                let var_name = first_arg.text();

                // Only accept unquoted arguments as variable names
                if first_arg.kind() == crate::syntax_kind::SyntaxKind::UNQUOTED_ARGUMENT
                    && !var_name.contains("${")
                {
                    // Collect remaining arguments as values
                    let values: Vec<String> = args
                        .iter()
                        .skip(1)
                        .map(|t| {
                            let text = t.text();
                            strip_quotes(text)
                        })
                        .collect();

                    vars.insert(var_name.to_string(), values);
                }
            }
        }
    }

    vars
}

/// Resolve a keyword list argument into individual keyword strings
///
/// Handles:
/// - Quoted strings with semicolons: "OPT1;OPT2;OPT3" -> ["OPT1", "OPT2", "OPT3"]
/// - Variable references: ${_options} or "${_options}" -> lookup in set_vars
/// - Empty strings: "" -> []
/// - Single literals: KEYWORD -> ["KEYWORD"]
fn resolve_keyword_list(
    arg_text: &str,
    set_vars: &HashMap<String, Vec<String>>,
    prefix: Option<&str>,
) -> Vec<String> {
    let text = strip_quotes(arg_text);

    // Handle empty string
    if text.is_empty() {
        return vec![];
    }

    // Check for variable reference: ${VAR_NAME}
    if text.starts_with("${") && text.ends_with("}") {
        let var_name = &text[2..text.len() - 1];

        // Try direct lookup
        if let Some(values) = set_vars.get(var_name) {
            return values.clone();
        }

        // Try with prefix prepended (common pattern: ${PREFIX_options})
        if let Some(pfx) = prefix {
            let prefixed_name = format!("{}_{}", pfx, var_name);
            if let Some(values) = set_vars.get(&prefixed_name) {
                return values.clone();
            }
        }

        // Variable not found, return empty
        return vec![];
    }

    // Split on semicolons and filter empty strings
    text.split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Strip surrounding quotes from a string if present
fn strip_quotes(s: &str) -> String {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_quotes() {
        assert_eq!(strip_quotes("\"hello\""), "hello");
        assert_eq!(strip_quotes("hello"), "hello");
        assert_eq!(strip_quotes("\"\""), "");
        assert_eq!(strip_quotes("\""), "\"");
    }

    #[test]
    fn test_resolve_keyword_list_semicolons() {
        let set_vars = HashMap::new();
        let result = resolve_keyword_list("OPT1;OPT2;OPT3", &set_vars, None);
        assert_eq!(result, vec!["OPT1", "OPT2", "OPT3"]);
    }

    #[test]
    fn test_resolve_keyword_list_empty() {
        let set_vars = HashMap::new();
        let result = resolve_keyword_list("", &set_vars, None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_resolve_keyword_list_single() {
        let set_vars = HashMap::new();
        let result = resolve_keyword_list("KEYWORD", &set_vars, None);
        assert_eq!(result, vec!["KEYWORD"]);
    }
}
