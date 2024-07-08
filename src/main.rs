#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

mod claims;
mod cmd;
mod config;
mod http;
mod oidc;
mod server;
mod utils;

use crate::cmd::Command;
use clap::Parser;
use log::LevelFilter;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::{path::PathBuf, process::ExitCode};

#[derive(Debug, clap::Parser)]
#[command(about, author, version, rename_all_env = "SNAKE_CASE")]
struct Cli {
    /// Be quiet, conflicts with 'verbose'
    #[arg(
        short,
        long,
        env = "OIDC_QUIET",
        global = true,
        conflicts_with = "verbose"
    )]
    pub quiet: bool,

    /// Be more verbose, conflicts with 'quiet'
    #[arg(short, long, env = "OIDC_VERBOSE", global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Override config file
    #[arg(short, long, env = "OIDC_CONFIG", global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

fn init_log(cli: &Cli) -> anyhow::Result<()> {
    let level = match (cli.quiet, cli.verbose) {
        (true, _) => LevelFilter::Error,
        (false, 0) => LevelFilter::Warn,
        (false, 1) => LevelFilter::Info,
        (false, 2) => LevelFilter::Debug,
        (false, _) => LevelFilter::Trace,
    };

    TermLogger::init(
        level,
        Config::default(),
        // all logging goes to stderr. stdout is reserved for actual output of the command.
        TerminalMode::Stderr,
        ColorChoice::Auto,
    )?;

    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    run(cli).await.unwrap_or_else(|err| {
        log::error!("{err}");
        for (n, cause) in err.chain().enumerate().skip(1) {
            log::error!("  {n}: {cause}");
        }
        ExitCode::FAILURE
    })
}

async fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    init_log(&cli)?;
    cli.command.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
