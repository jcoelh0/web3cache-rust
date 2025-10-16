use mongodb::bson::{doc, Document};
use mongodb::error::Error as MongoErr;
use mongodb::options::{DeleteOptions, InsertManyOptions};
use mongodb::results::{DeleteResult, InsertManyResult};
use mongodb::Collection;
use web3cache::database::*;

#[test]
fn test_env_var() {
    dotenv::dotenv().ok();
    let uri = std::env::var("MONGOURI_TEST");

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
    let db = connect_to_mongodb_test().await?;
    let col = db.collection(&name);

    Ok(col)
}

#[tokio::test]
async fn test_conection_to_mongodb() {
    let db = connect_to_mongodb_test().await;
    assert!(db.is_ok());
}

#[tokio::test]
async fn test_find_all() {
    let col = init_db_tests("findAll_test".to_string()).await.unwrap();
    let mut vec_doc = Vec::new();
    let mut doc = doc! {"address":1000, "balance ":0.123232,"kodiPckBought":5};

    for a in 0..3 {
        doc.insert("address", format!("0x{}", 1000 + a));
        vec_doc.push(doc.clone());
    }
    let insert_many = insert_many(col.clone(), &vec_doc, Default::default()).await;
    assert!(insert_many.is_ok());
    let filter = doc! {"kodiPckBought":5};

    let find_all = find_all(col.clone(), filter, Default::default()).await;

    assert!(find_all.is_ok());
    let find_all = find_all.unwrap();

    assert_eq!(find_all.len(), 3);
    assert!(clear_entries(col, Default::default()).await.is_ok())
}
