use crate::{
    claims::RefreshTokenClaims,
    config::{Client, ClientState, ClientType},
    http::{HttpOptions, create_client},
    utils::OrNone,
};
use anyhow::{anyhow, bail};
use biscuit::{Empty, jws::Compact};
use oauth2::RefreshToken;
use openidconnect::{
    ClientId, ClientSecret, IssuerUrl, Scope,
    core::{CoreClient, CoreProviderMetadata},
};
use time::OffsetDateTime;

pub enum TokenResult {
    Existing(ClientState),
    Refreshed(ClientState),
}

/// Fetch a new token
pub async fn fetch_token(config: &Client, http: &HttpOptions) -> anyhow::Result<TokenResult> {
    log::debug!("Fetching new token");

    let http = create_client(http).await?;

    match &config.r#type {
        ClientType::Confidential {
            client_id,
            client_secret,
        } => {
            let provider_metadata = CoreProviderMetadata::discover_async(
                IssuerUrl::from_url(config.issuer_url.clone()),
                &http,
            )
            .await?;

            let client = CoreClient::from_provider_metadata(
                provider_metadata,
                ClientId::new(client_id.clone()),
                Some(ClientSecret::new(client_secret.clone())),
            );

            let token = client
                .exchange_client_credentials()?
                .add_scopes(extra_scopes(config.scope.as_deref()))
                .request_async(&http)
                .await?;

            Ok(TokenResult::Refreshed(token.into()))
        }
        ClientType::Public {
            client_id,
            client_secret,
        } => {
            let Some(state) = &config.state else {
                bail!(
                    "Expired token of a public client, without a state. You will need to re-login."
                );
            };

            let provider_metadata = CoreProviderMetadata::discover_async(
                IssuerUrl::from_url(config.issuer_url.clone()),
                &http,
            )
            .await?;

            let refresh_token = state.refresh_token.clone().ok_or_else(|| anyhow!("Expired token of a public client, without having a refresh token. You will need to re-login."))?;

            if let Ok(token) = Compact::<RefreshTokenClaims, Empty>::new_encoded(&refresh_token)
                .unverified_payload()
            {
                log::debug!("refresh token expiration: {:?}", token.exp);

                if let Some(exp) = token
                    .exp
                    .and_then(|exp| OffsetDateTime::from_unix_timestamp(exp).ok())
                {
                    if exp < OffsetDateTime::now_utc() {
                        bail!("Refresh token expired. You need to re-login.");
                    }
                }
            }

            let client = CoreClient::from_provider_metadata(
                provider_metadata,
                ClientId::new(client_id.clone()),
                client_secret.clone().map(ClientSecret::new),
            );

            let token = client
                .exchange_refresh_token(&RefreshToken::new(refresh_token))?
                .add_scopes(extra_scopes(config.scope.as_deref()))
                .request_async(&http)
                .await?;

            Ok(TokenResult::Refreshed(token.into()))
        }
    }
}

/// Get the current token, or fetch a new one
pub async fn get_token(config: &Client, http: &HttpOptions) -> anyhow::Result<TokenResult> {
    if let Some(state) = &config.state {
        log::debug!("Token expires: {}", OrNone(&state.expires));
        if let Some(expires) = state.expires {
            if expires > OffsetDateTime::now_utc() {
                return Ok(TokenResult::Existing(state.clone()));
            }
        }
    }

    fetch_token(config, http).await
}

pub fn extra_scopes(scope: Option<&str>) -> impl Iterator<Item = Scope> {
    scope
        .into_iter()
        .flat_map(|s| s.split(' '))
        .map(|s| Scope::new(s.into()))
}
