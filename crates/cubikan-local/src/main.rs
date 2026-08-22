use std::io;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let exit = cubikan_local::run_process(
        std::env::args_os(),
        stdin.lock(),
        stdout.lock(),
        stderr.lock(),
    )
    .await;
    std::process::exit(exit.into());
}
