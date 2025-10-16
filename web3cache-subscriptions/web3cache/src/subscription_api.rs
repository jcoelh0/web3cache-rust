use crate::custom_error;
use crate::{
  contract_registration_lib::{
    get_chain_address, get_chain_id, get_contract_abi_if_available,
    get_initial_block_number_by_contract_address,
  },
  database::{create_entry, delete_one, find_all, find_one, insert_many, update_one},
  helper_functions::*,
};
use actix_web::{
  get, post,
  web::{self, Data},
  HttpRequest, HttpResponse,
};

use chrono::*;

use log::{error, info};
use reqwest::{header::HeaderMap, Client, Response};
use std::env;
use validator::Validate;
extern crate dotenv;
use bson::{oid::ObjectId, Bson, Document};
use dotenv::dotenv;
use mongodb::{
  bson::doc,
  options::{FindOneOptions, FindOptions, InsertManyOptions, InsertOneOptions, UpdateOptions},
  Database,
};

use snailquote::unescape;

pub async fn webhook_health_check() -> HttpResponse {
  HttpResponse::Ok().body("web3cache subscriptions OK")
}

fn get_read_url() -> String {
  dotenv().ok();

  env::var("READURL").unwrap()
}
use serde_json::{json, Value};

#[get("/healthcheck")]
async fn healthcheck(_req: HttpRequest) -> HttpResponse {
  HttpResponse::Ok().body("ok")
}

#[post("/black-hole-endpoint")]
async fn black_hole_endpoint(req: HttpRequest, body: web::Json<Transaction>) -> HttpResponse {
  crate::custom_info!("webhook destroyed!");
  if let Some(api_key) = get_api_key(&req) {
    crate::custom_info!("headers : {}", api_key);
  }
  crate::custom_info!("body:{}", &body.subid);
  HttpResponse::Ok().body("ok")
}

pub async fn get_subscription_from_subid(
  req: HttpRequest,
  path: web::Path<String>,
  data: Data<AppState>,
) -> HttpResponse {
  if let Some(api_key) = get_api_key(&req) {
    let sub_id: String = path.into_inner();
    let object_id = match ObjectId::parse_str(sub_id) {
      Ok(object_id) => object_id,
      Err(_) => {
        return HttpResponse::BadRequest().json(json!({
          "message": "invalid sub_id"
        }))
      }
    };

    let filter = doc! { "_id": object_id, "apikey": api_key };
    let mut find_option = FindOneOptions::default();
    find_option.projection = Some(doc! { "apikey": 0, "secret": 0, "__v": 0  });

    let subscription = find_one(data.db.collection("subscriptions"), filter, find_option)
      .await
      .unwrap();
    if subscription.is_some() {
      //crate::custom_info!("Subscriptions: {:?}", subscription);
      let sub = subscription.unwrap();
      let mut response = sub.clone();
      response.insert("_id", sub.get_object_id("_id").unwrap().to_string());
      response.insert(
        "createdAt",
        sub
          .get_datetime("createdAt")
          .unwrap()
          .to_chrono()
          .to_rfc3339(),
      );
      response.insert(
        "updatedAt",
        sub
          .get_datetime("updatedAt")
          .unwrap()
          .to_chrono()
          .to_rfc3339(),
      );
      HttpResponse::Ok().json(response)
    } else {
      //crate::custom_info!("Subscriptions: {:?}", subscription);
      HttpResponse::NotFound().json(json!({
        "message": "Subscription not found"
      }))
    }
  } else {
    HttpResponse::BadRequest().json(json!({
      "message": "missing x-webhook-api-key"
    }))
  }
}

