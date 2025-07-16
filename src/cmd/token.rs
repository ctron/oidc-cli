use crate::{
    config::Config,
    http::HttpOptions,
    oidc::{TokenResult, fetch_token, get_token},
    utils::inspect::inspect,
};
use anyhow::anyhow;
use std::path::PathBuf;

/// Get a valid token
#[derive(Debug, clap::Parser)]
#[command(rename_all_env = "SNAKE_CASE")]
pub struct GetToken {
    /// Name of the token to get
    pub name: String,

    #[arg(from_global)]
    pub config: Option<PathBuf>,

    /// Get the access token, conflicts with 'id' and 'refresh', the default.
    #[arg(id = "access", short, long, conflicts_with_all = ["id", "refresh"])]
    pub _access: bool,

    /// Get the ID token, conflicts with 'access' and 'refresh'
    #[arg(short, long, conflicts_with_all = ["access", "refresh"])]
    pub id: bool,

    /// Get the refresh token, conflicts with 'access' and 'id'
    #[arg(short, long, conflicts_with_all = ["access", "id"])]
    pub refresh: bool,

    /// Prefix with "Bearer ", for using it as a `Authorization` header value
    #[arg(short, long)]
    pub bearer: bool,

    /// Suitable for using directly with HTTPie as header
    #[arg(short = 'H', long, conflicts_with = "bearer")]
    pub header: bool,

    /// Inspect the token
    #[arg(short = 'I', long, conflicts_with = "bearer")]
    pub inspect: bool,

    /// Force a new token
    #[arg(short, long)]
    pub force: bool,

    #[command(flatten)]
    pub http: HttpOptions,
}

impl GetToken {
    pub async fn run(self) -> anyhow::Result<()> {
        let mut config = Config::load(self.config.as_deref())?;

        let client = config
            .by_name_mut(&self.name)
            .ok_or_else(|| anyhow!("unknown client '{}'", self.name))?;

        let token = match self.force {
            true => fetch_token(client, &self.http).await?,
            false => get_token(client, &self.http).await?,
        };

        let token = match token {
            TokenResult::Refreshed(token) => {
                log::info!("Got a refreshed token. Storing new state.");
                // update client state
                client.state = Some(token.clone());

                config.store(self.config.as_deref())?;

                token
            }
            TokenResult::Existing(token) => token,
        };

        let token = if self.id {
            token
                .id_token
                .ok_or_else(|| anyhow!("ID token not available"))?
        } else if self.refresh {
            token
                .refresh_token
                .ok_or_else(|| anyhow!("refresh token not available"))?
        } else {
            // access is the default
            token.access_token
        };

        match (self.bearer, self.header, self.inspect) {
            (true, _, _) => {
                println!("Bearer {token}");
            }
            (_, true, _) => {
                println!("Authorization:Bearer {token}");
            }
            (_, _, true) => {
                inspect(token)?;
            }
            _ => {
                println!("{token}");
            }
        }

        Ok(())
    }
}
