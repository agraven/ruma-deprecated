//! User-facing configuration.

use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use base64::decode;
use iron::typemap::Key;
use iron::{Plugin, Request};
use persistent::Read as PersistentRead;
use serde_json;
use serde_yaml;
use toml;

use crate::error::{ApiError, CliError};

/// Default paths where Ruma will look for a configuration file if left unspecified.
const DEFAULT_CONFIG_FILES: [&str; 4] = ["ruma.json", "ruma.toml", "ruma.yaml", "ruma.yml"];
/// Directory for system-wide configuration files.
const CONFIG_DIR: &str = "/etc/ruma";

/// The user's configuration as loaded from the configuration file.
///
/// Refer to `Config` for the description of the fields.
#[derive(Deserialize)]
#[serde(tag = "version")]
enum RawConfig {
    /// Version 1 of the configuration format.
    #[serde(rename = "1")]
    V1(V1Config),
}

/// Version 1 of the configuration format.
#[derive(Deserialize)]
struct V1Config {
    /// See the similarly named field on `Config`.
    bind_address: Option<String>,
    /// See the similarly named field on `Config`.
    bind_port: Option<String>,
    /// See the similarly named field on `Config`.
    domain: String,
    /// See the similarly named field on `Config`.
    macaroon_secret_key: String,
    /// See the similarly named field on `Config`.
    postgres_url: String,
}

/// Server configuration provided by the user.
#[derive(Debug, Clone)]
pub struct Config {
    /// The network address where the server should listen for connections. Defaults to 127.0.0.1.
    pub bind_address: String,
    /// The network port where the server should listen for connections. Defaults to 3000.
    pub bind_port: String,
    /// The DNS name where clients can reach the server. Used as the hostname portion of user IDs.
    pub domain: String,
    /// The secret key used for generating
    /// [Macaroons](https://research.google.com/pubs/pub41892.html). Must be 32
    /// cryptographically random bytes, encoded as a Base64 string. Changing this value will
    /// invalidate any previously generated macaroons.
    pub macaroon_secret_key: Vec<u8>,
    /// A [PostgreSQL connection string](http://www.postgresql.org/docs/current/static/libpq-connect.html#LIBPQ-CONNSTRING)
    /// for Ruma's PostgreSQL database.
    pub postgres_url: String,
}

impl Config {
    /// Load the user's configuration file.
    ///
    /// If a path is given, it will try to load the configuration there.
    /// Otherwise, try to load a file from the defaults locations.
    pub fn from_file(path: Option<&str>) -> Result<Self, CliError> {
        let config_path: Cow<Path> = if let Some(path_str) = path {
            let path = Path::new(path_str);
            if !path.is_file() {
                return Err(CliError::new(format!(
                    "Configuration file `{}` not found.",
                    path_str
                )));
            }
            path.into()
        // Look in working directory
        } else if let Some(path) = DEFAULT_CONFIG_FILES
            .iter()
            .map(|file| Path::new(file))
            .find(|path| path.is_file())
        {
            path.into()
        // Look in system config directory
        } else if let Some(path_buf) = DEFAULT_CONFIG_FILES
            .iter()
            .map(|file| Path::new(CONFIG_DIR).join(file))
            .find(|path| path.is_file())
        {
            path_buf.into()
        } else {
            return Err(CliError::new("No configuration file was found."));
        };

        let raw_config = match config_path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => Self::load_json(&config_path),
            Some("toml") => Self::load_toml(&config_path),
            Some("yml") | Some("yaml") => Self::load_yaml(&config_path),
            _ => Err(CliError::new("Unsupported configuration file format")),
        }?;

        let RawConfig::V1(v1_config) = raw_config;

        let macaroon_secret_key = match decode(&v1_config.macaroon_secret_key) {
            Ok(bytes) => match bytes.len() {
                32 => bytes,
                _ => Err(CliError::new("macaroon_secret_key must be 32 bytes."))?,
            },
            Err(_) => Err(CliError::new("macaroon_secret_key must be valid Base64."))?,
        };

        Ok(Self {
            bind_address: v1_config
                .bind_address
                .unwrap_or_else(|| "127.0.0.1".to_string()),
            bind_port: v1_config.bind_port.unwrap_or_else(|| "3000".to_string()),
            domain: v1_config.domain,
            macaroon_secret_key,
            postgres_url: v1_config.postgres_url,
        })
    }

    /// Load the `RawConfig` from a JSON configuration file.
    fn load_json(path: &Path) -> Result<RawConfig, CliError> {
        let contents = Self::read_file_contents(path);
        match serde_json::from_str(&contents) {
            Ok(config) => Ok(config),
            Err(error) => Err(CliError::from(error)),
        }
    }

    /// Load the `RawConfig` from a TOML configuration file.
    fn load_toml(path: &Path) -> Result<RawConfig, CliError> {
        let contents = Self::read_file_contents(path);
        match toml::from_str(&contents) {
            Ok(config) => Ok(config),
            Err(error) => Err(CliError::from(error)),
        }
    }

    /// Load the `RawConfig` from a YAML configuration file.
    fn load_yaml(path: &Path) -> Result<RawConfig, CliError> {
        let contents = Self::read_file_contents(path);
        match serde_yaml::from_str(&contents) {
            Ok(config) => Ok(config),
            Err(error) => Err(CliError::from(error)),
        }
    }

    /// Read the contents of a file.
    fn read_file_contents(path: &Path) -> String {
        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        contents
    }

    /// Extract the `Config` stored in the request.
    pub fn from_request(request: &mut Request<'_, '_>) -> Result<Arc<Self>, ApiError> {
        request
            .get::<PersistentRead<Self>>()
            .map_err(ApiError::from)
    }
}

impl Key for Config {
    type Value = Self;
}

#[cfg(test)]
mod tests {
    use serde_json;

    use super::RawConfig;

    #[test]
    fn deserialize_v1_config() {
        let raw_config: RawConfig = serde_json::from_str(
            r#"
            {
                "version": "1",
                "domain": "example.com",
                "macaroon_secret_key": "qbnabRiFu5fWzoijGmc6Kk2tRox3qJSWvL3VRl4Vhl8=",
                "postgres_url": "postgres://username:password@example.com:5432/ruma"
            }
        "#,
        )
        .unwrap();

        let RawConfig::V1(v1_config) = raw_config;

        assert_eq!(v1_config.domain, "example.com");
        assert_eq!(
            v1_config.macaroon_secret_key,
            "qbnabRiFu5fWzoijGmc6Kk2tRox3qJSWvL3VRl4Vhl8="
        );
        assert_eq!(
            v1_config.postgres_url,
            "postgres://username:password@example.com:5432/ruma"
        );
    }
}
