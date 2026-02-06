use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

use cmake_formatter::formatter::{format_text, FormatConfig};

/// Format CMake files
#[derive(Parser)]
#[command(name = "cmake-formatter", version, about = "Format CMake files")]
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

    /// Override config inline (e.g., "indent_width=4,max_line_length=100")
    #[arg(long)]
    pub style: Option<String>,
}

/// Run the CLI application
pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    // Determine if we're in check mode
    let check_mode = cli.check || cli.dry_run;

    // Create default config (config file loading comes in Plan 02)
    let config = FormatConfig::default();

    // Determine if we're processing stdin or files
    let is_stdin = cli.files.is_empty() || (cli.files.len() == 1 && cli.files[0] == PathBuf::from("-"));

    if is_stdin {
        process_stdin(&config, check_mode)
    } else {
        process_files(&cli.files, &config, cli.in_place, check_mode)
    }
}

/// Process stdin to stdout
fn process_stdin(config: &FormatConfig, check_mode: bool) -> Result<ExitCode> {
    use std::io::{stdin, stdout, Read, Write};

    let mut input = String::new();
    stdin().lock().read_to_string(&mut input)?;

    let formatted = format_text(&input, config);

    if check_mode {
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
    config: &FormatConfig,
    in_place: bool,
    check_mode: bool,
) -> Result<ExitCode> {
    use std::io::{stdout, Write};

    let mut any_need_formatting = false;
    let mut stdout_handle = if !in_place && !check_mode {
        Some(stdout().lock())
    } else {
        None
    };

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

        match process_file(file, config, in_place, check_mode) {
            Ok(needs_formatting) => {
                if needs_formatting {
                    any_need_formatting = true;
                }

                // If default mode (stdout), write the formatted content
                if let Some(ref mut handle) = stdout_handle {
                    let content = std::fs::read_to_string(file)?;
                    let formatted = format_text(&content, config);
                    write!(handle, "{}", formatted)?;
                }
            }
            Err(e) => {
                eprintln!("Error processing {}: {:#}", file.display(), e);
            }
        }
    }

    // Print summary in check mode
    if check_mode && any_need_formatting {
        let count = files.len();
        eprintln!("{} file(s) would be reformatted", count);
    }

    if check_mode && any_need_formatting {
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
) -> Result<bool> {
    use std::fs;

    let original = fs::read_to_string(path)?;
    let formatted = format_text(&original, config);

    if check_mode {
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
