use crate::helper_functions::*;
use actix_web::{
    get,
    web::{self, Data},
    HttpRequest, HttpResponse,
};
use log::info;
use mongodb::bson::doc;
use serde_json::json;

extern crate dotenv;

#[get("/start-write-service/{contract_id}")]
async fn start_new_write_service(
    _req: HttpRequest,
    data: Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    //if !check_api_key(&req, data.db.clone()).await.unwrap() {
    //    return HttpResponse::BadRequest().body("Invalid header x-read-api-key or not provided");
    //}
    let contract_id = path.into_inner();
    info!("Starting write service: {}!", contract_id);

    let result = add_write_deployment(contract_id.clone(), data.environment.clone()).await;

    if result.is_ok() {
        HttpResponse::Ok().json(serde_json::json!(doc! {"contract_id": contract_id}))
    } else {
        HttpResponse::InternalServerError()
            .json(json!({ "Error": format!("{:?}", result.unwrap_err()) }))
    }
}
