pub mod database;
pub mod helper_functions;
pub mod routes;
use crate::helper_functions::*;
use crate::routes::*;
use actix_web::{web, App, HttpServer};
use log::info;
use std::env;
extern crate dotenv;
use dotenv::dotenv;

use crate::database::connect_to_mongodb;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    dotenv().ok();
    env::remove_var("MONGOURI_TEST");
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap();

    let db = connect_to_mongodb().await.unwrap();

    info!("port chosen: {}", port);
    info!("Connected to mongodb");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState { db: db.clone() }))
            .service(
                web::scope("/web3cache/read")
                    .service(get_contract_nft)
                    .service(get_snapshot_contract_nft)
                    .service(get_owner_nft)
                    .service(get_user_transaction)
                    .service(get_contracts)
                    .service(get_contract)
                    .service(get_user_transaction_history)
                    .service(get_onwers)
                    .service(health_check),
            )
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
