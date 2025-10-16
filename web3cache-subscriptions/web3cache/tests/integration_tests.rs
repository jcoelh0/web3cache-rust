extern crate dotenv;
use actix_service::Service;
use actix_web::{dev::ServiceResponse};
use actix_web::{http::StatusCode, test, web, App};
use bson::doc;
use bson::oid::ObjectId;
use mongodb::options::{FindOneOptions, InsertOneOptions};
use serde::{Deserialize, Serialize};
use serde_json::json;
use web3cache::database::*;
use web3cache::helper_functions::AppState;
use web3cache::subscription_api::{
  contract_invalidation, contract_registration, delete_subscription_from_subid,
  get_contract_from_id, get_contracts, get_subscription_from_subid, get_subscriptions,
  replay_subscription, subscription_registration, subscription_state,
  update_subscription, webhook_health_check,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct DupSubscription {
  pub message: String,
  pub _id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Subscription {
  pub topics: Vec<String>,
  pub url: String,
  pub contract_id: String,
  pub _id: String,
}
async fn register_subscription(api_key: String) -> String {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/subscription-registration",
    web::post().to(subscription_registration),
  ))
  .await;
  let payload = json!({  "topics": [],"contract_id": "peeranha_user","url": "https://webhook.site/ab4ed3f3-f635-4761-accc-"});
  let req = test::TestRequest::post()
    .uri("/subscription-registration")
    .append_header(("x-webhook-api-key", api_key))
    .set_json(payload)
    .to_request();
  let response = test::call_service(&app, req).await;
  if response.response().status().is_client_error() {
    let body: DupSubscription = test::read_body_json(response).await;
    return body._id;
  } else {
    let body: Subscription = test::read_body_json(response).await;
    return body._id;
  }
}
async fn delete_subscription(sub_id: String, api_key: String) {
  let db = connect_to_mongodb(true).await.unwrap();

  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/delete-subscription/{sub_id}",
    web::post().to(delete_subscription_from_subid),
  ))
  .await;
  let req = test::TestRequest::post()
    .uri(format!("/delete-subscription/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", api_key))
    .to_request();
  let response = test::call_service(&app, req).await;
  assert!(response.status().is_success());
}

async fn register_test_contract(
  contract_id: &str,
  chain: &str,
  contract_address: &str,
) -> mongodb::error::Result<()> {
  let db = connect_to_mongodb(true).await.unwrap();
  let contract_doc = doc! {
      "contract_id": contract_id,
      "chain": chain,
      "contract_address": contract_address,
  };

  let _insert_contract = create_entry(
    db.collection("contracts"),
    contract_doc,
    mongodb::options::InsertOneOptions::default(),
  )
  .await?;

  Ok(())
}

async fn delete_test_contract(contract_id: &str) -> mongodb::error::Result<()> {
  let db = connect_to_mongodb(true).await.unwrap();
  let filter = doc! {
      "contract_id": contract_id,
  };

  let _delete_contract = delete_one(db.collection("contracts"), filter).await?;
  Ok(())
}

#[actix_web::test]
async fn get_subscription_by_id_success() {
  let sub_id = register_subscription("test_get_subscription".to_string()).await;
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/subscription/{sub_id}",
    web::get().to(get_subscription_from_subid),
  ))
  .await;
  let req = test::TestRequest::get()
    .uri(format!("/subscription/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "test_get_subscription"))
    .to_request();
  let response = test::call_service(&app, req).await;
  println!("response: {:?}", response.response());
  delete_subscription(sub_id, "test_get_subscription".to_string()).await;
  assert!(response.status().is_success());
}

#[actix_web::test]
async fn get_subscriptions_success() {
  let sub_id = register_subscription("test_get_subscriptions".to_string()).await;
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(
    App::new()
      .app_data(web::Data::new(AppState { db }))
      .route("/subscriptions", web::get().to(get_subscriptions)),
  )
  .await;
  let req = test::TestRequest::get()
    .uri("/subscriptions")
    .append_header(("x-webhook-api-key", "test_get_subscriptions"))
    .to_request();
  let response = test::call_service(&app, req).await;
  delete_subscription(sub_id, "test_get_subscriptions".to_string()).await;
  assert!(response.status().is_success());
}

#[actix_web::test]
async fn get_subscriptions_not_found() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(
    App::new()
      .app_data(web::Data::new(AppState { db }))
      .route("/subscriptions", web::get().to(get_subscriptions)),
  )
  .await;
  let req = test::TestRequest::get()
    .uri("/subscriptions")
    .append_header(("x-webhook-api-key", "sub_not_found"))
    .to_request();
  let resp = test::call_service(&app, req).await;
  assert_eq!(resp.status(), StatusCode::OK);

  let body = test::read_body(resp).await;
  println!("response: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"No subscription found\"}")
  );
}

