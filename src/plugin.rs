use crate::{
    config::Config,
    http::HttpOptions,
    oidc::{TokenResult, fetch_token, get_token},
};
use anyhow::{Context, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const LOCAL_CONFIG_BASENAME: &str = ".xh-auth-oidc";

/// Input sent by `xh` when it invokes `xh-plugin-oidc`.
///
/// This mirrors the custom auth plugin protocol proposed by `xh`: the next
/// request is included for plugins that need request-aware signing, `auth`
/// carries repeated `--auth` values, `state` carries redirect-local plugin
/// state, and `current_dir` is the directory where `xh` was invoked.
#[derive(Debug, Deserialize)]
pub struct PluginInput {
    /// Request that `xh` is about to send.
    pub next_request: NextRequest,
    /// Repeated `--auth` values passed to `xh`.
    #[serde(default)]
    pub auth: Vec<String>,
    /// Per-request plugin state supplied by `xh`.
    #[serde(default)]
    pub state: serde_json::Value,
    /// Working directory reported by `xh`.
    pub current_dir: PathBuf,
}

/// Request metadata supplied by `xh`.
///
/// `oidc-cli` does not currently use the request to select tokens, but the
/// known fields are retained so the input remains compatible with the plugin
/// protocol. Unknown future fields are ignored by serde during deserialization.
#[derive(Debug, Deserialize)]
pub struct NextRequest {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<Header>,
    pub body_base64: Option<String>,
}

/// Header representation used by the xh plugin protocol.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Header {
    pub name: String,
    pub value: String,
}

/// Header mutations returned to `xh`.
///
/// `xh` applies removals and additions to the request it is about to send.
/// This plugin always leaves `set_state` as JSON null because OIDC token state
/// is persisted in the normal oidc-cli configuration file instead.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct PluginResponse {
    pub remove_headers: Vec<String>,
    pub add_headers: Vec<Header>,
    pub set_state: serde_json::Value,
}

/// Token type to inject into the outgoing request header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    /// Use the OAuth2/OIDC access token.
    Access,
    /// Use the OIDC ID token.
    Id,
}

/// Parsed options for one plugin invocation.
///
/// These values come from repeated `xh --auth=...` arguments. The first bare
/// value is the optional client name, while named options use `key=value`.
#[derive(Debug, PartialEq, Eq)]
struct PluginOptions {
    /// Name of the configured OIDC client to use.
    client_name: Option<String>,
    /// Optional path to oidc-cli's main configuration file.
    config: Option<PathBuf>,
    /// Which token from the OIDC client state should be injected.
    token: TokenKind,
    /// Whether to bypass cached token state and request a new token.
    force: bool,
    /// Header name to remove and then add to the outgoing request.
    header: String,
    /// Optional prefix prepended before the token value.
    scheme: String,
}

/// Project-local plugin configuration loaded from `.xh-auth-oidc.*`.
///
/// `client_name` is optional when the client is passed explicitly through
/// `xh --auth=<name>`. The optional `http` section configures the HTTP client
/// used for OIDC discovery and token refresh requests when present.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
struct LocalPluginConfig {
    #[serde(default)]
    client_name: Option<String>,
    #[serde(default)]
    http: Option<HttpOptions>,
}

impl LocalPluginConfig {
    /// Build HTTP options, applying the local `http` section when present.
    fn http_options(&self, config_dir: Option<&Path>) -> HttpOptions {
        match &self.http {
            Some(http) => {
                let mut options = http.clone();
                // Certificate paths in a project-local config should be
                // stable regardless of where `xh` is invoked from.
                options.additional_root_certificates = options
                    .additional_root_certificates
                    .iter()
                    .map(|path| resolve_config_relative_path(config_dir, path))
                    .collect();
                options
            }
            None => HttpOptions::default(),
        }
    }
}

impl PluginOptions {
    /// Parse `xh --auth` values into plugin options.
    ///
    /// The first bare value is treated as the OIDC client name. Additional
    /// options are `key=value` pairs so they can be passed as repeated
    /// `--auth=...` arguments without requiring a separate CLI parser.
    fn parse(auth: &[String]) -> anyhow::Result<Self> {
        let mut options = Self {
            client_name: None,
            config: None,
            token: TokenKind::Access,
            force: false,
            header: "Authorization".into(),
            scheme: "Bearer".into(),
        };

        for item in auth {
            let Some((key, value)) = item.split_once('=') else {
                if options.client_name.is_some() {
                    bail!("multiple client names passed to auth plugin");
                }
                options.client_name = Some(non_empty("client name", item)?.to_string());
                continue;
            };

            match key {
                // Path to oidc-cli's main configuration file, not the local
                // `.xh-auth-oidc.*` discovery file.
                "config" => options.config = Some(PathBuf::from(non_empty("config", value)?)),
                "token" => {
                    options.token = match value {
                        "access" => TokenKind::Access,
                        "id" => TokenKind::Id,
                        _ => bail!("invalid token option '{value}', expected 'access' or 'id'"),
                    }
                }
                "force" => {
                    options.force = match value {
                        "true" => true,
                        "false" => false,
                        _ => bail!("invalid force option '{value}', expected 'true' or 'false'"),
                    }
                }
                "header" => options.header = non_empty("header", value)?.to_string(),
                "scheme" => options.scheme = value.to_string(),
                _ => bail!("unknown auth plugin option '{key}'"),
            }
        }

        validate_header(&options.header)?;

        Ok(options)
    }
}

