use cmake_format::cst::parse_text;
use cmake_format::formatter::{format_text, FormatConfig};
use std::panic;
use std::path::PathBuf;
use walkdir::WalkDir;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Format text on a thread with a larger stack to handle large files
/// (the recursive descent parser can overflow the default 8MB stack on 10k+ line files)
fn format_text_safe(input: &str, config: &FormatConfig) -> String {
    let input = input.to_string();
    let config = config.clone();
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024) // 16MB stack
        .spawn(move || format_text(&input, &config))
        .unwrap()
        .join()
        .unwrap()
}

/// Find all .cmake files in the corpus directory recursively
fn corpus_files() -> Vec<PathBuf> {
    let corpus_dir = "tests/corpus";
    let mut files = Vec::new();

    for entry in WalkDir::new(corpus_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            let is_cmake = path.extension().map_or(false, |ext| ext == "cmake");
            let is_cmakelists = path.file_name().and_then(|n| n.to_str()) == Some("CMakeLists.txt");
            if is_cmake || is_cmakelists {
                files.push(path.to_path_buf());
            }
        }
    }

    files.sort();
    files
}

/// Extract command names and their arguments (ignoring whitespace/trivia) for semantic comparison
fn extract_semantic_commands(source: &str) -> Vec<(String, Vec<String>)> {
    let cst = parse_text(source);
    cst.commands()
        .map(|cmd| {
            let name = cmd.name_text().unwrap_or_default().to_lowercase();
            let args: Vec<String> = cmd
                .argument_list()
                .map(|al| al.arguments().map(|a| a.text().to_string()).collect())
                .unwrap_or_default();
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

    assert!(
        !files.is_empty(),
        "No corpus files found in tests/corpus/"
    );

    let mut failed_files = Vec::new();

    for path in &files {
        let input = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read {}", path.display()));

        // Catch panics (use format_text_safe for stack safety on large files)
        let input_clone = input.clone();
        let config_clone = config.clone();
        let result = panic::catch_unwind(move || {
            format_text_safe(&input_clone, &config_clone);
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

    assert!(
        !files.is_empty(),
        "No corpus files found in tests/corpus/"
    );

    for path in &files {
        let input = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read {}", path.display()));

        let output = format_text_safe(&input, &config);

        let input_commands = extract_semantic_commands(&input);
        let output_commands = extract_semantic_commands(&output);

        assert_eq!(
            input_commands, output_commands,
            "Semantic preservation failed for {}: commands differ after formatting",
            path.display()
        );
    }
}

#[test]
fn test_corpus_idempotency() {
    let config = FormatConfig::default();
    let files = corpus_files();

    assert!(
        !files.is_empty(),
        "No corpus files found in tests/corpus/"
    );

    for path in &files {
        let input = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read {}", path.display()));

        let once = format_text_safe(&input, &config);
        let twice = format_text_safe(&once, &config);

        assert_eq!(
            once, twice,
            "Idempotency failed for {}: formatting twice produced different output",
            path.display()
        );
    }
}

#[test]
fn test_corpus_output_not_empty() {
    let config = FormatConfig::default();
    let files = corpus_files();

    assert!(
        !files.is_empty(),
        "No corpus files found in tests/corpus/"
    );

    for path in &files {
        let input = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read {}", path.display()));

        // Skip truly empty files
        if input.trim().is_empty() {
            continue;
        }

        let output = format_text_safe(&input, &config);

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

    assert!(
        !files.is_empty(),
        "No corpus files found in tests/corpus/"
    );

    for path in &files {
        let input = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read {}", path.display()));

        let output = format_text_safe(&input, &config);

        for (line_num, line) in output.lines().enumerate() {
            let has_trailing_whitespace =
                line.ends_with(' ') || line.ends_with('\t');

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

    assert!(
        !files.is_empty(),
        "No corpus files found in tests/corpus/"
    );

    for path in &files {
        let input = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read {}", path.display()));

        // Skip empty files
        if input.trim().is_empty() {
            continue;
        }

        let output = format_text_safe(&input, &config);

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
    let llvm_count = files.iter().filter(|p| p.to_string_lossy().contains("llvm")).count();
    let kde_count = files.iter().filter(|p| p.to_string_lossy().contains("kde")).count();
    let opencv_count = files.iter().filter(|p| p.to_string_lossy().contains("opencv")).count();

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
