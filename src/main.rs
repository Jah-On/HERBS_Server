use actix_web::{web, App, HttpServer};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::sync::Mutex;

mod database;
mod device;
mod firmware;
mod shared;
use shared::AppData;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let hostname = std::env::var("HOST").expect("${HOST} not found!");
    let mongo_uri = std::env::var("MONGODB_URI").expect("${MONGODB_URI} not found!");

    let conn: mongodb::Client = mongodb::Client::with_uri_str(mongo_uri)
        .await
        .expect("Could not establish connection!");

    let db = conn.database("beehivesensors");
    let existing = db.list_collection_names().await.unwrap();

    database::check_or_make_all(&db, &existing).await;

    let app_data = web::Data::new(Mutex::new(AppData::new(db).await));

    println!("*** Starting HTTP Server ***");

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("private.pem", SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file("public.pem").unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .configure(firmware::resources)
            .configure(device::resources)
    })
    .workers(128)
    .bind_openssl((hostname, 8080), builder)?
    .run()
    .await
}
