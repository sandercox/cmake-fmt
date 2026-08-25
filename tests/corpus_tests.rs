use cmake_fmt::cst::parse_text;
use cmake_fmt::formatter::{FormatConfig, format_text};
use std::panic;
use std::path::PathBuf;
use walkdir::WalkDir;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Find all .cmake files in the corpus directory recursively
fn corpus_files() -> Vec<PathBuf> {
    let corpus_dir = "tests/corpus";
    let mut files = Vec::new();

    for entry in WalkDir::new(corpus_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let is_cmake = path.extension().is_some_and(|ext| ext == "cmake");
            let is_cmakelists = path.file_name().and_then(|n| n.to_str()) == Some("CMakeLists.txt");
            if is_cmake || is_cmakelists {
                files.push(path.to_path_buf());
            }
        }
    }

    files.sort();
    files
}

/// Extract command names and their arguments (ignoring whitespace/trivia) for semantic comparison.
/// Block closer args (endif, endforeach, etc.) and else args are excluded since they are
/// semantically irrelevant in CMake and may be stripped by closing_style=remove.
fn extract_semantic_commands(source: &str) -> Vec<(String, Vec<String>)> {
    let cst = parse_text(source);
    cst.commands()
        .map(|cmd| {
            let name = cmd.name_text().unwrap_or_default().to_lowercase();
            let is_closer = matches!(
                name.as_str(),
                "endif" | "endforeach" | "endwhile" | "endfunction" | "endmacro" | "else"
            );
            let args: Vec<String> = if is_closer {
                Vec::new()
            } else {
                cmd.argument_list()
                    .map(|al| al.arguments().map(|a| a.text().to_string()).collect())
                    .unwrap_or_default()
            };
            (name, args)
        })
        .collect()
}

// ============================================================================
// CORPUS TESTS
// ============================================================================

#[test]
fn test_corpus_no_panics() {
    let config = FormatConfig::default();
    let files = corpus_files();

    assert!(!files.is_empty(), "No corpus files found in tests/corpus/");

    let mut failed_files = Vec::new();

    for path in &files {
        let input = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read {}", path.display()));

        // Catch panics (use format_text for stack safety on large files)
        let input_clone = input.clone();
        let config_clone = config.clone();
        let result = panic::catch_unwind(move || {
            format_text(&input_clone, &config_clone);
        });

        if result.is_err() {
            failed_files.push(path.display().to_string());
        }
    }

    if !failed_files.is_empty() {
        panic!(
            "Formatter panicked on {} file(s):\n  - {}",
            failed_files.len(),
            failed_files.join("\n  - ")
        );
    }
}

#[test]
fn test_corpus_semantic_preservation() {
    let config = FormatConfig::default();
    let files = corpus_files();

    assert!(!files.is_empty(), "No corpus files found in tests/corpus/");

    for path in &files {
        let input = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read {}", path.display()));

        let output = format_text(&input, &config);

        let input_commands = extract_semantic_commands(&input);
        let output_commands = extract_semantic_commands(&output);

        assert_eq!(
            input_commands,
            output_commands,
            "Semantic preservation failed for {}: commands differ after formatting",
            path.display()
        );
    }
}

#[test]
fn test_corpus_idempotency() {
    let config = FormatConfig::default();
    let files = corpus_files();

    assert!(!files.is_empty(), "No corpus files found in tests/corpus/");

    for path in &files {
        let input = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read {}", path.display()));

        let once = format_text(&input, &config);
        let twice = format_text(&once, &config);

        assert_eq!(
            once,
            twice,
            "Idempotency failed for {}: formatting twice produced different output",
            path.display()
        );
    }
}

#[test]
fn test_corpus_output_not_empty() {
    let config = FormatConfig::default();
    let files = corpus_files();

    assert!(!files.is_empty(), "No corpus files found in tests/corpus/");

    for path in &files {
        let input = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read {}", path.display()));

        // Skip truly empty files
        if input.trim().is_empty() {
            continue;
        }

        let output = format_text(&input, &config);

        assert!(
            !output.trim().is_empty(),
            "Formatted output is empty for non-empty corpus file: {}",
            path.display()
        );
    }
}

