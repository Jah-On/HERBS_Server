use crate::shared::{AuthToken, SharedData};
use actix_web::{web, HttpResponse, Resource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct DBDevice {
    pub auth_token: String,
    pub serial_number: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DBSensorReading {
    serial_number: String,
    timestamp: bson::DateTime,
    sensor_name: String,
    value: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SensorReading {
    class: u8,
    value: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SensorReadings {
    id: u8,
    timestamp: i64,
    values: Vec<SensorReading>,
}

impl SensorReading {
    fn to_string(self: &Self) -> String {
        let upper = self.class >> 2;
        let lower = self.class & 0x03;

        let sensor_type = match upper {
            0x00 => "TEMPERATURE",
            0x01 => "HUMIDITY",
            0x02 => "SOUND",
            0x03 => "CO2",
            _ => "WEIGHT",
        }
        .to_string();

        match lower {
            0 => return sensor_type,
            _ => return format!("{}_{}", sensor_type, lower),
        }
    }
}

async fn recieve_readings(
    path: web::Path<AuthToken>,
    body: String,
    shared: SharedData,
) -> HttpResponse {
    let token = path.into_inner();

    let Ok(locked) = shared.lock() else {
        return HttpResponse::InternalServerError().finish();
    };

    if !locked.auth_token_valid(token.clone()) {
        return HttpResponse::Unauthorized().finish();
    }

    let Ok(new_readings): Result<Vec<SensorReadings>, _> = serde_json::from_str(&body) else {
        return HttpResponse::NotAcceptable().finish();
    };

    for device_reading in new_readings {
        let Some(raw_serial_number) = locked.get_serial_number(token) else {
            continue;
        };

        for reading in device_reading.values {
            let sensor_name = reading.to_string();
            let value = reading.value;
            let timestamp = bson::DateTime::from_millis(device_reading.timestamp);
            let serial_number = raw_serial_number.to_string();

            let doc = DBSensorReading {
                serial_number,
                timestamp,
                sensor_name,
                value,
            };

            let res = locked
                .get_collection("sensor_readings".to_string())
                .insert_one(doc)
                .await;

            if res.is_err() {
                println!("Uploaded data has format error! {}", res.err().unwrap());
                return HttpResponse::NotAcceptable().finish();
            }
        }
    }

    HttpResponse::Ok().finish()
}

pub fn resources(config: &mut web::ServiceConfig) {
    // Add endpoints that should be served
    config.service(Resource::new("/data/{token}").post(recieve_readings));
}
