use dotenv::dotenv;
use futures::stream::TryStreamExt;
use log::info;
use mongodb::error::Error as MongoErr;
use mongodb::options::{FindOptions, InsertManyOptions, InsertOneOptions, UpdateOptions};
use mongodb::results::{DeleteResult, InsertManyResult, InsertOneResult, UpdateResult};
use mongodb::{bson::Document, options::FindOneOptions};
use mongodb::{options::ClientOptions, Client};
use mongodb::{Collection, Database};
use std::env;

pub async fn connect_to_mongodb(is_test_mongo: bool) -> mongodb::error::Result<Database> {
  dotenv().ok();
  // Parse your connection string into an options struct
  let mongo_uri = if is_test_mongo {
    env::var("MONGOURI_TEST").expect("$MONGOURI_TEST must be set")
  } else {
    env::var("MONGOURI").expect("$MONGOURI must be set")
  };
  //info!("{}", mongo_uri);

  let mut client_options = ClientOptions::parse(mongo_uri).await?;

  crate::custom_info!("Parsed mongo uri successfully.");
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

  crate::custom_info!("Connected to MongoDB successfully.");
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

pub async fn find_one(
  col: Collection<Document>,
  filter: Document,
  option: FindOneOptions,
) -> Result<Option<Document>, MongoErr> {
  col.find_one(filter, option).await
}

pub async fn delete_one(
  col: Collection<Document>,
  filter: Document,
) -> Result<DeleteResult, MongoErr> {
  col.delete_one(filter, None).await
}

pub async fn update_one(
  col: Collection<Document>,
  filter: Document,
  doc: Document,
  option: UpdateOptions,
) -> Result<UpdateResult, MongoErr> {
  col.update_one(filter, doc, option).await
}

pub async fn create_entry(
  col: Collection<Document>,
  doc: Document,
  option: InsertOneOptions,
) -> Result<InsertOneResult, MongoErr> {
  col.insert_one(doc, option).await
}

pub async fn insert_many(
  col: Collection<Document>,
  docs: &Vec<Document>,
  option: InsertManyOptions,
) -> Result<InsertManyResult, MongoErr> {
  col.clone().insert_many(docs, option).await
}
