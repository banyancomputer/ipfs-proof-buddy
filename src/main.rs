extern crate lazy_static;
#[macro_use]
extern crate rocket;

mod deal_tracker_db;
mod estuary_talker;
mod proof_gen;
mod talk_to_ipfs;
mod talk_to_vitalik;
mod types;

use tokio::time::{self, Duration};

// TODO: real error handling... :}
// TODO: separation of async and non async functions. PURITY OF CODE. uwu

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

// want to be able to accept a file from estuary, stick it in ipfs, keep it in a database with proof info, submit proofs regularly, and close out of deals.
#[rocket::main]
async fn main() {
    // start the webserver... bye bitch. off you go. get in a thread lol
    // this will probably just be for like getting information, i think.
    // probably this should be the client for all communications with its ipfs node, estuary, and ethereum.
    // maybe the storage client will talk to this API for deal negotiations, idk.
    tokio::spawn(async move {
        // TODO what do we do if it dies...?
        let _result = rocket::build()
            .mount("/", routes![index])
            .launch()
            .await
            .unwrap();
    });

    // TODO make this configurable
    let mut interval = time::interval(Duration::from_secs(60 * 15));

    loop {
        interval.tick().await;
        // database wakeup
        tokio::spawn(async move {
            // TODO what do we do if it dies...?
            // TODO what do we do if it takes fucking forever?
            estuary_talker::wake_up().await
        });
        tokio::spawn(async move {
            // TODO what do we do if it dies...?
            // TODO what do we do if it takes fucking forever?
            deal_tracker_db::DB.wake_up().await
        });
    }
}
