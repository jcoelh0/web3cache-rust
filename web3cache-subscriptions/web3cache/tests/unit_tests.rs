use actix_web::test;
use bson::{doc, oid::ObjectId, DateTime};
use chrono::{DateTime as ChronoDateTime, Timelike, Utc};
use std::env;
use validator::ValidationError;
use web3cache::database::*;
use web3cache::helper_functions::*;
#[actix_web::test]
async fn test_get_api_key_with_valid_header() {
  let req = test::TestRequest::default()
    .insert_header(("x-webhook-api-key", "test-api-key"))
    .to_http_request();

  let api_key = get_api_key(&req);
  assert_eq!(api_key, Some("test-api-key"));
}

#[actix_web::test]
async fn test_get_api_key_with_missing_header() {
  let req = test::TestRequest::default().to_http_request();

  let api_key = get_api_key(&req);
  assert_eq!(api_key, None);
}

#[actix_web::test]
async fn test_get_api_key_with_invalid_header_value() {
  let invalid_value = "\u{FFFD}"; // Invalid Unicode replacement character
  let req = test::TestRequest::default()
    .insert_header(("x-webhook-api-key", invalid_value))
    .to_http_request();

  let api_key = get_api_key(&req);
  assert_eq!(api_key, None);
}
#[test]
async fn test_validate_empty_url() {
  let result = validate_url("");
  assert_eq!(result, Err(ValidationError::new("URL cannot be empty.")));
}

#[test]
async fn test_validate_valid_url_with_scheme() {
  let result = validate_url("https://www.example.com");
  assert_eq!(result, Ok(()));

  let result = validate_url("http://www.example.com");
  assert_eq!(result, Ok(()));
}

#[test]
async fn test_validate_valid_url_without_scheme() {
  let result = validate_url("www.example.com");
  assert_eq!(result, Err(ValidationError::new("Invalid URL.")));
}

#[test]
async fn test_validate_valid_url_with_port() {
  let result = validate_url("http://www.example.com:8080");
  assert_eq!(result, Ok(()));
}

#[test]
async fn test_validate_valid_url_with_path() {
  let result = validate_url("https://www.example.com/some/path");
  assert_eq!(result, Ok(()));
}

#[test]
async fn test_validate_invalid_url() {
  let result = validate_url("invalid_url");
  assert_eq!(result, Err(ValidationError::new("Invalid URL.")));

  let result = validate_url("htp://www.example.com");
  assert_eq!(result, Err(ValidationError::new("Invalid URL.")));

  let result = validate_url("www.example..com");
  assert_eq!(result, Err(ValidationError::new("Invalid URL.")));
}

#[test]
async fn test_validate_invalid_url_with_port() {
  let result = validate_url("https://www.example.com:65536");
  assert_eq!(result, Err(ValidationError::new("Invalid URL.")));
}

#[test]
async fn test_env_var() {
  dotenv::dotenv().ok();
  let uri = std::env::var("MONGOURI_TEST");

  assert!(uri.is_ok());
}
#[tokio::test]
async fn test_conection_to_mongodb() {
  dotenv::dotenv().ok();
  let db = connect_to_mongodb(true).await;
  assert!(db.is_ok());
}

#[test]
async fn test_validate_events_empty() {
  let events = "";
  assert!(validate_events(events).is_err());
}

#[test]
async fn test_validate_events_valid_single() {
  let events = "Transfer";
  assert!(validate_events(events).is_ok());
}

#[test]
async fn test_validate_events_valid_multiple() {
  let events = "Transfer,Approval,Custom";
  assert!(validate_events(events).is_ok());
}

#[test]
async fn test_validate_events_invalid_extra_comma() {
  let events = "Transfer,Approval,";
  assert!(validate_events(events).is_err());
}

#[test]
async fn test_validate_events_invalid_extra_space() {
  let events = "Transfer, Approval";
  assert!(validate_events(events).is_ok());
}

#[test]
async fn test_validate_events_invalid_whitespace() {
  let events = " ";
  assert!(validate_events(events).is_err());
}

#[test]
async fn test_validate_chain_valid_polygon() {
  let chain = "polygon";
  assert!(validate_chain(chain).is_ok());
}

#[test]
async fn test_validate_chain_valid_ethereum() {
  let chain = "ethereum";
  assert!(validate_chain(chain).is_ok());
}

#[test]
async fn test_validate_chain_valid_mumbai() {
  let chain = "mumbai";
  assert!(validate_chain(chain).is_ok());
}

#[test]
async fn test_validate_chain_valid_sepolia() {
  let chain = "sepolia";
  assert!(validate_chain(chain).is_ok());
}

#[test]
async fn test_validate_chain_invalid_chain() {
  let chain = "invalid_chain";
  assert!(validate_chain(chain).is_err());
}

#[test]
async fn test_validate_chain_empty_chain() {
  let chain = "";
  assert!(validate_chain(chain).is_err());
}

#[test]
async fn test_format_sub() {
  let id = ObjectId::new();
  let apikey = "test_api_key";
  let now = Utc::now().with_nanosecond(0).unwrap();
  let created_at = DateTime::from_chrono(now);
  let updated_at = DateTime::from_chrono(now);

  let subscription = doc! {
      "_id": id.clone(),
      "apikey": apikey,
      "createdAt": created_at,
      "updatedAt": updated_at
  };

  let formatted_sub = format_sub(subscription, id.clone());

  assert_eq!(formatted_sub.get_str("_id").unwrap(), id.to_hex());
  assert!(formatted_sub.get_str("apikey").is_err());
  let formatted_created_at = formatted_sub
    .get_str("createdAt")
    .unwrap()
    .parse::<ChronoDateTime<Utc>>()
    .unwrap();
  let formatted_updated_at = formatted_sub
    .get_str("updatedAt")
    .unwrap()
    .parse::<ChronoDateTime<Utc>>()
    .unwrap();
  let now = now.with_nanosecond(0).unwrap();
  let formatted_created_at = formatted_created_at.with_nanosecond(0).unwrap();
  let formatted_updated_at = formatted_updated_at.with_nanosecond(0).unwrap();
  assert_eq!(formatted_created_at, now);
  assert_eq!(formatted_updated_at, now);
}

#[test]
async fn test_read_api_key() {
  env::set_var("READAPIKEY", "test_read_api_key");

  let api_key = read_api_key();
  assert_eq!(api_key, "test_read_api_key");
  env::remove_var("READAPIKEY");
}

#[test]
async fn test_validate_vec_events_empty() {
  let events: Vec<String> = Vec::new();

  let result = validate_vec_events(&events);

  assert!(result.is_ok());
}

#[test]
async fn test_validate_vec_events_valid() {
  let events = vec![
    "Transfer".to_string(),
    "Approval".to_string(),
    "Custom".to_string(),
  ];

  let result = validate_vec_events(&events);

  assert!(result.is_ok());
}
