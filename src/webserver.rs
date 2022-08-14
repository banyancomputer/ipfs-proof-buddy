use crate::deal_tracker_db::ProofScheduleDb;
use crate::estuary_talker::estuary_call_handler;
use crate::talk_to_vitalik::VitalikProvider;
use crate::types::DealID;
use anyhow::Result;
use rocket::{http::Status, response::status::Custom, serde::json::Json, Ignite, Rocket, State};
use std::sync::Arc;

struct WebserverState(Arc<VitalikProvider>, Arc<ProofScheduleDb>);

#[put("/submit_deal", data = "<deal_ids>")]
async fn submit_deal(
    state: &State<WebserverState>,
    deal_ids: Json<Vec<DealID>>,
) -> Result<Json<Vec<DealID>>, Custom<String>> {
    let deal_ids = deal_ids.into_inner();

    // get the lock on the vitalikprovider
    match estuary_call_handler(deal_ids, state.0.as_ref(), state.1.as_ref()).await {
        Ok(accepted_deal_ids) => Ok(Json(accepted_deal_ids)),
        Err(e) => {
            warn!("there was an error handling the estuary call: {:?}", e);
            Err(Custom(
                Status::InternalServerError,
                "internal server error :) sowwy... check the logs if you're running this :3"
                    .parse()
                    .unwrap(),
            ))
        }
    }
}

pub async fn launch_webserver(
    eth_provider: Arc<VitalikProvider>,
    db_provider: Arc<ProofScheduleDb>,
) -> Result<Rocket<Ignite>> {
    Ok(rocket::build()
        .mount("/submit_deal", routes![submit_deal])
        .manage(WebserverState(eth_provider, db_provider))
        .launch()
        .await?)
}
