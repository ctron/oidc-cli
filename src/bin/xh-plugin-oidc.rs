#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use std::{
    io::{stdin, stdout},
    process::ExitCode,
};

#[tokio::main]
async fn main() -> ExitCode {
    match oidc_cli::plugin::run(stdin().lock(), stdout().lock()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            for (n, cause) in err.chain().enumerate().skip(1) {
                eprintln!("  {n}: {cause}");
            }
            ExitCode::FAILURE
        }
    }
}
