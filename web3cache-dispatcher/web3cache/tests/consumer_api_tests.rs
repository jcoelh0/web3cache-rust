use bson::{doc, oid::ObjectId, Document};
use serde_json::json;
use std::str::FromStr;
use web3cache::consumer_api::*;

fn create_test_transaction_blocks() -> Vec<TransactionBlock> {
  vec![
    TransactionBlock {
      block_number: 1,
      event_name: "Event1".to_string(),
      transactions: vec![
        json!({"transaction_id": "tx1", "amount": 100}),
        json!({"transaction_id": "tx2", "amount": 200}),
      ],
    },
    TransactionBlock {
      block_number: 2,
      event_name: "Event2".to_string(),
      transactions: vec![
        json!({"transaction_id": "tx3", "amount": 300}),
        json!({"transaction_id": "tx4", "amount": 400}),
      ],
    },
  ]
}

fn create_test_subscriptions() -> Vec<Document> {
  vec![
    doc! {
        "_id": ObjectId::from_str("605c72ef1531a577f67dbe10").unwrap(),
        "email": "user1@example.com",
    },
    doc! {
        "_id": ObjectId::from_str("605c72ef1531a577f67dbe11").unwrap(),
        "email": "user2@example.com",
    },
  ]
}

#[tokio::test]
async fn test_filter_contract_info_empty_payload() -> anyhow::Result<()> {
  let payload = Transactions {
    contract_id: "test_contract".to_string(),
    reset_nonce: 1,
    data: vec![],
  };

  let result = filter_contract_info(&payload, None)?;

  assert_eq!(
    result.0.len(),
    0,
    "No transaction blocks should be returned"
  );
  assert_eq!(
    result.1,
    doc! { "reset_nonce": 1i64 },
    "Only reset_nonce should be in the update document"
  );

  Ok(())
}

#[tokio::test]
async fn test_filter_contract_info_new_transactions() -> anyhow::Result<()> {
  let payload = Transactions {
    contract_id: "test_contract".to_string(),
    reset_nonce: 1,
    data: vec![
      json!({
          "event_name": "Event1",
          "block_number": 5,
          "transactions": [],
      }),
      json!({
          "event_name": "Event2",
          "block_number": 3,
          "transactions": [],
      }),
    ],
  };

  let result = filter_contract_info(&payload, None)?;

  assert_eq!(
    result.0.len(),
    2,
    "Both transaction blocks should be returned"
  );
  assert_eq!(
    result.1,
    doc! { "reset_nonce": 1i64, "Event1": 5i64, "Event2": 3i64 },
    "Update document should contain both events with their respective block numbers"
  );

  let event1 = result.0.get(0).unwrap();
  assert_eq!(event1.event_name, "Event1");
  assert_eq!(event1.block_number, 5);

  let event2 = result.0.get(1).unwrap();
  assert_eq!(event2.event_name, "Event2");
  assert_eq!(event2.block_number, 3);

  Ok(())
}

#[test]
fn test_generate_dbdata_from_records_docs_count() {
  let transaction_blocks = create_test_transaction_blocks();
  let subscriptions = create_test_subscriptions();

  let result = generate_dbdata_from_records(&transaction_blocks, &subscriptions).unwrap();
  let insert_docs = &result.0;

  // 2 transaction blocks * 2 subscriptions = 4 documents
  assert_eq!(insert_docs.len(), 4);
}

#[test]
fn test_generate_dbdata_from_records_docs_content() {
  let transaction_blocks = create_test_transaction_blocks();
  let subscriptions = create_test_subscriptions();

  let result = generate_dbdata_from_records(&transaction_blocks, &subscriptions).unwrap();
  let insert_docs = &result.0;

  for (index, doc) in insert_docs.iter().enumerate() {
    let transaction_block = &transaction_blocks[index / subscriptions.len()];
    let subscription = &subscriptions[index % subscriptions.len()];

    let subid = doc.get_str("subid").unwrap();
    let expected_subid = subscription.get_object_id("_id").unwrap().to_string();
    assert_eq!(subid, expected_subid);

    let block_number: i64 = doc.get_i64("block_number").unwrap();
    assert_eq!(block_number, transaction_block.block_number);

    let event_name = doc.get_str("event_name").unwrap();
    assert_eq!(event_name, transaction_block.event_name);

    let transactions: Vec<Document> = doc
      .get_array("transactions")
      .unwrap()
      .iter()
      .map(|bson| bson.as_document().unwrap().clone())
      .collect();

    let expected_transactions: Vec<Document> = transaction_block
      .transactions
      .iter()
      .map(|tx| tx.as_object().unwrap().clone().try_into().unwrap())
      .collect();

    assert_eq!(transactions, expected_transactions);
  }
}

#[test]
fn test_generate_dbdata_from_records_send_transactions() {
  let transaction_blocks = create_test_transaction_blocks();
  let subscriptions = create_test_subscriptions();

  let result = generate_dbdata_from_records(&transaction_blocks, &subscriptions).unwrap();
  let send_transactions = &result.1;

  let mut expected_send_transactions = Vec::new();
  for transaction_block in &transaction_blocks {
    for tx in &transaction_block.transactions {
      expected_send_transactions.push(tx.to_owned());
    }
  }

  assert_eq!(send_transactions, &expected_send_transactions);
}