fn non_empty<'a>(name: &str, value: &'a str) -> anyhow::Result<&'a str> {
    if value.is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(value)
}

fn validate_header(name: &str) -> anyhow::Result<()> {
    reqwest::header::HeaderName::from_bytes(name.as_bytes())
        .with_context(|| format!("invalid header name '{name}'"))?;
    Ok(())
}

fn validate_header_value(value: &str) -> anyhow::Result<()> {
    reqwest::header::HeaderValue::from_str(value).context("invalid header value")?;
    Ok(())
}

fn resolve_config_relative_path(config_dir: Option<&Path>, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else if let Some(config_dir) = config_dir {
        config_dir.join(path)
    } else {
        path.to_path_buf()
    }
}

/// Run the plugin protocol against arbitrary input/output streams.
///
/// The binary passes locked stdin/stdout here. Tests can call this with in-memory
/// streams, which also protects the invariant that stdout contains only the JSON
/// protocol response.
pub async fn run<R, W>(mut reader: R, mut writer: W) -> anyhow::Result<()>
where
    R: Read,
    W: Write,
{
    let mut input = Vec::new();
    reader.read_to_end(&mut input)?;
    let input = serde_json::from_slice::<PluginInput>(&input)
        .context("failed to parse xh auth plugin input")?;
    let response = authenticate(input).await?;
    serde_json::to_writer(&mut writer, &response)?;
    writeln!(writer)?;
    Ok(())
}

/// Resolve an OIDC token and convert it into an `xh` header mutation response.
pub async fn authenticate(input: PluginInput) -> anyhow::Result<PluginResponse> {
    let mut options = PluginOptions::parse(&input.auth)?;
    let local_config = find_local_config(&input.current_dir)?
        .map(|(path, config)| {
            let config = config.with_context(|| {
                format!("failed to load local plugin config {}", path.display())
            })?;
            Ok::<_, anyhow::Error>((path, config))
        })
        .transpose()?;
    let http = local_config
        .as_ref()
        .map(|(path, config)| config.http_options(path.parent()))
        .unwrap_or_default();

    // Explicit `--auth=<client>` wins. Local discovery is only a convenience
    // fallback for project directories that want to avoid repeating the client.
    if options.client_name.is_none()
        && let Some((_, config)) = &local_config
    {
        options.client_name = config
            .client_name
            .clone()
            .map(non_empty_client_name)
            .transpose()?;
    }

    let client_name = options.client_name.as_deref().ok_or_else(|| {
        anyhow!("missing OIDC client name; pass --auth=<name> or add local config")
    })?;

    let mut config = Config::load(options.config.as_deref())?;
    let (token, refreshed) = {
        let client = config
            .by_name_mut(client_name)
            .ok_or_else(|| anyhow!("unknown client '{client_name}'"))?;

        let result = if options.force {
            fetch_token(client, &http).await?
        } else {
            get_token(client, &http).await?
        };

        match result {
            TokenResult::Refreshed(token) => {
                client.state = Some(token.clone());
                (token, true)
            }
            TokenResult::Existing(token) => (token, false),
        }
    };

    // Persist only when the OIDC state actually changed. Cached valid tokens
    // should not rewrite the user's config file on every request.
    if refreshed {
        config.store(options.config.as_deref())?;
    }

    let token = match options.token {
        TokenKind::Access => token.access_token,
        TokenKind::Id => token
            .id_token
            .ok_or_else(|| anyhow!("ID token not available"))?,
    };

    let value = match options.scheme.as_str() {
        "" => token,
        scheme => format!("{scheme} {token}"),
    };
    validate_header_value(&value)?;

    Ok(PluginResponse {
        remove_headers: vec![options.header.clone()],
        add_headers: vec![Header {
            name: options.header,
            value,
        }],
        set_state: serde_json::Value::Null,
    })
}

fn non_empty_client_name(client_name: String) -> anyhow::Result<String> {
    if client_name.is_empty() {
        bail!("local plugin config client_name must not be empty");
    }
    Ok(client_name)
}