#[test]
fn test_corpus_no_trailing_whitespace() {
    let config = FormatConfig::default();
    let files = corpus_files();

    assert!(!files.is_empty(), "No corpus files found in tests/corpus/");

    for path in &files {
        let input = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read {}", path.display()));

        let output = format_text(&input, &config);

        for (line_num, line) in output.lines().enumerate() {
            let has_trailing_whitespace = line.ends_with(' ') || line.ends_with('\t');

            assert!(
                !has_trailing_whitespace,
                "Line {} in {} has trailing whitespace after formatting: {:?}",
                line_num + 1,
                path.display(),
                line
            );
        }
    }
}

#[test]
fn test_corpus_ends_with_newline() {
    let config = FormatConfig::default();
    let files = corpus_files();

    assert!(!files.is_empty(), "No corpus files found in tests/corpus/");

    for path in &files {
        let input = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read {}", path.display()));

        // Skip empty files
        if input.trim().is_empty() {
            continue;
        }

        let output = format_text(&input, &config);

        assert!(
            !output.is_empty() && output.ends_with('\n'),
            "Formatted output for {} does not end with exactly one newline",
            path.display()
        );

        // Verify it's exactly one newline, not multiple
        assert!(
            !output.ends_with("\n\n"),
            "Formatted output for {} ends with multiple newlines",
            path.display()
        );
    }
}

#[test]
fn test_corpus_files_exist() {
    let files = corpus_files();

    // Verify we have files from all 3 project categories
    let llvm_count = files
        .iter()
        .filter(|p| p.to_string_lossy().contains("llvm"))
        .count();
    let kde_count = files
        .iter()
        .filter(|p| p.to_string_lossy().contains("kde"))
        .count();
    let opencv_count = files
        .iter()
        .filter(|p| p.to_string_lossy().contains("opencv"))
        .count();

    assert!(
        llvm_count >= 3,
        "Expected at least 3 LLVM corpus files, found {}",
        llvm_count
    );
    assert!(
        kde_count >= 2,
        "Expected at least 2 KDE corpus files, found {}",
        kde_count
    );
    assert!(
        opencv_count >= 2,
        "Expected at least 2 OpenCV corpus files, found {}",
        opencv_count
    );

    println!("Found {} total corpus files:", files.len());
    println!("  LLVM: {}", llvm_count);
    println!("  KDE: {}", kde_count);
    println!("  OpenCV: {}", opencv_count);
}

/// What the allowlist permits, keyed by command and — for multi-mode commands
/// like `list` and `install` — by mode: which keywords' values may be permuted,
/// and whether the keyword-less run may be.
///
/// Kept as a literal table rather than read from the grammar, so that widening
/// the allowlist in `builtins.rs` cannot silently widen this test with it.
/// Coverage is bounded by what the corpus contains, so the synthetic cases in
/// `tests/sort_sources_tests.rs` are the other half of this guard.
const ALLOWED_REORDERING: &[(&str, Option<&str>, &[&str], bool)] = &[
    ("set", None, &[], true),
    ("list", Some("APPEND"), &[], true),
    ("list", Some("PREPEND"), &[], true),
    ("list", Some("REMOVE_ITEM"), &[], true),
    (
        "add_library",
        None,
        &[
            "STATIC",
            "SHARED",
            "MODULE",
            "OBJECT",
            "INTERFACE",
            "EXCLUDE_FROM_ALL",
        ],
        true,
    ),
    (
        "add_executable",
        None,
        &["WIN32", "MACOSX_BUNDLE", "EXCLUDE_FROM_ALL"],
        true,
    ),
    (
        "target_sources",
        None,
        &["PUBLIC", "PRIVATE", "INTERFACE", "FILES"],
        false,
    ),
    ("source_group", None, &["FILES"], false),
    ("install", Some("FILES"), &["FILES"], false),
    ("install", Some("PROGRAMS"), &["PROGRAMS"], false),
];

/// Keyword names that auto-detected wrapper commands may reorder.
const CONVENTIONAL_FILE_LISTS: &[&str] = &["SOURCES", "SRCS", "FILES"];

