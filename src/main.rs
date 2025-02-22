use actix_web::http::StatusCode;
use actix_web::{get, post, put, web, App, HttpResponse, HttpServer};
use chrono::prelude::*;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

mod apiary;

struct AppData {
	db:        mongodb::Database,
	last_ping: HashMap<String, chrono::DateTime<Utc>>,
	apiaries:  HashMap<String, apiary::Apiary>
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

/******************************** FIRMWARE API *******************************/
#[get("/gateway/firmware/info/{apiary_id}/{gateway_id}")]
async fn gateway_firmware_info(path: web::Path<(String, String)>, shared: web::Data<Mutex<AppData>>) -> HttpResponse {
	let (apiary_id, gateway) = path.into_inner();

	let locked = match shared.lock() {
		Ok(res) => res,
		Err(_) =>  return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
	};

	let matching_apiary = match locked.apiaries.get(&apiary_id.clone()) {
		Some(res) => res,
		_ => {
			std::mem::drop(locked);
			return HttpResponse::new(StatusCode::UNAUTHORIZED);
		},
	};

	match matching_apiary.has_gateway(gateway.clone()) {
		false => {
			std::mem::drop(locked);
			return HttpResponse::new(StatusCode::UNAUTHORIZED);
		},
		true => {}
	}

	let file = format!("./fw/{}/{}.data", apiary_id, gateway);

	match match std::fs::exists(&file) {
		Ok(val) => val,
		Err(_) => return HttpResponse::new(StatusCode::NOT_FOUND),
	} {
 		false => return HttpResponse::new(StatusCode::NOT_FOUND),
		true  => return HttpResponse::Ok().body(
			std::fs::read_to_string(file).unwrap()
		),
	}
}

#[get("/gateway/firmware/bin/{apiary_id}/{gateway_id}")]
async fn gateway_firmware_binary(path: web::Path<(String, String)>, shared: web::Data<Mutex<AppData>>) -> HttpResponse {
	let (apiary_id, gateway) = path.into_inner();

	let locked = match shared.lock() {
		Ok(res) => res,
		Err(_) =>  return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
	};

	let _matching_apiary = match locked.apiaries.get(&apiary_id.clone()) {
		Some(res) => res,
		_ => {
			std::mem::drop(locked);
			return HttpResponse::new(StatusCode::UNAUTHORIZED);
		}
	};

	let file = format!("./fw/{}/{}.bin", apiary_id, gateway);

	match match std::fs::exists(&file) {
		Ok(val) => val,
		Err(_) => return HttpResponse::new(StatusCode::NOT_FOUND),
	} {
 		false => return HttpResponse::new(StatusCode::NOT_FOUND),
		true  => return HttpResponse::Ok()
    		.content_type("application/octet-stream")
    		.body(
      			std::fs::read(file).unwrap()
      		),
	}
}

/*****************************************************************************/

#[put("/gateway/ping/{apiary_id}/{gateway_id}")]
async fn ping_post(path: web::Path<(String, String)>, shared: web::Data<Mutex<AppData>>) -> HttpResponse {
	let (apiary_id, gateway) = path.into_inner();

	let mut locked = match shared.lock() {
		Ok(res) => res,
		Err(_) =>  return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
	};

	match locked.apiaries.get(&apiary_id.clone()) {
		Some(_) => {},
		_ => {
			std::mem::drop(locked);
			return HttpResponse::new(StatusCode::UNAUTHORIZED);
		}
	};

	// locked.last_ping = Utc::now();
	*locked.last_ping.entry(gateway).or_insert(
		Utc::now()
	) = Utc::now();
	std::mem::drop(locked);

	HttpResponse::new(StatusCode::OK)
}

#[get("/gateway/ping/{apiary_id}/{gateway_id}")]
async fn ping_get(path: web::Path<(String, String)>, shared: web::Data<Mutex<AppData>>) -> HttpResponse {
	let (apiary_id, gateway) = path.into_inner();

	let locked = match shared.lock() {
		Ok(res) => res,
		Err(_) => return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
	};

	match locked.apiaries.get(&apiary_id.clone()) {
		Some(_) => {},
		_ => {
			std::mem::drop(locked);
			return HttpResponse::new(StatusCode::UNAUTHORIZED);
		}
	};

	let time_as_string = match locked.last_ping.get(&gateway) {
		Some(ref_date_time) => ref_date_time.to_string(),
		_                   => chrono::DateTime::UNIX_EPOCH.to_string()
	};
	std::mem::drop(locked);

	HttpResponse::Ok().body(time_as_string)
}

#[post("/{access_key}/{monitor_id}")]
async fn monitor(path: web::Path<(String, String)>, req_body: String, shared: web::Data<Mutex<AppData>>) -> HttpResponse {
	let (apiary_id, monitor_id) = path.into_inner();

	let locked = match shared.lock() {
		Ok(res) => res,
		Err(_) => return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
	};

	let found_apiary = match locked.apiaries.get(&apiary_id.clone()) {
		Some(res) => res,
		_ => {
			std::mem::drop(locked);
			return HttpResponse::new(StatusCode::UNAUTHORIZED);
		}
	};

	match found_apiary.has_bridge(monitor_id.clone()) {
		true => {},
		_    => {
			std::mem::drop(locked);
			return HttpResponse::new(StatusCode::NOT_FOUND)
		}
	}

	let col: mongodb::Collection<DbMonitorData> = locked.db.collection(monitor_id.as_str());
	std::mem::drop(locked);

	let data: MonitorData;
	match serde_json::from_str(&req_body) {
		Ok(val) => data = val,
		Err(_)  => return HttpResponse::new(StatusCode::BAD_REQUEST)
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

    HttpResponse::new(StatusCode::OK)
}

#[get("/millis")]
async fn milliseconds() -> String {
    chrono::Utc::now().timestamp_millis().to_string()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let config = std::fs::read_to_string("data.conf").expect("Could not open config file!");
	let apiaries: HashMap<String, apiary::Apiary> = serde_json::from_str(config.as_str()).expect("Config file malformed!");

	let conn: mongodb::Client = mongodb::Client::with_uri_str(
		std::env::var("MONGODB_URI").expect(
			"Connection string environment variable does not exist!"
		)
	).await.expect("Could not establish connection!");

	let db =       conn.database("beehivesensors");
	let existing = db.list_collection_names().await.unwrap();

	for ap in apiaries.values() {
		for bridge in ap.get_bridges() {
			match existing.contains(&bridge) {
				true => continue,
				_    => db.create_collection(bridge).await.unwrap(),
			}
		}
	}

	let app_data = web::Data::new(Mutex::new(AppData{
		db,
		apiaries,
		last_ping: HashMap::new()
	}));

	println!("*** Starting HTTP Server ***");

	let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("private.pem", SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file("public.pem").unwrap();

	HttpServer::new(move || {
	    App::new()
	       	.service(monitor)
			.service(milliseconds)
			.service(ping_get)
			.service(ping_post)
			.service(gateway_firmware_info)
			.service(gateway_firmware_binary)
	        .app_data(app_data.clone())
	})
	.workers(128)
	.bind_openssl(
		(
			std::env::var("HOST").expect("HOST environment variable does not exist!"),
		   	8080
		),
		builder
	)?
	.run()
	.await
}
