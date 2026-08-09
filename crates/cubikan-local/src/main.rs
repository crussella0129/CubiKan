use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let exit = cubikan_local::run_process(
        std::env::args_os(),
        stdin.lock(),
        stdout.lock(),
        stderr.lock(),
    );
    std::process::exit(exit.into());
}
