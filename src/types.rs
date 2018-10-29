use dirs;
use std::env;
use std::fs::{self, File};
use std::path::PathBuf;

pub struct Feed {
    filename: String,
}

pub struct Item {
    filename: String,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    feed_path: PathBuf,
}
impl Config {
    pub fn defaults() -> Self {
        let c = Config {
            feed_path: Self::default_feed_dir(),
        };
        c.check_feed_dir();
        c
    }

    fn check_feed_dir(&self) {
        if !self.feed_path.exists() {
            fs::create_dir(&self.feed_path);
        }
    }

    fn default_feed_dir() -> PathBuf {
        dirs::data_dir().unwrap().join(env!("CARGO_PKG_NAME"))
    }

    pub fn load_config() -> Self {
        let path = dirs::config_dir()
            .unwrap()
            .join(env!("CARGO_PKG_NAME"))
            .join("config.yml");
        if !path.exists() {
            return Self::defaults();
        }
        let conf_str = fs::read_to_string(&path).unwrap();
        let config: Config = serde_yaml::from_str(&conf_str).unwrap();
        config.check_feed_dir();
        config
    }
}