/// `sub_keywords` and `collection_keywords` are deliberately not modelled. They
/// are set on `install`'s TARGETS and DIRECTORY modes and on `file`'s COPY and
/// INSTALL modes; none of those has a table entry, so every run there must be
/// byte-identical anyway. Splitting more finely than the parser is strictly
/// stricter, and both sides go through this same function, so the decomposition
/// stays canonical. A future table entry for one of those commands would need
/// this revisited.
///
/// Split an argument list into `(governing keyword, values)` runs using the
/// command's real grammar, so the split does not have to guess which tokens are
/// keywords. Guessing got this wrong both ways: an all-caps list *variable*
/// (`list(APPEND SOURCES …)`) read as a keyword, and a keyword moving between
/// runs was invisible.
fn split_into_keyword_runs(command: &str, args: &[String]) -> Vec<(Option<String>, Vec<String>)> {
    use cmake_fmt::formatter::grammar::{GrammarRegistry, KeywordType};

    let registry = GrammarRegistry::global();
    let grammar = registry.get(command);
    // Multi-mode commands resolve on their first argument
    let resolved = grammar.and_then(|g| {
        if g.is_multi_mode() {
            g.resolve(args.first().map(String::as_str))
        } else {
            g.resolve(None)
        }
    });

    let keyword_type = |arg: &str| match resolved {
        Some(cg) => cg.keyword_type(arg),
        // No grammar to ask, so treat any all-caps token as a keyword. Using
        // CONVENTIONAL_FILE_LISTS here would be circular: a keyword the
        // formatter starts reordering but this test doesn't know about would be
        // absorbed into a neighbouring run and vanish into its multiset check.
        // Over-splitting costs nothing now that keyword identity is compared.
        None => (arg.len() > 1
            && arg
                .chars()
                .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()))
        .then_some(KeywordType::MultiValue),
    };

    let mut runs: Vec<(Option<String>, Vec<String>)> = vec![(None, Vec::new())];

    for arg in args {
        if keyword_type(arg).is_some() {
            runs.push((Some(arg.clone()), Vec::new()));
            continue;
        }

        let current = runs.last_mut().expect("runs is never empty");
        // A single-value keyword takes exactly one value; the next argument
        // overflows into a positional run. Same rule the formatter applies, and
        // what makes `list(APPEND var a b)` two sections rather than one.
        let at_capacity = !current.1.is_empty()
            && current
                .0
                .as_deref()
                .and_then(keyword_type)
                .is_some_and(|ty| ty == KeywordType::SingleValue);

        if at_capacity {
            runs.push((None, vec![arg.clone()]));
        } else {
            current.1.push(arg.clone());
        }
    }

    runs
}

/// The mode a multi-mode command resolved to, for looking up the table.
fn resolved_mode(command: &str, args: &[String]) -> Option<String> {
    use cmake_fmt::formatter::grammar::GrammarRegistry;

    let grammar = GrammarRegistry::global().get(command)?;
    if !grammar.is_multi_mode() {
        return None;
    }
    let first = args.first()?;
    grammar.resolve(Some(first.as_str())).map(|_| first.clone())
}

