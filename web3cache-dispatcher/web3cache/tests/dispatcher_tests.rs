use actix_http::header::HeaderValue;
use anyhow::Result;
use bson::doc;
use bson::oid::ObjectId;
use bson::Document;
use httpmock::Method::POST;
use httpmock::MockServer;
use jwt::{Header, Token};
use log::info;
use mongodb::options::{FindOneOptions, FindOptions, InsertManyOptions, InsertOneOptions};
use mongodb::Database;
use serde_json::json;
use serial_test::serial;
use std::collections::{BTreeMap, HashMap, LinkedList};
use std::str::FromStr;
use tokio;
use web3cache::database::{connect_to_mongodb_test, delete_many};
use web3cache::database::{find_all, find_one, insert_many};
use web3cache::dispatcher::*;

async fn cleanup_subscriptions(db: &Database, subscription_ids: &[ObjectId]) {
  let collection = db.collection::<Document>("subscriptions");
  for id in subscription_ids {
    let filter = doc! { "_id": id.clone() };
    collection.delete_one(filter, None).await.unwrap();
  }
}

async fn cleanup_transactions(db: &Database, transaction_ids: &[ObjectId]) {
  let collection = db.collection::<Document>("transactionblocks");
  for transaction_id in transaction_ids {
    let filter = doc! { "_id": transaction_id.clone() };
    collection.delete_one(filter, None).await.unwrap();
  }
}

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

  let token_data: Token<Header, BTreeMap<String, String>, _> = Token::parse_unverified(token_str)?;

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

#[tokio::test]
#[serial]
async fn test_fill_queue_various_subscriptions() {
  let db = connect_to_mongodb_test().await.unwrap();
  let collection = db.collection::<Document>("subscriptions");

  let db_name = db.name();

  // Check if the database name is "test"
  if db_name == "test" {
    // Delete all documents in the collection if the db is "test"
    let delete_result = delete_many(collection.clone(), doc! {}).await;
    match delete_result {
      Ok(delete_response) => info!("Deleted {} documents", delete_response.deleted_count),
      Err(e) => info!("Error occurred during deletion: {}", e),
    }
  }

  // Test with 0 subscriptions
  let mut dispatcher_data = DispatcherData {
    queue_list: LinkedList::new(),
    queue_map: &mut HashMap::new(),
  };
  let result = dispatcher_data.fill_queue(&db).await;
  assert!(result.is_ok());
  assert!(dispatcher_data.queue_list.is_empty());

  // Add 1 subscription to the database
  let subscription = doc! { "isActive": true };
  let insert_result_1 = collection
    .insert_one(subscription.clone(), None)
    .await
    .unwrap();
  let subscription_id_1 = insert_result_1.inserted_id.as_object_id().unwrap().clone();

  // Test with 1 subscription
  let result = dispatcher_data.fill_queue(&db).await;
  assert!(result.is_ok());
  assert_eq!(dispatcher_data.queue_list.len(), 1);

  // Add 1 more subscription to the database (total of 2 subscriptions)
  let insert_result_2 = collection
    .insert_one(subscription.clone(), None)
    .await
    .unwrap();
  let subscription_id_2 = insert_result_2.inserted_id.as_object_id().unwrap().clone();
  dispatcher_data.queue_list.clear();
  dispatcher_data.queue_map.clear();

  // Test with 2 subscriptions
  let result = dispatcher_data.fill_queue(&db).await;
  assert!(result.is_ok());
  assert_eq!(dispatcher_data.queue_list.len(), 2);

  // Clean up the test data
  cleanup_subscriptions(&db, &[subscription_id_1, subscription_id_2]).await;
}

