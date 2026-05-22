//! Executable entrypoint for the `get-vector` operator CLI.

use get_vector::commands::update_mcp_vector::{UpdateOutcome, run};
use runtime_io::ProcessCommandExecutor;

#[allow(clippy::print_stdout, clippy::print_stderr)]
#[tokio::main]
async fn main() {
    let executor = ProcessCommandExecutor::default();
    eprintln!("Updating mcp-vector from git...");
    match run(
        &executor,
        |b| print!("{}", String::from_utf8_lossy(b)),
        |b| eprint!("{}", String::from_utf8_lossy(b)),
    )
    .await
    {
        Ok(UpdateOutcome::Installed) => {
            println!("mcp-vector installed successfully.");
        }
        Ok(_) => {}
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
