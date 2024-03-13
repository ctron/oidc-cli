use crate::config::{Client, ClientType};
use openid::{Bearer, Discovered, StandardClaims};

pub async fn get_token(config: &Client) -> anyhow::Result<Bearer> {
    let client = reqwest::ClientBuilder::new();
    let client = client.build()?;

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

            Ok(client.request_token_using_client_credentials(None).await?)
        }
    }
}
