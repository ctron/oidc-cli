use biscuit::{Base64Url, CompactPart};
use colored_json::to_colored_json_auto;
use serde_json::Value;
use std::io::{stdout, Write};
use tokio::io::{stdin, AsyncBufReadExt, BufReader};

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
            Self::inspect(token)?;
        }

        Ok(())
    }

    fn inspect(token: String) -> anyhow::Result<()> {
        let token = biscuit::Compact::decode(&token);

        for (n, part) in token.parts.into_iter().enumerate() {
            print!("  Part #{n}:");
            if let Err(err) = Self::inspect_part(part) {
                println!("Unable to decode: {err}");
            }
        }

        Ok(())
    }

    fn inspect_part(part: Base64Url) -> anyhow::Result<()> {
        let data = part.to_bytes()?;
        match serde_json::from_slice::<Value>(&data) {
            Err(err) => {
                println!(" Invalid JSON: {err}");
                stdout().lock().write_all(&data)?;
            }
            Ok(value) => {
                println!();
                println!("{}", to_colored_json_auto(&value)?);
            }
        }
        Ok(())
    }
}