#[actix_web::test]
async fn webhook_health_check_success() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(
    App::new()
      .app_data(web::Data::new(AppState { db }))
      .route("/webhook-health-check", web::get().to(webhook_health_check)),
  )
  .await;
  let req = test::TestRequest::get()
    .uri("/webhook-health-check")
    .to_request();
  let response = test::call_service(&app, req).await;
  print!("response: {:?}", response.response());
  assert!(response.status().is_success());
}
/* 
#[actix_web::test]
async fn register_subscription_success() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/subscription-registration",
    web::post().to(subscription_registration),
  ))
  .await;
  let payload = json!({  "topics": [],"contract_id": "peeranha_user","url": "https://webhook.site/ab4ed3f3-f635-4761-accc-cf8ab81ceaf3" });
  let req = test::TestRequest::post()
    .uri("/subscription-registration")
    .append_header(("x-webhook-api-key", "test_register_subscription"))
    .set_json(payload)
    .to_request();
  let response = test::call_service(&app, req).await;
  println!("response: {:?}", response.response().body());
  let body: Subscription = test::read_body_json(response).await;
  delete_subscription(body._id, "test_register_subscription".to_string()).await;
} */

#[actix_web::test]
async fn get_contracts_success() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(
    App::new()
      .app_data(web::Data::new(AppState { db }))
      .route("/get-contracts", web::get().to(get_contracts)),
  )
  .await;
  let req = test::TestRequest::get()
    .uri("/get-contracts")
    .append_header(("x-webhook-api-key", "test_get_contracts"))
    .to_request();
  let response = test::call_service(&app, req).await;
  println!("response: {:?}", response.response());
  assert!(response.status().is_success());
}

#[actix_web::test]
async fn get_contract_from_contract_id_success() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/get-contract/{contract_id}",
    web::get().to(get_contract_from_id),
  ))
  .await;
  let contract_id = "peeranha_user";
  let req = test::TestRequest::get()
    .append_header(("x-webhook-api-key", "test_get_contract"))
    .uri(format!("/get-contract/{}", contract_id).as_str())
    .to_request();
  let response = test::call_service(&app, req).await;
  println!("response: {:?}", response.response().body());
  assert!(response.status().is_success());
}

