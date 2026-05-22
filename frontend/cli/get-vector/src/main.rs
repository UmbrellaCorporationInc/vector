//! Executable entrypoint for the `get-vector` operator CLI.

use get_vector::commands::update_mcp_vector::{UpdateOutcome, run};
use runtime_io::ProcessCommandExecutor;

#[allow(clippy::print_stdout, clippy::print_stderr)]
#[tokio::main]
async fn main() {
    let executor = ProcessCommandExecutor::default();
    match run(&executor).await {
        Ok(UpdateOutcome::Installed) => {
            println!("mcp-vector installed from git");
        }
        Ok(_) => {}
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
