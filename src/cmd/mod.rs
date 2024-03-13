mod create;
mod delete;
mod token;

use std::process::ExitCode;

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    Create(create::Create),
    Delete(delete::Delete),
    Token(token::GetToken),
}

impl Command {
    pub async fn run(self) -> anyhow::Result<ExitCode> {
        match self {
            Self::Create(cmd) => cmd.run().await.map(|()| ExitCode::SUCCESS),
            Self::Delete(cmd) => cmd.run().await.map(|()| ExitCode::SUCCESS),
            Self::Token(cmd) => cmd.run().await.map(|()| ExitCode::SUCCESS),
        }
    }
}
