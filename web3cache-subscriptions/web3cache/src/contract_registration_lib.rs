use std::{fs, env};

use anyhow::{ensure, Error, Ok};
use bson::doc;
use mongodb::{options::FindOneOptions, Database};
use snailquote::unescape;

use crate::database::find_one;

pub async fn get_chain_address(db: Database, chain_id: i64, chain: String) -> anyhow::Result<(String, String)> {

  let filter = if chain_id < 0 {
    doc! { "chain": chain }
  } else {
    doc! { "chain_id": &chain_id.clone() }
  };
  
  let find_option = FindOneOptions::default();

  let result_chains = find_one(db.collection("metadatachains"), filter, find_option)
    .await
    .unwrap();

  ensure!(result_chains.is_some(), "No chain found in the database");

  let result_address = result_chains.unwrap();

  Ok((
    result_address
      .get_str("https")
      .unwrap_or("None")
      .to_string(),
    result_address.get_str("wss").unwrap_or("None").to_string(),
  ))
}

pub fn get_chain_id(chain: String) -> anyhow::Result<i64> {
  let dir_name = env::var("CARGO_MANIFEST_DIR").unwrap();
  let data_result = fs::read_to_string(dir_name + "/files/chains.json");

  if let Err(error) = data_result {
    return Err(Error::new(error).context("Could not read chains.json"));
  }
  let data = data_result.unwrap();

  let json: Vec<serde_json::Value> =
    serde_json::from_str(&data).expect("JSON was not well-formatted");

  let chain_object = json.iter().find(|x| {
    x.get("name")
      .unwrap()
      .to_string()
      .to_lowercase()
      .contains(&chain.to_lowercase())
  });

  let chain_object = chain_object.unwrap();

  Ok(chain_object.get("chainId").unwrap().as_i64().unwrap())
}

pub fn get_chain_api_url(chain_id: i64) -> String {
  let mut _chain_api_url = "";
  match chain_id {
    1 => _chain_api_url = "https://api.etherscan.io/",
    5 => _chain_api_url = "https://api-goerli.etherscan.io/",
    42 => _chain_api_url = "https://api-kovan.etherscan.io/",
    4 => _chain_api_url = "https://api-rinkeby.etherscan.io/",
    3 => _chain_api_url = "https://api-ropsten.etherscan.io/",
    11155111 => _chain_api_url = "https://api-sepolia.etherscan.io/",
    137 => _chain_api_url = "https://api.polygonscan.com/",
    80001 => _chain_api_url = "https://api-testnet.polygonscan.com/",
    _default => _chain_api_url = "",
  }
  _chain_api_url.to_string()
}

pub async fn get_contract_abi_if_available(
  db: Database,
  contract_address: String,
  chain_id: i64,
) -> anyhow::Result<String> {
  let filter = doc! { "chain_id": &chain_id };
  let find_option = FindOneOptions::default();

  let result = find_one(db.collection("metadatachains"), filter, find_option)
    .await
    .unwrap();

  let get_abi_url = get_chain_api_url(chain_id)
    + "/api?module=contract&action=getabi&address="
    + &contract_address
    + "&apikey="
    + result.unwrap().get_str("api_key").unwrap();

  let response = reqwest::get(get_abi_url)
    .await
    .unwrap()
    .text()
    .await
    .unwrap();

  let json: serde_json::Value =
    serde_json::from_str(&response).expect("JSON was not well-formatted");

  ensure!(
    json.get("status").unwrap() != "0",
    "Failed to get contract ABI"
  );

  Ok(json.get("result").unwrap().to_string())
}

pub async fn get_initial_block_number_by_contract_address(
  db: Database,
  contract_address: String,
  chain_id: i64,
) -> anyhow::Result<i64> {
  let filter = doc! { "chain_id": &chain_id };
  let find_option = FindOneOptions::default();
  crate::custom_info!("chainid: {}", chain_id);
  let result = find_one(db.collection("metadatachains"), filter, find_option)
    .await
    .unwrap();

  let result = result.unwrap();

  let api_key = result.get_str("api_key");

  let api_key = api_key.unwrap_or_default();
  let chain_api_url = get_chain_api_url(chain_id);

  let get_contract_creation_url = chain_api_url.clone()
    + "/api?module=contract&action=getcontractcreation&contractaddresses="
    + &contract_address
    + "&apikey="
    + api_key;

  let response = reqwest::get(get_contract_creation_url)
    .await
    .unwrap()
    .text()
    .await
    .unwrap();

  let response_json: serde_json::Value =
    serde_json::from_str(&response).expect("JSON was not well-formatted");

  ensure!(
    response_json.get("status").unwrap() != "0",
    "Failed to get contract creation API"
  );

  let get_transaction_by_hash_url = chain_api_url
    + "/api?module=proxy&action=eth_getTransactionByHash&txhash="
    + unescape(
      &response_json.get("result").unwrap()[0]
        .get("txHash")
        .unwrap()
        .to_string(),
    )
    .unwrap()
    .as_str()
    + "&apikey="
    + api_key;

  let response = reqwest::get(&get_transaction_by_hash_url)
    .await
    .unwrap()
    .text()
    .await
    .unwrap();

  let response_json: serde_json::Value =
    serde_json::from_str(&response).expect("JSON was not well-formatted");

  ensure!(
    response_json.get("result").unwrap().is_object(),
    "Failed to get contract creation API"
  );

  let mut block_number_hex = response_json
    .get("result")
    .unwrap()
    .get("blockNumber")
    .unwrap()
    .to_string();

  //remove first character
  block_number_hex = unescape(&block_number_hex)
    .unwrap()
    .trim_start_matches("0x")
    .to_string();

  let block_number = i64::from_str_radix(block_number_hex.as_str(), 16).unwrap();

  Ok(block_number)
}
