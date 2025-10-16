use bson::Document;
use mongodb::bson::DateTime;
use mongodb::Database;
use serde::{Deserialize, Serialize};
pub struct AppState {
  pub db: Database,
}

#[allow(dead_code)]
pub fn get_i64_from_doc(doc: &Document, key: String) -> i64 {
  let res64 = doc.get_i64(key.clone());

  if let Ok(v) = res64 {
    v
  } else {
    doc.get_i32(key).unwrap_or(0) as i64
  }
}

#[derive(Serialize, Deserialize)]
pub struct SingleTransaction {
  pub _id: String, //is it really needed?
  pub contract_id: String,
  pub from: String,
  pub to: String,
  pub token_id: i64,
  pub block_number: i64,
  pub transaction_hash: String,
  pub log_index: i64,
}

#[derive(Serialize, Deserialize)]
pub struct Transaction {
  pub subid: String,
  pub transactions: Vec<SingleTransaction>,
  pub secret: String,
  pub block_number: i64,
  pub locked_until: DateTime,
}

#[cfg(test)]
mod tests {
  use super::*;
  use bson::{doc, Bson};

  #[test]
  fn test_get_i64_from_doc() {
    let doc = doc! {
        "foo": 42i64,
        "bar": 23i32,
    };

    assert_eq!(get_i64_from_doc(&doc, "foo".to_string()), 42);
    assert_eq!(get_i64_from_doc(&doc, "bar".to_string()), 23);
    assert_eq!(get_i64_from_doc(&doc, "baz".to_string()), 0);
  }
}
