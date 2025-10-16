use actix_web::body::to_bytes;
use actix_web::http;
use actix_web::http::header::CONTENT_TYPE;
use actix_web::test::{call_service, init_service, TestRequest};
use actix_web::web::Data;
use actix_web::App;
use dotenv::dotenv;
use mongodb::bson::{doc, Document};
use mongodb::error::Error as MongoErr;
use mongodb::options::{DeleteOptions, InsertOneOptions};
use mongodb::results::{DeleteResult, InsertOneResult};
use mongodb::Collection;
use serde_json::json;
use web3cache::database::*;
use web3cache::helper_functions::*;
use web3cache::routes::*;

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
        doc! {"apikey":"testing_api_key"},
        InsertOneOptions::default(),
    )
    .await;
}

async fn delete_test_api_key(db: mongodb::Database) {
    let _ = delete_one(
        db.collection("apikeys"),
        doc! {"apikey":"testing_api_key"},
        DeleteOptions::default(),
    )
    .await;
}

#[actix_web::test]
async fn get_user_transactions_ok() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let doc = doc! {"contract_id":"testing_contract","event_name":"testing_event","block_number":1,"contract_address":"0x8000000000000000000000000000000000000000","transaction_hash":"0x8000000000000000000000000000000000000000000000000000000000000000","log_index":0,"from":"0x0000000000000000000000000000000000000000","to":"0x8000000000000000000000000000000000000000","tokenId":0};
    let _ = insert_test(
        db.collection("transactions"),
        doc,
        InsertOneOptions::default(),
    )
    .await;
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_user_transaction),
    )
    .await;
    let req = TestRequest::default()
        .uri("/transactions")
        .insert_header(("x-read-api-key", "testing_api_key"))
        .insert_header(("address", "0x8000000000000000000000000000000000000000"))
        .to_request();
    let resp = call_service(&app, req).await;
    println!("{:?}", resp);
    assert_eq!(resp.status(), http::StatusCode::OK);

    let query = doc! { "address" : "0x8000000000000000000000000000000000000000"};
    let _ = delete_one(
        db.collection("transactions"),
        query,
        DeleteOptions::default(),
    )
    .await;
    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_user_transactions_no_api_key_header() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_user_transaction),
    )
    .await;
    let req = TestRequest::default()
        .uri("/transactions")
        .insert_header(("address", "0x8000000000000000000000000000000000000000"))
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn get_user_transactions_invalid_api_key_header() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_user_transaction),
    )
    .await;
    let req = TestRequest::default()
        .uri("/transactions")
        .insert_header(("x-read-api-key", "invalid_api_key"))
        .insert_header(("address", "0x8000000000000000000000000000000000000000"))
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_user_transactions_no_address_header() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_user_transaction),
    )
    .await;
    let req = TestRequest::default()
        .uri("/transactions")
        .insert_header(("x-read-api-key", "testing_api_key"))
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::UNAUTHORIZED);
    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_owners_ok() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let doc = doc! {"owner":"0x8000000000000000000000000000000000000000","amount":1,"token_id":1 as i64,"block_number":1 as i64,"contract_address":"0x8000000000000000000000000000000000000000","contract_id":"testing_contract"};
    let _ = insert_test(db.collection("owners"), doc, InsertOneOptions::default()).await;

    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_onwers),
    )
    .await;
    let payload = json!({"contract_id":"testing_contract" , "address": "0x8000000000000000000000000000000000000000"});
    let req = TestRequest::default()
        .uri("/owners")
        .insert_header(("x-read-api-key", "testing_api_key"))
        .set_json(payload)
        .to_request();
    let resp = call_service(&app, req).await;
    println!("response: {:?}", resp.response().body());
    assert_eq!(resp.status(), http::StatusCode::OK);
    let query = doc! { "owner" : "0x8000000000000000000000000000000000000000" , "contract_id":"testing_contract"};
    let _ = delete_one(db.collection("owners"), query, DeleteOptions::default()).await;
    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_owners_no_api_key_header() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_onwers),
    )
    .await;
    let payload = json!({"contract_id": "testing_contract", "address": "0x8000000000000000000000000000000000000000"});
    let req = TestRequest::default()
        .uri("/owners")
        .set_json(payload)
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn get_owners_no_contract_id() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let doc = doc! {"owner":"0x8000000000000000000000000000000000000000","amount":1,"token_id":1 as i64,"block_number":1 as i64,"contract_address":"0x8000000000000000000000000000000000000000","contract_id":"testing_contract"};
    let _ = insert_test(db.collection("owners"), doc, InsertOneOptions::default()).await;

    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_onwers),
    )
    .await;
    let payload = json!({"address": "0x8000000000000000000000000000000000000000"});
    let req = TestRequest::default()
        .uri("/owners")
        .insert_header(("x-read-api-key", "testing_api_key"))
        .set_json(payload)
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::OK);

    let query = doc! { "owner" : "0x8000000000000000000000000000000000000000" , "contract_id":"testing_contract"};
    let _ = delete_one(db.collection("owners"), query, DeleteOptions::default()).await;
    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_owners_no_address() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_onwers),
    )
    .await;
    let payload = json!({"address": "", "contract_id": "testing_contract"});
    let req = TestRequest::default()
        .uri("/owners")
        .insert_header(("x-read-api-key", "testing_api_key"))
        .set_json(payload)
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    let content_type_header = resp.response().headers().get(CONTENT_TYPE).unwrap();
    let content_type = content_type_header.to_str().unwrap();
    assert_eq!(content_type, "text/plain; charset=utf-8");

    let body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(body.as_ref(), b"No wallet address");
}

