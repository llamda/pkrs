use serde_derive::{Deserialize, Serialize};
use std::fs;

const PATH: &str = "config.toml";

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde(default = "db_path")]
    db_file_path: String,
}

fn db_path() -> String {
    "./db/files".to_string()
}

impl Config {
    pub fn get() -> Self {
        let mut create_new = false;
        let content = match fs::read_to_string(PATH) {
            Ok(string) => string,
            Err(_) => {
                create_new = true;
                println!("Missing {}? Creating new config...", PATH);
                String::from("")
            }
        };

        let config: Config = toml::from_str(&content).expect("Failed to parse config file?");

        if create_new {
            let defaults = toml::to_string(&config).unwrap();
            fs::write(PATH, defaults).expect("Failed to write default config?");
        }
        config
    }
}
