use dotenv::dotenv;
use futures::stream::TryStreamExt;
use log::info;
use mongodb::bson::Document;
use mongodb::error::Error as MongoErr;
use mongodb::options::FindOptions;
use mongodb::{options::ClientOptions, Client};
use mongodb::{Collection, Database};
use std::env;

pub async fn connect_to_mongodb_test() -> mongodb::error::Result<Database> {
    // Parse your connection string into an options struct
    dotenv().ok();
    env::remove_var("MONGOURI");
    connect_to_mongodb().await
}

pub async fn connect_to_mongodb() -> mongodb::error::Result<Database> {
    // Parse your connection string into an options struct
    let mongo_uri = env::var("MONGOURI_TEST")
        .or_else(|_| env::var("MONGOURI"))
        .expect("Either $MONGOURI_TEST or $MONGOURI must be set");

    let mut client_options = ClientOptions::parse(mongo_uri).await?;

    info!("Parsed mongo uri successfully.");
    // Manually set an option
    client_options.app_name = Some("web3cache".to_string());
    // Get a handle to the cluster
    let client = Client::with_options(client_options.clone())?;
    // Ping the server to see if you can connect to the cluster
    let db = client.database(
        client_options
            .default_database
            .unwrap_or_else(|| "readClient".to_string())
            .as_str(),
    );

    info!("Connected to MongoDB successfully.");
    Ok(db)
}

pub async fn find_all(
    col: Collection<Document>,
    filter: Document,
    option: FindOptions,
) -> Result<Vec<Document>, MongoErr> {
    let cursor = col.find(filter, option).await.unwrap();
    let results: Vec<Document> = cursor.try_collect().await?;
    Ok(results)
}

#[cfg(test)]
mod test {}