pub async fn delete_subscription_from_subid(
  req: HttpRequest,
  path: web::Path<String>,
  data: Data<AppState>,
) -> HttpResponse {
  crate::custom_info! {"DELETE"};
  if let Some(api_key) = get_api_key(&req) {
    let sub_id: String = path.into_inner();

    let object_id = match ObjectId::parse_str(sub_id) {
      Ok(object_id) => object_id,
      Err(_) => {
        return HttpResponse::BadRequest().json(json!({
          "message": "invalid sub_id"
        }))
      }
    };
    let filter = doc! { "_id": object_id, "apikey": api_key };

    let delete_result = delete_one(data.db.collection("subscriptions"), filter)
      .await
      .unwrap();

    //63726164aa67dd30f3c4c3bc
    if delete_result.deleted_count > 0 {
      HttpResponse::Ok().json(json!({
        "message": "Ok"
      }))
    } else {
      HttpResponse::NotFound().json(json!({
        "message": "Subscription not found"
      }))
    }
  } else {
    HttpResponse::BadRequest().json(json!({
      "message": "missing x-webhook-api-key"
    }))
  }
}

pub async fn replay_subscription(
  req: HttpRequest,
  path: web::Path<String>,
  body: web::Json<ReplaySubscription>,
  data: Data<AppState>,
) -> HttpResponse {
  if let Some(api_key) = get_api_key(&req) {
    let sub_id: String = path.into_inner();

    let object_id = match ObjectId::parse_str(sub_id) {
      Ok(object_id) => object_id,
      Err(_) => {
        return HttpResponse::BadRequest().json(json!({
          "message": "invalid sub_id"
        }))
      }
    };
    let filter = doc! { "_id": object_id, "apikey": api_key };

    let mut find_option = FindOneOptions::default();
    find_option.projection = Some(doc! { "apikey": 0, "secret": 0, "__v": 0  });

    let subscription = find_one(data.db.collection("subscriptions"), filter, find_option)
      .await
      .unwrap();

    if subscription.is_some() {
      let subscription = subscription.unwrap();
      //body.block_number
      let response = get_history_block_number(body.block_number, &subscription, &data.db).await;
      if response.is_err() {
        custom_error!("ERROR: {:?}", response.unwrap_err());
        HttpResponse::BadRequest()
          .json(json!({"message":"Internal error, we were not able to restart the blocknumber"}))
      } else {
        crate::custom_info!("RESPONSE:{:?}", response);

        let subscription = format_sub(subscription, object_id);

        //crate::custom_info!("{:?}", register_sub_result.inserted_id);
        //subscription.extend(doc! {"_id":});
        HttpResponse::Ok()
          .content_type("application/json")
          .body(serde_json::to_string(&subscription).unwrap())
      }
    } else {
      //crate::custom_info!("Subscriptions: {:?}", subscription);
      HttpResponse::NotFound().json(json!({
        "message": "Subscription not found"
      }))
    }
  } else {
    HttpResponse::BadRequest().json(json!({
      "message": "missing x-webhook-api-key"
    }))
  }
}

