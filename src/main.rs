extern crate lazy_static;
#[macro_use]
extern crate rocket;

mod deal_tracker_db;
mod estuary_talker;
mod proof_utils;
mod talk_to_ipfs;
mod talk_to_vitalik;
mod types;
mod webserver;

use config::{Config, File, FileFormat};
use log::{error, info};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

// TODO: separation of async and non async functions. PURITY OF CODE. uwu

// TODO: don't hardcode this... set it up with clap.
const CONFIG_FILE: &str = "~/.ipfs_proof_buddy.toml";
const WAKE_UP_INTERVAL: &str = "wake_up_interval";
const ETH_API_ADDR: &str = "eth_api_addr";
const _IPFS_API_ADDR: &str = "ipfs_api_addr";
// want to be able to accept a file from estuary, stick it in ipfs, keep it in a database with proof info, submit proofs regularly, and close out of deals.
#[rocket::main]
async fn main() {
    let config = Config::builder()
        .set_default(WAKE_UP_INTERVAL, 60 * 15)
        .unwrap()
        .set_default(ETH_API_ADDR, "https://mainnet.infura.io/v3/YOUR_API_KEY")
        .unwrap()
        .set_default("ipfs_api_addr", "localhost:5050")
        .unwrap()
        .add_source(File::new(CONFIG_FILE, FileFormat::Json))
        .build()
        .unwrap();

    // initialize ethereum api provider
    let eth_api_url = config.get_string(ETH_API_ADDR).unwrap();
    // TODO error my handle baby
    // TODO figure out how to not make two eth_providers
    let eth_provider = match talk_to_vitalik::VitalikProvider::new(eth_api_url.clone()) {
        Ok(provider) => Arc::new(Mutex::new(provider)),
        Err(e) => {
            error!("failed to create ethereum provider: {:?}", e);
            return;
        }
    };

    // start the webserver... bye bitch. off you go. get in a thread lol
    // this will probably just be for like getting information, i think.
    // probably this should be the client for all communications with its ipfs node, estuary, and ethereum.
    // maybe the storage client will talk to this API for deal negotiations, idk.
    info!("starting webserver to receive estuary deals");
    let eth_provider_for_webserver = Arc::clone(&eth_provider);
    tokio::spawn(async move {
        // TODO make sure error handling is done right
        match webserver::launch_webserver(eth_provider_for_webserver).await {
            Ok(_) => info!("webserver finished and terminated ok"),
            Err(e) => {
                error!("failed to start webserver to receive estuary deals: {}", e);
                panic!();
            }
        }
    });

    let wake_up_interval = config.get_int(WAKE_UP_INTERVAL).unwrap();
    let mut interval = time::interval(Duration::from_secs(wake_up_interval as u64));

    loop {
        interval.tick().await;
        // database wakeup
        let eth_provider_for_db = Arc::clone(&eth_provider);
        tokio::spawn(async move {
            // TODO what do we do if it dies...?
            // TODO what do we do if it takes fucking forever?
            deal_tracker_db::DB.wake_up(eth_provider_for_db).await
        });
    }
}
