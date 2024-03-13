mod create;
mod token;

use std::process::ExitCode;

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    Create(create::Create),
    Token(token::GetToken),
}

impl Command {
    pub async fn run(self) -> anyhow::Result<ExitCode> {
        match self {
            Self::Create(cmd) => cmd.run().await.map(|()| ExitCode::SUCCESS),
            Self::Token(cmd) => cmd.run().await.map(|()| ExitCode::SUCCESS),
        }
    }
}
