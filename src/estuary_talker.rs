use crate::talk_to_vitalik::VitalikProvider;
use crate::types::{BlockNum, DealID, DealStatus, LocalDealInfo, OnChainDealInfo};
use crate::{deal_tracker_db, proof_utils, talk_to_ipfs};
use anyhow::{anyhow, Result};
use cid::Cid;
use log::info;
use std::io::Read;

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

// it should download files to IPFS as needed
// it should accept deals and submit them to chain as needed
// TODO: currently this fails the whole computation if one deal fails- need to change that
pub async fn estuary_call_handler(
    deal_ids: Vec<DealID>,
    eth_provider: &VitalikProvider,
    db_provider: &deal_tracker_db::ProofScheduleDb,
) -> Result<Vec<DealID>> {
    let mut accepted_deal_ids = Vec::new();
    for deal_id in deal_ids.iter() {
        let deal_info = eth_provider.get_onchain(*deal_id).await?;
        if !make_a_decision_on_acceptance(&deal_info, eth_provider).await? {
            info!("skipping deal: {:?}", &deal_info);
            continue;
        }
        talk_to_ipfs::download_file_from_ipfs(deal_info.ipfs_file_cid, deal_info.file_size).await?;
        let file_handle = talk_to_ipfs::get_handle_for_cid(deal_info.ipfs_file_cid).await?;
        let obao_cid = build_and_store_obao(file_handle, deal_info.blake3_checksum).await?;
        let onchain = eth_provider.accept_deal_on_chain().await?;
        db_provider
            .add_a_deal_to_db(
                LocalDealInfo {
                    onchain,
                    obao_cid,
                    last_submission: BlockNum(0),
                    status: DealStatus::Future,
                },
                onchain.deal_start_block,
            )
            .await?;
        accepted_deal_ids.push(*deal_id);
    }
    Ok(accepted_deal_ids)
}
