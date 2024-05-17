mod create;
mod delete;
mod inspect;
mod list;
mod token;

use std::process::ExitCode;

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    Create(create::Create),
    Delete(delete::Delete),
    Token(token::GetToken),
    List(list::List),
    Inspect(inspect::Inspect),
}

impl Command {
    pub async fn run(self) -> anyhow::Result<ExitCode> {
        match self {
            Self::Create(cmd) => cmd.run().await,
            Self::Delete(cmd) => cmd.run().await,
            Self::Token(cmd) => cmd.run().await,
            Self::List(cmd) => cmd.run().await,
            Self::Inspect(cmd) => cmd.run().await,
        }
        .map(|()| ExitCode::SUCCESS)
    }
}
