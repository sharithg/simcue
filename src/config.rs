use serde::Deserialize;
use std::fs;
use std::io::Read;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub message_retention_seconds: u64,
}

pub fn get_config() -> Config {
    let mut config_toml = String::new();

    // Read the file to a string
    let mut file = fs::File::open("simcue.toml").expect("Could not open the file");
    file.read_to_string(&mut config_toml)
        .expect("Could not read the file");

    // Parse and deserialize the TOML
    let config: Config = toml::from_str(&config_toml).expect("Could not deserialize the TOML");

    // Now you can use your config!
    config
}