#[tokio::test]
#[serial]
async fn test_try_send_transactions() {
  let db = connect_to_mongodb_test().await.unwrap();

  // Create the test subscription
  let subscription = doc! {
      "url": "http://localhost:2313/",
      "topics": ["topic3"],
      "contract_id": "test-dispatcher-subscription-id",
      "apikey" : "test_dispatcher"
  };
  db.collection("subscriptions")
    .insert_one(subscription.clone(), InsertOneOptions::default())
    .await
    .unwrap();
  let sub_id = find_one(
    db.collection("subscriptions"),
    doc! {"contract_id":"test-dispatcher-subscription-id"},
    FindOneOptions::default(),
  )
  .await
  .unwrap()
  .unwrap()
  .get_object_id("_id")
  .unwrap()
  .to_string();
  let transaction_blocks = vec![
    doc! {
        "subid": ObjectId::parse_str(&sub_id).unwrap(),
        "transactions": [
            {
                "transaction_id": "tx1",
                "block_number": 123,
                "event_name":"topic3"
            }
        ]
    },
    doc! {
        "subid": ObjectId::parse_str(&sub_id).unwrap(),
        "transactions": [
            {
                "transaction_id": "tx2",
                "block_number": 456,
                "event_name":"topic3"
            }
        ]
    },
  ];

  let transaction_blocks_collection = db.collection("transactionblocks");
  let insert_many_options = InsertManyOptions::default();
  insert_many(
    transaction_blocks_collection,
    &transaction_blocks,
    insert_many_options,
  )
  .await
  .unwrap();
  let mut dispatcher_data = DispatcherData {
    queue_list: LinkedList::new(),
    queue_map: &mut HashMap::new(),
  };

  // Call the try_send_transactions function
  dispatcher_data
    .try_send_transactions(&db, sub_id.clone(), 150)
    .await
    .unwrap();

  // Assert that the transactions were locked and unlocked correctly
  let transaction_blocks_collection = db.collection("transactionblocks");
  let transaction_blocks = find_all(
    transaction_blocks_collection.clone(),
    doc! { "subid": &sub_id },
    FindOptions::default(),
  )
  .await
  .unwrap();

  for transaction_block in transaction_blocks {
    assert_eq!(transaction_block.get_str("transaction_id").unwrap(), "tx1");
    assert!(transaction_block.get_datetime("locked_until").is_ok());
  }

  // Assert that the transactions were deleted after being dispatched successfully
  let remaining_transactions = find_all(
    transaction_blocks_collection,
    doc! { "subid": &sub_id },
    FindOptions::default(),
  )
  .await
  .unwrap();
  assert!(remaining_transactions.is_empty());
  cleanup_subscriptions(
    &db,
    &[ObjectId::from_str(&sub_id).expect("Failed to parse string to ObjectId")].to_vec(),
  )
  .await;
}

#[tokio::test]
#[serial]
async fn test_try_send_transactions_no_transactions() {
  let db = connect_to_mongodb_test().await.unwrap();
  let subscription = doc! {
      "url": "http://localhost:2313/",
      "topics": ["topic3"],
      "contract_id": "test-dispatcher-no-subscription-id",
      "isActive":true,
      "apikey" : "test_dispatcher_empty"
  };
  db.collection("subscriptions")
    .insert_one(subscription.clone(), InsertOneOptions::default())
    .await
    .unwrap();
  let sub_id = find_one(
    db.collection("subscriptions"),
    doc! {"contract_id":"test-dispatcher-no-subscription-id"},
    FindOneOptions::default(),
  )
  .await
  .unwrap()
  .unwrap()
  .get_object_id("_id")
  .unwrap()
  .to_string();
  let mut dispatcher_data = DispatcherData {
    queue_list: LinkedList::new(),
    queue_map: &mut HashMap::new(),
  };

  // Call the try_send_transactions function with no transactions
  let result = dispatcher_data
    .try_send_transactions(&db, sub_id.clone(), 150)
    .await;

  assert!(result.is_ok(), "No transactions test failed");
  cleanup_subscriptions(
    &db,
    &[ObjectId::from_str(&sub_id).expect("Failed to parse string to ObjectId")].to_vec(),
  )
  .await;
}

#[tokio::test]
#[serial]
async fn test_try_send_transactions_subscription_not_exist() {
  let db = connect_to_mongodb_test().await.unwrap();

  let subscription = doc! {
      "url": "http://localhost:2313/",
      "topics": ["topic3"],
      "contract_id": "test-dispatcher-no-sub",
      "apikey" : "test_dispatcher",
      "isActive":true
  };
  db.collection("subscriptions")
    .insert_one(subscription.clone(), InsertOneOptions::default())
    .await
    .unwrap();
  let sub_id = find_one(
    db.collection("subscriptions"),
    doc! {"contract_id":"test-dispatcher-no-sub"},
    FindOneOptions::default(),
  )
  .await
  .unwrap()
  .unwrap()
  .get_object_id("_id")
  .unwrap()
  .to_string();
  cleanup_subscriptions(
    &db,
    &[ObjectId::from_str(&sub_id).expect("Failed to parse string to ObjectId")].to_vec(),
  )
  .await;
  let transaction_blocks = vec![
    doc! {
        "subid": ObjectId::parse_str(&sub_id).unwrap(),
        "transactions": [
            {
                "transaction_id": "tx1",
                "block_number": 123,
                "event_name":"topic3"
            }
        ]
    },
    doc! {
        "subid": ObjectId::parse_str(&sub_id).unwrap(),
        "transactions": [
            {
                "transaction_id": "tx2",
                "block_number": 456,
                "event_name":"topic3"
            }
        ]
    },
  ];

  let transaction_blocks_collection = db.collection("transactionblocks");
  let insert_many_options = InsertManyOptions::default();
  insert_many(
    transaction_blocks_collection.clone(),
    &transaction_blocks,
    insert_many_options,
  )
  .await
  .unwrap();

  let mut dispatcher_data = DispatcherData {
    queue_list: LinkedList::new(),
    queue_map: &mut HashMap::new(),
  };

  // Call the try_send_transactions function with a nonexistent subscription
  let result = dispatcher_data
    .try_send_transactions(&db, sub_id.clone(), 150)
    .await;

  assert!(result.is_ok());
  let remaining_transactions = find_all(
    transaction_blocks_collection,
    doc! { "subid": &sub_id },
    FindOptions::default(),
  )
  .await
  .unwrap();
  assert!(remaining_transactions.is_empty());
}

