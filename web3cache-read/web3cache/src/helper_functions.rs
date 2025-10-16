use actix_web::HttpRequest;
use mongodb::{
    bson::{doc, Document},
    options::FindOneOptions,
    Database,
};

use serde::{Deserialize, Serialize};

pub struct AppState {
    pub db: Database,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NftResp {
    pub data: Vec<Document>,
    pub offset: u64,
}

#[derive(Deserialize)]
pub struct ContractNftInfo {
    pub contract_address: String,
    pub owner: Option<bool>,
    pub limit: u64,
    pub offset: u64,
    pub metadata: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct OnwerNftInfo {
    pub address: String,
    pub contract_address: String,
    pub limit: u64,
    pub offset: u64,
    pub metadata: Option<bool>,
}

#[derive(Deserialize)]
pub struct Info {
    pub contract_id: Option<String>,
    pub address: String,
}

pub fn get_address(req: &HttpRequest) -> Option<&str> {
    req.headers().get("address")?.to_str().ok()
}
pub fn get_contract_id(req: &HttpRequest) -> Option<&str> {
    req.headers().get("contract_id")?.to_str().ok()
}

pub async fn check_api_key(req: &HttpRequest, db: Database) -> anyhow::Result<bool> {
    let api_key = req.headers().get("x-read-api-key");

    if api_key.is_none() {
        return Ok(false);
    }
    let api_key = api_key.unwrap().to_str().unwrap().to_string();

    let find_option = FindOneOptions::default();
    //find_option.projection = Some(doc! { "apikey": 0, "secret": 0, "__v": 0  });

    let api_key_document: Option<Document> = db
        .collection("apikeys")
        .find_one(doc! { "apikey" : api_key }, find_option)
        .await
        .unwrap();

    if api_key_document.is_some() {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn validate_address(s: &str) -> bool {
    s.len() == 42 && s.get(0..2) == Some("0x") && s[2..].chars().all(|c| c.is_ascii_hexdigit())
}

pub fn get_block_number(req: &HttpRequest) -> Option<i64> {
    req.headers()
        .get("block_number")?
        .to_str()
        .ok()
        .unwrap()
        .to_string()
        .parse::<i64>()
        .ok()
}
