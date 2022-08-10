use std::sync::Arc;
// DataBase? more like DaBaby! https://www.youtube.com/watch?v=mxFstYSbBmc
use crate::proof_utils::gen_proof;
use crate::talk_to_ipfs;
use crate::talk_to_vitalik::VitalikProvider;
use crate::types::*;
use anyhow::{anyhow, Result};
use tokio::sync::Mutex;

// TODO: ensure this is safe if it falls over in the middle of a transaction. you've done half a job...
const SLED_FILE: &str = "deal_tracker.sled";
const DEAL_DB_IDENT: &str = "deal_db";
const SCHEDULE_DB_IDENT: &str = "schedule_db";

pub struct ProofScheduleDb {
    /// on_chain deal id --mapped_to--> DealParams
    deal_tree: typed_sled::Tree<DealID, DealParams>,
    /// window --mapped_to--> on_chain deal id vec
    schedule_tree: typed_sled::Tree<BlockNum, Vec<DealID>>,
}

lazy_static::lazy_static! {
    pub static ref DB: ProofScheduleDb = {
        let db = sled::open(SLED_FILE).unwrap();
        let deal_tree = typed_sled::Tree::open(&db, DEAL_DB_IDENT);
        let schedule_tree = typed_sled::Tree::open(&db, SCHEDULE_DB_IDENT);
        ProofScheduleDb {
            deal_tree,
            schedule_tree
        }
    };
}

impl ProofScheduleDb {
    pub(crate) fn schedule(&self, block_num: BlockNum, deal_id: DealID) -> Result<()> {
        let _ = self
            .schedule_tree
            .fetch_and_update(&block_num, |maybe_deal_ids| match maybe_deal_ids {
                Some(deal_ids) => {
                    let mut deal_ids = deal_ids;
                    deal_ids.push(deal_id);
                    Some(deal_ids)
                }
                None => Some(vec![deal_id]),
            })?;
        Ok(())
    }

    /// relate the on-chain ID to the DealParams struct.
    /// BEFORE YOU CALL THIS!: have accepted the deal on chain, have received and validated the file, and have generated and stored the obao.
    pub(crate) async fn add_a_deal_to_db(&self, deal_params: DealParams) -> Result<()> {
        // TODO: maybe we ought to add some checks for: having the obao, having the file, having the deal accepted on chain, timing, etc.
        self.deal_tree
            .insert(&deal_params.on_chain_deal_info.deal_id, &deal_params)?;

        // put into scheduler!
        // TODO: this is wrong!!! why is this wrong? what if the deal_start_block already happened? handle this logic somewhere.
        self.schedule(
            deal_params.on_chain_deal_info.deal_start_block,
            deal_params.on_chain_deal_info.deal_id,
        )
    }

    // TODO: make DB stuff atomic i think
    // TODO: ensure we aren't sitting here proving things that have already expired.
    pub(crate) async fn wake_up(&self, eth_provider: Arc<Mutex<VitalikProvider>>) -> Result<()> {
        let current_block_n = {
            // get the lock on the vitalikprovider
            let provider = (*eth_provider).lock().await;
            provider.get_latest_block_num().await
        }?;

        // TODO: update the proof information in the deal_tree (last_proven and next_proof and stuff)
        for block_and_deals in self.schedule_tree.range(BlockNum(0)..current_block_n) {
            let (block, deal_ids) = block_and_deals?;

            let block_hash = {
                // get the lock on the vitalikprovider
                let provider = (*eth_provider).lock().await;
                provider.get_block_hash_from_num(block).await
            }?;
            for deal_id in deal_ids.iter() {
                // TODO use sled compare_and_swap to atomically update the deal_params.
                let deal_params = self
                    .deal_tree
                    .get(deal_id)?
                    .ok_or_else(|| anyhow!("no deal params found for deal id {:?}", deal_id))?;
                let proof_to_post = gen_proof(
                    block,
                    block_hash,
                    talk_to_ipfs::get_handle_for_cid(deal_params.on_chain_deal_info.ipfs_file_cid)
                        .await?,
                    talk_to_ipfs::get_handle_for_cid(deal_params.obao_cid).await?,
                    deal_params.on_chain_deal_info.file_size,
                )
                .await?;
                {
                    // get the lock on the vitalikprovider
                    let provider = (*eth_provider).lock().await;
                    provider.post_proof(deal_id, proof_to_post).await
                }?;
            }
            self.schedule_tree.remove(&block)?;
        }

        Ok(())
    }
}
