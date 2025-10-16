use actix_service::ServiceFactory;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{get, web, App, Error, HttpResponse, HttpServer};
use dotenv::dotenv;
use log::info;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::io::Cursor;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::{fs, vec};

use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};

fn get_cached_files() -> HashSet<String> {
    let main_path = format!(
        "{}/static",
        std::env::current_dir()
            .unwrap()
            .into_os_string()
            .into_string()
            .unwrap()
    );
    let paths = fs::read_dir(main_path).unwrap();

    let mut hash_set: HashSet<String> = HashSet::new();
    for path in paths {
        let path_string_temp = path.unwrap().path().display().to_string();
        let path_string_vec: Vec<&str> = path_string_temp.split("static/").collect();
        let file_string = path_string_vec[1].to_string();
        if file_string.contains(".bin") {
            hash_set.insert(file_string);
        } else if file_string.contains(".temp") {
            if let Err(e) = fs::remove_file(path_string_temp) {
                println!("Writing error: {}", e);
            }
        }
    }

    hash_set
}

#[derive(Debug, Deserialize)]
pub struct QueryFile {
    file: String,
}

fn get_headers_content_type(file_path: &str) -> String {
    let mut content_type: String = tree_magic::from_filepath(Path::new(file_path));

    if content_type.eq("text/plain") {
        content_type = "application/json".to_string();
    }

    content_type
}

async fn request_and_cache_file(
    urls: Vec<String>,
    url_hash: &str, //urlHash
    store: &web::Data<AppState>,
) -> HttpResponse {
    let file_name = format!("{}.bin", base64::encode(url_hash));
    let path_file = format!("static/{}", &file_name);
    let file_exists;
    {
        let set_ptr: MutexGuard<HashSet<String>> = store.cacheset.lock().unwrap();
        file_exists = (*set_ptr).contains(&file_name);
    }

    if !file_exists {
        for url in &urls {
            let resp = reqwest::get(url).await;

            let body = resp.unwrap();
            if body.content_length()
                <= Some(
                    env::var("FILESIZE")
                        .unwrap_or("1073741824".to_owned())
                        .parse::<u64>()
                        .unwrap(),
                )
            {
                if body.status() != 200 {
                    //not enough, in case of folder in ipfs
                    continue;
                }

                let mut content = Cursor::new(body.bytes().await.unwrap());

                let file = File::create(&path_file).await;
                let x = fs::metadata(&path_file).unwrap().len();
                info!("{}", x);
                let res = if file.is_ok() {
                    let res = io::copy(&mut content, &mut file.unwrap()).await;
                    Ok(res)
                } else {
                    Err(file.unwrap_err())
                };

                if res.is_ok() {
                    let mut set_ptr: MutexGuard<HashSet<String>> = store.cacheset.lock().unwrap();
                    (*set_ptr).insert(file_name.clone());
                } else {
                    println!("Error storing file! {}", &file_name);
                }
                break;
            } else {
                return HttpResponse::NotFound().body("The file does not meet the requirements");
            }
        }
    }

    let opened_file = File::open(&path_file).await;
    if opened_file.is_ok() {
        let content_type = get_headers_content_type(path_file.as_str());

        let mut buffer: Vec<u8> = Vec::new();
        let mut file: File = opened_file.unwrap();

        file.read_to_end(&mut buffer).await.unwrap();
        return HttpResponse::Ok().content_type(content_type).body(buffer);
    }
    HttpResponse::NotFound().body("resourse not found")
}

#[get("/file")]
async fn file_cache_route(
    store: web::Data<AppState>,
    query: web::Query<QueryFile>,
) -> HttpResponse {
    let urls: Vec<String>;
    let file_name: &str;

    if !&query.file.contains("https://") {
        let array: Vec<&str> = query.file.split("ipfs://").collect();

        file_name = array[1];

        urls = vec![
            format!("https://ipfs.io/ipfs/{}", &file_name.to_string()),
            format!(
                "https://cloudflare-ipfs.com/ipfs/{}",
                &file_name.to_string()
            ),
        ];
    } else {
        file_name = &query.file;
        urls = vec![file_name.to_string()];
    }

    request_and_cache_file(urls, file_name, &store).await
}

#[get("/healthcheck")]
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().body("web3cache ipfs OK")
}

#[get("/{hash:.*}")]
async fn ipfs_route(params: web::Path<String>, store: web::Data<AppState>) -> HttpResponse {
    let urls = vec![
        format!("https://ipfs.io/ipfs/{}", &params.to_string()),
        format!("https://cloudflare-ipfs.com/ipfs/{}", &params.to_string()),
    ];

    request_and_cache_file(urls, &params, &store).await
}

struct AppState {
    cacheset: Mutex<HashSet<String>>,
}

pub fn get_app() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse,
        Error = Error,
        InitError = (),
    >,
> {
    let cacheset = web::Data::new(AppState {
        cacheset: Mutex::new(get_cached_files()),
    });

    App::new().app_data(cacheset).service(
        web::scope("/ipfs")
            .service(health_check)
            .service(file_cache_route)
            .service(ipfs_route),
    )
}

#[actix_web::main]
async fn main() {
    env_logger::init();
    dotenv().ok(); // run with: RUST_LOG=info cargo run

    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap();

    info!("listening on {}", port);

    let res = HttpServer::new(get_app)
        .workers(8)
        .bind(("0.0.0.0", port))
        .unwrap()
        .run()
        .await;
    if res.is_err() {
        info!("ERROR!!!");
    }
}

// https://ipfs.io/ipfs/QmUyARmq5RUJk5zt7KUeaMLYB8SQbKHp3Gdqy5WSxRtPNa/SeaofRoses.jpg
// https://ipfs.io/ipfs/QmQqzMTavQgT4f4T5v6PWBp7XNKtoPmC9jvn12WPT3gkSE

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use actix_web::test;

    #[actix_web::test]
    async fn test_ipfs_link() {
        let app = get_app();
        let service = test::init_service(app).await;

        let req = test::TestRequest::get()
        .uri("/file?file=ipfs://QmUM9kxKvozDKdVbh5Dpi4pT64saEdK7RPkSfWZ3ZGJx6B/Aniki_Common_1.png")
        .to_request();

        let resp = test::call_service(&service, req).await;

        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_fail_ipfs_link() {
        env::set_var("FILESIZE", "3221225472");
        let app = get_app();
        let service = test::init_service(app).await;

        let req = test::TestRequest::get().uri("/file").to_request();

        let resp = test::call_service(&service, req).await;

        assert!(resp.status().is_client_error());
    }
}