async fn get_history_block_number(
  block_number: i64,
  subscription: &Document,
  db: &Database,
) -> anyhow::Result<Value> {
  let mut headers = HeaderMap::new();
  crate::custom_info!("subscription: {:?}", subscription);
  headers.insert("x-read-api-key", read_api_key().parse()?);
  headers.insert("block_number", block_number.to_string().parse()?);
  headers.insert(
    "contract_id",
    unescape(subscription.get_str("contract_id")?)?.parse()?,
  );
  crate::custom_info!("headers: {:?}", headers);
  let client: Client = Client::new();
  let response_text = client
    .get(get_read_url() + "/transactions_history")
    .headers(headers)
    .send()
    .await
    .expect("failed to get response")
    .text()
    .await
    .expect("failed to get payload");

  let response_value: Value = response_text.parse()?;
  let transactions = response_value.as_array().unwrap();
  let subid = subscription.get_object_id("_id")?.to_string();
  crate::custom_info!("transactions len {}", transactions.len());

  let mut full_content: Vec<Document> = Vec::new();
  let mut send_transactions: Vec<Value> = Vec::new();

  let locked_until = bson::DateTime::now();
  for n in transactions {
    let transaction_block_number: i64 = n.get("block_number").unwrap().as_i64().unwrap();

    let transaction_event_name: String = n.get("event_name").unwrap().to_string();

    if !send_transactions.is_empty()
      && (transaction_block_number
        != send_transactions[send_transactions.len() - 1]
          .get("block_number")
          .unwrap()
          .as_i64()
          .unwrap()
        || !transaction_event_name.eq(
          &send_transactions[send_transactions.len() - 1]
            .get("event_name")
            .unwrap()
            .to_string(),
        ))
    {
      full_content.push(doc! {
          "subid": subid.clone(),
          "locked_until": locked_until,
          "block_number": send_transactions[0]["block_number"].as_i64(),
          "event_name": send_transactions[0]["event_name"].as_str(),
          "transactions": send_transactions
          .iter()
          .map(|d| {
            let value: serde_json::Map<String, Value> = d.as_object().unwrap().to_owned();
            Document::try_from(value).unwrap()
          })
          .collect::<Vec<Document>>()
      });
      send_transactions = Vec::new();
    }

    send_transactions.push(n.clone());
  }
  if !send_transactions.is_empty() {
    full_content.push(doc! {
        "subid": subid,
        "locked_until": locked_until,
        "block_number": send_transactions[0]["block_number"].as_i64(),
        "event_name": send_transactions[0]["event_name"].as_str(),
        "transactions": send_transactions
        .iter()
        .map(|d| {
          let value: serde_json::Map<String, Value> = d.as_object().unwrap().to_owned();
          Document::try_from(value).unwrap()
        })
        .collect::<Vec<Document>>()
    });
  };

  if !send_transactions.is_empty() {
    insert_many(
      db.collection("transactionblocks"),
      &full_content,
      InsertManyOptions::default(),
    )
    .await?;
  }

  /* if let Err(_err) = for_loop() {
    json!({ "error": _err.to_string() })
  } else {
    json!({})
  } */
  Ok(json!({ "message": "Ok"}))
}

pub async fn get_subscriptions(req: HttpRequest, data: Data<AppState>) -> HttpResponse {
  if let Some(api_key) = get_api_key(&req) {
    let filter = doc! { "apikey": api_key };

    let mut find_option = FindOptions::default();
    find_option.projection = Some(doc! { "apikey": 0, "secret": 0, "__v": 0  });

    let subscriptions = find_all(data.db.collection("subscriptions"), filter, find_option)
      .await
      .unwrap();
    let mut result: Vec<Document> = [].to_vec();
    for sub in subscriptions {
      result.push(doc! {"_id": sub.get_object_id("_id").unwrap().to_string() , "contract_id": sub.get("contract_id").unwrap() , "topics": sub.get_array("topics").unwrap().to_vec() , "isActive":sub.get_bool("isActive").unwrap() , "url": sub.get("url").unwrap(), "createdAt":sub.get_datetime("createdAt")
      .unwrap()
      .to_chrono()
      .to_rfc3339()
      .to_string(), "updatedAt":sub.get_datetime("updatedAt")
      .unwrap()
      .to_chrono()
      .to_rfc3339()
      .to_string(),})
    }
    if result.is_empty() {
      HttpResponse::Ok().json(json!({ "message": "No subscription found" }))
    } else {
      HttpResponse::Ok().json(json!({ "subscriptions": result }))
    }
  } else {
    HttpResponse::BadRequest().json(json!({
      "message": "missing x-webhook-api-key"
    }))
  }
}

pub async fn get_contract_from_id(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
  if let Some(_api_key) = get_api_key(&req) {
    let contract_id = path.into_inner();
    let client = Client::new();
    let res = client
      .get(get_read_url() + "/contract")
      .header("x-read-api-key", read_api_key())
      .header("contract_id", contract_id)
      .send()
      .await
      .expect("failed to get response")
      .text()
      .await
      .expect("failed to get payload");
    if res.len() <= 2 {
      HttpResponse::Ok().json(json!({
        "message":"Contract ID not found, please register your contract."
      }))
    } else {
      HttpResponse::Ok()
        .content_type("application/json")
        .body(res)
    }
  } else {
    HttpResponse::BadRequest().json(json!({
      "message": "missing x-webhook-api-key"
    }))
  }
}

