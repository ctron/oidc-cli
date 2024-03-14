use crate::oidc::TokenResult;
use crate::{
    cmd::create::CreateCommon,
    config::{Client, ClientType, Config},
    oidc::get_token,
    utils::OrNone,
};
use anyhow::{bail, Context};
use std::path::PathBuf;

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

        if !self.common.ignore_existing && config.clients.contains_key(&self.common.name) {
            bail!(
                "A client named '{}' already exists. You need to delete it first or use --ignore-existing",
                self.common.name
            );
        }

        let mut client = Client {
            issuer_url: self.common.issuer,
            r#type: ClientType::Confidential {
                client_id: self.client_id,
                client_secret: self.client_secret,
            },
            state: None,
        };

        if !self.common.skip_initial {
            let token = get_token(&client)
                .await
                .context("failed retrieving first token")?;

            let token = match token {
                TokenResult::Refreshed(token) | TokenResult::Existing(token) => token,
            };

            log::info!("First token:");
            log::info!("       ID: {}", OrNone(&token.id_token));
            log::info!("   Access: {}", token.access_token);
            log::info!("  Refresh: {}", OrNone(&token.refresh_token));

            client.state = Some(token);
        }

        config
            .clients
            .insert(self.common.name.clone(), client.clone());

        config.store(self.config.as_deref())?;

        Ok(())
    }
}
