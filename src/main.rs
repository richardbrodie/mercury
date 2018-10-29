// #![allow(unused)]
extern crate base64;
extern crate dirs;
extern crate dotenv;
#[macro_use]
extern crate lazy_static;
extern crate log;
extern crate pretty_env_logger;
extern crate regex;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde_yaml;

mod types;

use types::*;

use std::env;
use std::sync::Arc;

use dotenv::dotenv;

lazy_static! {
    static ref CONFIG: Arc<Config> = Arc::new(Config::load_config());
}

fn main() {
    dotenv().ok();
    env::set_var("RUST_LOG", "mercury=info");
    pretty_env_logger::init();
}