pub async fn sui_contract_registration(
  contract_id: String,
  contract_address: String,
  events: String,
  modules: String,
  chain: String,
  data: Data<AppState>,
) -> anyhow::Result<String> {

  let chain_addresses_result = get_chain_address(data.db.clone(), -1, chain.clone()).await;

  if chain_addresses_result.is_err() {
    return Err(chain_addresses_result.unwrap_err());
  }

  let (chain_address_https, _) = chain_addresses_result.unwrap();

  let contract_to_add = doc! { "$set": {
      "contract_id": contract_id.clone(),
      "contract_address": contract_address.to_string(),
      "events": events,
      "modules": modules,
      "next_cursor" : "",
      "status_requirement": "online",
      "chain_address_https": chain_address_https
    }
  };

  let mut update_options = UpdateOptions::default();
  update_options.upsert = Some(true);

  let register_contract_result = update_one(
    data.db.collection("contracts"),
    doc! { "contract_id": contract_id },
    contract_to_add,
    update_options,
  )
  .await
  .unwrap();

  crate::custom_info!("{:?}", register_contract_result);
  let result = if register_contract_result.matched_count == 0 {
    "added".to_string()
  } else {
    "updated".to_string()
  };
  Ok(result)
}

pub async fn contract_invalidation(
  req: HttpRequest,
  data: Data<AppState>,
  path: web::Path<String>,
) -> HttpResponse {
  if let Some(api_key) = get_api_key(&req) {
    let filter = doc! { "apikey": api_key };
    let find_option = FindOneOptions::default();

    match find_one(data.db.collection("apikeys"), filter, find_option)
      .await
      .unwrap()
    {
      Some(_apikey) => {}
      None => {
        return HttpResponse::BadRequest().json(json!({
          "message": "invalid x-webhook-api-key"
        }));
      }
    };
  } else {
    return HttpResponse::BadRequest().json(json!({
      "message": "missing x-webhook-api-key"
    }));
  }

  let contract_id = path.into_inner();
  let contracts_col = data.db.collection("contracts");

  let update_result = match update_one(
    contracts_col,
    doc! { "contract_id": contract_id.clone()},
    doc! {
        "$set": {
            "status_requirement": "offline"
        }
    },
    UpdateOptions::default(),
  )
  .await
  {
    Ok(val) => val,
    Err(err) => {
      custom_error!("{:?}", err);
      return HttpResponse::InternalServerError()
        .content_type("application/json")
        .json(json!({
          "result": format!("Internal server custom_error!")
        }));
    }
  };

  if update_result.modified_count == 0 {
    return HttpResponse::BadRequest()
      .content_type("application/json")
      .json(json!({
        "message": format!("Contract {} did not exist or was not modified", contract_id)
      }));
  }

  HttpResponse::Ok()
    .content_type("application/json")
    .json(json!({
      "message": format!("Contract {} invalidated", contract_id)
    }))
}

fn is_valid_contract_abi(json_str: &str) -> bool {
  match serde_json::from_str::<Value>(json_str) {
    Ok(value) => match value {
      Value::Object(obj) => !obj.is_empty(),
      Value::Array(arr) => !arr.is_empty(),
      _ => true,
    },
    Err(_) => false,
  }
}

pub fn process_contract_registration_validation(
  contract_id: String,
  chain: String,
  contract_address: String,
  contract_abi: Option<String>,
  events: Option<String>,
  modules: Option<String>,
) -> Result<(), String> {
  // Check if contract_id starts with "sui_"
  if contract_id.starts_with("sui_") {
    // Create a SuiContractRegistration
    let registration = SuiContractRegistration {
      contract_id,
      contract_address,
      chain,
      events,
      modules,
    };

    //info!("Validate the SuiContractRegistration");
    if let Err(e) = registration.validate() {
      return Err(format!(
        "Validation failed for SuiContractRegistration: {}",
        e
      ));
    }
  } else {
    // Create a ContractRegistration
    let registration = ContractRegistration {
      contract_id,
      chain,
      contract_address,
      contract_abi,
      events,
      modules,
    };

    //info!("Validate the ContractRegistration");
    if let Err(e) = registration.validate() {
      return Err(format!("Validation failed for ContractRegistration: {}", e));
    }
  }

  Ok(())
}

