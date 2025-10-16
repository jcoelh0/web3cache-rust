use actix_web::test::TestRequest;
use actix_web::HttpRequest;
use dotenv::dotenv;
use mongodb::bson::{doc, Document};
use mongodb::error::Error as MongoErr;
use mongodb::options::{DeleteOptions, InsertOneOptions};
use mongodb::results::{DeleteResult, InsertOneResult};
use mongodb::Collection;
use web3cache::database::connect_to_mongodb_test;
use web3cache::helper_functions::*;
#[test]
fn test_validate_address_valid() {
    let address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    assert_eq!(validate_address(address), true);
}

#[test]
fn test_validate_address_invalid_length() {
    let address = "0x742d35Cc6634C0532925a3b844Bc454e4438";
    assert_eq!(validate_address(address), false);
}

#[test]
fn test_validate_address_invalid_prefix() {
    let address = "1x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    assert_eq!(validate_address(address), false);
}

#[test]
fn test_validate_address_empty_string() {
    let address = "";
    assert_eq!(validate_address(address), false);
}

#[test]
fn test_validate_address_lowercase_prefix() {
    let address = "0x742d35cc6634c0532925a3b844bc454e4438f44e";
    assert_eq!(validate_address(address), true);
}

#[test]
fn test_validate_address_mixed_case() {
    let address = "0x742D35cC6634c0532925A3b844bc454E4438F44e";
    assert_eq!(validate_address(address), true);
}

#[test]
fn test_validate_address_invalid_characters() {
    let address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44g";
    assert_eq!(validate_address(address), false);
}

#[test]
fn test_get_block_number_valid() {
    let req = TestRequest::default()
        .insert_header(("block_number", "123456"))
        .to_http_request();

    assert_eq!(get_block_number(&req), Some(123456));
}

#[test]
fn test_get_block_number_invalid_value() {
    let req = TestRequest::default()
        .insert_header(("block_number", "invalid"))
        .to_http_request();
    assert_eq!(get_block_number(&req), None);
}

#[test]
fn test_get_block_number_missing_header() {
    let req = TestRequest::default().to_http_request();

    assert_eq!(get_block_number(&req), None);
}

#[test]
fn test_get_block_number_negative_number() {
    let req = TestRequest::default()
        .insert_header(("block_number", "-123456"))
        .to_http_request();

    assert_eq!(get_block_number(&req), Some(-123456));
}

#[test]
fn test_get_block_number_large_number() {
    let req = TestRequest::default()
        .insert_header(("block_number", "9223372036854775807"))
        .to_http_request();

    assert_eq!(get_block_number(&req), Some(9223372036854775807));
}

async fn create_test_request(api_key: Option<&str>) -> HttpRequest {
    let builder = TestRequest::default();
    let builder = if let Some(key) = api_key {
        builder.insert_header(("x-read-api-key", key))
    } else {
        builder
    };
    builder.to_http_request()
}
async fn insert_test(
    col: Collection<Document>,
    doc: Document,
    option: InsertOneOptions,
) -> Result<InsertOneResult, MongoErr> {
    col.insert_one(doc, option).await
}

async fn delete_one(
    col: Collection<Document>,
    query: Document,
    option: DeleteOptions,
) -> Result<DeleteResult, MongoErr> {
    col.delete_one(query, option).await
}

async fn create_test_api_key(db: mongodb::Database) {
    let _ = insert_test(
        db.collection("apikeys"),
        doc! {"apikey":"valid_key"},
        InsertOneOptions::default(),
    )
    .await;
}

async fn delete_test_api_key(db: mongodb::Database) {
    let _ = delete_one(
        db.collection("apikeys"),
        doc! {"apikey":"valid_key"},
        DeleteOptions::default(),
    )
    .await;
}

#[actix_web::test]
async fn test_check_api_key_valid() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let req = create_test_request(Some("valid_key")).await;

    let result = check_api_key(&req, db.clone()).await.unwrap();

    delete_test_api_key(db).await;
    assert_eq!(result, true);
}

#[actix_web::test]
async fn test_check_api_key_invalid() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;

    let req = create_test_request(Some("invalid_key")).await;

    // Pass the mongo_db variable to the check_api_key function:
    let result = check_api_key(&req, db.clone()).await.unwrap();

    delete_test_api_key(db).await;
    assert_eq!(result, false);
}

#[actix_web::test]
async fn test_check_api_key_missing() {
    // Connect to MongoDB
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let req = TestRequest::default().to_http_request();
    let result = check_api_key(&req, db.clone()).await.unwrap();

    // Delete the apikey entry
    delete_test_api_key(db).await;
    assert_eq!(result, false);
}

#[actix_web::test]
async fn test_get_contract_id_present() {
    let req = TestRequest::default()
        .insert_header(("contract_id", "1234"))
        .to_http_request();

    let result = get_contract_id(&req);
    assert_eq!(result, Some("1234"));
}

#[actix_web::test]
async fn test_get_contract_id_missing() {
    let req = TestRequest::default().to_http_request();

    let result = get_contract_id(&req);
    assert_eq!(result, None);
}

#[actix_web::test]
async fn test_get_address_present() {
    let req = TestRequest::default()
        .insert_header(("address", "0x8000000000000000000000000000000000000000"))
        .to_http_request();

    let result = get_address(&req);
    assert_eq!(result, Some("0x8000000000000000000000000000000000000000"));
}

#[actix_web::test]
async fn test_get_address_missing() {
    let req = TestRequest::default().to_http_request();

    let result = get_address(&req);
    assert_eq!(result, None);
}
