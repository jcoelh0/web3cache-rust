use bson::doc;
use futures::stream::TryStreamExt;
use log::info;
use mongodb::error::Error as MongoErr;
use mongodb::options::{
  FindOneAndUpdateOptions, FindOptions, IndexOptions, InsertManyOptions, UpdateOptions,
};
use mongodb::results::{DeleteResult, InsertManyResult, UpdateResult};
use mongodb::{bson::Document, options::FindOneOptions};
use mongodb::{options::ClientOptions, Client};
use mongodb::{Collection, Database, IndexModel};
use std::env;
extern crate dotenv;
use dotenv::dotenv;

pub async fn connect_to_mongodb_test() -> mongodb::error::Result<Database> {
  // Parse your connection string into an options struct
  connect_to_mongodb(true).await
}

pub async fn connect_to_mongodb(is_test_mongo: bool) -> mongodb::error::Result<Database> {
  dotenv().ok();
  // Parse your connection string into an options struct
  let mongo_uri = if is_test_mongo {
    env::var("MONGOURI_TEST").expect("$MONGOURI_TEST must be set")
  } else {
    env::var("MONGOURI").expect("$MONGOURI must be set")
  };
  //info!("{
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

pub async fn find_one_and_update(
  col: Collection<Document>,
  filter: Document,
  update: Document,
  option: Option<FindOneAndUpdateOptions>,
) -> Result<Option<Document>, MongoErr> {
  col.find_one_and_update(filter, update, option).await
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

pub async fn update_one(
  col: Collection<Document>,
  filter: Document,
  doc: Document,
  option: UpdateOptions,
) -> Result<UpdateResult, MongoErr> {
  col.update_one(filter, doc, option).await
}

pub async fn update_many(
  col: Collection<Document>,
  filter: Document,
  update: Document,
  option: UpdateOptions,
) -> Result<UpdateResult, MongoErr> {
  col.update_many(filter, update, option).await
}

pub async fn delete_many(
  col: Collection<Document>,
  filter: Document,
) -> Result<DeleteResult, MongoErr> {
  col.delete_many(filter, None).await
}

pub async fn insert_many(
  col: Collection<Document>,
  docs: &Vec<Document>,
  option: InsertManyOptions,
) -> Result<InsertManyResult, MongoErr> {
  col.clone().insert_many(docs, option).await
}

pub async fn setup_indexes(db: &Database) -> Result<(), MongoErr> {
  let transaction_col: Collection<Document> = db.collection("transactionblocks");

  //create indexes
  let mut index_model_options = IndexOptions::default();
  index_model_options.unique = Some(true);
  let mut index_model_keys = IndexModel::default();
  index_model_keys.keys = doc! { "subid": 1, "block_number": 1, "event_name": 1, };
  index_model_keys.options = Some(index_model_options.clone());
  transaction_col
    .clone()
    .create_index(index_model_keys, None)
    .await?;

  Ok(())
}
