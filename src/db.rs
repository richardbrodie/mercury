use chrono::{DateTime, Utc};
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel::{self, select, PgConnection};
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use std::collections::HashMap;
use std::{env, thread};

use super::models::{SubscribedFeed, SubscribedItem, User};
use super::schema::{
  feeds, feeds::dsl::*, items, subscribed_feeds, subscribed_feeds::dsl::*, subscribed_items,
  users::dsl::*, users::dsl::*,
};
use super::views::{subscribed_feeds_with_count_view, subscribed_items_view};

lazy_static! {
  static ref POOL: Pool<ConnectionManager<PgConnection>> = {
    let pg_user = env::var("PG_USER").expect("PG_USER must be set");
    let pg_pass = env::var("PG_PASS").expect("PG_PASS must be set");
    let db_host = env::var("DB_HOST").expect("DB_HOST must be set");
    let pg_db = env::var("PG_DB").expect("PG_DB must be set");
    let database_url = format!("postgres://{}:{}@{}/{}", pg_user, pg_pass, db_host, pg_db);
    info!("database url: {}", database_url);

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
      .build(manager)
      .expect("Failed to create pool.")
  };
}

pub fn establish_pool() -> Pool<ConnectionManager<PgConnection>> {
  POOL.clone()
}

// seed admin user
pub fn create_admin_user() {
  let pool = establish_pool();
  let connection = pool.get().unwrap();

  match select(exists(users.filter(username.eq("admin")))).get_result::<bool>(&*connection) {
    Ok(true) => (),
    Ok(false) => {
      let admin_pass = env::var("ADMIN_PASS").expect("ADMIN_PASS must be set");
      let pwh = User::hash_pw(&admin_pass);
      diesel::insert_into(users)
        .values((username.eq("admin"), password_hash.eq(&pwh.as_bytes())))
        .load::<User>(&*connection)
        .expect("Error creating admin user");
    }
    Err(_) => error!("could not check if admin user existed"),
  }
}

// channels

pub fn find_feed_by_url(url: &str) -> Option<Feed> {
  let pool = establish_pool();
  let connection = pool.get().unwrap();
  match feeds.filter(feed_link.eq(url)).first::<Feed>(&*connection) {
    Ok(ch) => Some(ch),
    Err(_) => None,
  }
}

pub fn get_feed_id(url: &str) -> Result<i32, diesel::result::Error> {
  let pool = establish_pool();
  let connection = pool.get().unwrap();
  feeds
    .filter(feed_link.eq(url))
    .select(feeds::id)
    .first(&*connection)
}

pub fn insert_channel(channel: NewFeed) -> Feed {
  let pool = establish_pool();
  let connection = pool.get().unwrap();

  diesel::insert_into(feeds::table)
    .values(&channel)
    .get_result::<Feed>(&*connection)
    .expect("Error saving new post")
}

// used during update loop
pub fn get_channel_urls_and_subscribers() -> Vec<(i32, String, Vec<i32>)> {
  let pool = establish_pool();
  let connection = pool.get().unwrap();

  let subscribed = subscribed_feeds::table
    .select((subscribed_feeds::feed_id, subscribed_feeds::user_id))
    .load::<(i32, i32)>(&*connection)
    .unwrap();

  let mut h: HashMap<i32, Vec<i32>> = HashMap::new();
  subscribed.iter().for_each(|x| {
    h.entry(x.0)
      .and_modify(|e| e.push(x.1))
      .or_insert(vec![x.1]);
  });

  let f = feeds::table
    .select((feeds::id, feeds::feed_link))
    .load::<(i32, String)>(&*connection)
    .unwrap();

  let res: Vec<(i32, String, Vec<i32>)> = f
    .into_iter()
    .map(|(i, u)| (i, u, h.remove(&i).unwrap()))
    .collect();
  res
}

//items

pub fn insert_items(items: &Vec<NewItem>) -> Option<Vec<Item>> {
  debug!("found {} new items", items.len());
  let pool = establish_pool();
  let connection = pool.get().unwrap();
  diesel::insert_into(items::table)
    .values(items)
    .get_results::<Item>(&*connection)
    .ok()
}

pub fn update_item(iid: i32, item: NewItem) {
  let pool = establish_pool();
  let connection = pool.get().unwrap();
  diesel::update(items::table.find(iid))
    .set((
      items::title.eq(item.title),
      items::link.eq(item.link),
      items::summary.eq(item.summary),
      items::published_at.eq(item.published_at),
      items::content.eq(item.content),
    )).execute(&*connection)
    .expect("failed to update item");
}

pub fn find_duplicates(guids: Vec<&str>) -> Option<Vec<(i32, String, Option<DateTime<Utc>>)>> {
  let pool = establish_pool();
  let connection = pool.get().unwrap();
  let results = items::table
    .filter(items::guid.eq_any(guids))
    .select((items::id, items::guid, items::published_at))
    .load(&*connection)
    .expect("Error loading items");
  match results.is_empty() {
    true => None,
    false => Some(results),
  }
}

