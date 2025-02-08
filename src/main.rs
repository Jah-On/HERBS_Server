use actix_web::{get, post, put, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use chrono::prelude::*;
use std::time::Duration;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Debug)]
struct Keys {
	access_key: String,
	nodes:      Vec<String>
}

struct AppData {
	keys:      Keys,
	db:        mongodb::Database,
	last_ping: chrono::DateTime<Utc>
}

#[derive(Serialize, Deserialize, Debug)]
struct MonitorData {
	battery:     u8,  // Battery %
	hive_temp:   i8,  // Celcius
	extern_temp: i8,  // Celcius
	humidity:    u8,  // % Relative Humidity
	pressure:    u16, // Millibars
	acoustics:   u16  // Loudness Value
}

#[derive(Serialize, Deserialize, Debug)]
struct DbMonitorData {
	timestamp:   String, // UTC String
	battery:     u8,     // Battery %
	hive_temp:   i8,     // Celcius
	extern_temp: i8,     // Celcius
	humidity:    u8,     // % Relative Humidity
	pressure:    u16,    // Millibars
	acoustics:   u16     // Loudness Value
}

#[put("/{ping_post_access_key}/ping")]
async fn ping_post(path: web::Path<String>, shared: web::Data<Mutex<AppData>>) -> impl Responder {
	let mut locked = match shared.lock() {
		Ok(res) => res,
		Err(_) => {
			println!("Poisoned mutex in ping put?");
			return HttpResponse::InternalServerError()
		}
	};

	if locked.keys.access_key != *path {
		std::mem::drop(locked);
		return HttpResponse::Unauthorized();
	}

	locked.last_ping = Utc::now();
	std::mem::drop(locked);

	HttpResponse::Ok()
}

#[get("/{access_key}/ping")]
async fn ping_get(path: web::Path<String>, shared: web::Data<Mutex<AppData>>) -> HttpResponse {
	let locked = match shared.lock() {
		Ok(res) => res,
		Err(_) => {
			println!("Poisoned mutex in ping get?");
			return HttpResponse::InternalServerError().finish()
		}
	};

	if locked.keys.access_key != *path {
		std::mem::drop(locked);
		return HttpResponse::Unauthorized().finish();
	}

	let time_as_string = locked.last_ping.to_string();
	std::mem::drop(locked);

	HttpResponse::Ok().body(time_as_string)
}

#[post("/{access_key}/{monitor_id}")]
async fn monitor(path: web::Path<(String, String)>, req_body: String, shared: web::Data<Mutex<AppData>>) -> impl Responder {
	let (access_key, monitor_id) = path.into_inner();

	let locked = match shared.lock() {
		Ok(res) => res,
		Err(_) => {
			println!("Poisoned mutex in data post?");
			return HttpResponse::InternalServerError()
		}
	};

	if locked.keys.access_key != access_key {
		std::mem::drop(locked);
		return HttpResponse::Unauthorized();
	}

	match locked.keys.nodes.contains(&monitor_id) {
		true => {},
		_    => {
			std::mem::drop(locked);
			return HttpResponse::NotFound()
		}
	}

	let col: mongodb::Collection<DbMonitorData> = locked.db.collection(monitor_id.as_str());
	std::mem::drop(locked);

	let data: MonitorData;
	match serde_json::from_str(&req_body) {
		Ok(val) => data = val,
		Err(_)  => return HttpResponse::BadRequest()
	}

    col.insert_one(DbMonitorData{
    	timestamp:   Utc::now().to_string(),
     	battery:     data.battery,
     	hive_temp:   data.hive_temp,
      	extern_temp: data.extern_temp,
      	humidity:    data.humidity,
       	pressure:    data.pressure,
        acoustics:   data.acoustics,
    }).await.expect("Could not insert into db!");

    HttpResponse::Ok()
}

#[get("/millis")]
async fn milliseconds() -> impl Responder {
    chrono::Utc::now().timestamp_millis().to_string()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let config = std::fs::read_to_string("data.conf").expect("Could not open config file!");
	let keys: Keys = serde_json::from_str(config.as_str()).expect("Config file malformed!");

	let conn: mongodb::Client = mongodb::Client::with_uri_str(
		std::env::var("MONGODB_URI").expect(
			"Connection string environment variable does not exist!"
		)
	).await.expect("Could not establish connection!");

	let db =       conn.database("beehivesensors");
	let existing = db.list_collection_names().await.unwrap();

	for node in &keys.nodes {
		match existing.contains(&node) {
			true => continue,
			_    => db.create_collection(node).await.unwrap(),
		}
	}

	let app_data = web::Data::new(Mutex::new(AppData{
		db,
		keys,
		last_ping: chrono::DateTime::from_timestamp_millis(0).unwrap()
	}));

	println!("*** Starting HTTP Server ***");

	HttpServer::new(move || {
	    App::new()
	       	.service(monitor)
			.service(milliseconds)
			.service(ping_get)
			.service(ping_post)
	        .app_data(app_data.clone())
	})
	.client_disconnect_timeout(Duration::from_millis(100))
	.client_request_timeout(Duration::from_millis(500))
	.bind((std::env::var("HOST").expect(
			"HOST environment variable does not exist!"
		),
	   	8080
	))?
	.run()
	.await
}
