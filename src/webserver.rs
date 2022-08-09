use crate::estuary_talker::estuary_call_handler;
use crate::types::DealID;
use anyhow::Result;
use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::serde::json::Json;
use rocket::{Ignite, Rocket};

// TODO: make this accept new deals!
#[put("/submit_deal", data = "<deal_ids>")]
async fn submit_deal(deal_ids: Json<Vec<DealID>>) -> Result<Json<Vec<DealID>>, Custom<String>> {
    let deal_ids = deal_ids.into_inner();
    match estuary_call_handler(deal_ids).await {
        Ok(accepted_deal_ids) => Ok(Json(accepted_deal_ids)),
        // TODO: don't leak the error message
        Err(e) => Err(Custom(Status::InternalServerError, e.to_string())),
    }
}

pub async fn launch_webserver() -> Result<Rocket<Ignite>> {
    Ok(rocket::build()
        .mount("/submit_deal", routes![submit_deal])
        .launch()
        .await?)
}
