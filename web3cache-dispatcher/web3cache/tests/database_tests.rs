use bson::Document;
use mongodb::bson::doc;
use mongodb::error::Error as MongoErr;
use mongodb::options::DeleteOptions;
use mongodb::results::DeleteResult;
use mongodb::Collection;
use web3cache::database::*;

#[test]
fn test_env_var() {
  dotenv::dotenv().ok();
  let uri = std::env::var("MONGOURI_TEST");

  assert!(uri.is_ok());
}

async fn clear_entries(
  col: Collection<Document>,
  option: DeleteOptions,
) -> Result<DeleteResult, MongoErr> {
  col.delete_many(doc! {}, option).await
}

async fn init_db_tests(name: String) -> Result<Collection<Document>, MongoErr> {
  let db = connect_to_mongodb_test().await?;
  let col = db.collection(&name);

  Ok(col)
}

#[tokio::test]
async fn test_conection_to_mongodb() {
  let db = connect_to_mongodb_test().await;
  assert!(db.is_ok());
}