#[actix_web::test]
async fn delete_subscription_success() {
  let sub_id = register_subscription("test_delete_subscription".to_string()).await;
  let db = connect_to_mongodb(true).await.unwrap();

  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/delete-subscription/{sub_id}",
    web::post().to(delete_subscription_from_subid),
  ))
  .await;
  let req = test::TestRequest::post()
    .uri(format!("/delete-subscription/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "test_delete_subscription"))
    .to_request();
  let response = test::call_service(&app, req).await;
  println!("response: {:?}", response.response().body());
  assert!(response.status().is_success());
}

#[actix_web::test]
async fn update_subscription_success() {
  let sub_id = register_subscription("test_update_subscription".to_string()).await;
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/update-subscription/{sub_id}",
    web::post().to(update_subscription),
  ))
  .await;
  let payload = json!({  "eventsAdd": ["transfer"]});
  let req = test::TestRequest::post()
    .uri(format!("/update-subscription/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "test_update_subscription"))
    .set_json(payload)
    .to_request();
  let response = test::call_service(&app, req).await;
  println!("response: {:?}", response.response().body());
  delete_subscription(sub_id, "test_update_subscription".to_string()).await;
  assert!(response.status().is_success());
}

#[actix_web::test]
async fn replay_subscription_success() {
  let sub_id = register_subscription("test_replay_subscription".to_string()).await;
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/replay-subscription/{sub_id}",
    web::post().to(replay_subscription),
  ))
  .await;
  let payload = json!({  "block_number": 0});
  let req = test::TestRequest::post()
    .uri(format!("/replay-subscription/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "test_replay_subscription"))
    .set_json(payload)
    .to_request();
  let response = test::call_service(&app, req).await;
  println!("response: {:?}", response.response().body());
  delete_subscription(sub_id, "test_replay_subscription".to_string()).await;
  assert!(response.status().is_success());
}
#[actix_web::test]
async fn subscription_state_success() {
  let sub_id = register_subscription("test_state_subscription".to_string()).await;
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/subscription_state/{sub_id}",
    web::post().to(subscription_state),
  ))
  .await;
  let payload = json!({  "activate": true});
  let req = test::TestRequest::post()
    .uri(format!("/subscription_state/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "test_state_subscription"))
    .set_json(payload)
    .to_request();
  let response = test::call_service(&app, req).await;
  println!("response: {:?}", response.response().body());
  delete_subscription(sub_id, "test_state_subscription".to_string()).await;
  assert!(response.status().is_success());
}

#[actix_web::test]
async fn missing_api_key() {
  let sub_id = register_subscription("missing_api_key".to_string()).await;
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(
    App::new()
      .app_data(web::Data::new(AppState { db }))
      .route(
        "/subscription_state/{sub_id}",
        web::post().to(subscription_state),
      )
      .route(
        "/replay-subscription/{sub_id}",
        web::post().to(replay_subscription),
      )
      .route(
        "/update-subscription/{sub_id}",
        web::post().to(update_subscription),
      )
      .route(
        "/delete-subscription/{sub_id}",
        web::post().to(delete_subscription_from_subid),
      )
      .route(
        "/get-contract/{contract_id}",
        web::get().to(get_contract_from_id),
      )
      .route("/get-contracts", web::get().to(get_contracts))
      .route(
        "/subscription-registration",
        web::post().to(subscription_registration),
      )
      .route("/subscriptions", web::get().to(get_subscriptions))
     /*  .route(
        "/sui-contract-registration",
        web::post().to(sui_contract_registration),
      ) */
      .route(
        "/subscription/{sub_id}",
        web::get().to(get_subscription_from_subid),
      ),
  )
  .await;
  let payload = json!({  "activate": true});
  let req = test::TestRequest::post()
    .uri(format!("/subscription_state/{}", sub_id).as_str())
    .set_json(payload)
    .to_request();
  let response: ServiceResponse = app.call(req).await.unwrap();
  let body = test::read_body(response).await;
  println!("response sub state: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"Invalid api key\"}")
  );

  let payload = json!({  "block_number": 0});
  let req = test::TestRequest::post()
    .uri(format!("/replay-subscription/{}", sub_id).as_str())
    .set_json(payload)
    .to_request();
  let response = test::call_service(&app, req).await;

  let body = test::read_body(response).await;
  println!("response replay sub: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"missing x-webhook-api-key\"}")
  );

  let payload = json!({  "eventsAdd": ["transfer"]});
  let req = test::TestRequest::post()
    .uri(format!("/update-subscription/{}", sub_id).as_str())
    .set_json(payload)
    .to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response update sub: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"missing x-webhook-api-key\"}")
  );

  let req = test::TestRequest::post()
    .uri(format!("/delete-subscription/{}", sub_id).as_str())
    .to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response delete sub: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"missing x-webhook-api-key\"}")
  );

  let contract_id = "peeranha_user";
  let req = test::TestRequest::get()
    .uri(format!("/get-contract/{}", contract_id).as_str())
    .to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response get contract: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"missing x-webhook-api-key\"}")
  );

  let req = test::TestRequest::get().uri("/get-contracts").to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response get contracts: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"missing x-webhook-api-key\"}")
  );

  let payload = json!({  "topics": [],"contract_id": "peeranha_user","url": "https://webhook.site/ab4ed3f3-f635-4761-accc-cf8ab81ceaf3" });
  let req = test::TestRequest::post()
    .uri("/subscription-registration")
    .set_json(payload)
    .to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response register sub: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"missing x-webhook-api-key\"}")
  );

  let req = test::TestRequest::get().uri("/subscriptions").to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response get subs: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"missing x-webhook-api-key\"}")
  );

  let registration_payload = json!({
      "contract_id": "test_sui_contract_registration",
      "contract_address": "0x394E3d3044fC89fCDd966D3cb35Ac0B32B0Cda91",
      "events": "event_one,event_two",
      "module": "test_module",
  });

  // Send the request with an API key
  let _req = test::TestRequest::post()
    .uri("/sui-contract-registration")
    .set_json(&registration_payload)
    .to_request();
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"missing x-webhook-api-key\"}")
  );
  let req = test::TestRequest::get()
    .uri(format!("/subscription/{}", sub_id).as_str())
    .to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response get sub : {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"missing x-webhook-api-key\"}")
  );
  delete_subscription(sub_id, "missing_api_key".to_string()).await;
}

