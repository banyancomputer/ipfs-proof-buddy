use std::collections::HashSet;
use std::sync::Arc;
// DataBase? more like DaBaby! https://www.youtube.com/watch?v=mxFstYSbBmc
use crate::proof_utils::gen_proof;
use crate::talk_to_ipfs;
use crate::talk_to_vitalik::VitalikProvider;
use crate::types::*;
use anyhow::{anyhow, Result};
use tokio::sync::Mutex;

// TODO: ensure this is safe if it falls over in the middle of a transaction. you've done half a job...
const DEAL_DB_IDENT: &str = "deal_db";
const SCHEDULE_DB_IDENT: &str = "schedule_db";

pub struct ProofScheduleDb {
    /// on_chain deal id --mapped_to--> DealParams
    deal_tree: Mutex<typed_sled::Tree<DealID, DealParams>>,
    /// window --mapped_to--> on_chain deal id vec
    // TODO are hashsets the right answer here? idk
    schedule_tree: Mutex<typed_sled::Tree<BlockNum, HashSet<DealID>>>,
}

impl ProofScheduleDb {
    pub fn new(sled_file: String) -> Result<Self> {
        let db = sled::open(sled_file)?;
        let deal_tree = Mutex::new(typed_sled::Tree::open(&db, DEAL_DB_IDENT));
        let schedule_tree = Mutex::new(typed_sled::Tree::open(&db, SCHEDULE_DB_IDENT));
        Ok(ProofScheduleDb {
            deal_tree,
            schedule_tree
        })
    }

    // TODO not sure this is entirely atomic lol?? it's not on the database...
    async fn unschedule_and_reschedule_atomic(&self, deal_id: DealID, old_block_num: Option<BlockNum>, new_block_num: Option<BlockNum>, updated_deal_params: Option<DealParams>) -> Result<()> {
        let deal_tree = self.deal_tree.lock().await;
        let sched_tree = self.schedule_tree.lock().await;
        sched_tree.transaction(|db| {
            // remove the deal from the schedule tree at old_block_num
            if let Some(old_block_num) = old_block_num {
                if let Some(deal_id_set) = db.get(&old_block_num)? {
                    let mut deal_id_set = deal_id_set;
                    deal_id_set.remove(&deal_id);
                    if deal_id_set.is_empty() {
                        let _ = db.remove(&old_block_num)?;
                    } else {
                        let _ = db.insert(&old_block_num, &deal_id_set)?;
                    }
                }
            };
            // insert the deal into the schedule tree at new_block_num
            if let Some(new_block_num) = new_block_num {
                if let Some(deal_id_set) = db.get(&new_block_num)? {
                    let _ = db.insert(&new_block_num, {
                        let mut deal_id_set = deal_id_set;
                        deal_id_set.insert(deal_id);
                        &deal_id_set
                    })?;
                } else {
                    let _ = db.insert(&new_block_num, &{
                        let mut deal_id_set = HashSet::new();
                        deal_id_set.insert(deal_id);
                        deal_id_set
                    })?;
                }
            };
            Ok(())
        })?;
        // and update the deal tree if necessary
        if let Some(updated_deal_params) = updated_deal_params {
            let _ = deal_tree.insert(&deal_id, &updated_deal_params)?;
        };
        Ok(())
    }

    // TODO: maybe we ought to add some checks for: having the obao, having the file, having the deal accepted on chain, timing, etc.
    // TODO: this is wrong!!! why is this wrong? what if the deal_start_block already happened? handle this logic somewhere.
    /// relate the on-chain ID to the DealParams struct.
    /// BEFORE YOU CALL THIS!: have accepted the deal on chain, have received and validated the file, and have generated and stored the obao.
    pub(crate) async fn add_a_deal_to_db(&self, deal_params: DealParams) -> Result<()> {
        self.unschedule_and_reschedule_atomic(deal_params.on_chain_deal_info.deal_id, None, Some(deal_params.next_proof_window_start_block_num), Some(deal_params)).await
    }



    // TODO: make DB stuff atomic i think
    // TODO: ensure we aren't sitting here proving things that have already expired.
    // TODO: add hella timeouts to DB tasks
    pub(crate) async fn wake_up(&self, eth_provider: Arc<VitalikProvider>) -> Result<()> {
        let current_block_n = eth_provider.get_latest_block_num().await?;

        // don't just sit there holding the mutex forever. this is just our target to get through on this wake_up
        let blocks_and_deals = {self.schedule_tree.lock().await.range(..current_block_n).collect::<Vec<Result<(BlockNum, HashSet<DealID>)>>>()};

        // TODO: update the proof information in the deal_tree (last_proven and next_proof and stuff)
        for block_and_deals in blocks_and_deals.iter() {
            let (block, deal_ids) = (*block_and_deals)?;

            let block_hash =
                // get the lock on the vitalikprovider
                eth_provider.get_block_hash_from_num(block).await
            ?;
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
                // TODO posting proof to chain may take a while, should probably wait until we're sure the transaction succeeded to update the database.
                eth_provider.post_proof(deal_id, proof_to_post).await?;
            }
            self.schedule_tree.remove(&block)?;
        }
        Ok(())
    }
}
