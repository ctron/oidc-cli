use anyhow::{anyhow, Context};
use num_traits::ToPrimitive;
use openid::Bearer;
use std::{
    collections::BTreeMap,
    fs::OpenOptions,
    io::{BufReader, BufWriter, ErrorKind},
    path::{Path, PathBuf},
};
use time::Duration;
use url::Url;

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

    pub fn store(&self, path: Option<impl AsRef<Path>>) -> anyhow::Result<()> {
        match path {
            Some(path) => self.store_to(path),
            None => self.store_to(Self::default_file_err()?),
        }
    }

    pub fn store_to(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
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
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Client {
    pub issuer_url: Url,
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

impl TryFrom<Bearer> for ClientState {
    type Error = anyhow::Error;

    fn try_from(token: Bearer) -> Result<Self, Self::Error> {
        let expires = token.expires_in.map(|exp| {
            time::OffsetDateTime::now_utc() + Duration::seconds(exp.to_i64().unwrap_or(i64::MAX))
        });
        Ok(ClientState {
            access_token: token.access_token,
            id_token: token.id_token,
            refresh_token: token.refresh_token,
            expires,
        })
    }
}