pub async fn contract_registration(
  req: HttpRequest,
  body: web::Json<ContractRegistration>,
  data: Data<AppState>,
) -> HttpResponse {
  let contract_id = &body.contract_id;
  let chain = &body.chain;
  let contract_address = &body.contract_address;
  let contract_abi_result: &Option<String> = &body.contract_abi;
  let events_with_commas_result: &Option<String> = &body.events;
  let modules: &Option<String> = &body.modules;

  match process_contract_registration_validation(
    contract_id.clone(),
    chain.clone(),
    contract_address.clone(),
    contract_abi_result.clone(),
    events_with_commas_result.clone(),
    modules.clone(),
  ) {
    std::result::Result::Ok(_) => (),
    Err(err) => return HttpResponse::BadRequest().json(err),
  };

  if let Some(api_key) = get_api_key(&req) {
    let filter = doc! { "apikey": api_key };
    let find_option = FindOneOptions::default();

    match find_one(data.db.collection("apikeys"), filter, find_option)
      .await
      .unwrap()
    {
      Some(_apikey) => {}
      None => {
        return HttpResponse::Unauthorized().json(json!({
          "message": "invalid x-webhook-api-key"
        }));
      }
    };
  } else {
    return HttpResponse::Forbidden().json(json!({
      "message": "missing x-webhook-api-key"
    }));
  }

  if contract_id.starts_with("sui_") {
    let result = sui_contract_registration(
      contract_id.to_owned(),
      contract_address.to_owned(),
      events_with_commas_result.clone().unwrap_or("".to_string()),
      modules.clone().unwrap_or("".to_string()),
      chain.clone(),
      data.clone(),
    )
    .await;

    return if result.is_err() {
      HttpResponse::BadRequest().json(json!({
        "message": result.unwrap_err().to_string()
      }))
    } else {
      HttpResponse::Ok()
      .content_type("application/json")
      .json(json!({
        "result": format!("Sui contract {contract_id} {} successfully", result.unwrap())
      }))
    };
  }

  let mut _events_with_commas: String = "".to_string();

  let mut contract_abi: String = "".to_string();

  if contract_abi_result.is_some() {
    let contract_abi_string = contract_abi_result.as_ref().unwrap().to_string();

    if is_valid_contract_abi(&contract_abi_string) {
      contract_abi = contract_abi_string;
    } else {
      contract_abi = "".to_string();
    }
  }
  
  let chain_id = get_chain_id(chain.to_string());

  if chain_id.is_err() {
    return HttpResponse::BadRequest().body(chain_id.unwrap_err().to_string());
  }
  let chain_id = chain_id.unwrap();

  if contract_abi.is_empty() {
    let contract_abi_result =
      get_contract_abi_if_available(data.db.clone(), contract_address.to_string(), chain_id).await;

    contract_abi = contract_abi_result.unwrap_or("".to_string());

    if contract_abi.is_empty() {
      return HttpResponse::NotFound().body("Contract abi not provided and not found through API");
    }
  }

  let initial_block_number = get_initial_block_number_by_contract_address(
    data.db.clone(),
    contract_address.to_string(),
    chain_id,
  )
  .await;

  if initial_block_number.is_err() {
    return HttpResponse::BadRequest().body(initial_block_number.unwrap_err().to_string());
  }
  let initial_block_number = initial_block_number.unwrap();

  let chain_addresses_result = get_chain_address(data.db.clone(), chain_id, chain.clone()).await;

  if chain_addresses_result.is_err() {
    return HttpResponse::BadRequest().body(chain_addresses_result.unwrap_err().to_string());
  }

  let (chain_address_https, chain_address_wss) = chain_addresses_result.unwrap();

  if events_with_commas_result.is_none() {
    let contract_abi_json: Vec<serde_json::Value> =
      serde_json::from_str(&unescape(&contract_abi).unwrap()).expect("JSON was not well-formatted");

    let events: Vec<String> = contract_abi_json
      .iter()
      .filter(|x| {
        unescape(&x.get("type").unwrap().to_string())
          .unwrap()
          .eq("event")
      })
      .map(|y| unescape(&y.get("name").unwrap().to_string()).unwrap())
      .collect();
    let events_with_commas_string: String = events
      .iter()
      .map(|x| x.to_string() + ",")
      .collect::<String>();

    _events_with_commas = events_with_commas_string.trim_end_matches(',').to_string();
  } else {
    _events_with_commas = events_with_commas_result.clone().unwrap().replace(" ", "");
  }

  let contract_to_add = doc! { "$set": {
      "contract_id": contract_id,
      "contract_address": contract_address.to_string(),
      "contract_abi": contract_abi.to_string(),
      "contract_block_number": initial_block_number,
      "owner_block_number": initial_block_number,
      "transfer_block_number": initial_block_number,
      "chain_address_wss": chain_address_wss,
      "chain_address_https": chain_address_https,
      "events": _events_with_commas,
      "status_requirement": "online"
    }
  };

  let mut update_options = UpdateOptions::default();
  update_options.upsert = Some(true);

  let register_contract_result = update_one(
    data.db.collection("contracts"),
    doc! { "contract_id": contract_id },
    contract_to_add,
    update_options,
  )
  .await
  .unwrap();

  crate::custom_info!("{:?}", register_contract_result);

  let result = if register_contract_result.matched_count == 0 {
    "added"
  } else {
    "updated"
  };

  let response = controller_start_write_service(contract_id.to_string()).await;

  if !response.status().is_success() {
    crate::custom_info!("{:?}", response);
    /* return HttpResponse::InternalServerError()
    .content_type("application/json")
    .json(json!({
      "result": response.text().await.unwrap_or("no body".to_string())
    })); */
  }

  HttpResponse::Ok()
    .content_type("application/json")
    .json(json!({
      "result": format!("Contract {contract_id} {result} successfully")
    }))
}

