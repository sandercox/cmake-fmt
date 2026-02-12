use anyhow::Result;
use clap::Parser;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use cmake_fmt::formatter::{format_text_with_diagnostics, format_text_with_diagnostics_and_path, FormatConfig, SuppressionWarning};

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
    #[arg(long, conflicts_with_all = ["in_place", "check", "dry_run", "diff"])]
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
}

/// Print suppression warnings to stderr
fn print_warnings(warnings: &[SuppressionWarning], file_label: &str) {
    for warning in warnings {
        eprintln!("{}: {}", file_label, warning);
    }
}

/// Print all available style settings
fn print_style_help() {
    println!("Available style settings for --style and config files (.cmake-fmt.toml / .cmake-fmt.yaml):");
    println!();
    println!("  {:<25} {:<15} {:<15} {}", "Setting", "Type", "Default", "Values");
    println!("  {:<25} {:<15} {:<15} {}", "-------", "----", "-------", "------");
    println!("  {:<25} {:<15} {:<15} {}", "indent_width", "integer", "4", "Number of spaces per indent level");
    println!("  {:<25} {:<15} {:<15} {}", "max_line_length", "integer", "80", "Maximum line length before wrapping");
    println!("  {:<25} {:<15} {:<15} {}", "use_tabs", "boolean", "true", "true, false");
    println!("  {:<25} {:<15} {:<15} {}", "command_case", "enum", "lowercase", "lowercase, uppercase, leave");
    println!("  {:<25} {:<15} {:<15} {}", "user_command_case", "enum", "infer", "lowercase, uppercase, leave, infer");
    println!("  {:<25} {:<15} {:<15} {}", "max_blank_lines", "integer", "1", "Maximum consecutive blank lines allowed");
    println!("  {:<25} {:<15} {:<15} {}", "line_ending", "enum", "auto", "auto, lf, crlf");
    println!("  {:<25} {:<15} {:<15} {}", "closing_style", "enum", "remove", "leave, remove, force");
    println!("  {:<25} {:<15} {:<15} {}", "force_break_keywords", "boolean", "false", "true, false");
    println!("  {:<25} {:<15} {:<15} {}", "source_grouping", "enum", "none", "none, headers_first, sources_first");
    println!();
    println!("CLI usage:  cmake-fmt --style \"indent_width=4,max_line_length=120\" <file>");
    println!();
    println!("Config file only (not available via --style):");
    println!("  command_grammars        map            {{}}              Custom command grammar definitions");
    println!();
    println!("Example .cmake-fmt.toml:");
    println!("  indent_width = 2");
    println!("  use_tabs = false");
    println!("  command_case = \"lowercase\"");
}