#[actix_web::test]
async fn get_subscription_by_id_invalid_subid() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/subscription/{sub_id}",
    web::get().to(get_subscription_from_subid),
  ))
  .await;
  let mut sub_id = "jz381054bca84737ce59b180a";
  let req = test::TestRequest::get()
    .uri(format!("/subscription/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "test_get_subscription"))
    .to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"invalid sub_id\"}")
  );
  sub_id = "6381054bca84737ce59b18-a";
  let req = test::TestRequest::get()
    .uri(format!("/subscription/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "test_get_subscription"))
    .to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"invalid sub_id\"}")
  );
}

#[actix_web::test]
async fn get_subscription_not_found() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/subscription/{sub_id}",
    web::get().to(get_subscription_from_subid),
  ))
  .await;

  let req = test::TestRequest::get()
    .uri("/subscription/607d9c9c2cf7e54c0f1d10c8")
    .append_header(("x-webhook-api-key", "test_get_subscription"))
    .to_request();

  let resp = test::call_service(&app, req).await;
  assert_eq!(resp.status(), StatusCode::NOT_FOUND);

  let body = test::read_body(resp).await;
  println!("response: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"Subscription not found\"}")
  );
}
#[actix_web::test]
async fn delete_subscription_invalid_subid() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/delete-subscription/{sub_id}",
    web::post().to(delete_subscription_from_subid),
  ))
  .await;

  let mut sub_id = "jz381054bca84737ce59b180a";
  let req = test::TestRequest::post()
    .uri(format!("/delete-subscription/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "api_key"))
    .to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"invalid sub_id\"}")
  );
  sub_id = "6381054bca84737ce59b18-a";
  let req = test::TestRequest::post()
    .uri(format!("/delete-subscription/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "api_key"))
    .to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"invalid sub_id\"}")
  );
}

#[actix_web::test]
async fn delete_subscription_not_found() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/delete-subscription/{sub_id}",
    web::post().to(delete_subscription_from_subid),
  ))
  .await;

  let sub_id = "607d9c9c2cf7e54c0f1d10c8";
  let req = test::TestRequest::post()
    .uri(format!("/delete-subscription/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "api_key"))
    .to_request();

  let resp = test::call_service(&app, req).await;
  assert_eq!(resp.status(), StatusCode::NOT_FOUND);

  let body = test::read_body(resp).await;
  println!("response: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"Subscription not found\"}")
  );
}

#[actix_web::test]
async fn replay_subscription_invalid_subid() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/replay-subscription/{sub_id}",
    web::post().to(replay_subscription),
  ))
  .await;
  let payload = json!({  "block_number": 0});

  let mut sub_id = "jz381054bca84737ce59b180a";
  let req = test::TestRequest::post()
    .uri(format!("/replay-subscription/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "test_replay_subscription"))
    .set_json(payload.clone())
    .to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"invalid sub_id\"}")
  );

  sub_id = "6381054bca84737ce59b18-a";
  let req = test::TestRequest::post()
    .uri(format!("/replay-subscription/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "test_replay_subscription"))
    .set_json(payload)
    .to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"invalid sub_id\"}")
  );
}

#[actix_web::test]
async fn replay_subscription_not_found() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/replay-subscription/{sub_id}",
    web::post().to(replay_subscription),
  ))
  .await;
  let payload = json!({  "block_number": 0});

  let sub_id = "607d9c9c2cf7e54c0f1d10c8";
  let req = test::TestRequest::post()
    .uri(format!("/replay-subscription/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "test_replay_subscription"))
    .set_json(payload.clone())
    .to_request();

  let resp = test::call_service(&app, req).await;
  assert_eq!(resp.status(), StatusCode::NOT_FOUND);

  let body = test::read_body(resp).await;
  println!("response: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"Subscription not found\"}")
  );
}

#[actix_web::test]
async fn subscription_state_invalid_subid() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/subscription_state/{sub_id}",
    web::post().to(subscription_state),
  ))
  .await;
  let mut sub_id = "jz381054bca84737ce59b180a";

  let payload = json!({  "activate": true});
  let req = test::TestRequest::post()
    .uri(format!("/subscription_state/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "test_state_subscription"))
    .set_json(payload.clone())
    .to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"invalid sub_id\"}")
  );

  sub_id = "6381054bca84737ce59b18-a";
  let req = test::TestRequest::post()
    .uri(format!("/subscription_state/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "test_state_subscription"))
    .set_json(payload)
    .to_request();
  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(b"{\"message\":\"invalid sub_id\"}")
  );
}