async fn controller_start_write_service(contract_id: String) -> Response {
  let controller_url = env::var("CONTROLLERURL").unwrap();

  reqwest::get(format!(
    "{controller_url}/start-write-service/{contract_id}"
  ))
  .await
  .unwrap()
}

pub async fn get_contract_metadata(path: web::Path<String>, data: Data<AppState>) -> HttpResponse {
  let contract_id = path.into_inner();

  let contract = find_one(
    data.db.collection("contracts"),
    doc! { "contract_id": &contract_id },
    FindOneOptions::default(),
  )
  .await
  .unwrap();

  if contract.is_some() {
    let contract = contract.unwrap();
    if !contract.is_empty() {
      return HttpResponse::Ok()
        .content_type("application/json")
        .json(contract);
    }
  }
  HttpResponse::BadRequest()
    .json(json!({"Error": "Could not find contract with this contract id."}))
}

pub async fn get_contracts(req: HttpRequest) -> HttpResponse {
  let client = Client::default();
  if let Some(_api_key) = get_api_key(&req) {
    let res = client
      .get(get_read_url() + "/contracts")
      .header("x-read-api-key", read_api_key())
      .send()
      .await
      .expect("failed to get response")
      .text()
      .await
      .expect("failed to get payload");

    //crate::custom_info!("Response: {:?}", res);
    HttpResponse::Ok()
      .content_type("application/json")
      .body(res)
  } else {
    HttpResponse::BadRequest().json(json!({
      "message": "missing x-webhook-api-key"
    }))
  }
}

