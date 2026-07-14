use anyhow::{Context, anyhow};
use oauth2::TokenResponse;
use openidconnect::IssuerUrl;
use openidconnect::core::CoreTokenResponse;
use std::{
    collections::BTreeMap,
    fs::OpenOptions,
    io::{BufReader, BufWriter, ErrorKind},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub clients: BTreeMap<String, Client>,
}

impl Config {
    pub fn default_file() -> Option<PathBuf> {
        let base = directories::ProjectDirs::from("de.dentrassi", "ctron", "oidc")?;

        Some(base.config_dir().join("config.yaml"))
    }

    pub fn default_file_err() -> anyhow::Result<PathBuf> {
        Self::default_file().ok_or_else(|| anyhow!("unable to evaluate default configuration file"))
    }

    pub fn load(path: Option<impl AsRef<Path>>) -> anyhow::Result<Self> {
        match path {
            Some(path) => Self::load_from(path),
            None => Self::load_from(Self::default_file_err()?),
        }
    }

    pub fn load_from(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        log::debug!("loading configuration from: {}", path.display());

        match std::fs::File::open(path) {
            Ok(file) => Ok(serde_yaml::from_reader(BufReader::new(file))?),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err.into()),
        }
    }

    fn store_to(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();
        log::debug!("storing configuration to: {}", path.display());

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("unable to create parent directory: {}", parent.display())
            })?;
        }

        let mut file = OpenOptions::new();
        file.write(true).create(true).truncate(true);
        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::OpenOptionsExt;
            file.mode(0o600);
        }

        let file = file.open(path)?;

        serde_yaml::to_writer(BufWriter::new(file), self)?;

        Ok(())
    }

    /// Get a mutable client by name
    pub fn by_name_mut(&mut self, name: &str) -> Option<&mut Client> {
        self.clients.get_mut(name)
    }

    /// Execute an async closure with exclusive file-system lock on the config.
    ///
    /// Acquires an advisory lock on a sidecar `.lock` file, loads the config,
    /// passes it to the closure, and stores the config back if the closure
    /// returns `Ok`. The lock is released when the file is dropped.
    pub async fn locked<F, T>(path: Option<&Path>, f: F) -> anyhow::Result<T>
    where
        F: AsyncFnOnce(&mut Config) -> anyhow::Result<T>,
    {
        let config_path = match path {
            Some(p) => p.to_path_buf(),
            None => Self::default_file_err()?,
        };
        let lock_path = lock_path_for(&config_path);

        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating lock file directory: {}", parent.display()))?;
        }

        let lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .with_context(|| format!("opening lock file: {}", lock_path.display()))?;

        let lock_file = tokio::task::spawn_blocking(move || lock_file.lock().map(|()| lock_file))
            .await?
            .with_context(|| format!("acquiring lock: {}", lock_path.display()))?;

        let mut config = Self::load_from(&config_path)?;
        let result = f(&mut config).await?;
        config.store_to(&config_path)?;

        drop(lock_file);

        Ok(result)
    }
}

/// Derive the lock file path from a config file path.
fn lock_path_for(config_path: &Path) -> PathBuf {
    let mut lock_path = config_path.as_os_str().to_owned();
    lock_path.push(".lock");
    PathBuf::from(lock_path)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Client {
    pub issuer_url: IssuerUrl,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub r#type: ClientType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<ClientState>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ClientType {
    Confidential {
        client_id: String,
        client_secret: String,
    },
    Public {
        client_id: String,
        #[serde(default)]
        client_secret: Option<String>,
    },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ClientState {
    pub access_token: String,
    pub id_token: Option<String>,
    pub refresh_token: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub expires: Option<time::OffsetDateTime>,
}

impl From<CoreTokenResponse> for ClientState {
    fn from(token: CoreTokenResponse) -> Self {
        let access_token = token.access_token().clone().into_secret();
        let refresh_token = token.refresh_token().cloned().map(|t| t.into_secret());
        let expires = token
            .expires_in()
            .map(|exp| time::OffsetDateTime::now_utc() + exp);

        let id_token = token
            .extra_fields()
            .id_token()
            .cloned()
            .map(|t| t.to_string());

        Self {
            access_token,
            id_token,
            refresh_token,
            expires,
        }
    }
}