#[actix_web::test]
async fn get_contract_invalid_contract_id() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/get-contract/{contract_id}",
    web::get().to(get_contract_from_id),
  ))
  .await;
  let contract_id = "invalid_contract";
  let req = test::TestRequest::get()
    .append_header(("x-webhook-api-key", "test_get_contract"))
    .uri(format!("/get-contract/{}", contract_id).as_str())
    .to_request();

  let response = test::call_service(&app, req).await;
  let body = test::read_body(response).await;
  println!("response: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(
      b"{\"message\":\"Contract ID not found, please register your contract.\"}"
    )
  );
}

/* #[actix_web::test]
async fn register_subscription_invalid_contract_id() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/subscription-registration",
    web::post().to(subscription_registration),
  ))
  .await;
  let payload = json!({  "topics": [],"contract_id": "invalid_contract","url": "https://webhook.site/ab4ed3f3-f635-4761-accc-cf8ab81ceaf3" });
  let req = test::TestRequest::post()
    .uri("/subscription-registration")
    .append_header(("x-webhook-api-key", "test_register_subscription"))
    .set_json(payload)
    .to_request();
  let response = test::call_service(&app, req).await;
  println!("response: {:?}", response.response().body());
  let body = test::read_body(response).await;
  println!("response: {:?}", body);
  assert_eq!(
    body,
    actix_web::web::Bytes::from_static(
      b"{\"message\":\"Contract ID not found, please register your contract.\"}"
    )
  );
} */
/* 
#[actix_web::test]
async fn register_subscription_duplicate() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/subscription-registration",
    web::post().to(subscription_registration),
  ))
  .await;

  // Create an initial subscription
  let payload = json!({
      "topics": ["topic1", "topic2"],
      "contract_id": "peeranha_user",
      "url": "https://webhook.site/ab4"
  });
  let req = test::TestRequest::post()
    .uri("/subscription-registration")
    .append_header(("x-webhook-api-key", "test_register_subscription"))
    .set_json(&payload)
    .to_request();
  let response = test::call_service(&app, req).await;
  println!("{:?}", response.status().is_success());
  assert!(
    response.status().is_success(),
    "Initial subscription failed"
  );

  let initial_sub_body = test::read_body(response).await;
  let initial_sub_json: serde_json::Value = serde_json::from_slice(&initial_sub_body).unwrap();
  let sub_id = initial_sub_json["_id"].as_str().unwrap().to_string();

  // Try to create a duplicate subscription with the same contract_id, api_key, and url, but different topics
  let payload_duplicate = json!({
      "topics": ["topic1", "topic2"],
      "contract_id": "peeranha_user",
      "url": "https://webhook.site/ab4"
  });
  let req_duplicate = test::TestRequest::post()
    .uri("/subscription-registration")
    .append_header(("x-webhook-api-key", "test_register_subscription"))
    .set_json(&payload_duplicate)
    .to_request();
  let response_duplicate = test::call_service(&app, req_duplicate).await;
  assert_eq!(
    response_duplicate.status(),
    StatusCode::BAD_REQUEST,
    "Duplicate check failed"
  );

  let body = test::read_body(response_duplicate).await;
  let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
  assert_eq!(
    body_json["message"].as_str().unwrap(),
    "Subscription already exists",
    "Incorrect response message"
  );

  // Clean up by deleting the initial subscription
  delete_subscription(sub_id, "test_register_subscription".to_string()).await;
} */
/* 
#[actix_web::test]
async fn register_subscription_with_block_number() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/subscription-registration",
    web::post().to(subscription_registration),
  ))
  .await;

  // Create a subscription with a block_number
  let payload = json!({
      "topics": ["topic3", "topic4"],
      "contract_id": "peeranha_user",
      "url": "https://webhook.site/ab4e",
      "block_number": 42
  });
  let req = test::TestRequest::post()
    .uri("/subscription-registration")
    .append_header(("x-webhook-api-key", "test_register_subscription"))
    .set_json(&payload)
    .to_request();
  let response = test::call_service(&app, req).await;
  assert!(
    response.status().is_success(),
    "Subscription with block_number failed"
  );

  let initial_sub_body = test::read_body(response).await;
  let initial_sub_json: serde_json::Value = serde_json::from_slice(&initial_sub_body).unwrap();
  let sub_id = initial_sub_json["_id"].as_str().unwrap().to_string();
  // Clean up by deleting the subscription
  delete_subscription(sub_id, "test_register_subscription".to_string()).await;
} */

