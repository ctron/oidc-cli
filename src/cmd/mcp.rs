use crate::{
    config::Config,
    http::HttpOptions,
    oidc::{TokenResult, get_token},
};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::io::stdio,
};
use std::path::PathBuf;

/// Start an MCP (Model Context Protocol) server on stdio
#[derive(Debug, clap::Parser)]
#[command(rename_all_env = "SNAKE_CASE")]
pub struct Mcp {
    #[arg(from_global)]
    pub config: Option<PathBuf>,

    #[command(flatten)]
    pub http: HttpOptions,
}

#[derive(Clone)]
struct OidcMcpServer {
    config_path: Option<PathBuf>,
    http: HttpOptions,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetTokenParams {
    /// Name of the configured OIDC client
    name: String,
    /// Type of token to retrieve: "access" (default), "id", or "refresh"
    #[serde(default = "default_token_type")]
    token_type: String,
}

fn default_token_type() -> String {
    "access".to_string()
}

#[tool_router]
impl OidcMcpServer {
    /// List all configured OIDC client names with their issuer URLs and token expiry status.
    #[tool(description = "List all configured OIDC clients")]
    async fn list_clients(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        let config = Config::load(self.config_path.as_deref()).map_err(|e| {
            rmcp::ErrorData::internal_error(format!("failed to load config: {e}"), None)
        })?;

        let mut lines = Vec::new();
        for (name, client) in &config.clients {
            let issuer = client.issuer_url.as_str();
            let status = match &client.state {
                Some(state) => match state.expires {
                    Some(exp) if exp > time::OffsetDateTime::now_utc() => "valid".to_string(),
                    Some(exp) => format!("expired ({})", exp),
                    None => "unknown expiry".to_string(),
                },
                None => "no token".to_string(),
            };
            lines.push(format!("{name}: issuer={issuer}, status={status}"));
        }

        if lines.is_empty() {
            return Ok(CallToolResult::success(vec![ContentBlock::text(
                "No clients configured. Use `oidc create` to add one.",
            )]));
        }

        Ok(CallToolResult::success(vec![ContentBlock::text(
            lines.join("\n"),
        )]))
    }

    /// Retrieve a token for a configured OIDC client, refreshing if expired.
    #[tool(description = "Get an OIDC token for a configured client")]
    async fn get_token(
        &self,
        Parameters(params): Parameters<GetTokenParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let http = self.http.clone();
        let token_type = params.token_type.clone();

        let token_value = Config::locked(self.config_path.as_deref(), async |config| {
            let client = config
                .by_name_mut(&params.name)
                .ok_or_else(|| anyhow::anyhow!("unknown client '{}'", params.name))?;

            let token = get_token(client, &http).await?;

            let state = match token {
                TokenResult::Refreshed(state) => {
                    client.state = Some(state.clone());
                    state
                }
                TokenResult::Existing(state) => state,
            };

            let token_value = match token_type.as_str() {
                "id" => state
                    .id_token
                    .ok_or_else(|| anyhow::anyhow!("ID token not available"))?,
                "refresh" => state
                    .refresh_token
                    .ok_or_else(|| anyhow::anyhow!("refresh token not available"))?,
                _ => state.access_token,
            };

            Ok(token_value)
        })
        .await
        .map_err(|e| rmcp::ErrorData::internal_error(format!("{e}"), None))?;

        Ok(CallToolResult::success(vec![ContentBlock::text(
            token_value,
        )]))
    }
}

#[tool_handler]
impl ServerHandler for OidcMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(rmcp::model::Implementation::new(
                "oidc-cli",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "OIDC token provider. Use list_clients to see configured clients, \
             then get_token to retrieve a valid access token.",
            )
    }
}

impl Mcp {
    /// Start the MCP server on stdio.
    pub async fn run(self) -> anyhow::Result<()> {
        let server = OidcMcpServer {
            config_path: self.config.clone(),
            http: self.http,
        };

        let service = server.serve(stdio()).await?;
        service.waiting().await?;

        Ok(())
    }
}
