use super::*;
use actix_web::http;
use actix_web::test::{call_service, init_service, TestRequest};
use mongodb::bson::{doc, Document};
use mongodb::error::Error as MongoErr;
use mongodb::options::{DeleteOptions, InsertOneOptions};
use mongodb::results::{DeleteResult, InsertOneResult};
use mongodb::Collection;
use serde_json::json;

async fn insert_test(
    col: Collection<Document>,
    doc: Document,
    option: InsertOneOptions,
) -> Result<InsertOneResult, MongoErr> {
    col.insert_one(doc, option).await
}

async fn delete_one(
    col: Collection<Document>,
    query: Document,
    option: DeleteOptions,
) -> Result<DeleteResult, MongoErr> {
    col.delete_one(query, option).await
}

async fn create_test_api_key(db: mongodb::Database) {
    let _ = insert_test(
        db.collection("apikeys"),
        doc! {"apikey":"testing_api_key"},
        InsertOneOptions::default(),
    )
    .await;
}

async fn delete_test_api_key(db: mongodb::Database) {
    let _ = delete_one(
        db.collection("apikeys"),
        doc! {"apikey":"testing_api_key"},
        DeleteOptions::default(),
    )
    .await;
}
