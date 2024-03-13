use crate::config::Config;
use crate::oidc::get_token;
use anyhow::anyhow;
use std::path::PathBuf;

/// Get a valid token
#[derive(Debug, clap::Parser)]
#[command(rename_all_env = "SNAKE_CASE")]
pub struct GetToken {
    /// Name of the token to get
    #[arg(short, long, env)]
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
}

impl GetToken {
    pub async fn run(self) -> anyhow::Result<()> {
        let config = Config::load(self.config.as_deref())?;

        let config = config
            .by_name(&self.name)
            .ok_or_else(|| anyhow!("unknown client '{}'", self.name))?;

        let token = get_token(config).await?;

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

        if self.bearer {
            println!("Bearer {token}");
        } else {
            println!("{token}");
        }

        Ok(())
    }
}
