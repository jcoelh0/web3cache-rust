mod consumer_api;
mod database;
mod dispatcher;
mod helper_functions;

use actix_web::{web, App, HttpServer};
use helper_functions::AppState;

use log::info;
use std::{
  collections::{HashMap, LinkedList},
  env,
};
extern crate dotenv;
use dotenv::dotenv;

use crate::database::connect_to_mongodb;
use crate::{
  database::setup_indexes,
  dispatcher::{Dispatcher, DispatcherData},
};

use crate::consumer_api::{consumer_health_check, push_transactions};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
  env_logger::init();
  dotenv().ok();

  let consumer_port = env::var("CONSUMER_PORT")
    .unwrap_or_else(|_| "3001".to_string())
    .parse::<u16>()
    .unwrap();

  let db = connect_to_mongodb(false).await.unwrap();

  info!("Connected to mongodb");
  setup_indexes(&db).await?;

  //Subscription API
  let db_clone = db.clone();

  //Consumer API
  let consumer_server = HttpServer::new(move || {
    App::new()
      .app_data(web::Data::new(AppState {
        db: db_clone.clone(),
      }))
      .service(consumer_health_check)
      .service(push_transactions)
  })
  .bind(format!("0.0.0.0:{consumer_port}"))? //hardcoded TODO
  .workers(1)
  .run();

  tokio::spawn(async move {
    let db3 = connect_to_mongodb(false).await.unwrap();
    let mut dispatcher_data = DispatcherData {
      queue_list: LinkedList::new(),
      queue_map: &mut HashMap::new(),
    };

    DispatcherData::start_dispatcher(&mut dispatcher_data, &db3)
      .await
      .unwrap();
  });
  info!("CI/CD working");
  consumer_server.await?;
  Ok(())
}
