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

/// Commands whose argument lists the allowlist permits reordering in.
///
/// Kept deliberately as a literal list rather than read from the grammar, so
/// that widening the allowlist in `builtins.rs` cannot silently widen this test
/// along with it.
const REORDERABLE_COMMANDS: &[&str] = &[
    "set",
    "list",
    "add_library",
    "add_executable",
    "target_sources",
    "source_group",
    "install",
];

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

    for arg in args {
        if looks_like_keyword(arg) {
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
    // globs in `install(DIRECTORY ... FILES_MATCHING ...)`, and shuffle MSVC
    // flag lists — all in real files under tests/corpus/.
    let plain = FormatConfig::default();
    let reordering = FormatConfig {
        sort_sources: cmake_fmt::formatter::SortSources::Alphabetical,
        source_grouping: cmake_fmt::formatter::SourceGrouping::HeadersFirst,
        ..Default::default()
    };

    let files = corpus_files();
    assert!(!files.is_empty(), "No corpus files found in tests/corpus/");

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

            if REORDERABLE_COMMANDS.contains(&name.as_str()) {
                // An allowlisted command may reorder, but must never gain or
                // lose an argument
                let mut plain_sorted = plain_args.clone();
                let mut reordered_sorted = reordered_args.clone();
                plain_sorted.sort();
                reordered_sorted.sort();
                assert_eq!(
                    plain_sorted,
                    reordered_sorted,
                    "{}({}) at index {} changed its arguments, not just their order",
                    name,
                    path.display(),
                    index
                );
            } else {
                // An unknown command may reorder only the values of a keyword
                // conventionally named after a file list. Everything else,
                // including a neighbouring DEPENDS or COMMAND, must be
                // byte-identical.
                let plain_runs = split_into_keyword_runs(plain_args);
                let reordered_runs = split_into_keyword_runs(reordered_args);

                assert_eq!(
                    plain_runs.len(),
                    reordered_runs.len(),
                    "{} at index {} in {} changed its keyword structure",
                    name,
                    index,
                    path.display()
                );

                for ((keyword, plain_run), (_, reordered_run)) in
                    plain_runs.iter().zip(reordered_runs.iter())
                {
                    let conventional = matches!(
                        keyword.as_deref(),
                        Some("SOURCES") | Some("SRCS") | Some("FILES")
                    );
                    if conventional {
                        let mut a = plain_run.clone();
                        let mut b = reordered_run.clone();
                        a.sort();
                        b.sort();
                        assert_eq!(
                            a,
                            b,
                            "{} at index {} in {} changed its {:?} values",
                            name,
                            index,
                            path.display(),
                            keyword
                        );
                    } else {
                        assert_eq!(
                            plain_run,
                            reordered_run,
                            "{} at index {} in {} reordered {:?}, which is not allowlisted",
                            name,
                            index,
                            path.display(),
                            keyword
                        );
                    }
                }
            }
        }
    }
}
