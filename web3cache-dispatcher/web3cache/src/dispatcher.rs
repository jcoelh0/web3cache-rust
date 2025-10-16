use crate::{
  database::{delete_many, find_all, find_one, update_many, update_one},
  helper_functions::get_i64_from_doc,
};
use actix_http::header::HeaderValue;
use anyhow::Ok;
use async_trait::async_trait;
use bson::{oid::ObjectId, Document};
use log::{error, info};
use mongodb::{
  bson::doc,
  options::{FindOneOptions, FindOptions, UpdateOptions},
  results::UpdateResult,
  Database,
};
use reqwest::header::HeaderMap;
use serde_json::{json, Value};
use std::{
  cmp,
  collections::{HashMap, LinkedList},
  time::SystemTime,
};
use tokio::time::{sleep, Duration};

use hmac::{Hmac, Mac};
use jwt::{Header, SignWithKey, Token};
use sha2::Sha256;
use std::collections::BTreeMap;

use chrono::prelude::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct DelayTimes {
  increase_timeout: u64,
  wait_until: bson::DateTime,
}

const MAX_RETRIES: i64 = 15;

pub struct DispatcherData<'a> {
  pub queue_list: LinkedList<String>,
  pub queue_map: &'a mut HashMap<String, DelayTimes>,
}

pub fn generate_dates(a: u64, b: u64) -> (u128, u128, u128) {
  let current_date = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)
    .expect("Time went backwards")
    .as_millis();

  let new_date = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)
    .expect("Time went backwards")
    .as_millis()
    + a as u128;

  let sent_date = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)
    .expect("Time went backwards")
    .as_millis()
    + b as u128;
  (current_date, new_date, sent_date)
}

pub fn create_webhook_headers(
  sub_id: String,
  subscription: &Document,
) -> anyhow::Result<(HeaderMap, String)> {
  let dt: DateTime<Utc> = Utc::now();

  let header = Header {
    ..Default::default()
  };
  let mut claims = BTreeMap::new();

  let temp_date_isostring = dt.format("%+").to_string();
  let date_isostring = temp_date_isostring.as_str();

  let contract_id = subscription.get_str("contract_id").unwrap();

  claims.insert("contract_id", contract_id);
  claims.insert("timestamp", date_isostring);
  claims.insert("subcription_id", &sub_id);

  let key: Hmac<Sha256> = Hmac::new_from_slice(subscription.get_str("apikey").unwrap().as_bytes())?;
  let token = Token::new(header, claims)
    .sign_with_key(&key)
    .unwrap()
    .as_str()
    .to_string();
  let mut headers = HeaderMap::new();

  headers.insert(
    "Content-Type",
    HeaderValue::from_str("application/json").unwrap(),
  );
  headers.insert(
    "x-msl-webhook-id",
    HeaderValue::from_str(sub_id.as_str()).unwrap(),
  );
  headers.insert(
    "x-msl-webhook-type",
    HeaderValue::from_str("web3.standard.events.v1").unwrap(),
  );
  headers.insert(
    "x-msl-webhook-format",
    HeaderValue::from_str("JSON").unwrap(),
  );
  headers.insert(
    "x-msl-webhook-signature-type",
    HeaderValue::from_str("jwt.light.v1").unwrap(),
  );
  headers.insert("x-msl-webhook-nonce", HeaderValue::from_str("-1").unwrap());
  headers.insert(
    "x-msl-webhook-timestamp",
    HeaderValue::from_str(date_isostring).unwrap(),
  );
  headers.insert(
    "x-msl-webhook-jwt-signature",
    HeaderValue::from_str(token.as_str()).unwrap(),
  );
  Ok((headers, contract_id.to_string()))
}

#[async_trait]
pub trait Dispatcher {
  async fn fill_queue(&mut self, db: &Database) -> anyhow::Result<()>;
  async fn dispatch_transactions(
    &mut self,
    transactions: Vec<Value>,
    subscription: &Document,
    sub_id: String,
  ) -> anyhow::Result<bool>;
  async fn any_transaction_pending(
    &mut self,
    db: &Database,
    sub_id: String,
  ) -> anyhow::Result<bool>;
  async fn start_dispatcher(&mut self, db: &Database) -> anyhow::Result<()>;
  fn merge_queues(&mut self, new_items: Vec<String>) -> anyhow::Result<()>;
  async fn try_send_transactions(
    &mut self,
    db: &Database,
    sub_id: String,
    current_time_increase: u64,
  ) -> anyhow::Result<()>;
}

