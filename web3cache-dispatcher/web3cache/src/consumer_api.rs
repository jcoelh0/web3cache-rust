use std::{collections::HashMap, env};

use actix_web::{
  get, post,
  web::{self, Data},
  HttpRequest, HttpResponse,
};

use crate::helper_functions::get_i64_from_doc;
use bson::Document;
use futures::StreamExt;

use crate::{
  database::{find_one, find_one_and_update, insert_many},
  helper_functions::AppState,
};
use log::{error, info, warn};
use mongodb::{
  bson::doc,
  options::{FindOneAndUpdateOptions, FindOneOptions, FindOptions, InsertManyOptions},
  Database,
};
use serde::Deserialize;

use serde_json::{json, Value};

use crate::database::find_all;

use snailquote::unescape;

#[derive(Deserialize, Debug, Clone)]
pub struct TransactionBlock {
  pub block_number: i64,
  pub event_name: String,
  pub transactions: Vec<Value>,
}

#[derive(Deserialize, Debug)]
pub struct Transactions {
  pub contract_id: String,
  pub reset_nonce: i64,
  pub data: Vec<Value>,
}

pub fn filter_contract_info(
  payload: &Transactions,
  doc_result: Option<Document>,
) -> anyhow::Result<(Vec<TransactionBlock>, Document)> {
  let mut hashmap: HashMap<String, i64> = HashMap::new();

  if let Some(doc) = doc_result {
    info!("doc: {}", doc);
    let old_reset = get_i64_from_doc(&doc, "reset_nonce".to_string());
    if old_reset == payload.reset_nonce {
      for key in doc.keys() {
        hashmap.insert(key.to_string(), get_i64_from_doc(&doc, key.to_string()));
      }
    }
  }
  let mut update_doc = doc! { "reset_nonce": payload.reset_nonce };
  let mut result_docs = vec![];
  for record in &payload.data {
    let transaction_block: TransactionBlock = serde_json::from_value(record.to_owned())?;
    let event_name = transaction_block.event_name.clone();
    let received_block_number = transaction_block.block_number;

    let previous_block_number = *hashmap.get(&event_name).unwrap_or(&-1);
    if previous_block_number < received_block_number {
      result_docs.push(transaction_block);
      update_doc.insert(event_name.clone(), received_block_number);
    } else {
      warn!(
        "Ignoring transactions on event {}, block_number {} <= {}",
        event_name, received_block_number, previous_block_number
      );
    }
  }
  Ok((result_docs, update_doc))
}

pub fn generate_dbdata_from_records(
  records: &Vec<TransactionBlock>,
  subscriptions: &Vec<Document>,
) -> anyhow::Result<(Vec<Document>, Vec<Value>)> {
  let mut insert_docs = Vec::new();
  let mut send_transactions = Vec::new();
  for transaction_block in records {
    let transactions_block_doc: Vec<Document> = transaction_block
      .transactions
      .iter()
      .map(|d| {
        let value: serde_json::Map<String, Value> = d.as_object().unwrap().to_owned();
        Document::try_from(value).unwrap()
      })
      .collect();
    // info!("transactions_block_doc: {:?}", transactions_block_doc);
    let locked_until = bson::DateTime::now();

    for item in subscriptions {
      insert_docs.push(doc! {
        "subid": item.get_object_id("_id").unwrap().to_string(),
        "transactions": transactions_block_doc.clone(),
        "block_number": transaction_block.block_number,
        "event_name": transaction_block.event_name.clone(),
        "locked_until": locked_until,
      })
    }
    for tx in &transaction_block.transactions {
      send_transactions.push(tx.to_owned());
    }
  }
  Ok((insert_docs, send_transactions))
}

#[get("/healthcheck")]
pub async fn consumer_health_check() -> HttpResponse {
  HttpResponse::Ok().body("web3cache dispatcher OK")
}

#[post("/push-transactions")]
pub async fn push_transactions(
  _req: HttpRequest,
  mut body: web::Payload,
  data: Data<AppState>,
) -> HttpResponse {
  info!("push-transaction request received!");
  let mut bytes = web::BytesMut::new();
  while let Some(item) = body.next().await {
    bytes.extend_from_slice(&item.unwrap());
  }

  let payload: Transactions = serde_json::from_slice(&bytes).unwrap();

  let contract_id = unescape(&payload.contract_id).unwrap();

  info!("Receiving transactions from {:?}\n", contract_id);

  info!("reset_nonce: {}", payload.reset_nonce);

  let filter = doc! { "contract_id": contract_id.clone(), "isActive": true };
  let find_option = FindOptions::default();
  let db: Database = data.db.clone();

  let subscriptions = find_all(db.collection("subscriptions"), filter, find_option)
    .await
    .unwrap();
  //info!("Subscriptions: {}", subscriptions.len());
  let transaction_col = db.collection("transactionblocks");

  let doc_result = find_one(
    db.collection("events_info"),
    doc! {
      "contract_id": contract_id.clone()
    },
    FindOneOptions::default(),
  )
  .await
  .unwrap_or_else(|_| None);

  let result = filter_contract_info(&payload, doc_result);
  if result.is_err() {
    return HttpResponse::InternalServerError().finish();
  }
  let (records, update_doc) = result.unwrap();

  let result = generate_dbdata_from_records(&records, &subscriptions);
  if result.is_err() {
    return HttpResponse::InternalServerError().finish();
  }
  let (insert_docs, send_transactions) = result.unwrap();

  // reqwest here
  tokio::spawn(async move {
    let realtime_url = env::var("REALTIME_URL").unwrap();
    //for tx in transaction_block.transactions.iter() {
    let client = reqwest::Client::new();
    let _res = client
      .post(format!("{}/notify-transactions", realtime_url))
      .json(&json!({ "transactions": send_transactions }))
      .send()
      .await;
    ////}
    //info!("sending to realtime: {:?}", res);
  });

  if !insert_docs.is_empty() {
    let mut insert_many_options = InsertManyOptions::default();
    insert_many_options.ordered = Some(false);
    info!("insert_docs.len() = {}", insert_docs.len());
    if let Err(err) = insert_many(transaction_col.clone(), &insert_docs, insert_many_options).await
    {
      error!("err.kind: {:?}!", err.kind);

      if !err.kind.to_string().contains(" code: 11000,") {
        return HttpResponse::BadRequest().json(json!({
            "message": "error inserting"
        }));
      }
    }

    let options = FindOneAndUpdateOptions::builder()
      .upsert(Some(true))
      .build();

    let result = find_one_and_update(
      db.collection("events_info"),
      doc! { "contract_id": contract_id },
      doc! { "$set": update_doc },
      Some(options),
    )
    .await;

    error!("{:?}", result);
  }

  HttpResponse::Ok().finish()
}

#[cfg(test)]
mod tests {
  use super::*;
  use bson::{doc, oid::ObjectId, Document};
  use serde_json::{json, Value};
  use std::str::FromStr;

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
}
