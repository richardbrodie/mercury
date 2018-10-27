#![allow(unused)]
#![allow(proc_macro_derive_resolution_fallback)]
extern crate base64;
extern crate chrono;
#[macro_use]
extern crate diesel;
extern crate dotenv;
#[macro_use]
extern crate log;
extern crate futures;
extern crate hermod;
extern crate jsonwebtoken;
#[macro_use]
extern crate lazy_static;
extern crate pretty_env_logger;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate regex;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate sha2;
extern crate tokio;
extern crate tokio_fs;
extern crate tokio_io;
extern crate url;

use dotenv::dotenv;
use hermod::futures::start_fetch_loop;
use hermod::models::Feed;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};

// pub mod db;
// pub mod feed;
// pub mod models;
// pub mod schema;
// pub mod views;

fn main() {
    dotenv().ok();
    env::set_var("RUST_LOG", "mercury=info");
    pretty_env_logger::init();

    let feeds = vec![
        "https://lorem-rss.herokuapp.com/feed".to_owned(),
        "https://feeds.feedburner.com/cyclingtipsblog/TJog".to_owned(),
    ];
    let feed_state = Arc::new(Mutex::new(feeds));
    let func = |s: Feed| println!("fetched: {}", s.channel.title);
    let work = start_fetch_loop(feed_state, 60, func);
    tokio::run(work);
}
