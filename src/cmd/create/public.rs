use crate::utils::OrNone;
use crate::{
    cmd::create::CreateCommon,
    config::{Client, ClientType, Config},
    http::create_client,
    server::Server,
};
use anyhow::bail;
use openid::{Discovered, Options, StandardClaims};
use std::path::PathBuf;

/// Create a new confidential client
#[derive(Debug, clap::Parser)]
pub struct CreatePublic {
    #[command(flatten)]
    pub common: CreateCommon,

    #[arg(from_global)]
    pub config: Option<PathBuf>,

    /// The client ID
    #[arg(short = 'i', long)]
    pub client_id: String,

    /// Force using a specific port for the local server
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Open the link automatically
    #[arg(short, long)]
    pub open: bool,
}

impl CreatePublic {
    pub async fn run(self) -> anyhow::Result<()> {
        log::debug!("creating new client: {}", self.common.name);

        let mut config = Config::load(self.config.as_deref())?;

        if !self.common.ignore_existing && config.clients.contains_key(&self.common.name) {
            bail!(
                "A client named '{}' already exists. You need to delete it first or use --ignore-existing",
                self.common.name
            );
        }

        let server = Server::new(self.port).await?;
        let redirect = format!("http://localhost:{}", server.port);

        let client = create_client().await?;
        let client = openid::Client::<Discovered, StandardClaims>::discover_with_client(
            client,
            self.client_id.clone(),
            None,
            Some(redirect),
            self.common.issuer.clone(),
        )
        .await?;

        let options = Options {
            ..Default::default()
        };
        let open = client.auth_url(&options);

        println!(
            r#"

Open the following URL in your browser and perform the interactive login process (use --open to do this automatically):

    {open}

"#
        );

        if let Err(err) = open::that(open.to_string()) {
            log::warn!(
                "Failed to open URL in browser. You can still copy the link from the console. Error: {err}"
            );
        }

        let result = server.receive_token().await?;
        let token = client.request_token(&result.code).await?;

        log::info!("First token:");
        log::info!("       ID: {}", OrNone(&token.id_token));
        log::info!("   Access: {}", token.access_token);
        log::info!("  Refresh: {}", OrNone(&token.refresh_token));

        let client = Client {
            issuer_url: self.common.issuer,
            r#type: ClientType::Public {
                client_id: self.client_id,
            },
            state: Some(token.try_into()?),
        };

        config
            .clients
            .insert(self.common.name.clone(), client.clone());

        config.store(self.config.as_deref())?;

        Ok(())
    }
}
