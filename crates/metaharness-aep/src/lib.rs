//! Governed AEP execution hosted by Metaharness.
macro_rules! outln {
    ($($arg:tt)*) => { aep_cli::write_out(&format!($($arg)*), true) };
}
pub mod drive;
pub mod eval;

/// Execute a governed AEP run.
#[must_use]
pub fn execute(command: drive::DriveCommand) -> std::process::ExitCode {
    match drive::run(command) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error:#}");
            std::process::ExitCode::from(1)
        }
    }
}