#[actix_web::test]
async fn get_contracts_ok() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let doc = doc! {"contract_id" : "testing_contract" };
    let _ = insert_test(db.collection("contracts"), doc, InsertOneOptions::default())
        .await
        .unwrap();
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_contracts),
    )
    .await;
    let req = TestRequest::default()
        .uri("/contracts")
        .insert_header(("x-read-api-key", "testing_api_key"))
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    let query = doc! { "contract_id":"testing_contract"};
    let _ = delete_one(db.collection("contracts"), query, DeleteOptions::default()).await;
    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_contracts_no_api_key_header() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_contracts),
    )
    .await;
    let req = TestRequest::default().uri("/contracts").to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn get_contracts_invalid_api_key_header() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_contracts),
    )
    .await;
    let req = TestRequest::default()
        .uri("/contracts")
        .insert_header(("x-read-api-key", "invalid_api_key"))
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_contract_ok() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let doc = doc! {"contract_id" : "testing_contract" };
    let _ = insert_test(db.collection("contracts"), doc, InsertOneOptions::default()).await;

    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_contract),
    )
    .await;
    let req = TestRequest::default()
        .uri("/contract")
        .insert_header(("x-read-api-key", "testing_api_key"))
        .insert_header(("contract_id", "testing_contract"))
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    let query = doc! { "contract_id":"testing_contract"};
    let _ = delete_one(db.collection("contracts"), query, DeleteOptions::default()).await;
    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_contract_no_api_key_header() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_contract),
    )
    .await;
    let req = TestRequest::default()
        .uri("/contract")
        .insert_header(("contract_id", "testing_contract"))
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn get_contract_invalid_api_key_header() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_contract),
    )
    .await;
    let req = TestRequest::default()
        .uri("/contract")
        .insert_header(("x-read-api-key", "invalid_api_key"))
        .insert_header(("contract_id", "testing_contract"))
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_contract_no_contract_id_header() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_contract),
    )
    .await;
    let req = TestRequest::default()
        .uri("/contract")
        .insert_header(("x-read-api-key", "testing_api_key"))
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_user_transaction_ok() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let doc = doc! {"owner":"0x8000000000000000000000000000000000000000","amount":1 as i64,"token_id":1 as i64,"block_number":1 as i64,"contract_address":"0x8000000000000000000000000000000000000000","contract_id":"testing_contract"};
    let _ = insert_test(
        db.collection("transactions"),
        doc,
        InsertOneOptions::default(),
    )
    .await;

    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_user_transaction),
    )
    .await;
    let req = TestRequest::default()
        .uri("/transactions")
        .insert_header(("x-read-api-key", "testing_api_key"))
        .insert_header(("address", "0x8000000000000000000000000000000000000000"))
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::OK);

    let query = doc! { "owner" : "0x8000000000000000000000000000000000000000"};
    let _ = delete_one(
        db.collection("transactions"),
        query,
        DeleteOptions::default(),
    )
    .await;
    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_transaction_history_ok() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let doc = doc! {"owner":"0x8000000000000000000000000000000000000000","amount":1,"token_id":1 as i64,"block_number":1 as i64,"contract_address":"0x8000000000000000000000000000000000000000","contract_id":"testing_contract"};
    let _ = insert_test(
        db.collection("transactions"),
        doc,
        InsertOneOptions::default(),
    )
    .await;

    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_user_transaction_history),
    )
    .await;
    let req = TestRequest::default()
        .uri("/transactions_history")
        .insert_header(("x-read-api-key", "testing_api_key"))
        .insert_header(("contract_id", "testing_contract"))
        .insert_header(("block_number", "0"))
        .to_request();
    let resp = call_service(&app, req).await;
    println!("response:{:?}", resp.response());
    assert_eq!(resp.status(), http::StatusCode::OK);
    let query = doc! { "address" : "0x8000000000000000000000000000000000000000"};
    let _ = delete_one(
        db.collection("transactions"),
        query,
        DeleteOptions::default(),
    )
    .await;
    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_transaction_history_no_contract_id_header() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_user_transaction_history),
    )
    .await;
    let req = TestRequest::default()
        .uri("/transactions_history")
        .insert_header(("x-read-api-key", "testing_api_key"))
        .insert_header(("block_number", "0"))
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn get_transaction_history_no_block_number_header() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_user_transaction_history),
    )
    .await;
    let req = TestRequest::default()
        .uri("/transactions_history")
        .insert_header(("x-read-api-key", "testing_api_key"))
        .insert_header(("contract_id", "testing_contract"))
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn get_transaction_history_no_headers() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_user_transaction_history),
    )
    .await;
    let req = TestRequest::default()
        .uri("/transactions_history")
        .insert_header(("x-read-api-key", "testing_api_key"))
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn get_nft_for_contract_ok() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let doc = doc! {"owner":"0x8000000000000000000000000000000000000000","amount":1,"token_id":1 as i64,"block_number":1,"contract_address":"0x8000000000000000000000000000000000000000","contract_id":"testing_contract"};
    let _ = insert_test(db.collection("owners"), doc, InsertOneOptions::default()).await;
    let doc = doc! { "contract_address":"0x8000000000000000000000000000000000000000", "contract_id":"testing_contract" , "token_id":1 as i64 ,"metadata":"testing_metadata", "token_uri":"testing_uri" };
    let _ = insert_test(db.collection("nfts"), doc, InsertOneOptions::default()).await;

    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_contract_nft),
    )
    .await;

    let contract_address = "0x8000000000000000000000000000000000000000";
    let limit = 5;
    let offset = 0;

    let query = format!(
        "/getContractNFT?contract_address={}&limit={}&offset={}",
        contract_address, limit, offset
    );

    let req = TestRequest::default()
        .uri(&query)
        .insert_header(("x-read-api-key", "testing_api_key"))
        .to_request();

    let resp = call_service(&app, req).await;
    println!("response:{:?}", resp.response());
    assert_eq!(resp.status(), http::StatusCode::OK);

    let query = doc! { "owner" : "0x8000000000000000000000000000000000000000" , "contract_id":"testing_contract"};
    let _ = delete_one(db.collection("owners"), query, DeleteOptions::default()).await;
    let query = doc! { "contract_address" : "0x8000000000000000000000000000000000000000" , "contract_id":"testing_contract"};
    let _ = delete_one(db.collection("nfts"), query, DeleteOptions::default()).await;
    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_nft_for_invalid_contract_address() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;

    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_contract_nft),
    )
    .await;

    let contract_address = "0xINVALID_ADDRESS";
    let limit = 5;
    let offset = 0;

    let query = format!(
        "/getContractNFT?contract_address={}&limit={}&offset={}",
        contract_address, limit, offset
    );

    let req = TestRequest::default()
        .uri(&query)
        .insert_header(("x-read-api-key", "testing_api_key"))
        .to_request();

    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);

    let body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(body.as_ref(), b"No wallet address");

    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_nft_for_contract_invalid_api_key() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();

    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_contract_nft),
    )
    .await;

    let contract_address = "0x8000000000000000000000000000000000000000";
    let limit = 5;
    let offset = 0;

    let query = format!(
        "/getContractNFT?contract_address={}&limit={}&offset={}",
        contract_address, limit, offset
    );

    let req = TestRequest::default()
        .uri(&query)
        .insert_header(("x-read-api-key", "invalid_api_key"))
        .to_request();

    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);

    let body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(
        body.as_ref(),
        b"Invalid header x-read-api-key or not provided"
    );
}