#[actix_web::test]
async fn update_subscription_invalid_body() {
  let sub_id = register_subscription("test_update_subscription_invalid".to_string()).await;
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/update-subscription/{sub_id}",
    web::put().to(update_subscription),
  ))
  .await;

  // Create an invalid update_sub object
  let invalid_payload = json!({
      "url": "",
      "add_topics": ["transfer"]
  });

  let req = test::TestRequest::put()
    .uri(format!("/update-subscription/{}", sub_id).as_str())
    .append_header(("x-webhook-api-key", "test_update_subscription_invalid"))
    .set_json(invalid_payload)
    .to_request();

  let response = test::call_service(&app, req).await;
  println!("response: {:?}", response.response().body());

  delete_subscription(sub_id, "test_update_subscription_invalid".to_string()).await;

  // Assert that the response has a BadRequest status
  assert_eq!(response.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn update_subscription_set_topics_success() {
  let sub_id = register_subscription("test_update_subscription_set_topics".to_string()).await;
  let db = connect_to_mongodb(true).await.unwrap();

  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/update-subscription/{sub_id}",
    web::put().to(update_subscription),
  ))
  .await;

  // Create a payload with set_topics
  let payload = json!({
      "set_topics": ["topic1", "topic2"]
  });

  let req = test::TestRequest::put()
    .uri(format!("/update-subscription/{}", sub_id.clone()).as_str())
    .append_header(("x-webhook-api-key", "test_update_subscription_set_topics"))
    .set_json(payload)
    .to_request();

  let response = test::call_service(&app, req).await;
  println!("response: {:?}", response.response().body());

  let db = connect_to_mongodb(true).await.unwrap();
  // Check if the topics were updated in the database
  let object_id = ObjectId::parse_str(sub_id.clone()).unwrap();
  let subscription = find_one(
    db.collection("subscriptions"),
    doc! {"_id": object_id , "apikey": "test_update_subscription_set_topics"},
    FindOneOptions::default(),
  )
  .await
  .unwrap()
  .unwrap();

  let topics: Vec<String> = subscription
    .get("topics")
    .unwrap()
    .as_array()
    .unwrap()
    .iter()
    .map(|bson| bson.as_str().unwrap().to_string())
    .collect();

  delete_subscription(sub_id, "test_update_subscription_set_topics".to_string()).await;
  assert!(response.status().is_success());
  assert_eq!(topics, vec!["topic1", "topic2"]);
}

#[actix_web::test]
async fn contract_registration_success() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(
    App::new()
      .app_data(web::Data::new(AppState { db })) // Make sure to clone the db
      .route(
        "/contract-registration",
        web::post().to(contract_registration),
      ),
  )
  .await;

  // Create a valid contract registration payload
  let valid_payload = json!({
      "contract_id": "test_contract",
      "chain": "ethereum",
      "contract_address": "0x394E3d3044fC89fCDd966D3cb35Ac0B32B0Cda91", // Example Ethereum contract address
  });

  let req = test::TestRequest::post()
    .uri("/contract-registration")
    .append_header(("x-webhook-api-key", "testing_api_key"))
    .set_json(&valid_payload)
    .to_request();

  /* let url = format!("{}/start-write-service/existing_contract", "fads");
  let response = reqwest::get(&url).await.unwrap();
 */
  let response = test::call_service(&app, req).await;

  println!("{:?}", response);
  // Assert that the response has an OK status
  //println!("{:?}", response.response().body());
  assert_eq!(response.status(), actix_web::http::StatusCode::OK);

  // Clean up: Remove the test contract from the database
  delete_test_contract("test_contract").await.unwrap();
}

