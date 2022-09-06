use crate::deal_tracker_db;
use anyhow::{anyhow, Result};
use banyan_shared::types::DealID;
use banyan_shared::types::*;
use banyan_shared::{eth, ipfs, proofs};
use cid::Cid;
use log::info;
use std::io::Read;
use std::sync::Arc;
use tokio_stream::StreamExt;

async fn make_a_decision_on_acceptance(
    new_deal_info: &OnChainDealInfo,
    eth_provider: &eth::VitalikProvider,
) -> Result<bool> {
    if new_deal_info.deal_start_block + new_deal_info.deal_length_in_blocks
        > eth_provider.get_latest_block_num().await?
    {
        return Err(anyhow!("deal ended"));
    }

    // TODO: you will need to check more than this... but this is a start. check on-chain state as you keep going
    unimplemented!("check the rest of the things you need to check for incoming deal parameters!")
}

/// validate the received file, generate the obao, store the obao locally
/// returns Ok(Cid) of the obao if things succeeded, Error if not
pub async fn build_and_store_obao<T: Read>(
    local_file_handle: T,
    blake3_hash: bao::Hash,
) -> Result<Cid> {
    let (obao_bytes, obao_digest) = proofs::gen_obao(local_file_handle)?;
    if obao_digest != blake3_hash {
        return Err(anyhow::anyhow!("obao does not match blake3 hash"));
    };
    ipfs::write_bytes_to_ipfs(obao_bytes).await
}

/// TODO: this needs better error handling!!!
/// TODO: returning the same dealID passed in is janky :|
pub async fn handle_incoming_deal(
    deal_id: DealID,
    eth_provider: Arc<eth::VitalikProvider>,
    db_provider: Arc<deal_tracker_db::ProofScheduleDb>,
) -> Result<DealID> {
    let deal_info = eth_provider.get_onchain(deal_id).await?;
    if !make_a_decision_on_acceptance(&deal_info, eth_provider.as_ref()).await? {
        info!("skipping deal: {:?}", &deal_info);
        return Err(anyhow!("chose not to accept deal {:?}", deal_id));
    }
    // this one is an external error- can continue if it screws up
    ipfs::download_file_from_ipfs(deal_info.ipfs_file_cid, deal_info.file_size).await?;
    let file_handle = ipfs::get_handle_for_cid(deal_info.ipfs_file_cid).await?;
    let obao_cid = build_and_store_obao(file_handle, deal_info.blake3_checksum).await?;
    let onchain = eth_provider.accept_deal_on_chain().await?;
    db_provider.add_a_deal_to_db(onchain, obao_cid).await?;
    Ok(deal_id)
}

// it should download files to IPFS as needed
// it should accept deals and submit them to chain as needed
/// Note: this does not error, it just logs at level warn if it can't do all the things that it needs to when attempting to ingest a deal
/// TODO: make sure all the log levels make sense and that none of these errors are actually things that ought to be fatal
/// TODO: this error handling is like laughably bad please claudia fix this
pub async fn estuary_call_handler(
    deal_ids: Vec<DealID>,
    eth_provider: Arc<eth::VitalikProvider>,
    db_provider: Arc<deal_tracker_db::ProofScheduleDb>,
) -> Result<Vec<DealID>> {
    // spins off a thread for each deal_id
    let mut stream = tokio_stream::iter(deal_ids.iter().map(|deal_id| {
        let eth_provider = eth_provider.clone();
        let db_provider = db_provider.clone();
        let deal_id = *deal_id;
        tokio::spawn(async move { handle_incoming_deal(deal_id, eth_provider, db_provider).await })
    }));
    let mut results = Vec::new();
    while let Some(v) = stream.next().await {
        match v.await {
            Err(e) => {
                panic!("something is wrong with the runtime! {:?}", e)
            }
            Ok(Err(e)) => {
                warn!("something is wrong with the database or something! {:?}", e);
                return Err(e);
            }
            Ok(Ok(deal_id)) => results.push(deal_id),
        }
    }
    Ok(results)
}
