use anyhow::Result;
use clap::Parser;
use rayon::prelude::*;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use cmake_fmt::formatter::{
    FormatConfig, FormatWarning, LineRange, format_text_with_diagnostics_and_path,
    format_with_line_ranges, parse_line_ranges,
};

/// Format CMake files
#[derive(Parser)]
#[command(name = "cmake-fmt", version, about = "Format CMake files")]
pub struct Cli {
    /// Files to format (use "-" for stdin)
    #[arg(value_name = "FILE")]
    pub files: Vec<PathBuf>,

    /// Format files in-place
    #[arg(short = 'i', long = "in-place")]
    pub in_place: bool,

    /// Check if files are formatted (exit 1 if changes needed)
    #[arg(long, conflicts_with = "in_place")]
    pub check: bool,

    /// Dry run mode (same as --check)
    #[arg(long = "dry-run", conflicts_with = "in_place")]
    pub dry_run: bool,

    /// Show diff of formatting changes
    #[arg(long, conflicts_with = "in_place")]
    pub diff: bool,

    /// Interactive mode: review formatting changes hunk-by-hunk
    #[arg(long, conflicts_with_all = ["in_place", "check", "dry_run", "diff", "line_ranges"])]
    pub interactive: bool,

    /// Override config inline (e.g., "indent_width=4,max_line_length=100")
    #[arg(long)]
    pub style: Option<String>,

    /// Export detected custom grammars to a file (scanned from input files)
    #[arg(long = "export-grammar", value_name = "FILE")]
    pub export_grammar: Option<PathBuf>,

    /// Export all grammars including builtins to a file
    #[arg(long = "export-all-grammar", value_name = "FILE")]
    pub export_all_grammar: Option<PathBuf>,

    /// Import additional grammar file(s) (can be specified multiple times)
    #[arg(long = "grammar-file", value_name = "FILE")]
    pub grammar_files: Vec<PathBuf>,

    /// Show verbose output during file scanning and analysis
    #[arg(long)]
    pub verbose: bool,

    /// Show all available style settings
    #[arg(long = "help-style", display_order = 900)]
    pub help_style: bool,

    /// Show grammar file format and keyword types
    #[arg(long = "help-grammar", display_order = 901)]
    pub help_grammar: bool,

    /// Treat stdin as if formatting this file (resolves config/grammar from its path)
    #[arg(long = "assume-filename", value_name = "PATH")]
    pub assume_filename: Option<PathBuf>,

    /// Format only specific line ranges (e.g., "1:5,10:15")
    #[arg(long = "line-ranges", value_name = "RANGES")]
    pub line_ranges: Option<String>,

    /// Recursively format files in directories
    #[arg(short = 'r', long = "recursive")]
    pub recursive: bool,

    /// Path to additional ignore file (gitignore syntax)
    #[arg(long = "ignore-file", value_name = "FILE")]
    pub ignore_file: Option<PathBuf>,
}

/// Print suppression warnings to stderr
fn print_warnings(warnings: &[FormatWarning], file_label: &str) {
    for warning in warnings {
        eprintln!("{}: {}", file_label, warning);
    }
}

/// What processing one file concluded.
struct FileOutcome {
    /// The file is not formatted as configured (check/diff mode reports this).
    needs_formatting: bool,
    /// The file was left alone because formatting would have changed its
    /// contents. Fails the run in every mode.
    content_changed: bool,
    /// The file named on the command line is not there, or is not a file. Fails
    /// the run in every mode, like a file that cannot be read: `cmake-fmt
    /// --check path/that/moved.cmake` reporting success is how an unformatted
    /// file reaches a release.
    missing: bool,
}

impl FileOutcome {
    fn needs_formatting(needs_formatting: bool) -> Self {
        Self {
            needs_formatting,
            content_changed: false,
            missing: false,
        }
    }

    fn missing() -> Self {
        Self {
            needs_formatting: false,
            content_changed: false,
            missing: true,
        }
    }
}

/// True when a file was left unformatted because the output would have said
/// something different from the input. The run reports failure so this surfaces
/// in CI rather than only in whoever reads stderr.
fn content_changed(warnings: &[FormatWarning]) -> bool {
    warnings
        .iter()
        .any(|w| matches!(w, FormatWarning::ContentChanged { .. }))
}