#[actix_web::test]
async fn contract_registration_invalid_payload() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(
    App::new()
      .app_data(web::Data::new(AppState { db })) // Make sure to clone the db
      .route(
        "/contract-registration",
        web::post().to(contract_registration),
      ),
  )
  .await;

  // Create an invalid contract registration payload with a missing required field
  let invalid_payload = json!({
      "chain": "ethereum",
      "contract_address": "0x394E3d3044fC89fCDd966D3cb35Ac0B32B0Cda91", // Example Ethereum contract address
  });

  let req = test::TestRequest::post()
    .uri("/contract-registration")
    .append_header(("x-webhook-api-key", "testing_api_key"))
    .set_json(&invalid_payload)
    .to_request();

  let response = test::call_service(&app, req).await;

  // Assert that the response has a Bad Request status
  assert_eq!(response.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn contract_registration_invalid_chain() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(
    App::new()
      .app_data(web::Data::new(AppState { db })) // Make sure to clone the db
      .route(
        "/contract-registration",
        web::post().to(contract_registration),
      ),
  )
  .await;

  // Create a contract registration payload with an invalid chain value
  let invalid_payload = json!({
      "contract_id": "test_contract_invalid_chain",
      "chain": "invalid_chain",
      "contract_address": "0x394E3d3044fC89fCDd966D3cb35Ac0B32B0Cda91", // Example Ethereum contract address
  });

  let req = test::TestRequest::post()
    .uri("/contract-registration")
    .append_header(("x-webhook-api-key", "testing_api_key"))
    .set_json(&invalid_payload)
    .to_request();

  let response = test::call_service(&app, req).await;

  // Assert that the response has a Bad Request status
  assert_eq!(response.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn contract_registration_events_with_underscores() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(
    App::new()
      .app_data(web::Data::new(AppState { db })) // Make sure to clone the db
      .route(
        "/contract-registration",
        web::post().to(contract_registration),
      ),
  )
  .await;

  // Create a contract registration payload with events that have spaces in their names
  let events_with_spaces_payload = json!({
      "contract_id": "test_contract_space",
      "chain": "ethereum",
      "contract_address": "0x394E3d3044fC89fCDd966D3cb35Ac0B32B0Cda91", // Example Ethereum contract address
      "events": "event, event_with_underscores",
  });

  let req = test::TestRequest::post()
    .uri("/contract-registration")
    .append_header(("x-webhook-api-key", "testing_api_key"))
    .set_json(&events_with_spaces_payload)
    .to_request();

  let response = test::call_service(&app, req).await;
  println!("{:?}", response.response().body());
  // Assert that the response has an OK status
  assert_eq!(response.status(), actix_web::http::StatusCode::OK);

  let db = connect_to_mongodb(true).await.unwrap();
  let db_result = find_one(
    db.collection("contracts"),
    doc! { "contract_id": "test_contract_space" },
    FindOneOptions::default(),
  )
  .await
  .unwrap();
  let doc = db_result.unwrap();
  let stored_events = doc.get_str("events").unwrap();
  assert_eq!(stored_events, "event,event_with_underscores");
  delete_test_contract("test_contract_space").await.unwrap();
}

#[actix_web::test]
async fn contract_registration_test() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(
    App::new()
      .app_data(web::Data::new(AppState { db })) // Make sure to clone the db
      .route(
        "/contract-registration",
        web::post().to(contract_registration),
      ),
  )
  .await;

  // Create a contract registration payload for an existing contract
  let existing_contract_payload = json!({
      "contract_id": "existing_contract",
      "chain": "ethereum",
      "contract_address": "0x394E3d3044fC89fCDd966D3cb35Ac0B32B0Cda91", // Example Ethereum contract address
  });

  // Add the existing contract to the database
  let existing_contract_doc = doc! {
      "contract_id": "existing_contract",
      "contract_address": "0x394E3d3044fC89fCDd966D3cb35Ac0B32B0Cda91",
      "contract_abi": "[{\"inputs\":[{\"internalType\":\"address\",\"name\":\"_logic\",\"type\":\"address\"},{\"internalType\":\"bytes\",\"name\":\"_data\",\"type\":\"bytes\"}],\"stateMutability\":\"payable\",\"type\":\"constructor\"},{\"anonymous\":false,\"inputs\":[{\"indexed\":false,\"internalType\":\"address\",\"name\":\"previousAdmin\",\"type\":\"address\"},{\"indexed\":false,\"internalType\":\"address\",\"name\":\"newAdmin\",\"type\":\"address\"}],\"name\":\"AdminChanged\",\"type\":\"event\"},{\"anonymous\":false,\"inputs\":[{\"indexed\":true,\"internalType\":\"address\",\"name\":\"beacon\",\"type\":\"address\"}],\"name\":\"BeaconUpgraded\",\"type\":\"event\"},{\"anonymous\":false,\"inputs\":[{\"indexed\":true,\"internalType\":\"address\",\"name\":\"implementation\",\"type\":\"address\"}],\"name\":\"Upgraded\",\"type\":\"event\"},{\"stateMutability\":\"payable\",\"type\":\"fallback\"},{\"stateMutability\":\"payable\",\"type\":\"receive\"}]",
      "contract_block_number": 15455855,
      "owner_block_number": 15455855,
      "transfer_block_number": 15455855,
      "chain_address_wss": "wss://quick-practical-pond.quiknode.pro/c96a68010922d81c423f74b182791ebb149ae085/",
      "chain_address_https": "https://quick-practical-pond.quiknode.pro/c96a68010922d81c423f74b182791ebb149ae085/",
      "events": "AdminChanged,BeaconUpgraded,Upgraded",
      "status_requirement": "online"
  };
  let db = connect_to_mongodb(true).await.unwrap();
  let _ = create_entry(
    db.clone().collection("contracts"),
    existing_contract_doc,
    InsertOneOptions::default(),
  )
  .await;

  let req = test::TestRequest::post()
    .uri("/contract-registration")
    .append_header(("x-webhook-api-key", "testing_api_key"))
    .set_json(&existing_contract_payload)
    .to_request();

  let response = test::call_service(&app, req).await;

  // Assert that the response has an OK status
  println!("{:?}", response.response().body());
  assert_eq!(response.status(), actix_web::http::StatusCode::OK);

  // Assert that the existing contract is updated
  let db = connect_to_mongodb(true).await.unwrap();
  let db_result = find_one(
    db.clone().collection("contracts"),
    doc! { "contract_id": "existing_contract" },
    FindOneOptions::default(),
  )
  .await
  .unwrap();
  let doc = db_result.unwrap();
  let stored_contract_address = doc.get_str("contract_address").unwrap();
  assert_eq!(
    stored_contract_address,
    "0x394E3d3044fC89fCDd966D3cb35Ac0B32B0Cda91"
  );
  delete_test_contract("existing_contract").await.unwrap();
}