#[actix_web::test]
async fn get_nft_for_owner_ok() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let doc = doc! {"owner":"0x8000000000000000000000000000000000000000","amount":1,"token_id":1 as i64,"block_number":1,"contract_address":"0x8000000000000000000000000000000000000000","contract_id":"testing_contract"};
    let _ = insert_test(db.collection("owners"), doc, InsertOneOptions::default()).await;
    let doc_nft = doc! {"contract_address":"0x8000000000000000000000000000000000000000", "contract_id":"testing_contract" , "token_id":1 as i64 ,"metadata":"testing_metadata", "token_uri":"testing_uri" };
    let _ = insert_test(db.collection("nfts"), doc_nft, InsertOneOptions::default()).await;

    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_owner_nft),
    )
    .await;
    let contract_address = ["0x8000000000000000000000000000000000000000"];
    let address = "0x8000000000000000000000000000000000000000";
    let limit = 100;
    let offset = 0;
    let metadata = true;

    let contract_address_str = contract_address
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
        .join(",");

    let query = format!(
        "/getOwnerNFT?address={}&contract_address={}&limit={}&offset={}&metadata={}",
        address, contract_address_str, limit, offset, metadata,
    );

    let req = TestRequest::default()
        .uri(&query)
        .insert_header(("x-read-api-key", "testing_api_key"))
        .to_request();
    let resp = call_service(&app, req).await;
    println!("response:{:?}", resp.response());
    assert_eq!(resp.status(), http::StatusCode::OK);

    let query = doc! { "owner" : "0x8000000000000000000000000000000000000000" , "contract_id":"testing_contract"};
    let _ = delete_one(db.collection("owners"), query, DeleteOptions::default()).await;
    let query = doc! { "contract_address" : "0x8000000000000000000000000000000000000000" , "contract_id":"testing_contract"};
    let _ = delete_one(db.collection("nfts"), query, DeleteOptions::default()).await;
    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_owner_nft_missing_api_key() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_owner_nft),
    )
    .await;
    let contract_address = ["0x8000000000000000000000000000000000000000"];
    let address = "0x8000000000000000000000000000000000000000";
    let limit = 100;
    let offset = 0;
    let metadata = true;

    let contract_address_str = contract_address
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
        .join(",");

    let query = format!(
        "/getOwnerNFT?address={}&contract_address={}&limit={}&offset={}&metadata={}",
        address, contract_address_str, limit, offset, metadata,
    );

    let req = TestRequest::default().uri(&query).to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    let body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(
        body.as_ref(),
        b"Invalid header x-read-api-key or not provided"
    );
}