pub async fn subscription_registration(
  req: HttpRequest,
  body: web::Json<Subscription>,
  data: Data<AppState>,
) -> HttpResponse {
  if let Some(api_key) = get_api_key(&req) {
    match body.validate() {
      Ok(_) => (),
      Err(err) => return HttpResponse::BadRequest().json(err),
    };
    let contract_id = &body.contract_id;
    let topics: Vec<String> = if body.topics.is_some() {
      body.topics.as_ref().unwrap().to_vec()
    } else {
      vec![]
    };
    let url = &body.url;
    let block_number = body.block_number;

    let client = Client::default();
    let res = client
      .get(get_read_url() + "/contract")
      .header("x-read-api-key", read_api_key())
      .header("contract_id", contract_id)
      .send()
      .await
      .expect("failed to get response")
      .text()
      .await
      .expect("failed to get payload");

    println!("Response: {:?}", res);
    if res.len() <= 2 {
      HttpResponse::BadRequest()
        .json(json!({"message":"Contract ID not found, please register your contract."}))
    } else {
      let dup = find_all(
        data.db.collection("subscriptions"),
        doc! {"contract_id":contract_id , "apikey": api_key ,"url": url},
        FindOptions::default(),
      )
      .await
      .unwrap();
      if !dup.is_empty() {
        for dup in dup {
          let mut a: Vec<String> = dup
            .get_array("topics")
            .unwrap()
            .iter()
            .map(|a| unescape(&serde_json::to_string(a).unwrap()).unwrap())
            .collect();
          a.sort();
          let b = topics.clone();
          let matching = a.iter().zip(&b).filter(|&(a, b)| a == b).count();
          if matching == b.len() {
            return HttpResponse::BadRequest()
              .json(json!({"message":"Subscription already exists" , "_id": dup.get_object_id("_id").unwrap().to_string() }));
          }
        }
      }

      let utc = Utc::now();
      let bson_date = Bson::from(utc);
      let mut subscription = doc! {"contract_id" : contract_id , "topics": topics , "apikey":api_key , "isActive":true , "url":url , "createdAt":bson_date.clone() , "updatedAt":bson_date};
      let register_sub_result = create_entry(
        data.db.collection("subscriptions"),
        subscription.clone(),
        InsertOneOptions::default(),
      )
      .await
      .unwrap();

      //crate::custom_info!("{:?}", register_sub_result);
      if block_number.is_some() {
        let id = register_sub_result.inserted_id.clone();
        crate::custom_info!("ID:{:?}", id);
        subscription.insert("_id", id);
        let response =
          get_history_block_number(block_number.unwrap(), &subscription.clone(), &data.db).await;
        if response.is_err() {
          HttpResponse::BadRequest()
            .json(json!({"message":"Internal error, we were not able to restart the block number"}))
        } else {
          let id: bson::oid::ObjectId = register_sub_result.inserted_id.as_object_id().unwrap();
          let subscription = format_sub(subscription, id);

          HttpResponse::Ok()
            .content_type("application/json")
            .body(serde_json::to_string(&subscription).unwrap())
        }
      } else {
        let id: bson::oid::ObjectId = register_sub_result.inserted_id.as_object_id().unwrap();
        let subscription = format_sub(subscription, id);

        //crate::custom_info!("{:?}", register_sub_result.inserted_id);
        //subscription.extend(doc! {"_id":});
        HttpResponse::Ok()
          .content_type("application/json")
          .body(serde_json::to_string(&subscription).unwrap())
      }
    }
  } else {
    HttpResponse::BadRequest().json(json!({
      "message": "missing x-webhook-api-key"
    }))
  }
}

