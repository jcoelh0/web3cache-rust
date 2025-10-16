use actix_web::HttpRequest;
use bson::{doc, Document};
use mongodb::bson::DateTime;
use std::env;
use url::Url;

use lazy_static::lazy_static;
use mongodb::Database;
use regex::Regex;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

pub struct AppState {
  pub db: Database,
}

pub fn get_api_key(req: &HttpRequest) -> Option<&str> {
  req.headers().get("x-webhook-api-key")?.to_str().ok()
}

/* pub fn get_object_id(sub_id: String) -> (ObjectId, Error) {
  let mut username_file = match ObjectId::parse_str(sub_id) {
    Ok(file) => return (file, None),
    Err(e) => return (ObjectId::new(), Err(e)),
  };
} */

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

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Subscription {
  #[validate(custom = "validate_vec_events")]
  pub topics: Option<Vec<String>>,
  #[validate(custom = "validate_url")]
  pub url: String,
  pub contract_id: String,
  #[validate(custom(
    function = "validate_block_number",
    message = "Block number validation failed!"
  ))]
  pub block_number: Option<i64>,
}

/* enum chain_options {
  "",
  "sfa"
} */

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct ContractRegistration {
  pub contract_id: String,
  #[validate(length(min = 1), custom = "validate_chain")]
  pub chain: String,
  pub contract_address: String,
  pub contract_abi: Option<String>,
  #[validate(length(min = 1), custom = "validate_events")]
  pub events: Option<String>,
  pub modules: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct SuiContractRegistration {
  pub contract_id: String,
  pub contract_address: String,
  #[validate(length(min = 1), custom = "validate_sui_chain")]
  pub chain: String,
  #[validate(length(min = 1), custom = "validate_events")]
  pub events: Option<String>,
  pub modules: Option<String>,
}

pub fn validate_events(events: &str) -> Result<(), ValidationError> {
  let re = Regex::new(r"^(?:\s*\w+\s*)(?:,\s*\w+\s*)*$").unwrap();

  if events.is_empty() {
    return Err(ValidationError::new("Events cannot be empty."));
  }

  if !re.is_match(events) {
    return Err(ValidationError::new(
      "Invalid events string: must be 'Transfer,Approval,Custom'",
    ));
  }

  Ok(())
}

pub fn validate_chain(chain: &str) -> Result<(), ValidationError> {
  let available_chains = Vec::from(["polygon", "ethereum", "mumbai", "sepolia"]);

  if !available_chains.contains(&chain.to_string().to_ascii_lowercase().as_str()) {
    return Err(ValidationError::new(
      "Supported chains are: 'polygon', 'ethereum', 'mumbai', 'sepolia'",
    ));
  }

  Ok(())
}

pub fn validate_sui_chain(chain: &str) -> Result<(), ValidationError> {
  let available_chains = Vec::from(["sui_testnet","sui_mainnet"]);

  if !available_chains.contains(&chain.to_string().to_ascii_lowercase().as_str()) {
    return Err(ValidationError::new(
      "Supported chains are: 'sui_testnet', 'sui_mainnet'",
    ));
  }

  Ok(())
}
/* impl Default for ContractRegistration {
  fn default() -> Self {
    ContractRegistration {
      contract_id: "".to_string(),
      chain: "".to_string(),
      contract_address: "".to_string(),
      contract_abi: None,
      events: None
    }
  }
} */

#[derive(Serialize, Deserialize)]
pub struct ReplaySubscription {
  pub block_number: i64,
}

#[derive(Serialize, Deserialize)]
pub struct SubState {
  pub activate: Option<bool>,
}

#[derive(Serialize, Deserialize, Validate)]
pub struct UpdateSub {
  #[validate(length(min = 1), custom = "validate_url")]
  pub url: Option<String>,
  #[validate(length(min = 1), custom = "validate_vec_events")]
  pub add_topics: Option<Vec<String>>,
  #[validate(length(min = 1), custom = "validate_vec_events")]
  pub remove_topics: Option<Vec<String>>,
  #[validate(length(min = 1), custom = "validate_vec_events")]
  pub set_topics: Option<Vec<String>>,
  pub activate: Option<bool>,
}

pub fn format_sub(mut subscription: Document, id: bson::oid::ObjectId) -> Document {
  subscription.insert("_id", id.to_hex());
  subscription.remove("apikey");
  subscription.insert(
    "createdAt",
    subscription
      .get_datetime("createdAt")
      .unwrap()
      .to_chrono()
      .to_rfc3339(),
  );
  subscription.insert(
    "updatedAt",
    subscription
      .get_datetime("updatedAt")
      .unwrap()
      .to_chrono()
      .to_rfc3339(),
  );
  subscription
}

pub fn validate_url(url: &str) -> Result<(), ValidationError> {
  if url.is_empty() {
    return Err(ValidationError::new("URL cannot be empty."));
  }

  lazy_static! {
      static ref URL_REGEX: Regex = Regex::new(
        r"^(https?://)?([a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,6}(:[0-9]{1,5})?(/.*)?$"
      ).unwrap();
  }
  if !URL_REGEX.is_match(url) {
    return Err(ValidationError::new("Invalid URL."));
  }
  if Url::parse(url).is_err() {
    return Err(ValidationError::new("Invalid URL."));
  }

  Ok(())
}

pub fn validate_block_number(block_number: i64) -> Result<(), ValidationError> {
  if block_number < 0 {
    return Err(ValidationError::new(
      "Block number must be a number greater or equal to 0",
    ));
  }
  Ok(())
}

pub fn read_api_key() -> String {
  dotenv::dotenv().ok();
  let uri = std::env::var("READAPIKEY");
  assert!(uri.is_ok());
  env::set_var("READAPIKEY", uri.unwrap());
  env::var("READAPIKEY").expect("$READAPIKEY is not set")
}

pub fn validate_vec_events(events: &Vec<String>) -> Result<(), ValidationError> {
  let re = Regex::new(r"^[^\s,]+$").unwrap();
  if events.is_empty() {
    return Ok(());
  }

  for event in events {
    if !re.is_match(event) {
      return Err(ValidationError::new(
        "Invalid events string: must be 'Transfer,Approval,Custom'",
      ));
    }
  }
  Ok(())
}