/// Search for `.xh-auth-oidc.{json,yaml,toml}` from `start` up to the root.
///
/// Search order is intentionally stable: for each directory, JSON wins over
/// YAML, which wins over TOML. The nearest parent directory wins over any
/// farther ancestor.
fn find_local_config(
    start: impl AsRef<Path>,
) -> anyhow::Result<Option<(PathBuf, anyhow::Result<LocalPluginConfig>)>> {
    let mut current = start.as_ref();

    loop {
        for extension in ["json", "yaml", "toml"] {
            let path = current.join(format!("{LOCAL_CONFIG_BASENAME}.{extension}"));
            if path.exists() {
                return Ok(Some((path.clone(), load_local_config(&path))));
            }
        }

        let Some(parent) = current.parent() else {
            return Ok(None);
        };

        current = parent;
    }
}

/// Load the local plugin discovery file selected by `find_local_config`.
fn load_local_config(path: &Path) -> anyhow::Result<LocalPluginConfig> {
    let data = std::fs::read(path)
        .with_context(|| format!("failed to read local plugin config {}", path.display()))?;

    let config =
        match path.extension().and_then(OsStr::to_str) {
            Some("json") => serde_json::from_slice(&data)?,
            Some("yaml") => serde_yaml::from_slice(&data)?,
            Some("toml") => toml::from_str(std::str::from_utf8(&data).with_context(|| {
                format!("local plugin config is not UTF-8: {}", path.display())
            })?)?,
            _ => bail!("unsupported local plugin config format: {}", path.display()),
        };

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Client, ClientState, ClientType},
        http::TlsVersion,
    };
    use openidconnect::IssuerUrl;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    use time::OffsetDateTime;

    /// Create a unique directory under the system temp directory.
    fn temp_dir() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!(
            "oidc-cli-plugin-test-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap_or_else(|err| panic!("failed to create temp dir: {err}"));
        path
    }

    /// Write a small fixture file for local config discovery tests.
    fn write(path: &Path, data: &str) {
        fs::write(path, data)
            .unwrap_or_else(|err| panic!("failed to write {}: {err}", path.display()));
    }

    /// Build the minimum xh plugin input needed by the authentication tests.
    fn input(current_dir: PathBuf, auth: Vec<&str>) -> PluginInput {
        PluginInput {
            next_request: NextRequest {
                method: "GET".into(),
                url: "https://example.com/".into(),
                headers: Vec::new(),
                body_base64: None,
            },
            auth: auth.into_iter().map(str::to_string).collect(),
            state: serde_json::Value::Null,
            current_dir,
        }
    }

    /// Store an oidc-cli config with non-expired tokens.
    ///
    /// The token expiration is intentionally in the future so `authenticate`
    /// can exercise cached-token behavior without making network requests.
    fn config_with_tokens(path: &Path) {
        let issuer_url =
            IssuerUrl::new("https://issuer.example.com".into()).unwrap_or_else(|err| {
                panic!("failed to create issuer URL: {err}");
            });
        let config = Config {
            clients: [(
                "my-client".into(),
                Client {
                    issuer_url,
                    scope: None,
                    r#type: ClientType::Public {
                        client_id: "client-id".into(),
                        client_secret: None,
                    },
                    state: Some(ClientState {
                        access_token: "access-token".into(),
                        id_token: Some("id-token".into()),
                        refresh_token: Some("refresh-token".into()),
                        expires: Some(OffsetDateTime::now_utc() + time::Duration::hours(1)),
                    }),
                },
            )]
            .into(),
        };
        config
            .store_to(path)
            .unwrap_or_else(|err| panic!("failed to store config: {err}"));
    }

    #[test]
    /// Parses the full supported `xh --auth` option surface.
    fn parses_plugin_options() {
        let options = PluginOptions::parse(&[
            "my-client".into(),
            "token=id".into(),
            "force=true".into(),
            "header=X-Auth".into(),
            "scheme=".into(),
        ])
        .unwrap_or_else(|err| panic!("failed to parse options: {err}"));

        assert_eq!(options.client_name.as_deref(), Some("my-client"));
        assert_eq!(options.token, TokenKind::Id);
        assert!(options.force);
        assert_eq!(options.header, "X-Auth");
        assert_eq!(options.scheme, "");
    }

    #[test]
    /// Rejects malformed or ambiguous auth option values before token lookup.
    fn rejects_invalid_options() {
        assert!(PluginOptions::parse(&["token=refresh".into()]).is_err());
        assert!(PluginOptions::parse(&["force=yes".into()]).is_err());
        assert!(PluginOptions::parse(&["header=".into()]).is_err());
        assert!(PluginOptions::parse(&["one".into(), "two".into()]).is_err());
    }

    #[test]
    /// Uses the documented per-directory format precedence.
    fn discovers_json_before_yaml_before_toml() {
        let root = temp_dir();
        // All formats are present in the same directory; JSON must win.
        write(
            &root.join(".xh-auth-oidc.json"),
            r#"{"client_name":"json"}"#,
        );
        write(&root.join(".xh-auth-oidc.yaml"), "client_name: yaml\n");
        write(&root.join(".xh-auth-oidc.toml"), r#"client_name = "toml""#);

        let (path, config) = find_local_config(&root)
            .unwrap_or_else(|err| panic!("failed to discover config: {err}"))
            .unwrap_or_else(|| panic!("config not found"));

        assert_eq!(
            path.file_name().and_then(OsStr::to_str),
            Some(".xh-auth-oidc.json")
        );
        assert_eq!(
            config.unwrap_or_else(|err| panic!("failed to parse config: {err}")),
            LocalPluginConfig {
                client_name: Some("json".into()),
                http: None,
            }
        );
    }

    #[test]
    /// Searches upward and stops at the nearest ancestor with a config file.
    fn discovers_nearest_parent_config() {
        let root = temp_dir();
        let nested = root.join("a").join("b");
        fs::create_dir_all(&nested)
            .unwrap_or_else(|err| panic!("failed to create nested dirs: {err}"));
        write(&root.join(".xh-auth-oidc.yaml"), "client_name: root\n");
        // The nearer TOML file should win over the farther YAML file.
        write(
            &root.join("a").join(".xh-auth-oidc.toml"),
            r#"client_name = "near""#,
        );

        let (_, config) = find_local_config(&nested)
            .unwrap_or_else(|err| panic!("failed to discover config: {err}"))
            .unwrap_or_else(|| panic!("config not found"));

        assert_eq!(
            config.unwrap_or_else(|err| panic!("failed to parse config: {err}")),
            LocalPluginConfig {
                client_name: Some("near".into()),
                http: None,
            }
        );
    }

    #[test]
    /// Deserializes the local `http` section through the real `HttpOptions`.
    fn reads_http_options_from_local_config() {
        let root = temp_dir();
        write(
            &root.join(".xh-auth-oidc.yaml"),
            r#"
client_name: my-client
http:
  tls_insecure: true
  additional_root_certificates:
    - certs/root.pem
  disable_system_certificates: true
  connect_timeout: 5s
  timeout: 10s
  min_tls_version: "1.3"
"#,
        );

        let (path, config) = find_local_config(&root)
            .unwrap_or_else(|err| panic!("failed to discover config: {err}"))
            .unwrap_or_else(|| panic!("config not found"));
        let config = config.unwrap_or_else(|err| panic!("failed to parse config: {err}"));
        let http = config.http_options(path.parent());

        // Relative certificate paths are resolved from the config file's
        // directory, not from the process working directory.
        assert!(http.tls_insecure);
        assert!(http.disable_system_certificates);
        assert_eq!(
            http.connect_timeout,
            std::time::Duration::from_secs(5).into()
        );
        assert_eq!(http.timeout, std::time::Duration::from_secs(10).into());
        assert_eq!(http.min_tls_version, TlsVersion::Tls1_3);
        assert_eq!(
            http.additional_root_certificates,
            vec![root.join("certs").join("root.pem")]
        );
    }

    #[tokio::test]
    /// Gives an explicitly passed client name priority over local config.
    async fn explicit_client_takes_precedence_over_local_config() {
        let root = temp_dir();
        let config_path = root.join("config.yaml");
        config_with_tokens(&config_path);
        write(
            &root.join(".xh-auth-oidc.json"),
            r#"{"client_name":"other"}"#,
        );

        let response = authenticate(input(
            root,
            vec!["my-client", &format!("config={}", config_path.display())],
        ))
        .await
        .unwrap_or_else(|err| panic!("failed to authenticate: {err}"));

        assert_eq!(
            response,
            PluginResponse {
                remove_headers: vec!["Authorization".into()],
                add_headers: vec![Header {
                    name: "Authorization".into(),
                    value: "Bearer access-token".into(),
                }],
                set_state: serde_json::Value::Null,
            }
        );
    }

    #[tokio::test]
    /// Falls back to the discovered local config when no client is passed.
    async fn uses_discovered_client_config() {
        let root = temp_dir();
        let config_path = root.join("config.yaml");
        config_with_tokens(&config_path);
        write(
            &root.join(".xh-auth-oidc.toml"),
            r#"client_name = "my-client""#,
        );

        let response = authenticate(input(
            root,
            vec![
                &format!("config={}", config_path.display()),
                "token=id",
                "header=X-Auth",
                "scheme=",
            ],
        ))
        .await
        .unwrap_or_else(|err| panic!("failed to authenticate: {err}"));

        assert_eq!(
            response.add_headers,
            vec![Header {
                name: "X-Auth".into(),
                value: "id-token".into(),
            }]
        );
        assert_eq!(response.remove_headers, vec!["X-Auth"]);
    }
}
