use crate::estuary_talker::estuary_call_handler;
use crate::talk_to_vitalik::VitalikProvider;
use crate::types::DealID;
use anyhow::Result;
use rocket::{http::Status, response::status::Custom, serde::json::Json, Ignite, Rocket, State};
use std::sync::Arc;

struct WebserverState(Arc<VitalikProvider>);

#[put("/submit_deal", data = "<deal_ids>")]
async fn submit_deal(
    state: &State<WebserverState>,
    deal_ids: Json<Vec<DealID>>,
) -> Result<Json<Vec<DealID>>, Custom<String>> {
    let deal_ids = deal_ids.into_inner();

    // get the lock on the vitalikprovider
    match estuary_call_handler(deal_ids, state.0.as_ref()).await {
        Ok(accepted_deal_ids) => Ok(Json(accepted_deal_ids)),
        // TODO: don't leak the error message
        Err(e) => Err(Custom(Status::InternalServerError, e.to_string())),
    }
}

pub async fn launch_webserver(eth_provider: Arc<VitalikProvider>) -> Result<Rocket<Ignite>> {
    Ok(rocket::build()
        .mount("/submit_deal", routes![submit_deal])
        .manage(WebserverState(eth_provider))
        .launch()
        .await?)
}