/// Print all available style settings
#[allow(clippy::print_literal)]
fn print_style_help() {
    println!(
        "Available style settings for --style and config files (.cmake-fmt.toml / .cmake-fmt.yaml / .cmake-fmt):"
    );
    println!();
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "Setting", "Type", "Default", "Values"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "-------", "----", "-------", "------"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "disable_format", "boolean", "false", "true, false — skip formatting entirely"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "indent_width", "integer", "4", "Number of spaces per indent level"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "max_line_length", "integer", "80", "Max line length (0 = unlimited)"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "use_tabs", "boolean", "true", "true, false"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "command_case", "enum", "lowercase", "lowercase, uppercase, preserve"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "user_command_case", "enum", "infer", "lowercase, uppercase, preserve, infer"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "max_blank_lines", "integer", "1", "Maximum consecutive blank lines allowed"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "line_ending", "enum", "auto", "auto, lf, crlf"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "closing_style", "enum", "remove", "preserve, remove, force"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "force_break_keywords", "boolean", "false", "true, false"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "final_newline", "enum", "preserve", "preserve, remove, force (also accepts true/false)"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "comment_style", "enum", "hash_space", "preserve, hash_space, hash_no_space"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "source_grouping", "enum", "none", "none, headers_first, sources_first"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "sort_sources", "enum", "none", "none, alphabetical"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "collapse_empty_flags",
        "boolean",
        "true",
        "true, false — keep a no-argument flag on the previous line"
    );
    println!(
        "  {:<25} {:<15} {:<15} {}",
        "inline_single_keyword",
        "boolean",
        "false",
        "true, false — keep a lone keyword on the opening line"
    );
    println!();
    println!("CLI usage:  cmake-fmt --style \"indent_width=4,max_line_length=120\" <file>");
    println!();
    println!("Config file only (not available via --style):");
    println!(
        "  command_grammars        map            {{}}              Custom command grammar definitions"
    );
    println!();
    println!("Example .cmake-fmt.toml:");
    println!("  indent_width = 2");
    println!("  use_tabs = false");
    println!("  command_case = \"lowercase\"");
}

/// Print grammar file format documentation
#[allow(clippy::print_literal)]
fn print_grammar_help() {
    println!("Grammar files teach cmake-fmt about custom CMake commands");
    println!();
    println!("Supported formats:");
    println!("  - YAML (.yaml, .yml)");
    println!("  - TOML (.toml, .tml)");
    println!();
    println!("Example grammar file (YAML):");
    println!();
    println!("  grammar:");
    println!("    - command: my_custom_command");
    println!("      keywords:");
    println!("        REQUIRED: Flag");
    println!("        DESTINATION: SingleValue");
    println!("        SOURCES: MultiValue");
    println!("        COMMAND: BinPack");
    println!("        PROPERTIES: PairValue");
    println!("      sortable_keywords: [SOURCES]");
    println!("      sortable_positional: false");
    println!();
    println!("Keyword types:");
    println!();
    println!("  {:<15} {}", "Type", "Description");
    println!("  {:<15} {}", "----", "-----------");
    println!(
        "  {:<15} {}",
        "Flag", "No value consumed (e.g., REQUIRED, QUIET)"
    );
    println!(
        "  {:<15} {}",
        "SingleValue", "Consumes exactly one value (e.g., VERSION 1.0, DESTINATION /usr/lib)"
    );
    println!(
        "  {:<15} {}",
        "MultiValue", "Consumes all values until next keyword (e.g., SOURCES a.cpp b.cpp c.cpp)"
    );
    println!(
        "  {:<15} {}",
        "BinPack", "Packs values to fill lines (e.g., COMMAND echo hello world)"
    );
    println!(
        "  {:<15} {}",
        "PairValue", "Consumes alternating key/value pairs (e.g., PROPERTIES CXX_STANDARD 17)"
    );
    println!();
    println!("Reordering:");
    println!("  sort_sources and source_grouping only reorder keywords listed in");
    println!("  'sortable_keywords', plus the keyword-less arguments when");
    println!("  'sortable_positional: true'. That reaches two runs: the leading");
    println!("  one, whose first argument is pinned because it names the list or");
    println!("  the target, and the run overflowing a leading single-value");
    println!("  keyword, with nothing pinned because the keyword consumed the");
    println!("  name — which is what sorts list(APPEND SRCS ...), and applies to");
    println!("  your own grammar too. Do not set it on a command whose tail after");
    println!("  such a keyword is order-significant.");
    println!("  Argument order usually carries meaning, so nothing is sortable");
    println!("  until something says it is.");
    println!("  In a grammar file that is the whole rule: a keyword is sortable");
    println!("  only if it is listed, and there is no fallback. A .cmake-fmt");
    println!("  config entry has one — omitting 'sortable_keywords' falls back to");
    println!("  keywords named SOURCES, SRCS or FILES, and writing it as an empty");
    println!("  list says nothing in that command is sortable.");
    println!("  BinPack and PairValue keywords are never reordered, and a");
    println!("  positional run additionally requires every value to look like a");
    println!("  source file, so flag and library lists are left alone.");
    println!();
    println!("Multi-mode commands:");
    println!("  NOT YET SUPPORTED ON IMPORT. --export-all-grammar writes these entries,");
    println!("  and --grammar-file skips every one of them with a warning, so the export");
    println!("  does not round-trip. The shape is documented because that is what the");
    println!("  export writes:");
    println!();
    println!("  For commands like install() that have different keyword sets per sub-command,");
    println!("  a 'mode' field distinguishes the entries:");
    println!();
    println!("    - command: install");
    println!("      mode: TARGETS");
    println!("      keywords:");
    println!("        DESTINATION: SingleValue");
    println!();
    println!("    - command: install");
    println!("      mode: FILES");
    println!("      keywords:");
    println!("        DESTINATION: SingleValue");
    println!("        FILES: MultiValue");
    println!();
    println!("Usage:");
    println!("  --grammar-file <path>        Import grammar file");
    println!("  --export-grammar <path>      Export custom grammars from input files");
    println!("  --export-all-grammar <path>  Export all grammars including builtins");
    println!();
    println!("Config file:");
    println!("  grammar_files = [\"path/to/grammars.yaml\"]");
}