#[tokio::test]
#[serial]
async fn test_try_send_transactions_no_pending_transactions() {
  let db = connect_to_mongodb_test().await.unwrap();
  let subscription = doc! {
      "url": "http://localhost:2313/",
      "topics": ["topic3"],
      "contract_id": "test-dispatcher-no-pending-tranasactions",
      "apikey" : "test_dispatcher",
      "isActive":true
  };
  // Create a subscription in the database
  db.collection("subscriptions")
    .insert_one(subscription.clone(), InsertOneOptions::default())
    .await
    .unwrap();
  let sub_id = find_one(
    db.collection("subscriptions"),
    doc! {"contract_id":"test-dispatcher-no-pending-tranasactions"},
    FindOneOptions::default(),
  )
  .await
  .unwrap()
  .unwrap()
  .get_object_id("_id")
  .unwrap()
  .to_string();

  let mut dispatcher_data = DispatcherData {
    queue_list: LinkedList::new(),
    queue_map: &mut HashMap::new(),
  };

  // Call the try_send_transactions function with a subscription that has no pending transactions
  let result = dispatcher_data
    .try_send_transactions(&db, sub_id.clone(), 150)
    .await;
  assert!(result.is_ok(), "No pending transactions test failed");

  cleanup_subscriptions(
    &db,
    &[ObjectId::from_str(&sub_id).expect("Failed to parse string to ObjectId")].to_vec(),
  )
  .await;
}

#[tokio::test]
#[serial]
async fn test_dispatch_transactions() {
  // Start a mock HTTP server
  let mock_server = MockServer::start();
  let db = connect_to_mongodb_test().await.unwrap();
  let subscription = doc! {
     "url": format!("{}webhook", mock_server.url("/")),
      "topics": ["topic3"],
      "contract_id": "test-dispatch-transactions",
      "apikey" : "test_dispatcher",
      "isActive":true
  };
  // Create a subscription in the database
  db.collection("subscriptions")
    .insert_one(subscription.clone(), InsertOneOptions::default())
    .await
    .unwrap();
  let sub_id = find_one(
    db.collection("subscriptions"),
    doc! {"contract_id":"test-dispatch-transactions"},
    FindOneOptions::default(),
  )
  .await
  .unwrap()
  .unwrap()
  .get_object_id("_id")
  .unwrap()
  .to_string();

  // Mock the external HTTP service
  let webhook_mock = mock_server.mock(|when, then| {
    when.method(POST).path("/webhook");
    then.status(200);
  });

  // Create a transaction vector
  let transactions = vec![
    json!({ "tx_hash": "0x123", "value": 100 }),
    json!({ "tx_hash": "0x456", "value": 200 }),
  ];

  // Call the dispatch_transactions function
  let mut dispatcher_data = DispatcherData {
    queue_list: LinkedList::new(),
    queue_map: &mut HashMap::new(),
  };
  let result = dispatcher_data
    .dispatch_transactions(transactions, &subscription, sub_id.clone())
    .await;

  // Check if the result is Ok and returns true
  assert!(result.is_ok());
  assert!(result.unwrap());

  // Ensure that the mock was called
  webhook_mock.assert();
  //clean up subscription
  cleanup_subscriptions(
    &db,
    &[ObjectId::from_str(&sub_id).expect("Failed to parse string to ObjectId")].to_vec(),
  )
  .await;
}

#[tokio::test]
async fn test_any_transaction_pending_no_transactions() {
  let db = connect_to_mongodb_test().await.unwrap();

  let sub_id = "test_no_transations".to_string();

  let mut dispatcher_data = DispatcherData {
    queue_list: LinkedList::new(),
    queue_map: &mut HashMap::new(),
  };
  let result = dispatcher_data.any_transaction_pending(&db, sub_id).await;

  assert!(result.is_ok());
  assert!(!result.unwrap());
}

#[tokio::test]
async fn test_any_transaction_pending_with_transactions() {
  let db = connect_to_mongodb_test().await.unwrap();

  // Insert a transaction into the 'transactionblocks' collection
  let sub_id = "test_sub_id".to_string();
  let transaction_data = doc! {
      "subid": sub_id.clone(),
      "transactions": [
          {
              "transaction_id": "tx1",
              "block_number": 123,
              "event_name":"topic3"
          }
      ]
  };
  let insert_result = db
    .collection::<Document>("transactionblocks")
    .insert_one(transaction_data.clone(), InsertOneOptions::default())
    .await
    .unwrap();
  let transaction_id = insert_result.inserted_id.as_object_id().unwrap().clone();

  let mut dispatcher_data = DispatcherData {
    queue_list: LinkedList::new(),
    queue_map: &mut HashMap::new(),
  };
  let result = dispatcher_data
    .any_transaction_pending(&db, sub_id.clone())
    .await;
  assert!(result.is_ok());
  assert!(result.unwrap());

  // Clean up the test data
  cleanup_transactions(&db, &[transaction_id]).await;
}
