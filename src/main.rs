#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use log::LevelFilter;
use oidc_cli::cmd;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::{
    env, io,
    path::{Path, PathBuf},
    process::ExitCode,
};

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

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Command {
    Create(cmd::create::Create),
    Delete(cmd::delete::Delete),
    Token(cmd::token::GetToken),
    List(cmd::list::List),
    Inspect(cmd::inspect::Inspect),
    Completion(GetCompletion),
}

impl Command {
    async fn run(self) -> anyhow::Result<ExitCode> {
        match self {
            Self::Create(cmd) => cmd.run().await,
            Self::Delete(cmd) => cmd.run().await,
            Self::Token(cmd) => cmd.run().await,
            Self::List(cmd) => cmd.run().await,
            Self::Inspect(cmd) => cmd.run().await,
            Self::Completion(cmd) => cmd.run().await,
        }
        .map(|()| ExitCode::SUCCESS)
    }
}

/// Generate shell completion
#[derive(Debug, clap::Parser)]
#[command(rename_all_env = "SNAKE_CASE")]
struct GetCompletion {
    /// The shell to generate completions for
    #[arg(value_enum)]
    shell: Shell,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
#[value(rename_all = "lowercase")]
enum Shell {
    /// Bourne Again `SHell` (bash)
    Bash,
    /// Elvish shell
    Elvish,
    /// Friendly Interactive `SHell` (fish)
    Fish,
    /// `PowerShell`
    #[value(alias = "ps")]
    #[allow(clippy::enum_variant_names)]
    PowerShell,
    /// Z `SHell` (zsh)
    Zsh,
}

impl From<Shell> for clap_complete::Shell {
    fn from(value: Shell) -> Self {
        match value {
            Shell::Bash => Self::Bash,
            Shell::Elvish => Self::Elvish,
            Shell::Fish => Self::Fish,
            Shell::PowerShell => Self::PowerShell,
            Shell::Zsh => Self::Zsh,
        }
    }
}

impl GetCompletion {
    async fn run(self) -> anyhow::Result<()> {
        let mut cmd = Cli::command();
        let bin_name = env::args()
            .next()
            .and_then(|path| {
                Path::new(&path)
                    .file_stem()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| env!("CARGO_BIN_NAME").to_string());

        generate(
            clap_complete::Shell::from(self.shell),
            &mut cmd,
            &bin_name,
            &mut io::stdout(),
        );

        Ok(())
    }
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