/// Export all grammars (including builtins) to a file
fn export_all_grammar_to_file(path: &std::path::Path) -> Result<ExitCode> {
    use anyhow::Context;
    use cmake_fmt::formatter::grammar::builtin_grammars;
    use cmake_fmt::formatter::{detect_grammar_format, export_grammars};
    use std::fs;

    let grammars = builtin_grammars();
    let format = detect_grammar_format(path);
    let content = export_grammars(&grammars, &format);

    fs::write(path, content)
        .with_context(|| format!("Failed to write grammar file: {}", path.display()))?;

    eprintln!("Exported {} grammars to {}", grammars.len(), path.display());
    Ok(ExitCode::SUCCESS)
}

/// Export custom grammars (detected from input files) to a file
fn export_custom_grammar_to_file(
    path: &std::path::Path,
    input_files: &[PathBuf],
    style_override: Option<&str>,
    grammar_files_arg: &[PathBuf],
    verbose: bool,
) -> Result<ExitCode> {
    use anyhow::Context;
    use cmake_fmt::formatter::grammar::{
        builtin_grammars, config_grammars_to_map, get_project_user_commands,
        get_project_user_grammars,
    };
    use cmake_fmt::formatter::{detect_grammar_format, export_command_grammars};
    use std::collections::{HashMap, HashSet};
    use std::fs;

    let mut merged_grammars = HashMap::new();
    let mut all_user_commands = HashMap::new();

    // Scan each input file for auto-detected grammars and config grammars
    for file in input_files {
        // Get auto-detected grammars from this file's project
        let auto_detected = get_project_user_grammars(file, verbose);

        // Merge auto-detected (don't override existing)
        for (name, grammar) in auto_detected {
            merged_grammars.entry(name).or_insert(grammar);
        }

        // Get config grammars for this file
        let config = crate::config::resolve_config(Some(file), style_override, grammar_files_arg);
        let config_grammar_map = config_grammars_to_map(&config.command_grammars);

        // Config grammars override auto-detected
        merged_grammars.extend(config_grammar_map);

        // Collect all user command definitions
        let user_cmds = get_project_user_commands(file, verbose);
        all_user_commands.extend(user_cmds);
    }

    // Detect grammarless custom commands
    let builtin_names: HashSet<String> = builtin_grammars().keys().cloned().collect();
    let mut grammarless_commands: Vec<String> = all_user_commands
        .iter()
        .filter_map(|(lowercase_name, original_name)| {
            // Skip if this command has a grammar (auto-detected or config)
            if merged_grammars.contains_key(lowercase_name) {
                return None;
            }
            // Skip if this is a builtin command
            if builtin_names.contains(lowercase_name) {
                return None;
            }
            // This is a custom command with no grammar
            Some(original_name.clone())
        })
        .collect();

    // Sort for deterministic output
    grammarless_commands.sort();

    // Warn about grammarless commands
    if !grammarless_commands.is_empty() {
        for cmd in &grammarless_commands {
            eprintln!(
                "warning: no grammar found for custom command '{}' (define in config or grammar file)",
                cmd
            );
        }
        eprintln!(
            "{} custom command(s) have no grammar definition",
            grammarless_commands.len()
        );
    }

    let format = detect_grammar_format(path);
    let content = export_command_grammars(&merged_grammars, &format, Some(&all_user_commands));

    fs::write(path, content)
        .with_context(|| format!("Failed to write grammar file: {}", path.display()))?;

    if merged_grammars.is_empty() {
        eprintln!("No custom grammars detected in input files");
    } else {
        eprintln!(
            "Exported {} custom grammars to {}",
            merged_grammars.len(),
            path.display()
        );
    }

    Ok(ExitCode::SUCCESS)
}

