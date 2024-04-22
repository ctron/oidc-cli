use crate::config::{ClientType, Config};
use comfy_table::{presets, Cell, CellAlignment, Color, ContentArrangement, Row, Table};
use std::path::PathBuf;
use time::macros::format_description;
use time::OffsetDateTime;

#[derive(Debug, clap::Parser)]
#[command(rename_all_env = "SNAKE_CASE")]
pub struct List {
    #[arg(from_global)]
    pub config: Option<PathBuf>,
}

impl List {
    pub async fn run(self) -> anyhow::Result<()> {
        let config = Config::load(self.config.as_deref())?;

        let mut table = Table::new();
        table
            .load_preset(presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(["Name", "Issuer", "Client", "Public", "Access Token"]);

        for (name, client) in config.clients {
            let mut row = Row::new();
            row.add_cell(name.into());
            row.add_cell(client.issuer_url.to_string().into());

            match &client.r#type {
                ClientType::Public { client_id } => {
                    row.add_cell(client_id.into());
                    row.add_cell(Cell::from("X").set_alignment(CellAlignment::Center));
                }
                ClientType::Confidential { client_id, .. } => {
                    row.add_cell(client_id.into());
                    row.add_cell("".into());
                }
            }

            if let Some(state) = &client.state {
                match state.expires {
                    None => {
                        row.add_cell("∞".into());
                    }
                    Some(expires) => {
                        let rem = expires - OffsetDateTime::now_utc();

                        // truncate to seconds
                        let format_rem = humantime::Duration::from(std::time::Duration::from_secs(
                            rem.unsigned_abs().as_secs(),
                        ));
                        let expires = expires.format(format_description!(
                            "[year]-[month]-[day] [hour]:[minute]:[second]Z"
                        ))?;

                        let (prefix, color) = if rem.is_positive() {
                            ("valid", Color::Green)
                        } else {
                            ("expired", Color::Grey)
                        };

                        let cell =
                            Cell::new(format!("{prefix}: {format_rem} ({expires})")).fg(color);

                        row.add_cell(cell);
                    }
                }
            }

            table.add_row(row);
        }

        println!("{table}");

        Ok(())
    }
}
