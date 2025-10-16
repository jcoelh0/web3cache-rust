use futures::stream::TryStreamExt;
use log::info;
use mongodb::bson::Document;
use mongodb::error::Error as MongoErr;
use mongodb::options::FindOptions;

use mongodb::{options::ClientOptions, Client};
use mongodb::{Collection, Database};
use std::env;

pub async fn connect_to_mongodb() -> mongodb::error::Result<Database> {
    // Parse your connection string into an options struct
    let uri = std::env::var("MONGOURI");
    assert!(uri.is_ok());
    env::set_var("MONGOURI", uri.unwrap());
    let mongo_uri = env::var("MONGOURI").expect("$MONGOURI is not set");

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
mod test {
    use mongodb::bson::doc;
    use mongodb::options::{DeleteOptions, InsertManyOptions};
    use mongodb::results::{DeleteResult, InsertManyResult};

    use super::*;

    #[test]
    fn test_env_var() {
        dotenv::dotenv().ok();
        let uri = std::env::var("MONGOURI");

        assert!(uri.is_ok());
    }

    async fn insert_many(
        col: Collection<Document>,
        docs: &Vec<Document>,
        option: InsertManyOptions,
    ) -> Result<InsertManyResult, MongoErr> {
        col.clone().insert_many(docs, option).await
    }

    async fn clear_entries(
        col: Collection<Document>,
        option: DeleteOptions,
    ) -> Result<DeleteResult, MongoErr> {
        col.delete_many(doc! {}, option).await
    }

    async fn init_db_tests(name: String) -> Result<Collection<Document>, MongoErr> {
        let db = connect_to_mongodb().await?;
        let col = db.collection(&name);

        Ok(col)
    }

    #[tokio::test]
    async fn test_conection_to_mongodb() {
        let db = connect_to_mongodb().await;
        assert!(db.is_ok());
    }
}