pub async fn update_subscription(
  req: HttpRequest,
  path: web::Path<String>,
  body: web::Json<UpdateSub>,
  data: Data<AppState>,
) -> HttpResponse {
  if let Some(api_key) = get_api_key(&req) {
    match body.validate() {
      Ok(_) => (),
      Err(err) => return HttpResponse::BadRequest().json(err),
    };
    let sub_id = path.into_inner();
    let object_id = match ObjectId::parse_str(sub_id) {
      Ok(object_id) => object_id,
      Err(_) => {
        return HttpResponse::BadRequest().json(json!({
          "message": "invalid sub_id"
        }))
      }
    };

    let subscription = find_one(
      data.db.collection("subscriptions"),
      doc! {"_id":object_id , "apikey": api_key},
      FindOneOptions::default(),
    )
    .await
    .unwrap();
    if subscription.is_some() && !subscription.unwrap().is_empty() {
      let utc = Utc::now();
      let bson_date = Bson::from(utc);
      let mut set_object = doc! {"updatedAt":bson_date};
      let activate: bool = if body.activate.is_some() {
        body.activate.unwrap()
      } else {
        true
      };

      if body.url.is_some() {
        set_object.extend(doc! {"url":body.url.as_ref().unwrap() , "isActive":activate})
      } else {
        set_object.extend(doc! {"isActive":activate})
      }
      if body.set_topics.is_some() && !body.set_topics.as_ref().unwrap().is_empty() {
        let set_topics = body.set_topics.as_ref().unwrap();
        set_object.extend(doc! {"topics":set_topics});
        let _result = update_one(
          data.db.collection("subscriptions"),
          doc! {"_id":object_id , "apikey": api_key},
          doc! {"$set":set_object.clone()},
          UpdateOptions::default(),
        )
        .await
        .unwrap();
      } else if (body.add_topics.is_some() && !body.add_topics.as_ref().unwrap().is_empty())
        || (body.remove_topics.is_some() && !body.remove_topics.as_ref().unwrap().is_empty())
      {
        if body.add_topics.is_some() && !body.add_topics.as_ref().unwrap().is_empty() {
          let add_topics = body.add_topics.as_ref().unwrap();
          let _result = update_one(
            data.db.collection("subscriptions"),
            doc! {"_id":object_id , "apikey": api_key},
            doc! {"$addToSet":{"topics":{"$each":add_topics}} , "$set":set_object.clone()},
            UpdateOptions::default(),
          )
          .await
          .unwrap();
        }
        if body.remove_topics.is_some() && !body.remove_topics.as_ref().unwrap().is_empty() {
          let remove_topics = body.remove_topics.as_ref().unwrap();
          let _result = update_one(
            data.db.collection("subscriptions"),
            doc! {"_id":object_id , "apikey": api_key},
            doc! {"$pull":{"topics":{"$in":remove_topics}} , "$set":set_object.clone()},
            UpdateOptions::default(),
          )
          .await
          .unwrap();
        }
      } else {
        crate::custom_info!("update url and activate only");
        let _result = update_one(
          data.db.collection("subscriptions"),
          doc! {"_id":object_id , "apikey": api_key},
          doc! {"$set":set_object},
          UpdateOptions::default(),
        )
        .await
        .unwrap();
      }
      let mut subscription = find_one(
        data.db.collection("subscriptions"),
        doc! {"_id":object_id , "apikey": api_key},
        FindOneOptions::default(),
      )
      .await
      .unwrap()
      .unwrap();
      subscription = format_sub(subscription, object_id);
      HttpResponse::Ok()
        .content_type("application/json")
        .json(subscription)
    } else {
      HttpResponse::BadRequest().json(json!({"message":"SubscriptionID apikey pair not found "}))
    }
  } else {
    HttpResponse::BadRequest().json(json!({"message": "missing x-webhook-api-key"}))
  }
}

pub async fn subscription_state(
  req: HttpRequest,
  body: web::Json<SubState>,
  data: Data<AppState>,
  path: web::Path<String>,
) -> HttpResponse {
  if let Some(api_key) = get_api_key(&req) {
    let sub_id = path.into_inner();
    let object_id = match ObjectId::parse_str(sub_id) {
      Ok(object_id) => object_id,
      Err(_) => {
        return HttpResponse::BadRequest().json(json!({
          "message": "invalid sub_id"
        }))
      }
    };
    let state: bool = if body.activate.is_some() {
      body.activate.unwrap()
    } else {
      true
    };
    let utc = Utc::now();
    let bson_date = Bson::from(utc);
    let set_object = doc! {"updatedAt":bson_date, "isActive":state};
    let subscription = find_one(
      data.db.collection("subscriptions"),
      doc! {"_id":object_id , "apikey": api_key},
      FindOneOptions::default(),
    )
    .await
    .unwrap();
    if subscription.is_some() && !subscription.clone().unwrap().is_empty() {
      let _result = update_one(
        data.db.collection("subscriptions"),
        doc! {"_id":object_id , "apikey": api_key},
        doc! {"$set":set_object},
        UpdateOptions::default(),
      )
      .await
      .unwrap();

      let mut sub_result = format_sub(subscription.unwrap(), object_id);
      sub_result.insert("isActive", state);
      HttpResponse::Ok()
        .content_type("application/json")
        .json(sub_result)
    } else {
      HttpResponse::BadRequest().json(json!({"message":"SubscriptionID apikey pair not found "}))
    }
  } else {
    HttpResponse::BadRequest().json(json!({"message":"Invalid api key"}))
  }
}
