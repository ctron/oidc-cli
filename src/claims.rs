use biscuit::{CompactJson, SingleOrMultiple};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

/// Access token claims
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccessTokenClaims {
    #[serde(default)]
    pub azp: Option<String>,
    pub sub: String,
    pub iss: Url,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aud: Option<SingleOrMultiple<String>>,

    #[serde(default)]
    pub exp: Option<i64>,
    #[serde(default)]
    pub iat: Option<i64>,
    #[serde(default)]
    pub auth_time: Option<i64>,

    #[serde(flatten)]
    pub extended_claims: Value,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub scope: String,
}

impl CompactJson for AccessTokenClaims {}

/// Access token claims
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RefreshTokenClaims {
    #[serde(default)]
    pub exp: Option<i64>,

    #[serde(flatten)]
    pub extended_claims: Value,
}

impl CompactJson for RefreshTokenClaims {}
