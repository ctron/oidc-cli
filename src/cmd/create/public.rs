use crate::{
    cmd::create::CreateCommon,
    config::{Client, ClientType, Config},
    http::{HttpOptions, create_client},
    server::{Bind, Server},
    utils::OrNone,
};
use anyhow::bail;
use openid::{Discovered, Options, StandardClaims};
use std::path::PathBuf;

/// Create a new public client
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

    /// Choose how to bind the local server
    #[arg(short, long, env = "BIND_MODE", value_enum, default_value_t = Bind::Prefer6)]
    pub bind: Bind,

    /// Use IPv4 only binding (equivalent to --bind only4)
    #[arg(short = '4', conflicts_with_all = ["bind", "only6"])]
    pub only4: bool,

    /// Use IPv6 only binding (equivalent to --bind only6)
    #[arg(short = '6', conflicts_with = "bind")]
    pub only6: bool,

    #[command(flatten)]
    pub http: HttpOptions,
}

impl CreatePublic {
    pub async fn run(self) -> anyhow::Result<()> {
        log::debug!("creating new client: {}", self.common.name);

        let mut config = Config::load(self.config.as_deref())?;

        if !self.common.force && config.clients.contains_key(&self.common.name) {
            bail!(
                "A client named '{}' already exists. You need to delete it first or use --force",
                self.common.name
            );
        }

        let server = Server::new(self.bind_mode(), self.port).await?;
        let redirect = format!("http://localhost:{}", server.port);

        let client = create_client(&self.http).await?;
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

    fn bind_mode(&self) -> Bind {
        if self.only4 {
            Bind::Only4
        } else if self.only6 {
            Bind::Only6
        } else {
            self.bind
        }
    }
}
