use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use chrono::prelude::*;
use std::sync::Mutex;

struct AppData {
	db:      mongodb::Database,
	nodes:   Vec<String>
}

#[derive(Serialize, Deserialize, Debug)]
struct MonitorData {
    temperature: i8,  // Celcius
    humidity:    u8,  // % Relative Humidity
    pressure:    u16, // Millibars
    accoustics:  u16, // ADC Ouput Value
    hive_mass:   u16  // Grams
}

#[derive(Serialize, Deserialize, Debug)]
struct DbMonitorData {
	timestamp:   String, // UTC String
    temperature: i8,     // Celcius
    humidity:    u8,     // % Relative Humidity
    pressure:    u16,    // Millibars
    accoustics:  u16,    // ADC Ouput Value
    hive_mass:   u16     // Grams
}

//
#[post("/{monitor_id}")]
async fn index(path: web::Path<String>, req_body: String, shared: web::Data<Mutex<AppData>>) -> impl Responder {
	let locked = match shared.lock() {
		Ok(res) => res,
		Err(_) => return HttpResponse::Unauthorized()
	};

	match locked.nodes.contains(&path) {
		true => {},
		_    => return HttpResponse::NotFound()
	}

	let col: mongodb::Collection<DbMonitorData> = locked.db.collection(path.as_str());

	let data: MonitorData;
    match serde_json::from_str(&req_body) {
        Ok(val) => data = val,
        Err(_)  => return HttpResponse::BadRequest()
    }

    col.insert_one(DbMonitorData{
    	timestamp:   Utc::now().to_string(),
     	temperature: data.temperature,
      	humidity:    data.humidity,
       	pressure:    data.pressure,
        accoustics:  data.accoustics,
        hive_mass:   data.hive_mass
    }).await.expect("Could not insert into db!");

    HttpResponse::Ok()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let nodes_file = std::fs::read_to_string("nodes.txt").expect("Could not open nodes file!");
	let mut nodes: Vec<String> = Vec::new();

	for line in nodes_file.lines() {
		nodes.push(line.to_string());
	}

	let conn: mongodb::Client = mongodb::Client::with_uri_str(
		std::env::var("MONGODB_URI").expect(
			"Connection string environment variable does not exist!"
		)
	).await.expect("Could not exstablish connection!");

	let db =       conn.database("beehivesensors");
	let existing = db.list_collection_names().await.unwrap();

	for node in &nodes {
		match existing.contains(&node) {
			true => continue,
			_    => db.create_collection(node).await.unwrap(),
		}
	}

	let app_data = web::Data::new(Mutex::new(AppData{
		db:    db,
		nodes: nodes
	}));

	println!("*** Starting HTTP Server ***");

	HttpServer::new(move || {
        App::new()
        	.service(index)
            .app_data(app_data.clone())
    })
    .bind((std::env::var("HOST").expect(
			"HOST environment variable does not exist!"
		),
    	8080
    ))?
    .run()
    .await
}
