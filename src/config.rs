use std::{path::PathBuf, sync::OnceLock};

use figment::{
    Figment,
    providers::{Env, Serialized},
};
use serde::{Deserialize, Serialize};

static CONFIG: OnceLock<Config> = OnceLock::new();

/// The main configuration struct. See the impl for Default for default values.
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
    pub key: String,
    pub secret: String,
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
    pub fn init() -> Result<(), String> {
        let default = Figment::new().merge(Serialized::defaults(Config::default()));

        let with_env = default.clone().merge(Env::prefixed("TYP_").split("_"));

        match with_env.extract::<Config>() {
            Ok(c) => {
                CONFIG.set(c).expect("Tried to set config twice.");
                Ok(())
            }
            Err(e) => {
                let c = default.extract::<Config>().unwrap();
                CONFIG.set(c).expect("Tried to set config twice.");
                Err(e.to_string())
            }
        }
    }

    pub fn get() -> &'static Config {
        // MAYBE: I think expect is fine here; way to ensure? as long as init() is the first thing in main()?
        CONFIG.get().expect("Settings are not initialised.")
    }
}
