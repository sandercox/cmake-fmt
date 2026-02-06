mod cli;
mod config;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::run() {
        Ok(code) => code,
        Err(e) => {
            // Handle BrokenPipe gracefully
            if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                if io_err.kind() == std::io::ErrorKind::BrokenPipe {
                    return ExitCode::SUCCESS;
                }
            }

            eprintln!("Error: {:#}", e);
            ExitCode::FAILURE
        }
    }
}
