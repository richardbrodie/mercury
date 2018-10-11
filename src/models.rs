use base64::{decode, encode};
use chrono::{DateTime, Utc};
use hermod::models::Item;
use sha2::{Digest, Sha256};
use std::str;

use super::db::get_user;
use super::schema::*;

//////////////////
// Subscription //
//////////////////

#[derive(Debug, Queryable, Serialize)]
pub struct SubscribedItem {
  pub id: i32,
  #[serde(skip_serializing)]
  pub guid: String,
  pub link: String,
  pub title: String,
  pub summary: Option<String>,
  pub content: Option<String>,
  pub published_at: Option<DateTime<Utc>>,
  pub updated_at: Option<DateTime<Utc>>,
  pub feed_id: i32,
  pub subscribed_item_id: i32,
  pub user_id: i32,
  pub seen: bool,
}

#[derive(Debug, Queryable, Serialize, Associations)]
#[belongs_to(User)]
pub struct SubscribedFeed {
  pub id: i32,
  pub title: String,
  pub description: Option<String>,
  pub site_link: String,
  pub feed_link: String,
  pub updated_at: DateTime<Utc>,
  pub user_id: i32,
  pub unseen_count: i32,
}

///////////////
// Composite //
///////////////

#[derive(Debug, Serialize, Clone)]
pub struct CompositeItem {
  pub id: i32,
  pub title: String,
  pub link: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub summary: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content: Option<String>,
  pub published_at: Option<DateTime<Utc>>,
  pub updated_at: Option<DateTime<Utc>>,
  pub seen: bool,
}
impl CompositeItem {
  pub fn from_item(item: &Item) -> Self {
    CompositeItem {
      id: item.id,
      title: item.title.clone(),
      link: item.link.clone(),
      summary: item.summary.clone(),
      content: item.content.clone(),
      published_at: item.published_at,
      updated_at: item.updated_at,
      seen: false,
    }
  }
  pub fn from_subscribed(item: &SubscribedItem) -> Self {
    CompositeItem {
      id: item.id,
      title: item.title.clone(),
      link: item.link.clone(),
      summary: item.summary.clone(),
      content: item.content.clone(),
      published_at: item.published_at,
      updated_at: item.updated_at,
      seen: item.seen,
    }
  }
}

//////////
// User //
//////////

#[derive(Debug, Queryable, Associations, Identifiable, Serialize)]
pub struct User {
  pub id: i32,
  pub username: String,
  pub password_hash: Vec<u8>,
}
impl User {
  pub fn check_user(username: &str, pass: &str) -> Option<User> {
    match get_user(username) {
      Some(user) => match user.verifies(pass) {
        true => Some(user),
        false => None,
      },
      None => None,
    }
  }

  pub fn hash_pw(s: &str) -> String {
    let mut hasher = Sha256::default();
    hasher.input(s.as_bytes());
    let output = hasher.result();
    let hash = &output[..];
    let e = encode(hash);
    e
  }

  fn verifies(&self, pass: &str) -> bool {
    let orig_hash = decode(&self.password_hash).unwrap();
    let mut hasher = Sha256::default();
    hasher.input(pass.as_bytes());
    let output = hasher.result();
    let hashed_pw = &output[..];
    orig_hash == hashed_pw
  }
}

////////////
// Claims //
////////////

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
  pub name: String,
  pub id: i32,
}

fn parse_date(date: &str) -> Option<DateTime<Utc>> {
  match DateTime::parse_from_rfc2822(date) {
    Ok(d) => Some(d.with_timezone(&Utc)),
    Err(_) => date.parse::<DateTime<Utc>>().ok(),
  }
}
