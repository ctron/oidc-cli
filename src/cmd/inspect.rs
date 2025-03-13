use crate::utils::inspect::inspect;
use tokio::io::{AsyncBufReadExt, BufReader, stdin};

/// Inspect tokens
#[derive(Debug, clap::Parser)]
pub struct Inspect {
    /// The tokens to inspect, if one is present it will read from stdin.
    pub token: Vec<String>,
}

impl Inspect {
    pub async fn run(self) -> anyhow::Result<()> {
        let mut tokens = self.token;

        let mut lines = BufReader::new(stdin()).lines();
        while let Some(line) = lines.next_line().await? {
            tokens.push(line);
        }

        for (n, token) in tokens.into_iter().enumerate() {
            log::debug!("Inspecting token: {token}");
            println!("Token #{n}:");
            inspect(token)?;
        }

        Ok(())
    }
}