#[test]
fn test_corpus_reordering_confined_to_allowlist() {
    // The guard that was missing: `test_corpus_semantic_preservation` runs with
    // FormatConfig::default(), where both reordering passes are off, so it never
    // exercised sort_sources or source_grouping at all. Enabling them used to
    // rewrite `set(... CACHE PATH "docs")`, tear `PATTERN` keywords off their
    // globs in `install(DIRECTORY ... FILES_MATCHING ...)`, shuffle MSVC flag
    // lists, and reorder GCC warning flags in
    // tests/corpus/llvm/HandleLLVMOptions.cmake.
    //
    // Every keyword keeps its identity and position, and only the runs the
    // allowlist names may be permuted — so re-marking `set`'s CACHE keyword or
    // `install`'s DIRECTORY mode as sortable fails this test. A mode the corpus
    // never uses cannot fail it: nothing here calls `list(POP_BACK ...)`, which
    // is what `test_unlisted_list_modes_hold` is for.
    let plain = FormatConfig::default();
    let reordering = FormatConfig {
        sort_sources: cmake_fmt::formatter::SortSources::Alphabetical,
        source_grouping: cmake_fmt::formatter::SourceGrouping::HeadersFirst,
        ..Default::default()
    };

    let files = corpus_files();
    assert!(!files.is_empty(), "No corpus files found in tests/corpus/");

    let mut problems: Vec<String> = Vec::new();

    for path in &files {
        let input = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read {}", path.display()));

        let baseline = extract_semantic_commands(&format_text(&input, &plain));
        let reordered = extract_semantic_commands(&format_text(&input, &reordering));

        assert_eq!(
            baseline.len(),
            reordered.len(),
            "Command count changed in {}",
            path.display()
        );

        for (index, ((name, plain_args), (reordered_name, reordered_args))) in
            baseline.iter().zip(reordered.iter()).enumerate()
        {
            assert_eq!(
                name,
                reordered_name,
                "Command {} changed name in {}",
                index,
                path.display()
            );

            let command_has_grammar = cmake_fmt::formatter::grammar::GrammarRegistry::global()
                .get(name)
                .is_some();
            let mode = resolved_mode(name, plain_args);
            let entry = ALLOWED_REORDERING
                .iter()
                .find(|(cmd, cmd_mode, _, _)| cmd == name && *cmd_mode == mode.as_deref());

            let plain_runs = split_into_keyword_runs(name, plain_args);
            let reordered_runs = split_into_keyword_runs(name, reordered_args);

            if plain_runs.len() != reordered_runs.len() {
                problems.push(format!(
                    "{}:{} {} changed its keyword structure",
                    path.display(),
                    index,
                    name
                ));
                continue;
            }

            for (run_index, ((keyword, plain_run), (reordered_keyword, reordered_run))) in
                plain_runs.iter().zip(reordered_runs.iter()).enumerate()
            {
                // A keyword moving is the original bug — PATTERN piled up at the
                // end of FILES_MATCHING — so identity and order are compared.
                if keyword != reordered_keyword {
                    problems.push(format!(
                        "{}:{} {} moved keyword {:?} to {:?}",
                        path.display(),
                        index,
                        name,
                        keyword,
                        reordered_keyword
                    ));
                    continue;
                }

                // The formatter permits a keyword-less run only when it leads
                // the command, or when it overflowed from the command's first
                // section (`list(APPEND var a b)`). A stray run later on is not
                // the command's argument list.
                let positional_run_permitted = run_index == 0
                    || (run_index == 2
                        && plain_runs[0].1.is_empty()
                        && plain_runs[1].0.is_some()
                        && plain_runs[1].1.len() == 1);

                let may_permute = match (entry, keyword.as_deref()) {
                    (Some((_, _, _, positional)), None) => *positional && positional_run_permitted,
                    (Some((_, _, keywords, _)), Some(kw)) => keywords.contains(&kw),
                    // A command with no grammar at all: only conventionally
                    // named file lists, which is all the formatter reorders
                    // there. A command that HAS a grammar but no table entry is
                    // a different thing and gets nothing — otherwise marking,
                    // say, add_custom_target's SOURCES sortable would be
                    // permitted without the table saying so.
                    (None, Some(kw)) if !command_has_grammar => {
                        CONVENTIONAL_FILE_LISTS.contains(&kw)
                    }
                    (None, _) => false,
                };

                if may_permute {
                    let mut a = plain_run.clone();
                    let mut b = reordered_run.clone();
                    a.sort();
                    b.sort();
                    if a != b {
                        problems.push(format!(
                            "{}:{} {} {:?} gained or lost an argument",
                            path.display(),
                            index,
                            name,
                            keyword
                        ));
                    }
                    // The leading positional run starts with the variable or
                    // target name, which is pinned. A run opened by a mode
                    // keyword has already given up its name to that keyword.
                    if run_index == 0
                        && keyword.is_none()
                        && plain_run.first() != reordered_run.first()
                    {
                        problems.push(format!(
                            "{}:{} {} moved its first positional argument {:?}",
                            path.display(),
                            index,
                            name,
                            plain_run.first()
                        ));
                    }
                } else if plain_run != reordered_run {
                    problems.push(format!(
                        "{}:{} {} reordered {:?}, which is not allowlisted:\n    {:?}\n -> {:?}",
                        path.display(),
                        index,
                        name,
                        keyword,
                        plain_run,
                        reordered_run
                    ));
                }
            }
        }
    }

    assert!(
        problems.is_empty(),
        "reordering escaped the allowlist in {} place(s):\n  {}",
        problems.len(),
        problems.join("\n  ")
    );
}
