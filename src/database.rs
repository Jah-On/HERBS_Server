use mongodb::{
    options::{TimeseriesGranularity, TimeseriesOptions},
    Database,
};

pub async fn check_or_make_pings(db: &Database, created: &Vec<String>) {
    if created.contains(&"gateway_pings".to_string()) {
        return;
    }

    let pings_config = TimeseriesOptions::builder()
        .granularity(TimeseriesGranularity::Minutes)
        .time_field("timestamp")
        .meta_field("serial_number".to_string())
        .build();
    db.create_collection("gateway_pings")
        .timeseries(pings_config)
        .await
        .unwrap();
}

pub async fn check_or_make_sensor_readings(db: &Database, created: &Vec<String>) {
    if created.contains(&"sensor_readings".to_string()) {
        return;
    }

    let readings_config = TimeseriesOptions::builder()
        .granularity(TimeseriesGranularity::Minutes)
        .time_field("timestamp")
        .meta_field("serial_number".to_string())
        .build();
    db.create_collection("sensor_readings")
        .timeseries(readings_config)
        .await
        .unwrap();
}

pub async fn check_or_make_devices(db: &Database, created: &Vec<String>) {
    if created.contains(&"devices".to_string()) {
        return;
    }

    db.create_collection("devices").await.unwrap();
}

pub async fn check_or_make_all(db: &Database, created: &Vec<String>) {
    check_or_make_pings(db, created).await;
    check_or_make_sensor_readings(db, created).await;
    check_or_make_devices(db, created).await;
}
