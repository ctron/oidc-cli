mod confidential;
mod public;

use crate::cmd::create::{confidential::CreateConfidential, public::CreatePublic};
use url::Url;

/// Create a new client
#[derive(Debug, clap::Parser)]
pub struct Create {
    #[command(subcommand)]
    pub r#type: CreateType,
}

impl Create {
    pub async fn run(self) -> anyhow::Result<()> {
        self.r#type.run().await
    }
}

#[derive(Debug, clap::Parser)]
#[command(rename_all_env = "SNAKE_CASE")]
pub struct CreateCommon {
    /// Name of the client, used to locally identify it
    pub name: String,

    /// Overwrite and existing client with the same name
    #[arg(short, long)]
    pub force: bool,

    /// Skip fetching the initial token
    #[arg(long)]
    pub skip_initial: bool,

    /// URL of the issuer
    #[arg(long)]
    pub issuer: Url,
}

#[derive(Debug, clap::Subcommand)]
pub enum CreateType {
    Confidential(CreateConfidential),
    Public(CreatePublic),
}

impl CreateType {
    pub async fn run(self) -> anyhow::Result<()> {
        match self {
            Self::Confidential(cmd) => cmd.run().await,
            Self::Public(cmd) => cmd.run().await,
        }
    }
}
