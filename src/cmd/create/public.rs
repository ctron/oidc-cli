use crate::{
    cmd::create::CreateCommon,
    config::{Client, ClientType, Config},
    http::{HttpOptions, create_client},
    oidc::extra_scopes,
    server::{Bind, Server},
    utils::OrNone,
};
use anyhow::{Context, bail};
use oauth2::{
    AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, RedirectUrl, TokenResponse,
};
use openidconnect::{
    AuthenticationFlow, IssuerUrl, Nonce,
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
};
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

        let http = create_client(&self.http).await?;

        let provider_metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::from_url(self.common.issuer.clone()),
            &http,
        )
        .await?;

        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(self.client_id.clone()),
            None,
        )
        .set_redirect_uri(RedirectUrl::new(redirect)?);

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let req = client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scopes(extra_scopes(self.common.scope.as_deref()));

        let (open, csrf_token, nonce) = req.set_pkce_challenge(pkce_challenge).url();

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

        // receive the result from the local server

        let result = server.receive_token().await?;

        // validate CSRF token

        match result.state {
            None => {
                bail!("missing 'state' parameter from server");
            }
            Some(state) if &state != csrf_token.secret() => {
                bail!("state mismatch");
            }
            Some(_) => {}
        }

        // fetch token

        let token = client
            .exchange_code(AuthorizationCode::new(result.code))?
            .set_pkce_verifier(pkce_verifier)
            .request_async(&http)
            .await?;

        // check ID token

        if let Some(id_token) = token.extra_fields().id_token() {
            id_token
                .clone()
                .into_claims(&client.id_token_verifier(), &nonce)
                .context("failed to verify ID token")?;
        }

        // log info

        log::info!("First token:");
        log::info!(
            "       ID: {}",
            OrNone(
                &token
                    .extra_fields()
                    .id_token()
                    .cloned()
                    .map(|t| t.to_string())
            )
        );
        log::info!("   Access: {}", token.access_token().clone().into_secret());
        log::info!(
            "  Refresh: {}",
            OrNone(&token.refresh_token().cloned().map(|t| t.into_secret()))
        );

        // create client

        let client = Client {
            issuer_url: self.common.issuer,
            scope: self.common.scope,
            r#type: ClientType::Public {
                client_id: self.client_id,
            },
            state: Some(token.into()),
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