#[async_trait]
impl Dispatcher for DispatcherData<'_> {
  async fn fill_queue(&mut self, db: &Database) -> anyhow::Result<()> {
    info!("Filling queue");
    let filter = doc! { "isActive": true };
    let mut find_option = FindOptions::default();
    find_option.projection = Some(doc! { "_id": 1 });

    find_all(db.collection("subscriptions"), filter, find_option)
      .await
      .unwrap()
      .iter()
      .for_each(|tx| {
        //info!("doc: {:?}", tx);
        let id_str: &String = &tx.get_object_id("_id").unwrap().to_string();
        self.queue_list.push_back(id_str.to_string());

        self.queue_map.insert(
          id_str.to_string(),
          DelayTimes {
            increase_timeout: 100,
            wait_until: bson::DateTime::now(),
          },
        );
      });
    //_ = dispatcher_data.start_dispatcher(db).await;
    info!("self.queue_list: {:?}", self.queue_list);
    Ok(())
  }

  async fn start_dispatcher(&mut self, db: &Database) -> anyhow::Result<()> {
    info!(
      "start_dispatcher called, current items: {:?}",
      self.queue_list
    );

    loop {
      // inside loop
      let mut number_retries: i64 = MAX_RETRIES;
      while !self.queue_list.is_empty() {
        let next_sub_id: String = self.queue_list.pop_front().unwrap();
        let next_sub_id_clone = next_sub_id.clone();
        let next_sub_id_clone2 = next_sub_id_clone.clone();
        let next_item: &DelayTimes = self.queue_map.get(&next_sub_id).unwrap();
        //self.queue_map.remove(&next_sub_id);

        let increase_timeout = &next_item.increase_timeout;
        let wait_until = next_item.wait_until.to_chrono().timestamp_millis();
        let current_date = SystemTime::now()
          .duration_since(SystemTime::UNIX_EPOCH)
          .expect("Time went backwards")
          .as_millis();
        if current_date < wait_until.try_into().unwrap() {
          self.queue_list.push_back(next_sub_id);
          let next = next_item.clone();
          self.queue_map.insert(next_sub_id_clone, next);

          number_retries -= 1;
          if number_retries <= 0 {
            let filter = doc! { "isActive": true };
            let find_option = FindOptions::default();

            let subscriptions: Vec<String> =
              find_all(db.collection("subscriptions"), filter, find_option)
                .await
                .unwrap()
                .iter()
                .map(|subs| subs.get_object_id("_id").unwrap().to_string())
                .collect();
            _ = self.merge_queues(subscriptions);
            number_retries = MAX_RETRIES;
          }
          sleep(Duration::from_millis(50)).await;
          continue;
        }
        number_retries = MAX_RETRIES;
        let increase_timeout_clone = *increase_timeout;

        if self
          .any_transaction_pending(db, next_sub_id_clone)
          .await
          .unwrap()
        {
          self
            .try_send_transactions(db, next_sub_id_clone2, increase_timeout_clone)
            .await?;
        }

        sleep(Duration::from_millis(200)).await;
      }

      let init_queue = self.fill_queue(db);
      let sleep_1000 = sleep(Duration::from_millis(1000));

      info!("Queue cool off 1sec");
      let (_, _) = tokio::join!(init_queue, sleep_1000);
    }
  }

  fn merge_queues(&mut self, new_items: Vec<String>) -> anyhow::Result<()> {
    info!("merging queues!");

    for item in new_items {
      match self.queue_map.get(&item) {
        Some(_item) => {}
        None => {
          let item_clone: String = item.clone();
          self.queue_list.push_back(item);
          self.queue_map.insert(
            item_clone,
            DelayTimes {
              increase_timeout: 100,
              wait_until: bson::DateTime::now(),
            },
          );
        }
      }
    }

    info!("After: {:?}", self.queue_list);

    Ok(())
  }

  async fn try_send_transactions(
    &mut self,
    db: &Database,
    sub_id: String,
    current_time_increase: u64,
  ) -> anyhow::Result<()> {
    //info!("trySendTransactions called on sub_id {}", &sub_id);

    let filter = doc! { "subid": &sub_id };
    let mut find_option = FindOptions::default();
    find_option.sort = Some(doc! { "subid": 1, "block_number": 1 });
    find_option.limit = Some(50);
    //find_option.projection = Some(doc! {"_id": 0});

    let transaction_blocks_collection = db.collection("transactionblocks");
    let transaction_group = find_all(transaction_blocks_collection, filter, find_option)
      .await
      .unwrap();
    let transaction_group_clone = transaction_group.clone();

    info!(
      "transactionGroup to send in batch: {}",
      transaction_group.len()
    );

    let mut with_problems: bool = false;

    let (current_date, new_date, sent_date) = generate_dates(10000, 60000);

    let mut update_result: Option<UpdateResult> = None;
    if !transaction_group.is_empty() {
      update_result = Some(update_one(
            db.collection("transactionblocks"),
            doc! {
              "_id": transaction_group[0].get_object_id("_id").unwrap(),
              "locked_until": { "$lte": bson::DateTime::from_millis(current_date.try_into().unwrap()) }
            },
            doc! { "$set": { "locked_until": bson::DateTime::from_millis(new_date.try_into().unwrap()) } },
            UpdateOptions::default(),
          )
          .await
        .unwrap());
    }
    info!("update result: {:?}", update_result);
    let filter = doc! { "_id": ObjectId::parse_str(sub_id.as_str()).unwrap() };
    let find_option = FindOneOptions::default();

    let subscription = find_one(db.collection("subscriptions"), filter, find_option)
      .await
      .unwrap();

    if subscription.is_none() {
      delete_many(db.collection("transactionblocks"), doc! { "subid": sub_id })
        .await
        .unwrap();
      error!("Subscription does not exit anymore");
      return Ok(());
    }

    let mut transaction_vec: Vec<Value> = Vec::new();
    let mut ack_ids: Vec<ObjectId> = Vec::new();

    let is_locked = update_result.is_none() || update_result.unwrap().matched_count == 0;
    for item in &transaction_group_clone {
      let transaction_block = item;

      if is_locked {
        info!("Already lock");
        with_problems = true;

        break;
      }
      let transactions: Vec<Document> = transaction_block
        .get_array("transactions")
        .unwrap()
        .iter()
        .map(|tx| {
          let mut doc = tx.as_document().unwrap().to_owned();
          doc.remove("_id");
          doc
        })
        .collect();

      transaction_vec.push(json!({
          "transactions": transactions,
          "block_number":  get_i64_from_doc(&transactions[0], "block_number".to_string()),
          "event_name": transactions[0].get_str("event_name").unwrap(),
      }));

      ack_ids.push(transaction_block.get_object_id("_id").unwrap());
    }

    if !transaction_vec.is_empty() {
      if self
        .dispatch_transactions(transaction_vec, &subscription.unwrap(), sub_id.clone())
        .await
        .unwrap()
      {
        let _ = update_many(
          db.collection("transactionblocks"),
          doc! { "_id": { "$in": ack_ids.clone() } },
          doc! { "$set": { "locked_until": bson::DateTime::from_millis(sent_date.try_into().unwrap()) } },
          UpdateOptions::default(),
        )
        .await;

        _ = delete_many(
          db.collection("transactionblocks"),
          doc! { "_id": { "$in": ack_ids } },
        )
        .await;
      } else {
        error!("Failed to dispatch! ");
        let unlock_result = update_one(db.collection("transactionblocks"), doc! { "_id":  ack_ids[0] }, doc! { "$set": { "locked_until": bson::DateTime::from_millis(current_date.try_into().unwrap()) } }, UpdateOptions::default()).await;
        error!("unlock result: {:?}", unlock_result);
        with_problems = true;
      }
    } else {
      with_problems = true;
    }

    if self
      .any_transaction_pending(db, sub_id.clone())
      .await
      .unwrap()
    {
      //info!("Automatic added subid {} to the queue", sub_id.clone());

      let next_delay = if with_problems {
        cmp::min(current_time_increase * 2, 10000)
      } else {
        150
      };

      if !self.queue_map.contains_key(&sub_id.clone()) {
        self.queue_list.push_back(sub_id.clone());
      }
      self.queue_map.insert(
        sub_id,
        DelayTimes {
          increase_timeout: next_delay,
          wait_until: bson::DateTime::from_millis(
            (SystemTime::now()
              .duration_since(SystemTime::UNIX_EPOCH)
              .expect("Time went backwards")
              .as_millis()
              + u128::try_from(next_delay).unwrap())
            .try_into()
            .unwrap(),
          ),
        },
      );
    }

    Ok(())
  }

  async fn any_transaction_pending(
    &mut self,
    db: &Database,
    sub_id: String,
  ) -> anyhow::Result<bool> {
    let filter = doc! { "subid": &sub_id };
    let find_option = FindOneOptions::default();

    let result = find_one(db.collection("transactionblocks"), filter, find_option)
      .await
      .unwrap();

    Ok(result.is_some())
  }

  async fn dispatch_transactions(
    &mut self,
    transactions: Vec<Value>,
    subscription: &Document,
    sub_id: String,
  ) -> anyhow::Result<bool> {
    let (headers, contract_id) = create_webhook_headers(sub_id, subscription)?;

    let client = reqwest::Client::new();
    let res = client
      .post(subscription.get_str("url").unwrap())
      .headers(headers)
      .json(&json!({
        "metadata": {
          "contract_id": contract_id
        },
        "payload_count": transactions.len(),
        "payload": transactions
      }))
      .send()
      .await;

    let is_good = res.is_ok() && res.as_ref().unwrap().status().is_success();

    info!("transaction was sent? {}", is_good);
    if !is_good {
      info!("response was {:?}", res);
    }
    Ok(is_good)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use actix_http::header::HeaderValue;
  use anyhow::Result;
  use bson::doc;
  use jwt::{Header, Token};
  use std::collections::{HashMap, LinkedList};

  #[test]
  fn test_merge_queues_with_duplicates_isolated() -> anyhow::Result<()> {
    let mut queue_map = HashMap::new();
    let mut dispatcher_data = DispatcherData {
      queue_list: LinkedList::new(),
      queue_map: &mut queue_map,
    };
    let initial_items = vec!["item1".to_string(), "item2".to_string()];
    dispatcher_data.merge_queues(initial_items.clone())?;

    let new_items = vec![
      "item2".to_string(),
      "item3".to_string(),
      "item4".to_string(),
    ];
    dispatcher_data.merge_queues(new_items.clone())?;

    let expected_items = vec![
      "item1".to_string(),
      "item2".to_string(),
      "item3".to_string(),
      "item4".to_string(),
    ];

    assert_eq!(dispatcher_data.queue_list.len(), expected_items.len());
    for item in &expected_items {
      assert!(dispatcher_data.queue_list.contains(item));
      assert!(dispatcher_data.queue_map.contains_key(item));
    }

    // Ensure the duplicates are not inserted again
    let count_item1 = dispatcher_data
      .queue_list
      .iter()
      .filter(|&item| item == "item1")
      .count();
    let count_item2 = dispatcher_data
      .queue_list
      .iter()
      .filter(|&item| item == "item2")
      .count();

    assert_eq!(count_item1, 1);
    assert_eq!(count_item2, 1);
    Ok(())
  }

  #[test]
  fn test_create_webhook_headers() -> Result<()> {
    let sub_id = "123".to_string();
    let subscription = doc! {
        "contract_id": "contract123",
        "apikey": "supersecretapikey"
    };

    let (headers, contract_id) = create_webhook_headers(sub_id.clone(), &subscription)?;

    // Check contract_id
    assert_eq!(contract_id, "contract123");

    // Check headers
    assert_eq!(
      headers.get("Content-Type").unwrap(),
      HeaderValue::from_str("application/json").unwrap()
    );
    assert_eq!(
      headers.get("x-msl-webhook-id").unwrap(),
      HeaderValue::from_str(&sub_id).unwrap()
    );
    assert_eq!(
      headers.get("x-msl-webhook-type").unwrap(),
      HeaderValue::from_str("web3.standard.events.v1").unwrap()
    );
    assert_eq!(
      headers.get("x-msl-webhook-format").unwrap(),
      HeaderValue::from_str("JSON").unwrap()
    );
    assert_eq!(
      headers.get("x-msl-webhook-signature-type").unwrap(),
      HeaderValue::from_str("jwt.light.v1").unwrap()
    );
    assert_eq!(
      headers.get("x-msl-webhook-nonce").unwrap(),
      HeaderValue::from_str("-1").unwrap()
    );

    // Check x-msl-webhook-timestamp
    let timestamp = headers.get("x-msl-webhook-timestamp").unwrap().to_str()?;
    let _parsed_timestamp = chrono::DateTime::parse_from_rfc3339(timestamp)?;

    // Check x-msl-webhook-jwt-signature
    let token_str = headers
      .get("x-msl-webhook-jwt-signature")
      .unwrap()
      .to_str()?;

    let token_data: Token<Header, BTreeMap<String, String>, _> =
      Token::parse_unverified(token_str)?;

    assert_eq!(
      token_data.claims().get("contract_id").unwrap(),
      "contract123"
    );
    assert_eq!(token_data.claims().get("subcription_id").unwrap(), "123");

    Ok(())
  }

  #[test]
  fn test_generate_dates() {
    let a = 1000;
    let b = 2000;
    let (current_date, new_date, sent_date) = generate_dates(a, b);

    // Check if the new_date is in the expected range
    assert!(
          new_date >= current_date + a as u128 && new_date <= current_date + (a + 2) as u128,
          "The new_date should be greater than or equal to current_date + a and less than or equal to current_date + (a + 2)"
      );

    // Check if the sent_date is in the expected range
    assert!(
          sent_date >= current_date + b as u128 && sent_date <= current_date + (b + 2) as u128,
          "The sent_date should be greater than or equal to current_date + b and less than or equal to current_date + (b + 2)"
      );
  }
}
