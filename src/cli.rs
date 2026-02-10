use anyhow::Result;
use clap::Parser;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use cmake_fmt::formatter::{format_text_with_diagnostics, FormatConfig, SuppressionWarning};

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
}

/// Print suppression warnings to stderr
fn print_warnings(warnings: &[SuppressionWarning], file_label: &str) {
    for warning in warnings {
        eprintln!("{}: {}", file_label, warning);
    }
}

/// Run the CLI application
pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    // Handle interactive mode first (if --interactive flag is set)
    if cli.interactive {
        // TTY guard (INT-06): Check stdin and stderr are both terminals
        if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
            eprintln!("error: interactive mode requires a terminal (TTY)");
            return Ok(ExitCode::FAILURE);
        }

        // Determine if stdin input is specified
        let is_stdin = cli.files.is_empty() || (cli.files.len() == 1 && cli.files[0] == PathBuf::from("-"));

        // Validate exactly one file is provided (no stdin, no multi-file)
        if cli.files.is_empty() || is_stdin {
            eprintln!("error: interactive mode requires a file argument");
            return Ok(ExitCode::FAILURE);
        }
        if cli.files.len() > 1 {
            eprintln!("error: interactive mode supports one file at a time");
            return Ok(ExitCode::FAILURE);
        }

        // Resolve config for the file
        let config = crate::config::resolve_config(Some(&cli.files[0]), cli.style.as_deref());

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
        // For stdin, resolve config from current directory
        let config = crate::config::resolve_config(None, cli.style.as_deref());
        process_stdin(&config, check_mode, diff_mode)
    } else {
        // Expand glob patterns
        let expanded = expand_files(&cli.files)?;

        // Handle case where glob patterns match no files
        if expanded.is_empty() {
            eprintln!("No files found");
            return Ok(ExitCode::SUCCESS);
        }

        process_files(&expanded, cli.style.as_deref(), cli.in_place, check_mode, diff_mode)
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
            if let Some(diff_output) = cmake_fmt::diff::generate_diff(&input, &formatted, "stdin") {
                cmake_fmt::diff::print_colored_diff(&diff_output);
            }
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
    in_place: bool,
    check_mode: bool,
    diff_mode: bool,
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
            crate::config::resolve_config(Some(file), style_override)
        });

        match process_file(file, config, in_place, check_mode, diff_mode) {
            Ok(needs_formatting) => {
                if needs_formatting {
                    any_need_formatting = true;
                }

                // If default mode (stdout), write the formatted content
                if let Some(ref mut handle) = stdout_handle {
                    let content = std::fs::read_to_string(file)?;
                    let (formatted, warnings) = format_text_with_diagnostics(&content, config);
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
) -> Result<bool> {
    use std::fs;

    // In stdout mode (no flags set), don't process here - it's handled by process_files
    if !in_place && !check_mode && !diff_mode {
        return Ok(false);
    }

    let original = fs::read_to_string(path)?;
    let (formatted, warnings) = format_text_with_diagnostics(&original, config);
    print_warnings(&warnings, &path.display().to_string());

    if diff_mode {
        if original != formatted {
            if let Some(diff_output) = cmake_fmt::diff::generate_diff(&original, &formatted, &path.display().to_string()) {
                cmake_fmt::diff::print_colored_diff(&diff_output);
            }
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
