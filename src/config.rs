use serde::Deserialize;
use std::fs;
use std::io::Read;

#[non_exhaustive]
struct Defaults;

impl Defaults {
    // 2 days in seconds
    pub const MESSAGE_RETENTION_SECONDS: u64 = 2 * 24 * 60;
}

#[derive(Debug, Deserialize)]
pub struct TomlConfig {
    pub message_retention_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub message_retention_seconds: u64,
}

impl Config {
    pub fn get() -> Config {
        let mut config_toml = String::new();

        // Read the file to a string
        let mut file = fs::File::open("simcue.toml").expect("Could not open the file");
        file.read_to_string(&mut config_toml)
            .expect("Could not read the file");

        // Parse and deserialize the TOML
        let config: TomlConfig =
            toml::from_str(&config_toml).expect("Could not deserialize the TOML");

        let message_retention_seconds = config
            .message_retention_seconds
            .unwrap_or(Defaults::MESSAGE_RETENTION_SECONDS);

        Config {
            message_retention_seconds,
        }
    }
}
