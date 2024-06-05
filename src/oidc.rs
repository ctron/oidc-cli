use crate::{
    config::{Client, ClientState, ClientType},
    http::create_client,
    utils::OrNone,
};
use anyhow::{anyhow, bail};
use openid::{Bearer, Discovered, StandardClaims};
use time::OffsetDateTime;

pub enum TokenResult {
    Existing(ClientState),
    Refreshed(ClientState),
}

/// Fetch a new token
pub async fn fetch_token(config: &Client) -> anyhow::Result<TokenResult> {
    let client = create_client().await?;

    match &config.r#type {
        ClientType::Confidential {
            client_id,
            client_secret,
        } => {
            let client = openid::Client::<Discovered, StandardClaims>::discover_with_client(
                client,
                client_id.clone(),
                Some(client_secret.clone()),
                None,
                config.issuer_url.clone(),
            )
            .await?;

            Ok(TokenResult::Refreshed(
                client
                    .request_token_using_client_credentials(None)
                    .await?
                    .try_into()?,
            ))
        }
        ClientType::Public { client_id } => {
            let client = openid::Client::<Discovered, StandardClaims>::discover_with_client(
                client,
                client_id.clone(),
                None,
                None,
                config.issuer_url.clone(),
            )
            .await?;

            let Some(state) = &config.state else {
                bail!(
                    "Expired token of a public client, without a state. You will need to re-login."
                );
            };

            // we only need the `refresh_token`
            let token = Box::new(Bearer {
                access_token: state.access_token.clone(),
                token_type: "".to_string(),
                scope: None,
                state: None,
                refresh_token: Some(state.refresh_token.clone().ok_or_else(||anyhow!("Expired token of a public client, without having a refresh token. You will need to re-login."))?),
                expires_in: None,
                id_token: None,
                extra: None,
            });

            let token = client.refresh_token(token, None).await?;

            Ok(TokenResult::Refreshed(token.try_into()?))
        }
    }
}

/// Get the current token, or fetch a new one
pub async fn get_token(config: &Client) -> anyhow::Result<TokenResult> {
    if let Some(state) = &config.state {
        log::debug!("Token expires: {}", OrNone(&state.expires));
        if let Some(expires) = state.expires {
            if expires > OffsetDateTime::now_utc() {
                return Ok(TokenResult::Existing(state.clone()));
            }
        }
    }

    fetch_token(config).await
}
