use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use serde::{de::value, Deserialize, Serialize};
use serde_json::Result;
use chrono::prelude::*;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
struct MonitorData {
    temperature: i8,
    humidity:    u8,
    preassure:   u16,
}

#[post("/{monitor_id}")]
async fn index(path: web::Path<(String)>, req_body: String) -> impl Responder {
    println!("Timestamp: {}", Local::now());
    println!("{}", path);
    let data: MonitorData;
    println!("{}", req_body);
    match serde_json::from_str(&req_body) {
        Ok(val) => data = val,
        Err(_)  => return HttpResponse::BadRequest()
    }
    println!("{:?}", data);
    HttpResponse::Ok()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(index)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
