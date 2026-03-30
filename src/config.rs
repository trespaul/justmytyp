use std::path::PathBuf;

use figment::{
    Figment,
    providers::{Env, Serialized},
};
use s3::Credentials;
use serde::{Deserialize, Serialize};

/// The main configuration struct. See the `Default` impl for default values.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// One of ERROR, WARN, INFO, DEBUG, TRACE.
    pub loglevel: log::LevelFilter,
    /// Path from where Typst searches for files.
    pub rootdir: PathBuf,
    /// Where Typst packages are cached.
    pub cachedir: PathBuf,
    /// Seconds until connections are dropped. Increase if your Typst compilations are very large or your server is underpowered.
    pub timeout: u64,
    /// Socket address to bind.
    pub bindaddress: String,
    /// s3 bucket settings. If empty, documents are returned in the HTTP response.
    pub s3: Option<S3Config>,
    /// Format string for timestamp prefixed to upload filenames;
    /// currently used to prevent clobbering files with the same name.
    /// Refer to [the format-string specification in the time-rs crate](https://time-rs.github.io/book/api/format-description.html).
    pub timestampformat: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct S3Config {
    pub url: String,
    pub bucket: String,
    pub region: String,

    #[serde(with = "CredentialsDef")]
    pub credentials: Credentials,
}

/// This dummy struct is copied from s3 in order to add ser/de.
#[derive(Serialize, Deserialize)]
#[serde(remote = "Credentials")]
pub struct CredentialsDef {
    /// Access key identifier. Use `KEY`.
    #[serde(rename = "key")]
    pub access_key_id: String,
    /// Secret access key. Use `SECRET`.
    #[serde(rename = "secret")]
    pub secret_access_key: String,
    /// Optional session token for temporary credentials. Use `SESSION`
    #[serde(rename = "session")]
    pub session_token: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            loglevel: log::LevelFilter::Info,
            rootdir: PathBuf::from("./"),
            cachedir: PathBuf::from("./.cache"),
            timeout: 10,
            bindaddress: String::from("0.0.0.0:3000"),
            s3: None,
            timestampformat: String::from(
                "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:2][offset_hour]:[offset_minute]",
            ),
        }
    }
}

impl Config {
    pub fn init() -> Self {
        let default = Figment::new().merge(Serialized::defaults(Config::default()));

        let with_env = default.clone().merge(Env::prefixed("TYP_").split("_"));

        with_env.extract::<Config>().unwrap_or_else(|e| {
            // TODO: logger is not yet initialised; find way to warn that init failed.
            log::warn!("Failed to load config: {e}; using defaults.");
            default.extract::<Config>().unwrap()
        })
    }
}
