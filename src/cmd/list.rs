use crate::{
    claims::{AccessTokenClaims, RefreshTokenClaims},
    config::{ClientType, Config},
};
use biscuit::{CompactJson, Empty, jws::Compact};
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement, Row, Table, presets};
use std::path::PathBuf;
use time::{OffsetDateTime, macros::format_description};

/// List configured clients
#[derive(Debug, clap::Parser)]
#[command(rename_all_env = "SNAKE_CASE")]
pub struct List {
    #[arg(from_global)]
    pub config: Option<PathBuf>,

    /// Show more details
    #[arg(short, long)]
    pub details: bool,
}

impl List {
    pub async fn run(self) -> anyhow::Result<()> {
        let config = Config::load(self.config.as_deref())?;

        let mut table = Table::new();
        table
            .load_preset(presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header([
                "Name",
                "Issuer",
                "Client",
                "Public",
                "Access Token",
                "Refresh Token",
            ]);

        for (name, client) in config.clients {
            let mut row = Row::new();
            row.add_cell(name.into());
            row.add_cell(client.issuer_url.to_string().into());

            match &client.r#type {
                ClientType::Public { client_id, .. } => {
                    row.add_cell(client_id.into());
                    row.add_cell(Cell::from("X").set_alignment(CellAlignment::Center));
                }
                ClientType::Confidential { client_id, .. } => {
                    row.add_cell(client_id.into());
                    row.add_cell("".into());
                }
            }

            if let Some(state) = &client.state {
                let access = self.token::<_, AccessTokenClaims>(&state.access_token, |token| {
                    self.expiration(
                        token
                            .exp
                            .and_then(|exp| OffsetDateTime::from_unix_timestamp(exp).ok()),
                    )
                });

                row.add_cell(access);

                let refresh = state
                    .refresh_token
                    .as_ref()
                    .map(|refresh| {
                        self.token::<_, RefreshTokenClaims>(refresh, |token| {
                            self.expiration(
                                token
                                    .exp
                                    .and_then(|exp| OffsetDateTime::from_unix_timestamp(exp).ok()),
                            )
                        })
                    })
                    .unwrap_or_else(|| Cell::new(""));

                row.add_cell(refresh);
            }

            table.add_row(row);
        }

        println!("{table}");

        Ok(())
    }

    /// Decode a token and call the function to extract cell information
    ///
    /// NOTE: The token is not being verified.
    fn token<F, T>(&self, token: &str, f: F) -> Cell
    where
        F: FnOnce(T) -> Cell,
        T: CompactJson,
    {
        let token = match Compact::<T, Empty>::new_encoded(token).unverified_payload() {
            Ok(token) => token,
            Err(err) => return Cell::new(err.to_string()).fg(Color::Red),
        };

        f(token)
    }

    fn expiration(&self, expires: Option<OffsetDateTime>) -> Cell {
        match expires {
            None => "∞".into(),
            Some(expires) => {
                let rem = expires - OffsetDateTime::now_utc();

                // truncate to seconds
                let format_rem = humantime::Duration::from(std::time::Duration::from_secs(
                    rem.unsigned_abs().as_secs(),
                ));

                let details = match self.details {
                    true => match expires.format(format_description!(
                        " ([year]-[month]-[day] [hour]:[minute]:[second]Z)"
                    )) {
                        Ok(format) => format,
                        Err(err) => return Cell::new(err.to_string()).fg(Color::Red),
                    },
                    false => "".into(),
                };

                if rem.is_positive() {
                    Cell::new(format!("valid: {format_rem}{details}")).fg(Color::Green)
                } else {
                    Cell::new(format!("expired: {format_rem}{details}")).fg(Color::DarkGrey)
                }
            }
        }
    }
}