#[actix_web::test]
async fn get_owner_nft_missing_address() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_owner_nft),
    )
    .await;
    let contract_address = ["0x8000000000000000000000000000000000000000"];
    let address = "";
    let limit = 100;
    let offset = 0;
    let metadata = true;

    let contract_address_str = contract_address
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
        .join(",");

    let query = format!(
        "/getOwnerNFT?address={}&contract_address={}&limit={}&offset={}&metadata={}",
        address, contract_address_str, limit, offset, metadata,
    );

    let req = TestRequest::default()
        .uri(&query)
        .insert_header(("x-read-api-key", "testing_api_key"))
        .to_request();
    let resp = call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    let body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(body.as_ref(), b"No wallet address");

    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_nft_for_owner_empty_contract_address() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_owner_nft),
    )
    .await;
    let contract_address = [""];
    let address = "0x8000000000000000000000000000000000000000";
    let limit = 100;
    let offset = 0;
    let metadata = true;

    let contract_address_str = contract_address
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
        .join(",");

    let query = format!(
        "/getOwnerNFT?address={}&contract_address={}&limit={}&offset={}&metadata={}",
        address, contract_address_str, limit, offset, metadata,
    );

    let req = TestRequest::default()
        .uri(&query)
        .insert_header(("x-read-api-key", "testing_api_key"))
        .to_request();
    let resp = call_service(&app, req).await;

    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    let content_type_header = resp.response().headers().get(CONTENT_TYPE).unwrap();
    let content_type = content_type_header.to_str().unwrap();
    assert_eq!(content_type, "text/plain; charset=utf-8");

    let body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(body.as_ref(), b"Invalid contract address");

    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_nft_for_owner_invalid_contract_address() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;
    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_owner_nft),
    )
    .await;
    let contract_address = ["0x1", "invalid_address"];
    let address = "0x8000000000000000000000000000000000000000";
    let limit = 100;
    let offset = 0;
    let metadata = true;

    let contract_address_str = contract_address
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
        .join(",");

    let query = format!(
        "/getOwnerNFT?address={}&contract_address={}&limit={}&offset={}&metadata={}",
        address, contract_address_str, limit, offset, metadata,
    );

    let req = TestRequest::default()
        .uri(&query)
        .insert_header(("x-read-api-key", "testing_api_key"))
        .to_request();
    let resp = call_service(&app, req).await;

    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    let content_type_header = resp.response().headers().get(CONTENT_TYPE).unwrap();
    let content_type = content_type_header.to_str().unwrap();
    assert_eq!(content_type, "text/plain; charset=utf-8");

    let body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(body.as_ref(), b"Invalid contract address");

    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_owner_nft_invalid_address() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;

    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_owner_nft),
    )
    .await;

    let contract_address = ["0x8000000000000000000000000000000000000000"];
    let address = "invalidaddress";
    let limit = 100;
    let offset = 0;
    let metadata = true;

    let contract_address_str = contract_address
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
        .join(",");

    let query = format!(
        "/getOwnerNFT?address={}&contract_address={}&limit={}&offset={}&metadata={}",
        address, contract_address_str, limit, offset, metadata,
    );

    let req = TestRequest::default()
        .uri(&query)
        .insert_header(("x-read-api-key", "testing_api_key"))
        .to_request();
    let resp = call_service(&app, req).await;

    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let body_str = std::str::from_utf8(&body).unwrap();

    assert_eq!(body_str, "No wallet address");

    delete_test_api_key(db.clone()).await;
}

