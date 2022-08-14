use crate::talk_to_vitalik::VitalikProvider;
use crate::types::{DealID, OnChainDealInfo, ProofBuddyError};
use crate::{deal_tracker_db, proof_utils, talk_to_ipfs};
use anyhow::{anyhow, Result};
use cid::Cid;
use log::info;
use std::io::Read;
use std::sync::Arc;
use tokio_stream::StreamExt;

async fn make_a_decision_on_acceptance(
    new_deal_info: &OnChainDealInfo,
    eth_provider: &VitalikProvider,
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
    let (obao_digest, obao_file) = proof_utils::gen_obao(local_file_handle).await?;
    if obao_digest != blake3_hash {
        return Err(anyhow::anyhow!("obao does not match blake3 hash"));
    };
    talk_to_ipfs::write_file_to_ipfs(obao_file).await
}

/// this needs better error handling!!!
/// TODO returning the same dealID passed in is janky and you need to sleep & eat :|
pub async fn handle_incoming_deal(
    deal_id: DealID,
    eth_provider: Arc<VitalikProvider>,
    db_provider: Arc<deal_tracker_db::ProofScheduleDb>,
) -> Result<DealID, ProofBuddyError> {
    let deal_info = eth_provider
        .get_onchain(deal_id)
        .await
        .map_err(|e| ProofBuddyError::InformWebserver(e.to_string()))?;
    if !make_a_decision_on_acceptance(&deal_info, eth_provider.as_ref())
        .await
        .map_err(|e| ProofBuddyError::InformWebserver(e.to_string()))?
    {
        info!("skipping deal: {:?}", &deal_info);
        return Err(ProofBuddyError::NonFatal(format!(
            "chose not to accept deal {:?}",
            deal_id
        )));
    }
    // this one is an external error- can continue if it screws up
    talk_to_ipfs::download_file_from_ipfs(deal_info.ipfs_file_cid, deal_info.file_size)
        .await
        .map_err(|e| ProofBuddyError::InformWebserver(e.to_string()))?;
    let file_handle = talk_to_ipfs::get_handle_for_cid(deal_info.ipfs_file_cid)
        .await
        .map_err(|e| ProofBuddyError::InformWebserver(e.to_string()))?;
    let obao_cid = build_and_store_obao(file_handle, deal_info.blake3_checksum)
        .await
        .map_err(|e| ProofBuddyError::InformWebserver(e.to_string()))?;
    let onchain = eth_provider
        .accept_deal_on_chain()
        .await
        .map_err(|e| ProofBuddyError::InformWebserver(e.to_string()))?;
    db_provider
        .add_a_deal_to_db(onchain, obao_cid)
        .await
        .map_err(|e| ProofBuddyError::InformWebserver(e.to_string()))?;
    Ok(deal_id)
}

// it should download files to IPFS as needed
// it should accept deals and submit them to chain as needed
/// Note: this does not error, it just logs at level warn if it can't do all the things that it needs to when attempting to ingest a deal
/// TODO: make sure all the log levels make sense and that none of these errors are actually things that ought to be fatal
/// TODO: this error handling is like laughably bad please claudia fix this
pub async fn estuary_call_handler(
    deal_ids: Vec<DealID>,
    eth_provider: Arc<VitalikProvider>,
    db_provider: Arc<deal_tracker_db::ProofScheduleDb>,
) -> Result<Vec<DealID>, ProofBuddyError> {
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
