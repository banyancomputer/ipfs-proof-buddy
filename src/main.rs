#![deny(unused_crate_dependencies)]
#![deny(unused_unsafe)]

extern crate lazy_static;
#[macro_use]
extern crate rocket;

mod database_types;
mod deal_tasks;
mod deal_tracker_db;
mod estuary_talker;
mod webserver;

use config::{Config, File, FileFormat};
use lazy_static::lazy_static;
use log::{error, info};
use std::path::Path;
use std::sync::Arc;
use tokio::time::{self, Duration};

use banyan_shared::eth;

// TODO: don't hardcode this... set it up with clap.
const CONFIG_DIR: &str = "~/.ipfs_proof_buddy";
lazy_static! {
    static ref CONFIG_FILE_PATH: String = format!("{}/config.toml", CONFIG_DIR);
    static ref SLED_FILE_PATH: String = format!("{}/sled.db", CONFIG_DIR);
}

const WAKE_UP_INTERVAL_KEY: &str = "wake_up_interval";
const ETH_API_ADDR_KEY: &str = "eth_api_addr";
const ETH_API_TIMEOUT_KEY: &str = "eth_api_timeout";
const IPFS_API_ADDR_KEY: &str = "ipfs_api_addr";
const SLED_FILE_PATH_KEY: &str = "sled_file_path";

// TODO one day you need to tear out anyhow, you SINNER

// want to be able to accept a file from estuary, stick it in ipfs, keep it in a database with proof info, submit proofs regularly, and close out of deals.
#[rocket::main]
async fn main() {
    if !Path::new(CONFIG_DIR).is_dir() {
        panic!(
            "config folder at {} does not exist- please create it and try again",
            CONFIG_DIR
        );
    }

    let config = Config::builder()
        .set_default(WAKE_UP_INTERVAL_KEY, 60 * 15)
        .unwrap()
        .set_default(
            ETH_API_ADDR_KEY,
            "https://mainnet.infura.io/v3/YOUR_API_KEY",
        )
        .expect("set your api key in the config file") // TODO handle unset API key correctly
        .set_default(ETH_API_TIMEOUT_KEY, 5)
        .unwrap()
        .set_default(IPFS_API_ADDR_KEY, "localhost:5050")
        .unwrap()
        .set_default(SLED_FILE_PATH_KEY, &SLED_FILE_PATH[..])
        .unwrap()
        .add_source(File::new(&CONFIG_FILE_PATH, FileFormat::Json))
        .build()
        .unwrap();

    // initialize ethereum api provider
    let eth_api_url = config.get_string(ETH_API_ADDR_KEY).unwrap();
    let eth_provider = match eth::VitalikProvider::new(eth_api_url.clone(), "aaaaaaaaa".to_string())
    {
        Ok(provider) => Arc::new(provider),
        Err(e) => {
            error!("failed to create ethereum provider: {:?}", e);
            return;
        }
    };

    // initialize database provider
    let sled_file_path = config.get_string(ETH_API_ADDR_KEY).unwrap();
    let db_provider = match deal_tracker_db::ProofScheduleDb::new(sled_file_path.clone()) {
        Ok(provider) => Arc::new(provider),
        Err(e) => {
            error!("failed to create database provider: {:?}", e);
            return;
        }
    };

    // start the webserver... bye bitch. off you go. get in a thread lol
    // this will probably just be for like getting information, i think.
    // probably this should be the client for all communications with its ipfs node, estuary, and ethereum.
    // maybe the storage client will talk to this API for deal negotiations, idk.
    info!("starting webserver to receive estuary deals");
    let eth_provider_for_webserver = Arc::clone(&eth_provider);
    let db_provider_for_webserver = Arc::clone(&db_provider);
    tokio::spawn(async move {
        // TODO make sure error handling is done right
        match webserver::launch_webserver(eth_provider_for_webserver, db_provider_for_webserver)
            .await
        {
            Ok(_) => info!("webserver finished and terminated ok"),
            Err(e) => {
                error!("failed to start webserver to receive estuary deals: {}", e);
                panic!();
            }
        }
    });

    let wake_up_interval = config.get_int(WAKE_UP_INTERVAL_KEY).unwrap();
    let mut interval = time::interval(Duration::from_secs(wake_up_interval as u64));

    loop {
        interval.tick().await;
        let eth_provider = eth_provider.clone();
        let db_provider = db_provider.clone();
        // database wakeup
        tokio::spawn(async move {
            // TODO what do we do if it dies...? handle better.
            deal_tracker_db::wake_up(db_provider, eth_provider)
                .await
                .unwrap();
        });
    }
}