#[actix_web::test]
async fn get_owner_nft_invalid_api_key() {
    dotenv().ok();
    let db = connect_to_mongodb_test().await.unwrap();
    create_test_api_key(db.clone()).await;

    let app = init_service(
        App::new()
            .app_data(Data::new(AppState { db: db.clone() }))
            .service(get_owner_nft),
    )
    .await;

    let contract_address = ["0x8000000000000000000000000000000000000000"];
    let address = "0x8000000000000000000000000000000000000000";
    let limit = 100;
    let offset = 0;
    let metadata = true;

    let contract_address_str = contract_address
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
        .join(",");

    let query = format!(
        "/getOwnerNFT?address={}&contract_address={}&limit={}&offset={}&metadata={}",
        address, contract_address_str, limit, offset, metadata,
    );

    let req = TestRequest::default()
        .uri(&query)
        .insert_header(("x-read-api-key", "invalid_api_key"))
        .to_request();
    let resp = call_service(&app, req).await;

    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);

    let content_type_header = resp.response().headers().get(CONTENT_TYPE).unwrap();
    let content_type = content_type_header.to_str().unwrap();
    assert_eq!(content_type, "text/plain; charset=utf-8");

    let body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(
        body.as_ref(),
        b"Invalid header x-read-api-key or not provided"
    );

    delete_test_api_key(db.clone()).await;
}