/// Run the CLI application
pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    // Handle --help-style
    if cli.help_style {
        print_style_help();
        return Ok(ExitCode::SUCCESS);
    }

    // Handle --help-grammar
    if cli.help_grammar {
        print_grammar_help();
        return Ok(ExitCode::SUCCESS);
    }

    // Handle --export-all-grammar (exports all builtins)
    if let Some(ref export_path) = cli.export_all_grammar {
        return export_all_grammar_to_file(export_path);
    }

    // Parse and validate --line-ranges if provided
    let parsed_line_ranges = if let Some(ref ranges_str) = cli.line_ranges {
        match parse_line_ranges(ranges_str) {
            Ok(ranges) => Some(ranges),
            Err(e) => {
                eprintln!("error: {}", e);
                return Ok(ExitCode::FAILURE);
            }
        }
    } else {
        None
    };

    // Handle interactive mode first (if --interactive flag is set)
    if cli.interactive {
        // Determine if stdin input is specified
        let is_stdin =
            cli.files.is_empty() || (cli.files.len() == 1 && cli.files[0] == Path::new("-"));

        // Validate exactly one file is provided (no stdin, no multi-file)
        // Do this BEFORE TTY check so error messages are more specific
        if cli.files.is_empty() || is_stdin {
            eprintln!("error: interactive mode requires a file argument");
            return Ok(ExitCode::FAILURE);
        }
        if cli.files.len() > 1 {
            eprintln!("error: interactive mode supports one file at a time");
            return Ok(ExitCode::FAILURE);
        }

        // TTY guard (INT-06): Check stdin and stderr are both terminals
        if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
            eprintln!("error: interactive mode requires a terminal (TTY)");
            return Ok(ExitCode::FAILURE);
        }

        // Resolve config for the file
        let config = crate::config::resolve_config(
            Some(&cli.files[0]),
            cli.style.as_deref(),
            &cli.grammar_files,
        );

        // Run interactive mode
        match cmake_fmt::interactive::run_interactive(&cli.files[0], &config) {
            Ok(result) if result.content_changed => return Ok(ExitCode::from(1)),
            Ok(_result) => return Ok(ExitCode::SUCCESS),
            Err(e) => {
                eprintln!("error: {:#}", e);
                return Ok(ExitCode::FAILURE);
            }
        }
    }

    // Determine if we're in check mode
    let check_mode = cli.check || cli.dry_run;

    // Determine if we're in diff mode
    let mut diff_mode = cli.diff;

    // Determine if we're processing stdin or files.
    // stdin is active when:
    //   - files list is empty AND stdin is NOT a terminal (piped input), OR
    //   - the single argument is "-"
    let explicit_stdin = cli.files.len() == 1 && cli.files[0] == Path::new("-");
    let stdin_is_terminal = std::io::stdin().is_terminal();
    let is_stdin = explicit_stdin || (cli.files.is_empty() && !stdin_is_terminal);

    // Validate --assume-filename is only used with stdin
    if cli.assume_filename.is_some() && !is_stdin {
        eprintln!("error: --assume-filename can only be used with stdin input");
        return Ok(ExitCode::FAILURE);
    }

    if is_stdin {
        // Handle --export-grammar with stdin (error)
        if cli.export_grammar.is_some() {
            eprintln!("error: --export-grammar requires input files to scan for custom grammars");
            return Ok(ExitCode::FAILURE);
        }

        // Canonicalize assume_filename if provided
        let assume_path = cli.assume_filename.as_ref().map(|p| {
            if p.is_relative() {
                std::env::current_dir().unwrap_or_default().join(p)
            } else {
                p.clone()
            }
        });

        // For stdin, resolve config from assume_filename path or current directory
        let config = crate::config::resolve_config(
            assume_path.as_deref(),
            cli.style.as_deref(),
            &cli.grammar_files,
        );
        process_stdin(
            &config,
            check_mode,
            diff_mode,
            assume_path.as_deref(),
            parsed_line_ranges.as_deref(),
        )
    } else {
        // Determine the paths to process.
        // If no paths given and stdin is a terminal, default to current directory ".".
        let paths_to_search: Vec<PathBuf> = if cli.files.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            cli.files.clone()
        };

        // Collect files, expanding directories and glob patterns
        let collected =
            collect_cmake_files(&paths_to_search, cli.recursive, cli.ignore_file.as_deref())?;
        let unmatched_pattern = collected.unmatched_pattern;
        let collected = collected.files;

        // Handle case where no files found
        if collected.is_empty() {
            eprintln!("No files found");
            return Ok(if unmatched_pattern {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            });
        }

        // Validate --line-ranges with multiple files
        if parsed_line_ranges.is_some() && collected.len() > 1 {
            eprintln!("error: --line-ranges can only be used with a single file");
            return Ok(ExitCode::FAILURE);
        }

        // Implicit diff mode: when multiple files are found AND outputting to a
        // terminal AND not in-place/check mode, imply --diff so output is useful.
        let stdout_is_terminal = std::io::stdout().is_terminal();
        if collected.len() > 1 && !cli.in_place && !check_mode && stdout_is_terminal {
            diff_mode = true;
        }

        // Handle --export-grammar (exports custom grammars from input files)
        if let Some(ref export_path) = cli.export_grammar {
            return export_custom_grammar_to_file(
                export_path,
                &collected,
                cli.style.as_deref(),
                &cli.grammar_files,
                cli.verbose,
            );
        }

        let outcome = process_files(
            &collected,
            cli.style.as_deref(),
            &cli.grammar_files,
            cli.in_place,
            check_mode,
            diff_mode,
            cli.verbose,
            parsed_line_ranges.as_deref(),
        )?;

        // A pattern that matched nothing fails the run even when its siblings
        // matched, for the same reason a named path that is not there does.
        Ok(if unmatched_pattern {
            ExitCode::FAILURE
        } else {
            outcome
        })
    }
}

