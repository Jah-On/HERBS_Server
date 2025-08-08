use crate::shared::{AuthToken, SharedData};
use actix_web::{web, HttpResponse, Resource};

/////////////////////////////////////// Old Firmware API \\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\
/******************************** FIRMWARE API *******************************/
async fn gateway_firmware_info(
    path: web::Path<(String, String)>,
    _shared: SharedData,
) -> HttpResponse {
    let (apiary_id, gateway) = path.into_inner();

    let file = format!("./fw/{}/{}.data", apiary_id, gateway);

    match match std::fs::exists(&file) {
        Ok(val) => val,
        Err(_) => return HttpResponse::NotFound().finish(),
    } {
        false => return HttpResponse::NotFound().finish(),
        true => return HttpResponse::Ok().body(std::fs::read_to_string(file).unwrap()),
    }
}

async fn gateway_firmware_binary(
    path: web::Path<(String, String)>,
    _shared: SharedData,
) -> HttpResponse {
    let (apiary_id, gateway) = path.into_inner();

    let file = format!("./fw/{}/{}.bin", apiary_id, gateway);

    match match std::fs::exists(&file) {
        Ok(val) => val,
        Err(_) => return HttpResponse::NotFound().finish(),
    } {
        false => return HttpResponse::NotFound().finish(),
        true => {
            return HttpResponse::Ok()
                .content_type("application/octet-stream")
                .body(std::fs::read(file).unwrap())
        }
    }
}
/*****************************************************************************/

/////////////////////////////////////// New Firmware API \\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\
async fn get_firmware(path: web::Path<(AuthToken, String)>, shared: SharedData) -> HttpResponse {
    let (token, version) = path.into_inner();

    let Ok(locked) = shared.lock() else {
        return HttpResponse::InternalServerError().finish();
    };

    if !locked.auth_token_valid(token.clone()) {
        return HttpResponse::Unauthorized().finish();
    }

    let dir_path = format!(
        "./fw/{}/{}.bin",
        locked.get_serial_number(token).unwrap(),
        version
    );

    drop(locked);

    let Ok(mut dir) = std::fs::read_dir(dir_path) else {
        return HttpResponse::InternalServerError().finish();
    };
    let Some(entry) = dir.next() else {
        return HttpResponse::InternalServerError().finish();
    };
    let Ok(contents) = std::fs::read(entry.unwrap().path()) else {
        return HttpResponse::InternalServerError().finish();
    };

    return HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(contents);
}
/*****************************************************************************/

// pub fn resources() -> Vec<Resource> {
//     let mut resourses: Vec<Resource> = Vec::new();

//     // Add endpoints that should be served
//     resourses.push(Resource::new("/fw/stream/{token}/{current}").get(get_firmware));
//     resourses.push(
//         Resource::new("/gateway/firmware/info/{apiary_id}/{gateway_id}").get(gateway_firmware_info),
//     );
//     resourses.push(
//         Resource::new("/gateway/firmware/bin/{apiary_id}/{gateway_id}")
//             .get(gateway_firmware_binary),
//     );
//     return resourses;
// }

pub fn resources(config: &mut web::ServiceConfig) {
    // Add endpoints that should be served
    config.service(Resource::new("/fw/stream/{token}/{current}").get(get_firmware));
    config.service(
        Resource::new("/gateway/firmware/info/{apiary_id}/{gateway_id}").get(gateway_firmware_info),
    );
    config.service(
        Resource::new("/gateway/firmware/bin/{apiary_id}/{gateway_id}")
            .get(gateway_firmware_binary),
    );
}
