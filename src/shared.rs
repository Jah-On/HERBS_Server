use actix_web::web;
use mongodb::{bson::doc, Collection};

use std::{collections::HashMap, str::FromStr, sync::Mutex};
use uuid::Uuid;

use crate::device::DBDevice;

pub type AuthToken = Uuid;
pub type SerialNumber = Uuid;
pub type AuthData = HashMap<AuthToken, SerialNumber>;

pub struct AppData {
    db: mongodb::Database,
    authed: AuthData,
}

impl AppData {
    pub async fn new(db: mongodb::Database) -> Self {
        let mut authed: AuthData = HashMap::new();

        let collection: Collection<DBDevice> = db.collection("devices");
        let query = collection.find(doc! {}).await;

        let Ok(mut entries) = query else {
            return AppData { db, authed };
        };

        while entries.advance().await.unwrap() {
            let Ok(entry) = entries.deserialize_current() else {
                break;
            };

            authed.insert(
                Uuid::from_str(&entry.auth_token).unwrap(),
                Uuid::from_str(&entry.serial_number).unwrap(),
            );
        }

        println!("Found {} devices...", authed.len());

        return AppData { db, authed };
    }

    pub fn auth_token_valid(&self, token: AuthToken) -> bool {
        return self.authed.contains_key(&token);
    }

    pub fn get_serial_number(&self, token: AuthToken) -> Option<AuthToken> {
        return self.authed.get(&token).cloned();
    }

    pub fn get_collection<T: Sync + Send>(self: &Self, name: String) -> mongodb::Collection<T> {
        return self.db.collection(&name);
    }
}

pub type SharedData = web::Data<Mutex<AppData>>;
