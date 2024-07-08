use anyhow::Context;
use reqwest::{header, tls::Version};
use std::path::PathBuf;

const USER_AGENT: &str = concat!("OIDC-CLI/", env!("CARGO_PKG_VERSION"));

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, clap::ValueEnum)]
pub enum TlsVersion {
    /// TLS 1.0
    #[value(name("1.0"))]
    Tls1_0,
    /// TLS 1.1
    #[value(name("1.1"))]
    Tls1_1,
    /// TLS 1.2
    #[value(name("1.2"))]
    Tls1_2,
    /// TLS 1.3
    #[value(name("1.3"))]
    Tls1_3,
}

impl From<TlsVersion> for Version {
    fn from(value: TlsVersion) -> Self {
        match value {
            TlsVersion::Tls1_0 => Version::TLS_1_0,
            TlsVersion::Tls1_1 => Version::TLS_1_1,
            TlsVersion::Tls1_2 => Version::TLS_1_2,
            TlsVersion::Tls1_3 => Version::TLS_1_3,
        }
    }
}

/// HTTP client options
#[derive(Clone, Debug, PartialEq, Eq, clap::Args)]
#[command(next_help_heading = "HTTP client options")]
pub struct HttpOptions {
    /// Disable TLS validation (INSECURE!)
    #[arg(long)]
    pub tls_insecure: bool,

    /// Additional root certificates
    #[arg(long = "root-certificate", alias = "cacert", short = 'C')]
    pub additional_root_certificates: Vec<PathBuf>,

    /// Disable system root certificates
    #[arg(long = "no-system-certificates")]
    pub disable_system_certificates: bool,

    /// Connect timeout
    #[arg(long, default_value = "30s")]
    pub connect_timeout: humantime::Duration,

    /// Request timeout
    #[arg(long, default_value = "60s", short = 't')]
    pub timeout: humantime::Duration,

    /// Minimum TLS version
    #[arg(long, value_enum, default_value_t = TlsVersion::Tls1_2)]
    pub min_tls_version: TlsVersion,
}

/// A common way to create an HTTP client
pub async fn create_client(options: &HttpOptions) -> anyhow::Result<reqwest::Client> {
    let mut headers = header::HeaderMap::new();
    headers.insert("User-Agent", header::HeaderValue::from_static(USER_AGENT));

    let mut client = reqwest::ClientBuilder::new().default_headers(headers);

    // tls validation

    if options.tls_insecure {
        log::warn!("Disabling TLS validation");
        client = client
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true);
    }

    // timeouts

    client = client.connect_timeout(options.connect_timeout.into());
    client = client.timeout(options.timeout.into());

    // certs

    client = client.tls_built_in_root_certs(!options.disable_system_certificates);

    for cert in &options.additional_root_certificates {
        let cert = std::fs::read(&cert)
            .with_context(|| format!("Reading certificate: {}", cert.display()))?;
        let cert = reqwest::tls::Certificate::from_pem(&cert)?;
        client = client.add_root_certificate(cert);
    }

    // tls version

    client = client.min_tls_version(options.min_tls_version.into());

    // build

    let client = client.build()?;

    // done

    Ok(client)
}
