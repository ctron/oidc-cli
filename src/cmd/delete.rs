use crate::config::Config;
use std::path::PathBuf;

/// Delete a client
#[derive(Debug, clap::Parser)]
pub struct Delete {
    /// The name of the client to delete
    pub name: String,

    #[arg(from_global)]
    pub config: Option<PathBuf>,
}

impl Delete {
    pub async fn run(self) -> anyhow::Result<()> {
        log::debug!("deleting client: {}", self.name);

        Config::locked(self.config.as_deref(), async |config| {
            if config.clients.remove(&self.name).is_some() {
                log::info!("deleted client: {}", self.name);
            } else {
                log::info!("client did not exist: {}", self.name);
            }
            Ok(())
        })
        .await
    }
}
