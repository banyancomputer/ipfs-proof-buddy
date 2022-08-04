use crate::types::BlockNum;
use crate::{deal_tracker_db, talk_to_ipfs, talk_to_vitalik};
use anyhow::Result;
use cid::Cid;

struct EstuaryNewDealInfo {
    ipfs_cid: Cid,
    file_length: u64,
    proposed_start: BlockNum,
    proposed_length: BlockNum,
    proposed_proof_frequency: BlockNum,
    proposed_price: u64,
}

async fn get_new_deals_from_estuary() -> Result<Vec<EstuaryNewDealInfo>> {
    unimplemented!("https://open.spotify.com/track/0oxYB9GoOIDrdzniNdKC44?si=71f88a0b1afa47a4")
}

async fn make_a_decision_on_acceptance(_new_deal_info: &EstuaryNewDealInfo) -> Result<bool> {
    unimplemented!("https://open.spotify.com/track/0oxYB9GoOIDrdzniNdKC44?si=71f88a0b1afa47a4")
}

// this should... talk to estuary and find out the status of any new deal negotiations
// it should download files to IPFS as needed
// it should accept deals and submit them to chain as needed
pub async fn wake_up() -> Result<()> {
    let dealios = get_new_deals_from_estuary().await?;
    for dealio in dealios.iter() {
        if !make_a_decision_on_acceptance(dealio).await? {
            // TODO: log skipping dealio
            continue;
        }
        talk_to_ipfs::download_file_from_ipfs(dealio.ipfs_cid, dealio.file_length).await?;
        let deal_id = talk_to_vitalik::accept_deal_on_chain().await?;
        deal_tracker_db::DB
            .add_a_deal_to_db(deal_id, dealio.ipfs_cid)
            .await?;
    }
    Ok(())
}
