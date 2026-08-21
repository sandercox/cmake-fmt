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

/// What the allowlist permits, per command: which keywords' values may be
/// permuted, and whether the positional run may be.
///
/// Kept as a literal table rather than read from the grammar, so that widening
/// the allowlist in `builtins.rs` cannot silently widen this test with it.
const ALLOWED_REORDERING: &[(&str, &[&str], bool)] = &[
    ("set", &[], true),
    ("list", &[], true),
    (
        "add_library",
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
        &["WIN32", "MACOSX_BUNDLE", "EXCLUDE_FROM_ALL"],
        true,
    ),
    (
        "target_sources",
        &["PUBLIC", "PRIVATE", "INTERFACE", "FILES"],
        false,
    ),
    ("source_group", &["FILES"], false),
    ("install", &["FILES", "PROGRAMS"], false),
];

/// Keyword names that auto-detected wrapper commands may reorder.
const CONVENTIONAL_FILE_LISTS: &[&str] = &["SOURCES", "SRCS", "FILES"];

/// True for an argument that reads as a CMake keyword (`SOURCES`, `DEPENDS`).
fn looks_like_keyword(arg: &str) -> bool {
    arg.len() > 1
        && arg
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
}

/// Split an argument list into `(governing keyword, values)` runs, so a
/// reordering can be attributed to the keyword that permitted it.
fn split_into_keyword_runs(args: &[String]) -> Vec<(Option<String>, Vec<String>)> {
    let mut runs: Vec<(Option<String>, Vec<String>)> = vec![(None, Vec::new())];

    for (index, arg) in args.iter().enumerate() {
        // The first argument is the variable or target name, never a keyword —
        // `set(PROTO_FILES ...)` would otherwise read as a keyword section.
        if index > 0 && looks_like_keyword(arg) {
            runs.push((Some(arg.clone()), Vec::new()));
        } else {
            runs.last_mut()
                .expect("runs is never empty")
                .1
                .push(arg.clone());
        }
    }

    runs
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
    // Every keyword must keep its identity and position, and only the runs the
    // allowlist names may be permuted — so re-marking `set`'s CACHE keyword or
    // `install`'s DIRECTORY mode as sortable fails this test.
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

            let entry = ALLOWED_REORDERING.iter().find(|(cmd, _, _)| cmd == name);
            let plain_runs = split_into_keyword_runs(plain_args);
            let reordered_runs = split_into_keyword_runs(reordered_args);

            if plain_runs.len() != reordered_runs.len() {
                problems.push(format!(
                    "{}:{} {} changed its keyword structure",
                    path.display(),
                    index,
                    name
                ));
                continue;
            }

            for ((keyword, plain_run), (reordered_keyword, reordered_run)) in
                plain_runs.iter().zip(reordered_runs.iter())
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

                let may_permute = match (entry, keyword.as_deref()) {
                    (Some((_, _, positional)), None) => *positional,
                    (Some((_, keywords, _)), Some(kw)) => keywords.contains(&kw),
                    // Unknown command: only conventionally named file lists
                    (None, Some(kw)) => CONVENTIONAL_FILE_LISTS.contains(&kw),
                    (None, None) => false,
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
                    // target name, which is pinned
                    if keyword.is_none() && plain_run.first() != reordered_run.first() {
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