/// Print grammar file format documentation
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
    println!("        COMMAND: CommandLine");
    println!("        PROPERTIES: PairValue");
    println!();
    println!("Keyword types:");
    println!();
    println!("  {:<15} {}", "Type", "Description");
    println!("  {:<15} {}", "----", "-----------");
    println!("  {:<15} {}", "Flag", "No value consumed (e.g., REQUIRED, QUIET)");
    println!("  {:<15} {}", "SingleValue", "Consumes exactly one value (e.g., VERSION 1.0, DESTINATION /usr/lib)");
    println!("  {:<15} {}", "MultiValue", "Consumes all values until next keyword (e.g., SOURCES a.cpp b.cpp c.cpp)");
    println!("  {:<15} {}", "CommandLine", "Bin-packs values to fill lines (e.g., COMMAND echo hello world)");
    println!("  {:<15} {}", "PairValue", "Consumes alternating key/value pairs (e.g., PROPERTIES CXX_STANDARD 17)");
    println!();
    println!("Multi-mode commands:");
    println!("  For commands like install() that have different keyword sets per sub-command,");
    println!("  add a 'mode' field to each grammar entry:");
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
    use cmake_fmt::formatter::grammar::{builtin_grammars, config_grammars_to_map, get_project_user_commands, get_project_user_grammars};
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
            eprintln!("warning: no grammar found for custom command '{}' (define in config or grammar file)", cmd);
        }
        eprintln!("{} custom command(s) have no grammar definition", grammarless_commands.len());
    }

    let format = detect_grammar_format(path);
    let content = export_command_grammars(&merged_grammars, &format, Some(&all_user_commands));

    fs::write(path, content)
        .with_context(|| format!("Failed to write grammar file: {}", path.display()))?;

    if merged_grammars.is_empty() {
        eprintln!("No custom grammars detected in input files");
    } else {
        eprintln!("Exported {} custom grammars to {}", merged_grammars.len(), path.display());
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

    // Handle interactive mode first (if --interactive flag is set)
    if cli.interactive {
        // Determine if stdin input is specified
        let is_stdin = cli.files.is_empty() || (cli.files.len() == 1 && cli.files[0] == PathBuf::from("-"));

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
        let config = crate::config::resolve_config(Some(&cli.files[0]), cli.style.as_deref(), &cli.grammar_files);

        // Run interactive mode
        match cmake_fmt::interactive::run_interactive(&cli.files[0], &config) {
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
    let diff_mode = cli.diff;

    // Determine if we're processing stdin or files
    let is_stdin = cli.files.is_empty() || (cli.files.len() == 1 && cli.files[0] == PathBuf::from("-"));

    if is_stdin {
        // Handle --export-grammar with stdin (error)
        if cli.export_grammar.is_some() {
            eprintln!("error: --export-grammar requires input files to scan for custom grammars");
            return Ok(ExitCode::FAILURE);
        }

        // For stdin, resolve config from current directory
        let config = crate::config::resolve_config(None, cli.style.as_deref(), &cli.grammar_files);
        process_stdin(&config, check_mode, diff_mode)
    } else {
        // Expand glob patterns
        let expanded = expand_files(&cli.files)?;

        // Handle case where glob patterns match no files
        if expanded.is_empty() {
            eprintln!("No files found");
            return Ok(ExitCode::SUCCESS);
        }

        // Handle --export-grammar (exports custom grammars from input files)
        if let Some(ref export_path) = cli.export_grammar {
            return export_custom_grammar_to_file(
                export_path,
                &expanded,
                cli.style.as_deref(),
                &cli.grammar_files,
                cli.verbose,
            );
        }

        process_files(&expanded, cli.style.as_deref(), &cli.grammar_files, cli.in_place, check_mode, diff_mode, cli.verbose)
    }
}

/// Expand glob patterns in file paths
fn expand_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut expanded = Vec::new();

    for path in paths {
        let path_str = path.to_string_lossy();

        // Check if the path contains glob characters
        if path_str.contains('*') || path_str.contains('?') || path_str.contains('[') {
            // Expand glob pattern
            match glob::glob(&path_str) {
                Ok(entries) => {
                    let mut found_any = false;
                    for entry in entries {
                        match entry {
                            Ok(p) => {
                                if p.is_file() {
                                    expanded.push(p);
                                    found_any = true;
                                }
                            }
                            Err(e) => {
                                eprintln!("Warning: Error reading glob entry: {:#}", e);
                            }
                        }
                    }
                    if !found_any {
                        eprintln!("Warning: No files matched pattern: {}", path_str);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Invalid glob pattern '{}': {:#}", path_str, e);
                }
            }
        } else {
            // Not a glob pattern, use as-is
            expanded.push(path.clone());
        }
    }

    // Sort for deterministic order
    expanded.sort();

    Ok(expanded)
}

/// Process stdin to stdout
fn process_stdin(config: &FormatConfig, check_mode: bool, diff_mode: bool) -> Result<ExitCode> {
    use std::io::{stdin, stdout, Read, Write};

    let mut input = String::new();
    stdin().lock().read_to_string(&mut input)?;

    let (formatted, warnings) = format_text_with_diagnostics(&input, config);
    print_warnings(&warnings, "stdin");

    if diff_mode {
        if input != formatted {
            cmake_fmt::diff::print_colored_diff(&input, &formatted, "stdin");
            Ok(ExitCode::from(1))
        } else {
            Ok(ExitCode::SUCCESS)
        }
    } else if check_mode {
        if input != formatted {
            eprintln!("Would reformat: stdin");
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
fn process_files(
    files: &[PathBuf],
    style_override: Option<&str>,
    grammar_files: &[PathBuf],
    in_place: bool,
    check_mode: bool,
    diff_mode: bool,
    verbose: bool,
) -> Result<ExitCode> {
    use std::io::{stdout, Write};
    use std::collections::HashMap;

    let mut any_need_formatting = false;
    let mut stdout_handle = if !in_place && !check_mode && !diff_mode {
        Some(stdout().lock())
    } else {
        None
    };

    // Cache configs by parent directory to avoid redundant file system walks
    let mut config_cache: HashMap<PathBuf, FormatConfig> = HashMap::new();

    for file in files {
        // Validate file exists
        if !file.exists() {
            eprintln!("Warning: File not found: {}", file.display());
            continue;
        }

        if !file.is_file() {
            eprintln!("Warning: Not a file: {}", file.display());
            continue;
        }

        // Resolve config for this file (using cache for efficiency)
        let parent = file.parent().unwrap_or_else(|| std::path::Path::new("."));
        let config = config_cache.entry(parent.to_path_buf()).or_insert_with(|| {
            crate::config::resolve_config(Some(file), style_override, grammar_files)
        });

        match process_file(file, config, in_place, check_mode, diff_mode, verbose) {
            Ok(needs_formatting) => {
                if needs_formatting {
                    any_need_formatting = true;
                }

                // If default mode (stdout), write the formatted content
                if let Some(ref mut handle) = stdout_handle {
                    let content = std::fs::read_to_string(file)?;
                    let (formatted, warnings) = format_text_with_diagnostics_and_path(&content, config, Some(file.as_path()), verbose);
                    print_warnings(&warnings, &file.display().to_string());
                    write!(handle, "{}", formatted)?;
                }
            }
            Err(e) => {
                eprintln!("Error processing {}: {:#}", file.display(), e);
            }
        }
    }

    // Print summary in check mode (but not in diff mode where diffs speak for themselves)
    if check_mode && !diff_mode && any_need_formatting {
        let count = files.len();
        eprintln!("{} file(s) would be reformatted", count);
    }

    if (check_mode || diff_mode) && any_need_formatting {
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
) -> Result<bool> {
    use std::fs;

    // In stdout mode (no flags set), don't process here - it's handled by process_files
    if !in_place && !check_mode && !diff_mode {
        return Ok(false);
    }

    let original = fs::read_to_string(path)?;
    let (formatted, warnings) = format_text_with_diagnostics_and_path(&original, config, Some(path), verbose);
    print_warnings(&warnings, &path.display().to_string());

    if diff_mode {
        if original != formatted {
            cmake_fmt::diff::print_colored_diff(&original, &formatted, &path.display().to_string());
            Ok(true)
        } else {
            Ok(false)
        }
    } else if check_mode {
        if original != formatted {
            eprintln!("Would reformat: {}", path.display());
            Ok(true)
        } else {
            Ok(false)
        }
    } else if in_place {
        // Only write if content changed
        if original != formatted {
            write_file_atomically(path, &formatted)?;
        }
        Ok(false)
    } else {
        // Default mode: stdout (handled by caller)
        Ok(false)
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
