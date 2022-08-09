use crate::talk_to_vitalik::get_on_chain_deal_info;
use crate::types::{BlockNum, DealID, DealParams, OnChainDealInfo};
use crate::{deal_tracker_db, talk_to_ipfs, talk_to_vitalik};
use anyhow::Result;
use log::info;

async fn _get_new_deals_from_estuary() -> Result<Vec<OnChainDealInfo>> {
    unimplemented!("https://open.spotify.com/track/0oxYB9GoOIDrdzniNdKC44?si=71f88a0b1afa47a4")
}

async fn make_a_decision_on_acceptance(_new_deal_info: &OnChainDealInfo) -> Result<bool> {
    unimplemented!("https://open.spotify.com/track/0oxYB9GoOIDrdzniNdKC44?si=71f88a0b1afa47a4")
}

// it should download files to IPFS as needed
// it should accept deals and submit them to chain as needed
// TODO: currently this fails the whole computation if one deal fails- need to change that
pub async fn estuary_call_handler(deal_ids: Vec<DealID>) -> Result<Vec<DealID>> {
    let mut accepted_deal_ids = Vec::new();
    for deal_id in deal_ids.iter() {
        let deal_info = get_on_chain_deal_info(*deal_id).await?;
        if !make_a_decision_on_acceptance(&deal_info).await? {
            info!("skipping deal: {:?}", &deal_info);
            continue;
        }
        talk_to_ipfs::download_file_from_ipfs(deal_info.ipfs_file_cid, deal_info.file_size).await?;
        let obao_cid = talk_to_ipfs::validate_file_and_gen_obao(
            deal_info.ipfs_file_cid,
            deal_info.blake3_file_checksum,
        )
        .await?;
        let on_chain_deal_info = talk_to_vitalik::accept_deal_on_chain().await?;
        deal_tracker_db::DB
            .add_a_deal_to_db(DealParams {
                on_chain_deal_info,
                obao_cid,
                next_proof_window_start_block_num: on_chain_deal_info.deal_start_block,
                last_proof_submission_block_num: BlockNum(0),
            })
            .await?;
        accepted_deal_ids.push(*deal_id);
    }
    Ok(accepted_deal_ids)
}
