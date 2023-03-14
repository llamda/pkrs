use serde_derive::{Deserialize, Serialize};
use std::{fs, io, path::Path};

const PATH: &str = "config.toml";

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde(default = "db_sql_path")]
    pub db_sql_path: String,

    #[serde(default = "db_file_path")]
    pub db_file_path: String,

    #[serde(default = "db_thumbnail_path")]
    pub db_thumbnail_path: String,
}

fn db_sql_path() -> String {
    "./db/sqlite.db".to_string()
}

fn db_file_path() -> String {
    "./db/files".to_string()
}

fn db_thumbnail_path() -> String {
    "./db/thumbnails".to_string()
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
            .create_config_dirs()
            .expect("Failed to create config dirs?");

        config
    }

    fn create_config_dirs(&self) -> io::Result<()> {
        fs::create_dir_all(Path::new(&self.db_sql_path).parent().unwrap())?;
        fs::create_dir_all(Path::new(&self.db_file_path))?;
        Ok(())
    }
}