/// Collect CMake files from a list of paths, expanding directories.
///
/// For each path:
/// - If it's a file, include it directly.
/// - If it's a glob pattern, expand it and include matching files.
/// - If it's a directory, walk it and collect .cmake / CMakeLists.txt files.
///
/// Directory walking respects .gitignore, .cmake-fmt-ignore (in every walked
/// directory), and the optional extra `ignore_file`.  When `recursive` is false
/// the walk is limited to depth 1 (immediate directory contents).
/// The CMake files named, globbed or walked, and whether any pattern matched
/// nothing.
struct Collected {
    files: Vec<PathBuf>,
    unmatched_pattern: bool,
}

fn collect_cmake_files(
    paths: &[PathBuf],
    recursive: bool,
    ignore_file: Option<&Path>,
) -> Result<Collected> {
    use ignore::WalkBuilder;

    let mut result: Vec<PathBuf> = Vec::new();
    let mut dir_paths: Vec<PathBuf> = Vec::new();
    // A pattern that matched nothing is a failed run, for the same reason a
    // named path that is not there is: `cmake-fmt --check 'src/**/*.cmake'`
    // reporting success after a directory rename is how an unformatted file
    // reaches a release. A *directory* holding no CMake files is not — there was
    // nothing to do and the caller said so.
    //
    // cmake-fmt expands its own globs, so this is the normal spelling wherever
    // the shell does not.
    let mut unmatched_pattern = false;

    for path in paths {
        let path_str = path.to_string_lossy();

        if path_str.contains('*') || path_str.contains('?') || path_str.contains('[') {
            // Glob pattern — expand and collect files
            match glob::glob(&path_str) {
                Ok(entries) => {
                    let mut found_any = false;
                    for entry in entries {
                        match entry {
                            Ok(p) if p.is_file() => {
                                result.push(p);
                                found_any = true;
                            }
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("Warning: Error reading glob entry: {:#}", e);
                            }
                        }
                    }
                    if !found_any {
                        eprintln!("Warning: No files matched pattern: {}", path_str);
                        unmatched_pattern = true;
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Invalid glob pattern '{}': {:#}", path_str, e);
                }
            }
        } else if path.is_dir() {
            dir_paths.push(path.clone());
        } else {
            // Regular file path — include as-is
            result.push(path.clone());
        }
    }

    // Walk directories using the `ignore` crate
    if !dir_paths.is_empty() {
        let first = dir_paths[0].clone();
        let mut builder = WalkBuilder::new(&first);

        // Add remaining directories
        for dir in &dir_paths[1..] {
            builder.add(dir);
        }

        // Depth: 1 means the directory itself + its immediate children (depth 0 = root only)
        if !recursive {
            builder.max_depth(Some(1));
        }

        // Respect .cmake-fmt-ignore in every walked directory (like .gitignore)
        builder.add_custom_ignore_filename(".cmake-fmt-ignore");

        // Respect .gitignore, global gitignore, and .git/info/exclude
        builder.git_ignore(true);
        builder.git_global(true);
        builder.git_exclude(true);

        // Do NOT skip hidden directories/files — let ignore rules handle exclusions
        builder.hidden(false);

        // Load the user-specified extra ignore file if provided
        if let Some(extra) = ignore_file {
            builder.add_ignore(extra);
        }

        let walk = builder.build();

        for entry in walk {
            match entry {
                Ok(e) => {
                    if e.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                        let p = e.into_path();
                        if is_cmake_file(&p) {
                            result.push(p);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Error walking directory: {:#}", e);
                }
            }
        }
    }

    result.sort();
    result.dedup();
    Ok(Collected {
        files: result,
        unmatched_pattern,
    })
}

/// Returns true if the path is a CMake file (CMakeLists.txt or *.cmake)
fn is_cmake_file(path: &Path) -> bool {
    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();
        if name_str == "CMakeLists.txt" {
            return true;
        }
        if let Some(ext) = path.extension()
            && ext.to_string_lossy().eq_ignore_ascii_case("cmake")
        {
            return true;
        }
    }
    false
}

/// Process stdin to stdout
fn process_stdin(
    config: &FormatConfig,
    check_mode: bool,
    diff_mode: bool,
    assume_path: Option<&std::path::Path>,
    line_ranges: Option<&[LineRange]>,
) -> Result<ExitCode> {
    use std::io::{Read, Write, stdin, stdout};

    let mut input = String::new();
    stdin().lock().read_to_string(&mut input)?;

    let (formatted, warnings) = if let Some(ranges) = line_ranges {
        format_with_line_ranges(&input, config, ranges, assume_path, false)
    } else {
        format_text_with_diagnostics_and_path(&input, config, assume_path, false)
    };
    let label = assume_path
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "stdin".to_string());
    print_warnings(&warnings, &label);

    // Left unchanged on purpose, but the run still fails so CI notices
    if content_changed(&warnings) {
        if !check_mode && !diff_mode {
            use std::io::Write as _;
            write!(stdout().lock(), "{}", input)?;
        }
        return Ok(ExitCode::from(1));
    }

    if diff_mode {
        if input != formatted {
            cmake_fmt::diff::print_colored_diff(&input, &formatted, &label);
            Ok(ExitCode::from(1))
        } else {
            Ok(ExitCode::SUCCESS)
        }
    } else if check_mode {
        if input != formatted {
            eprintln!("Would reformat: {}", label);
            Ok(ExitCode::from(1))
        } else {
            Ok(ExitCode::SUCCESS)
        }
    } else {
        let mut handle = stdout().lock();
        write!(handle, "{}", formatted)?;
        Ok(ExitCode::SUCCESS)
    }
}

/// Process files
#[allow(clippy::too_many_arguments)]
fn process_files(
    files: &[PathBuf],
    style_override: Option<&str>,
    grammar_files: &[PathBuf],
    in_place: bool,
    check_mode: bool,
    diff_mode: bool,
    verbose: bool,
    line_ranges: Option<&[LineRange]>,
) -> Result<ExitCode> {
    use std::collections::HashMap;
    use std::io::{Write, stdout};

    // Detect stdout mode - this must remain sequential to avoid interleaved output
    let stdout_mode = !in_place && !check_mode && !diff_mode;

    if stdout_mode {
        // SEQUENTIAL PATH: stdout mode
        let mut stdout_handle = stdout().lock();
        let mut config_cache: HashMap<PathBuf, FormatConfig> = HashMap::new();
        let mut any_content_changed = false;
        let mut any_error = false;

        for file in files {
            // A file named on the command line that is not there is a failed
            // run, for the same reason an unreadable one is: `cmake-fmt --check
            // path/that/moved.cmake` reporting success is how an unformatted
            // file reaches a release.
            if !file.exists() {
                eprintln!("Warning: File not found: {}", file.display());
                any_error = true;
                continue;
            }

            if !file.is_file() {
                eprintln!("Warning: Not a file: {}", file.display());
                any_error = true;
                continue;
            }

            // Resolve config for this file (using cache for efficiency)
            let parent = file.parent().unwrap_or_else(|| std::path::Path::new("."));
            let config = config_cache.entry(parent.to_path_buf()).or_insert_with(|| {
                crate::config::resolve_config(Some(file), style_override, grammar_files)
            });

            // Format and write to stdout
            let content = std::fs::read_to_string(file)?;
            let (formatted, warnings) = if let Some(ranges) = line_ranges {
                format_with_line_ranges(&content, config, ranges, Some(file.as_path()), verbose)
            } else {
                format_text_with_diagnostics_and_path(
                    &content,
                    config,
                    Some(file.as_path()),
                    verbose,
                )
            };
            print_warnings(&warnings, &file.display().to_string());
            if content_changed(&warnings) {
                any_content_changed = true;
            }
            write!(stdout_handle, "{}", formatted)?;
        }

        // A file the formatter refused to touch fails the run in every mode, or
        // the one mode a user runs by hand is the one that says nothing is wrong
        return Ok(if any_content_changed || any_error {
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        });
    }

    // PARALLEL PATH: in-place, check, or diff mode

    // Step 1: Pre-populate config cache (by parent directory)
    let mut config_cache: HashMap<PathBuf, FormatConfig> = HashMap::new();
    for file in files {
        let parent = file
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        config_cache.entry(parent).or_insert_with(|| {
            crate::config::resolve_config(Some(file), style_override, grammar_files)
        });
    }
    let config_cache = Arc::new(config_cache);

    // Step 2: Pre-populate grammar cache (deduplicate by project root)
    let mut project_roots = std::collections::HashSet::new();
    for file in files {
        if file.exists() && file.is_file() {
            let parent = file.parent().unwrap_or_else(|| std::path::Path::new("."));
            let abs_parent = if parent.is_absolute() {
                parent.to_path_buf()
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(parent))
                    .unwrap_or_else(|_| parent.to_path_buf())
            };
            let project_root = cmake_fmt::formatter::grammar::user_scanner::find_project_root(
                &abs_parent,
                verbose,
            );
            project_roots.insert(project_root);
        }
    }

    // Populate grammar cache for each unique project root
    for project_root in &project_roots {
        let _ = cmake_fmt::formatter::grammar::get_project_user_grammars(
            &project_root.join("CMakeLists.txt"),
            verbose,
        );
    }

    // Step 3: Determine if we should use parallel processing
    let use_parallel = files.len() >= 4;

    // Step 4: Setup progress tracking (only if parallel and enough files)
    let total = files.len();
    let completed = Arc::new(AtomicUsize::new(0));

    let progress_handle = if use_parallel {
        let progress_counter = Arc::clone(&completed);
        Some(std::thread::spawn(move || {
            loop {
                let current = progress_counter.load(Ordering::Relaxed);
                if current >= total {
                    break;
                }
                eprint!("\rFormatting {}/{} files", current, total);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            eprintln!("\rFormatted {}/{} files   ", total, total);
        }))
    } else {
        None
    };

    // Step 5: Process files (parallel or sequential based on threshold)
    let diff_lock = Arc::new(std::sync::Mutex::new(()));

    let process_file_closure = |file: &PathBuf| -> Result<FileOutcome> {
        // A file named on the command line that is not there is a failed run,
        // for the same reason an unreadable one is — see the stdout path.
        if !file.exists() {
            eprintln!("Warning: File not found: {}", file.display());
            completed.fetch_add(1, Ordering::Relaxed);
            return Ok(FileOutcome::missing());
        }

        if !file.is_file() {
            eprintln!("Warning: Not a file: {}", file.display());
            completed.fetch_add(1, Ordering::Relaxed);
            return Ok(FileOutcome::missing());
        }

        // Look up config from pre-populated cache
        let parent = file.parent().unwrap_or_else(|| std::path::Path::new("."));
        let config = config_cache
            .get(&parent.to_path_buf())
            .expect("Config should be pre-populated");

        // For diff mode, serialize output to prevent interleaving
        let result = if diff_mode {
            let _guard = diff_lock.lock().unwrap();
            process_file(
                file,
                config,
                in_place,
                check_mode,
                diff_mode,
                verbose,
                line_ranges,
            )
        } else {
            process_file(
                file,
                config,
                in_place,
                check_mode,
                diff_mode,
                verbose,
                line_ranges,
            )
        };

        completed.fetch_add(1, Ordering::Relaxed);
        result
    };

    let results: Vec<Result<FileOutcome>> = if use_parallel {
        files.par_iter().map(process_file_closure).collect()
    } else {
        files.iter().map(process_file_closure).collect()
    };

    // Step 6: Wait for progress thread to complete
    if let Some(handle) = progress_handle {
        handle.join().unwrap();
    }

    // Step 7: Aggregate results
    let mut any_need_formatting = false;
    let mut any_content_changed = false;
    let mut any_error = false;
    let mut need_formatting_count = 0usize;
    for result in results {
        match result {
            Ok(outcome) => {
                any_need_formatting |= outcome.needs_formatting;
                any_content_changed |= outcome.content_changed;
                any_error |= outcome.missing;
                if outcome.needs_formatting {
                    need_formatting_count += 1;
                }
            }
            Err(e) => {
                // A file that could not be read or written is a failed run: a
                // report nobody reads is how an unformatted file reaches a
                // release
                eprintln!("Error processing file: {:#}", e);
                any_error = true;
            }
        }
    }

    // Print summary in check mode (but not in diff mode where diffs speak for themselves)
    // Counted from the outcomes, not from how many files were looked at: the
    // old line said "2 file(s) would be reformatted" for one that would and one
    // that was already formatted, and it counted a file that is missing or was
    // refused — neither of which is going to be reformatted at all.
    if check_mode && !diff_mode && any_need_formatting {
        eprintln!("{} file(s) would be reformatted", need_formatting_count);
    }

    // A file left alone because formatting would have changed it fails the run
    // in every mode, so CI sees it rather than only whoever reads stderr.
    if any_error || any_content_changed || ((check_mode || diff_mode) && any_need_formatting) {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

/// Process a single file
fn process_file(
    path: &std::path::Path,
    config: &FormatConfig,
    in_place: bool,
    check_mode: bool,
    diff_mode: bool,
    verbose: bool,
    line_ranges: Option<&[LineRange]>,
) -> Result<FileOutcome> {
    use std::fs;

    // In stdout mode (no flags set), don't process here - it's handled by process_files
    if !in_place && !check_mode && !diff_mode {
        return Ok(FileOutcome::needs_formatting(false));
    }

    let original = fs::read_to_string(path)?;
    let (formatted, warnings) = if let Some(ranges) = line_ranges {
        format_with_line_ranges(&original, config, ranges, Some(path), verbose)
    } else {
        format_text_with_diagnostics_and_path(&original, config, Some(path), verbose)
    };
    print_warnings(&warnings, &path.display().to_string());

    // Left unchanged on purpose; the caller fails the run regardless of mode
    if content_changed(&warnings) {
        return Ok(FileOutcome {
            needs_formatting: false,
            content_changed: true,
            missing: false,
        });
    }

    if diff_mode {
        if original != formatted {
            cmake_fmt::diff::print_colored_diff(&original, &formatted, &path.display().to_string());
            Ok(FileOutcome::needs_formatting(true))
        } else {
            Ok(FileOutcome::needs_formatting(false))
        }
    } else if check_mode {
        if original != formatted {
            eprintln!("Would reformat: {}", path.display());
            Ok(FileOutcome::needs_formatting(true))
        } else {
            Ok(FileOutcome::needs_formatting(false))
        }
    } else if in_place {
        // Only write if content changed
        if original != formatted {
            write_file_atomically(path, &formatted)?;
        }
        Ok(FileOutcome::needs_formatting(false))
    } else {
        // Default mode: stdout (handled by caller)
        Ok(FileOutcome::needs_formatting(false))
    }
}

/// Write file atomically using temp file + rename
fn write_file_atomically(path: &std::path::Path, content: &str) -> Result<()> {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let dir = path.parent().unwrap_or(std::path::Path::new("."));
    let mut temp = NamedTempFile::new_in(dir)?;
    temp.write_all(content.as_bytes())?;
    temp.flush()?;
    temp.persist(path)?;

    Ok(())
}
