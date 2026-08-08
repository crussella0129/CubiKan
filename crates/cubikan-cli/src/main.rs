use std::{io, process::ExitCode};

fn main() -> ExitCode {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    ExitCode::from(cubikan_cli::run_process(
        stdin.lock(),
        stdout.lock(),
        stderr.lock(),
    ))
}