#[actix_web::test]
async fn sui_contract_registration_success() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/contract-registration",
    web::post().to(contract_registration),
  ))
  .await;

  // Create a contract registration payload
  let registration_payload = json!({
      "contract_id": "sui_test_contract_registration",
      "chain": "sui_testnet",
      "contract_address": "0x394E3d3044fC89fCDd966D3cb35Ac0B32B0Cda91",
  });

  // Send the request with an API key
  let req = test::TestRequest::post()
    .uri("/contract-registration")
    .append_header(("x-webhook-api-key", "ICJA26UZJDIMCPVBMMHHA9J72FTWNTGERJ"))
    .set_json(&registration_payload)
    .to_request();
  let response = test::call_service(&app, req).await;
  println!("{:?}", response.response().body());
  // Assert that the response has an OK status
  assert_eq!(response.status(), actix_web::http::StatusCode::OK);

  // Clean up the test contract
  delete_test_contract("sui_test_contract_registration")
    .await
    .unwrap();
}

#[actix_web::test]
async fn sui_contract_registration_validation_error() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/contract-registration",
    web::post().to(contract_registration),
  ))
  .await;

  // Create a contract registration payload with empty events
  let registration_payload = json!({
      "contract_id": "sui_validation_error",
      "contract_address": "0x394E3d3044fC89fCDd966D3cb35Ac0B32B0Cda91",
      "events": "",
      "module": "test_module",
  });

  let req = test::TestRequest::post()
    .uri("/contract-registration")
    .append_header(("x-webhook-api-key", "test_key_sui"))
    .set_json(&registration_payload)
    .to_request();
  let response = test::call_service(&app, req).await;

  // Assert that the response has a BadRequest status
  assert_eq!(response.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn contract_invalidation_contract_not_modified() {
  let db = connect_to_mongodb(true).await.unwrap();
  let app = test::init_service(App::new().app_data(web::Data::new(AppState { db })).route(
    "/contract-invalidation/{contract_id}",
    web::post().to(contract_invalidation),
  ))
  .await;

  let req = test::TestRequest::post()
    .uri("/contract-invalidation/non_existent_contract")
    .append_header(("x-webhook-api-key", "ICJA26UZJDIMCPVBMMHHA9J72FTWNTGERJ"))
    .to_request();
  let response = test::call_service(&app, req).await;

  // Assert that the response has a BadRequest status
  assert_eq!(response.status(), actix_web::http::StatusCode::BAD_REQUEST);

  let response_body: serde_json::Value = test::read_body_json(response).await;
  println!("{:?}", response_body);
  assert_eq!(
    response_body["message"],
    "Contract non_existent_contract did not exist or was not modified"
  );
}