pub fn get_item_ids(fid: &i32) -> Option<Vec<i32>> {
  let pool = establish_pool();
  let connection = pool.get().unwrap();
  match items::table
    .filter(items::feed_id.eq(fid))
    .select(items::id)
    .load(&*connection)
  {
    Ok(i) => Some(i),
    Err(_) => None,
  }
}

pub fn get_latest_item_date(fid: i32) -> Option<DateTime<Utc>> {
  let pool = establish_pool();
  let connection = pool.get().unwrap();
  match items::table
    .filter(items::feed_id.eq(fid))
    .order(items::published_at.desc())
    .first::<Item>(&*connection)
  {
    Ok(item) => item.published_at,
    Err(_) => None,
  }
}

// users

pub fn get_user(uname: &str) -> Option<User> {
  let pool = establish_pool();
  let connection = pool.get().unwrap();
  match users.filter(username.eq(uname)).first::<User>(&*connection) {
    Ok(user) => Some(user),
    Err(_) => None,
  }
}

pub fn create_user(uname: &str, pw_hash: &str) -> Result<usize, diesel::result::Error> {
  let pool = establish_pool();
  let connection = pool.get().unwrap();
  diesel::insert_into(users)
    .values((username.eq(uname), password_hash.eq(pw_hash.as_bytes())))
    .execute(&*connection)
}

// subscribed_feeds

pub fn subscribe_feed(uid: &i32, fid: &i32) {
  let pool = establish_pool();
  let connection = pool.get().unwrap();

  match diesel::insert_into(subscribed_feeds)
    .values((feed_id.eq(fid), user_id.eq(uid)))
    .execute(&*connection)
  {
    Ok(_) => info!("subscribed: '{}' by '{}'", fid, uid),
    Err(e) => error!("subscribe failure: '{}' by '{}': {}", fid, uid, e),
  };
}

pub fn get_subscribed_feed(uid: &i32, fid: &i32) -> Option<SubscribedFeed> {
  let pool = establish_pool();
  let connection = pool.get().unwrap();
  subscribed_feeds_with_count_view::table
    .filter(subscribed_feeds_with_count_view::user_id.eq(uid))
    .filter(subscribed_feeds_with_count_view::id.eq(fid))
    .first::<SubscribedFeed>(&*connection)
    .ok()
}

pub fn get_subscribed_feeds(uid: &i32) -> Option<Vec<SubscribedFeed>> {
  let pool = establish_pool();
  let connection = pool.get().unwrap();
  subscribed_feeds_with_count_view::table
    .filter(subscribed_feeds_with_count_view::user_id.eq(uid))
    .order(subscribed_feeds_with_count_view::title.asc())
    .load::<SubscribedFeed>(&*connection)
    .ok()
}

pub fn get_subscribed_items(
  fid: i32,
  uid: i32,
  updated: Option<DateTime<Utc>>,
) -> Option<Vec<SubscribedItem>> {
  let pool = establish_pool();
  let handle = thread::spawn(move || {
    let connection = pool.get().unwrap();
    let mut query = subscribed_items_view::table
      .filter(subscribed_items_view::feed_id.eq(fid))
      .filter(subscribed_items_view::user_id.eq(uid))
      .order(subscribed_items_view::published_at.desc())
      .into_boxed();
    if let Some(d) = updated {
      query = query.filter(subscribed_items_view::published_at.lt(d))
    }
    query.limit(50).load::<SubscribedItem>(&*connection).ok()
  });
  handle.join().unwrap()
}

pub fn get_subscribed_item(iid: i32, uid: i32) -> Option<SubscribedItem> {
  let pool = establish_pool();
  let handle = thread::spawn(move || {
    let connection = pool.get().unwrap();

    match subscribed_items_view::table
      .filter(subscribed_items_view::id.eq(iid))
      .filter(subscribed_items_view::user_id.eq(uid))
      .first::<SubscribedItem>(&*connection)
    {
      Ok(item) => {
        diesel::update(
          subscribed_items::table.filter(subscribed_items::id.eq(item.subscribed_item_id)),
        ).set(subscribed_items::seen.eq(true))
        .execute(&*connection)
        .expect("Failed to update 'seen' status");
        Some(item)
      }
      Err(_) => None,
    }
  });
  handle.join().unwrap()
}

pub fn mark_subscribed_item_as_read(iid: i32) {
  let pool = establish_pool();
  let connection = pool.get().unwrap();

  diesel::update(subscribed_items::table.filter(subscribed_items::id.eq(iid)))
    .set(subscribed_items::seen.eq(true))
    .execute(&*connection)
    .expect("Failed to update 'seen' status");
}

pub fn insert_subscribed_items(items: Vec<(&i32, &i32, bool)>) {
  let insertables: Vec<_> = items
    .iter()
    .map(|i| {
      (
        subscribed_items::user_id.eq(i.0),
        subscribed_items::item_id.eq(i.1),
        subscribed_items::seen.eq(i.2),
      )
    }).collect();

  let pool = establish_pool();
  let connection = pool.get().unwrap();
  diesel::insert_into(subscribed_items::table)
    .values(insertables)
    .execute(&*connection)
    .expect("Error saving new post");
}
