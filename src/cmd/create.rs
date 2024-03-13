use crate::config::{Client, ClientType, Config};
use crate::oidc::get_token;
use crate::utils::OrNone;
use anyhow::{bail, Context};
use std::path::PathBuf;
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
    #[arg(long)]
    pub ignore_existing: bool,

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
}

impl CreateType {
    pub async fn run(self) -> anyhow::Result<()> {
        match self {
            Self::Confidential(cmd) => cmd.run().await,
        }
    }
}

/// Create a new confidential client
#[derive(Debug, clap::Parser)]
pub struct CreateConfidential {
    #[command(flatten)]
    pub common: CreateCommon,

    #[arg(from_global)]
    pub config: Option<PathBuf>,

    /// The client ID
    #[arg(short = 'i', long)]
    pub client_id: String,

    /// The client secret
    #[arg(short = 's', long)]
    pub client_secret: String,
}

impl CreateConfidential {
    pub async fn run(self) -> anyhow::Result<()> {
        log::debug!("creating new client: {}", self.common.name);

        let mut config = Config::load(self.config.as_deref())?;

        let client = Client {
            issuer_url: self.common.issuer,
            r#type: ClientType::Confidential {
                client_id: self.client_id,
                client_secret: self.client_secret,
            },
        };

        let old = config
            .clients
            .insert(self.common.name.clone(), client.clone());

        if !self.common.ignore_existing && old.is_some() {
            bail!(
                "A client named '{}' already exists. You need to delete it first or use --ignore-existing",
                self.common.name
            );
        }

        if !self.common.skip_initial {
            let token = get_token(&client)
                .await
                .context("failed retrieving first token")?;

            log::info!("First token:");
            log::info!("       ID: {}", OrNone(token.id_token));
            log::info!("   Access: {}", token.access_token);
            log::info!("  Refresh: {}", OrNone(token.refresh_token));
        }

        config.store(self.config.as_deref())?;

        Ok(())
    }
}
