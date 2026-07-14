mod completion;
mod create;
mod delete;
mod inspect;
mod list;
#[cfg(feature = "mcp")]
mod mcp;
mod token;

use std::process::ExitCode;

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Command {
    Create(create::Create),
    Delete(delete::Delete),
    Token(token::GetToken),
    List(list::List),
    Inspect(inspect::Inspect),
    Completion(completion::GetCompletion),
    #[cfg(feature = "mcp")]
    Mcp(mcp::Mcp),
}

impl Command {
    pub async fn run(self) -> anyhow::Result<ExitCode> {
        match self {
            Self::Create(cmd) => cmd.run().await,
            Self::Delete(cmd) => cmd.run().await,
            Self::Token(cmd) => cmd.run().await,
            Self::List(cmd) => cmd.run().await,
            Self::Inspect(cmd) => cmd.run().await,
            Self::Completion(cmd) => cmd.run().await,
            #[cfg(feature = "mcp")]
            Self::Mcp(cmd) => cmd.run().await,
        }
        .map(|()| ExitCode::SUCCESS)
    }
}
