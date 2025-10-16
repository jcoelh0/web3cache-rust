mod contract_registration_lib;
mod database;
mod helper_functions;
mod logging;
mod subscription_api;
use actix_web::{
  web::{self},
  App, HttpServer,
};
use helper_functions::AppState;

use log::info;
use std::{env, fs};
extern crate dotenv;
use dotenv::dotenv;

use crate::{
  database::connect_to_mongodb,
  subscription_api::{contract_registration, get_contract_metadata, sui_contract_registration},
};

use crate::subscription_api::{
  contract_invalidation, delete_subscription_from_subid, get_contract_from_id, get_contracts,
  get_subscription_from_subid, get_subscriptions, replay_subscription, subscription_registration,
  subscription_state, update_subscription, webhook_health_check,
};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
  env_logger::init();
  dotenv().ok();

  //make sure controller url exists
  let _controller_url = env::var("CONTROLLERURL").unwrap();

  let subscriptions_port = env::var("SUBSCRIPTION_PORT")
    .unwrap_or_else(|_| "3000".to_string())
    .parse::<u16>()
    .unwrap();

  let db = connect_to_mongodb(false).await.unwrap();

  crate::custom_info!("Connected to mongodb");

  let db_clone = db.clone();
  //Subscription API

  crate::custom_info!("Subscriptions Server started on port {subscriptions_port}");
  let _subscriptions_server = HttpServer::new(move || {
    App::new()
      .app_data(web::Data::new(AppState {
        db: db_clone.clone(),
      }))
      .service(
        web::scope("/web3cache/events")
          .route(
            "/get-contract/{contract_id}",
            web::get().to(get_contract_from_id),
          )
          .route("/get-contracts", web::get().to(get_contracts))
          .route(
            "/subscription-registration",
            web::post().to(subscription_registration),
          )
          .route(
            "/subscription-state/{sub_id}",
            web::post().to(subscription_state),
          )
          .route(
            "/update-subscription/{sub_id}",
            web::post().to(update_subscription),
          )
          .route("/subscriptions", web::get().to(get_subscriptions))
          .route(
            "/subscription/{sub_id}",
            web::get().to(get_subscription_from_subid),
          )
          .route(
            "/delete-subscription/{sub_id}",
            web::post().to(delete_subscription_from_subid),
          )
          .route("/healthcheck", web::get().to(webhook_health_check))
          .route(
            "/replay-subscription/{sub_id}",
            web::post().to(replay_subscription),
          )
          .route(
            "/contract-registration",
            web::post().to(contract_registration),
          )
          .route(
            "/contract-invalidation/{contract_id}",
            web::post().to(contract_invalidation),
          )
          .route(
            "/get-contract-metadata/{contract_id}",
            web::get().to(get_contract_metadata),
          ),
      )
  })
  .bind(format!("0.0.0.0:{subscriptions_port}"))?
  .workers(1)
  .run()
  .await;

  Ok(())
}
