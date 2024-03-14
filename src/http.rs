/// A common way to create an HTTP client
pub async fn create_client() -> anyhow::Result<reqwest::Client> {
    let client = reqwest::ClientBuilder::new();
    let client = client.build()?;

    Ok(client)
}
