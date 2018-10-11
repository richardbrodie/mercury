use futures::future::IntoFuture;
use std::io::BufReader;
use std::option::Option;
use std::str;
use std::time::{Duration, Instant};
use tokio::timer::Interval;

// use super::db::{
//   find_duplicates, get_channel_urls_and_subscribers, get_feed_id, get_item_ids,
//   get_subscribed_items, insert_channel, insert_items, insert_subscribed_items, subscribe_feed,
//   update_item,
// };
use super::models::CompositeItem;

////////////////////////
/// Future sequences ///
////////////////////////

pub fn start_interval_loops() {
  let update_subscriptions = Interval::new(Instant::now(), Duration::from_secs(300))
    .for_each(move |_| {
      get_channel_urls_and_subscribers().into_iter().for_each(
        |(feed_id, feed_url, subscriber_ids)| {
          let sid = subscriber_ids.clone();
          let work = update_feed(feed_id, feed_url, subscriber_ids).and_then(move |new_items| {
            match new_items {
              Some(items) => {
                debug!("found {} new items for {}", items.len(), &feed_id);
              }
              None => (),
            };
            Ok(())
          });
          rt::spawn(work);
        },
      );
      Ok(())
    }).map_err(|e| panic!("delay errored; err={:?}", e));
  rt::spawn(update_subscriptions);
}

pub fn subscribe(url: String, user_id: i32) {
  debug!("subscribing: '{}' by '{}'", url, user_id);
  let work = get_feed_id(&url)
    .into_future()
    .and_then(|feed_id| {
      debug!("in db: '{}'", feed_id);
      Ok((feed_id, get_item_ids(&feed_id)))
    }).or_else(|_| {
      debug!("not in db: '{}'", url);
      add_feed(url)
    }).and_then(move |(feed_id, item_ids)| {
      subscribe_feed(&user_id, &feed_id);
      Ok((feed_id, item_ids))
    }).and_then(move |(feed_id, item_ids)| {
      match item_ids {
        Some(item_ids) => subscribe_new_items(&item_ids, &vec![user_id]),
        None => (),
      };
      Ok(())
    });
  rt::spawn(work);
}

pub fn update_feed(
  feed_id: i32,
  channel_url: String,
  subscriber_ids: Vec<i32>,
) -> impl Future<Item = Option<Vec<Item>>, Error = ()> {
  let local = channel_url.clone();
  fetch_feed(channel_url)
    .and_then(|data| parse_fetched_data(&data))
    .and_then(move |data| handle_feed_types(data, &local))
    .and_then(move |(_, items)| Ok(handle_item_types(items, &feed_id)))
    .and_then(|items| Ok(process_duplicates(items)))
    .and_then(move |new_items| match new_items {
      Some(items) => {
        let items = insert_items(&items).unwrap();
        let item_ids = items.iter().map(|i| i.id).collect();
        subscribe_new_items(&item_ids, &subscriber_ids);
        Ok(Some(items))
      }
      None => Ok(None),
    })
}

fn subscribe_new_items(inserted_items: &Vec<i32>, subscribers: &Vec<i32>) {
  let insertables: Vec<(&i32, &i32, bool)> = subscribers
    .iter()
    .flat_map(|s| {
      inserted_items
        .iter()
        .map(move |i| (s, i, false))
        .collect::<Vec<(&i32, &i32, bool)>>()
    }).collect::<Vec<(&i32, &i32, bool)>>();
  insert_subscribed_items(insertables);
}

fn process_duplicates(items: Vec<NewItem>) -> Option<Vec<NewItem>> {
  let new_items = match find_duplicates(items.iter().map(|x| x.guid.as_str()).collect()) {
    Some(dupes) => {
      let guids: Vec<&str> = dupes.iter().map(|x| x.1.as_str()).collect();
      let (new_items, mut duplicated_items): (Vec<NewItem>, Vec<NewItem>) = items
        .into_iter()
        .partition(|x| !guids.contains(&x.guid.as_str()));

      let updated_items: Vec<(i32, NewItem)> = duplicated_items
        .into_iter()
        .filter_map(|d| {
          let idx = dupes.iter().find(|(_, y, _)| y == &d.guid).unwrap();
          if d.published_at != idx.2 {
            Some((idx.0, d))
          } else {
            None
          }
        }).collect();
      debug!("found {} updated items", updated_items.len());
      updated_items
        .into_iter()
        .for_each(|(id, item)| update_item(id, item));
      new_items
    }
    None => items,
  };
  match new_items.is_empty() {
    false => Some(new_items),
    true => None,
  }
}
