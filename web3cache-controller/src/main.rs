mod database;
mod helper_functions;
mod routes;

use crate::database::find_all;
use crate::helper_functions::*;
use crate::routes::*;
use actix_web::{web, App, HttpServer};
use log::{error, info, warn};
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use serde_json::Value;

use std::env;
use std::fs::File;
use std::io::Read;
extern crate dotenv;
use dotenv::dotenv;

use crate::database::connect_to_mongodb;

use std::time::Duration;
use tokio::time::sleep;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    dotenv().ok();

    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap();

    //add_write_service("test_id").await.unwrap();

    //let default_env_load =
    let mut file = File::open("deployments/deployment.json").expect("Failed to open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read file");
    //
    //// Parse the JSON data into a vector of serde_json::Value objects
    let default_env_load: Vec<Value> =
        serde_json::from_str(&contents).expect("Failed to parse JSON");

    let db = connect_to_mongodb().await.unwrap();
    let db_clone = db.clone();

    info!("port chosen: {}", port);
    info!("Connected to mongodb");

    tokio::spawn(async move {
        let mut delay_sec = 1;
        loop {
            //A = grab writes from DB

            info!("Waiting for {delay_sec}");
            sleep(Duration::from_secs(delay_sec)).await;
            delay_sec = 30;

            let contracts_col = db.collection("contracts");
            let mut find_options = FindOptions::default();
            find_options.projection = Some(doc! { "contract_id": 1, "_id": 0});

            let mut missing_write_deployments: Vec<String> = match find_all(
                contracts_col,
                doc! {
                    "status_requirement": "online"
                },
                find_options,
            )
            .await
            {
                Ok(docs) => docs,
                Err(_err) => continue,
            }
            .iter()
            .map(|d| contractid_to_deployment(d.get_str("contract_id").unwrap()))
            .collect();

            info!("{:?}!", missing_write_deployments);

            let contracts_col = db.collection("contracts");
            let mut find_options = FindOptions::default();
            find_options.projection = Some(doc! { "contract_id": 1, "_id": 0});

            let mut dismissed_write_deployments: Vec<String> = match find_all(
                contracts_col,
                doc! {
                    "status_requirement": "offline"
                },
                find_options,
            )
            .await
            {
                Ok(docs) => docs,
                Err(_err) => continue,
            }
            .iter()
            .map(|d| contractid_to_deployment(d.get_str("contract_id").unwrap()))
            .collect();

            //B = grab active write services
            let result: Result<Vec<String>, _> = get_write_deployments().await;
            if let Err(e) = result {
                error!("get_write_deployments not working: {:?}", e);
                std::process::exit(1);
            }

            let active_web3cache_write_deployments = result.unwrap();

            warn!("{:?}", active_web3cache_write_deployments);

            missing_write_deployments.retain(|s| !active_web3cache_write_deployments.contains(s));

            dismissed_write_deployments.retain(|s| active_web3cache_write_deployments.contains(s));

            info!(
                "Active deployments: {:?}",
                active_web3cache_write_deployments
            );
            info!("Deploying deployments: {:?}", missing_write_deployments);

            info!("Deleting deployments: {:?}", dismissed_write_deployments);

            //left overs of active_web3cache_write_deployments need to be deleted
            for deployment_name in dismissed_write_deployments {
                match delete_write_deployments(deployment_name.clone()).await {
                    Ok(()) => info!("{deployment_name} deployment successfully deleted!"),
                    Err(error) => {
                        error!("{deployment_name} failed to delete deployment: {}", error)
                    }
                }
            }
            //left overs of db_web3cache_write_deployments need to be deployed
            for deployment_name in missing_write_deployments {
                match add_write_deployment(
                    deployment_to_contractid(&deployment_name),
                    default_env_load.clone(),
                )
                .await
                {
                    Ok(()) => info!("{deployment_name} deployment successfully launched!"),
                    Err(error) => error!("{deployment_name} failed to launch: {}", error),
                }
            }
        }
    });

    let controller_server = HttpServer::new(move || {
        let mut file = File::open("deployments/deployment.json").expect("Failed to open file");
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect("Failed to read file");
        //
        //// Parse the JSON data into a vector of serde_json::Value objects
        let default_env_load: Vec<Value> =
            serde_json::from_str(&contents).expect("Failed to parse JSON");

        App::new()
            .app_data(web::Data::new(AppState {
                db: db_clone.clone(),
                environment: default_env_load,
            }))
            .service(web::scope("/web3cache/controller").service(start_new_write_service))
    })
    .bind(("0.0.0.0", port))?
    .run();

    controller_server.await?;

    Ok(())
}

#[cfg(test)]
mod integration_tests;
