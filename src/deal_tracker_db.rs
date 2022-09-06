use std::collections::HashSet;
use std::sync::Arc;
// DataBase? more like DaBaby! https://www.youtube.com/watch?v=mxFstYSbBmc
use anyhow::{anyhow, Result};
use banyan_shared::{eth::VitalikProvider, types::*};
use cid::Cid;
use sled::transaction::UnabortableTransactionError;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use typed_sled::Batch;
use typed_sled::TransactionalTree;

use crate::database_types::*;

const DEAL_DB_IDENT: &str = "deal_db";
const SCHEDULE_DB_IDENT: &str = "schedule_db";

pub struct ProofScheduleDb {
    /// on_chain deal id --mapped_to--> LocalDealInfo
    deal_tree: RwLock<typed_sled::Tree<DealID, LocalDealInfo>>,
    /// window --mapped_to--> on_chain deal id vec
    // TODO are hashsets the right answer here? idk
    schedule_tree: RwLock<typed_sled::Tree<BlockNum, HashSet<DealID>>>,
}

impl ProofScheduleDb {
    pub fn new(sled_file: String) -> Result<Self> {
        let db = sled::open(sled_file)?;
        let deal_tree = RwLock::new(typed_sled::Tree::open(&db, DEAL_DB_IDENT));
        let schedule_tree = RwLock::new(typed_sled::Tree::open(&db, SCHEDULE_DB_IDENT));
        Ok(ProofScheduleDb {
            deal_tree,
            schedule_tree,
        })
    }

    // if the deal_id is scheduled at block_num, remove it from block_num
    // if it was the last scheduled in block_num, delete block_num from the sched_tree
    // if it wasn't in block_num, do nothing.
    fn unschedule_dealid(
        db: &TransactionalTree<BlockNum, HashSet<DealID>>,
        deal_id: DealID,
        block_num: BlockNum,
    ) -> Result<(), UnabortableTransactionError> {
        let mut batch = Batch::default();
        if let Some(mut deal_id_set) = db.get(&block_num)? {
            deal_id_set.remove(&deal_id);
            if deal_id_set.is_empty() {
                batch.remove(&block_num)
            } else {
                batch.insert(&block_num, &deal_id_set)
            }
        }
        db.apply_batch(&batch)
    }

    // if the deal_id is scheduled at block_num, do nothing
    // if block_num is not in the tree already, add it
    // if the deal id is not in the block_num add it to the hashset
    fn schedule_dealid(
        db: &TransactionalTree<BlockNum, HashSet<DealID>>,
        deal_id: DealID,
        block_num: BlockNum,
    ) -> Result<(), UnabortableTransactionError> {
        let mut batch = Batch::default();
        if let Some(mut deal_id_set) = db.get(&block_num)? {
            batch.insert(&block_num, &{
                deal_id_set.insert(deal_id);
                deal_id_set
            });
        } else {
            batch.insert(&block_num, &HashSet::from([deal_id]));
        }
        db.apply_batch(&batch)
    }

    // TODO not sure this is entirely atomic lol?? it's not fully on the database... this is because of typedtransactiontree things
    // TODO the deal_params never get removed from the database. perhaps we should implement it later.
    async fn unschedule_and_reschedule_atomic(
        &self,
        deal_id: DealID,
        old_block_num: Option<BlockNum>,
        new_block_num: Option<BlockNum>,
        updated_deal_params: Option<LocalDealInfo>,
    ) -> Result<()> {
        let deal_tree = self.deal_tree.write().await;
        let sched_tree = self.schedule_tree.write().await;
        sched_tree.transaction(|db: &TransactionalTree<BlockNum, HashSet<DealID>>| -> std::result::Result<(), sled::transaction::ConflictableTransactionError> {
            // remove the deal from the schedule tree at old_block_num
            if let Some(old_block_num) = old_block_num {
                ProofScheduleDb::unschedule_dealid(db, deal_id, old_block_num)?;
            };
            // insert the deal into the schedule tree at new_block_num
            if let Some(new_block_num) = new_block_num {
                ProofScheduleDb::schedule_dealid(db, deal_id, new_block_num)?;
            };
            Ok(())
        })?;
        // and update the deal tree if necessary
        if let Some(updated_deal_params) = updated_deal_params {
            let _ = deal_tree.insert(&deal_id, &updated_deal_params)?;
        };
        Ok(())
    }

    /// relate the on-chain ID to the LocalDealInfo struct.
    /// BEFORE YOU CALL THIS!: have accepted the deal on chain, have received and validated the file, and have generated and stored the obao.
    /// this schedules the deal for the deal_start_block and sets the status to future. on the next DB wakeup, it'll get scheduled correctly
    pub(crate) async fn add_a_deal_to_db(
        &self,
        deal_params: OnChainDealInfo,
        obao_cid: Cid,
    ) -> Result<()> {
        let local_deal_info = LocalDealInfo {
            on_chain: deal_params,
            obao_cid,
            last_successful_proof_submission: None,
            deal_todo: DealTodo::SubmitProof,
        };
        self.unschedule_and_reschedule_atomic(
            deal_params.deal_id,
            None,
            Some(deal_params.deal_start_block),
            Some(local_deal_info),
        )
        .await
        // TODO should we add a wakeup right now? idk probably not... unless its like TIME RIGHT NOW TO DO THE THING AIEEEE LAST MINUTE
    }

    async fn handle_deal_id(
        &self,
        eth_provider: Arc<VitalikProvider>,
        deal_id: DealID,
        wakeup_block: BlockNum,
    ) -> Result<()> {
        let current_block_n = eth_provider.get_latest_block_num().await?;
        let deal_info = self
            .deal_tree
            .read()
            .await
            .get(&deal_id)?
            .ok_or_else(|| anyhow!("no deal params found for deal id {:?}", deal_id))?;

        match deal_info.deal_todo {
            DealTodo::SubmitProof => {
                let (new_block_num, new_deal_info) = deal_info
                    .submit_proof(deal_id, Arc::clone(&eth_provider), current_block_n)
                    .await?;
                self.unschedule_and_reschedule_atomic(
                    deal_id,
                    Some(wakeup_block),
                    new_block_num,
                    Some(new_deal_info),
                )
                .await
            }
            DealTodo::Cancel => {
                unimplemented!("write me one day!");
            }
            DealTodo::InitiateChainlinkFinalization => {
                let (new_block_num, new_deal_info) = deal_info
                    .submit_finalization(deal_id, Arc::clone(&eth_provider), current_block_n)
                    .await?;
                self.unschedule_and_reschedule_atomic(
                    deal_id,
                    Some(wakeup_block),
                    new_block_num, // see if chainlink responded in 5 minutes! TODO actually write 5 minutes correctly
                    Some(new_deal_info),
                )
                .await
            }
            DealTodo::CheckChainlinkFinalization => {
                unimplemented!();
            }
            DealTodo::WithdrawEarnings => {
                // TODO check to make sure it's worth the gas
                // TODO if it is, withdraw your earnings to your local wallet of choice
                // TODO if it's not, do nothing
                unimplemented!();
            }
        }
    }
}

pub(crate) async fn wake_up(
    db_provider: Arc<ProofScheduleDb>,
    eth_provider: Arc<VitalikProvider>,
) -> Result<()> {
    let current_block_n = eth_provider.get_latest_block_num().await?;

    let blocks_and_deals = {
        let locked_tree = db_provider.schedule_tree.read().await;
        locked_tree
            .range(..current_block_n)
            .map(|item| item.map_err(|e| anyhow!("{:?}", e)))
            .collect::<Result<Vec<(BlockNum, HashSet<DealID>)>>>()
    }?;

    // this iterates over everything scheduled before the current block number.
    for block_and_deals in blocks_and_deals.iter() {
        let (wakeup_block, deal_ids) = block_and_deals;
        let eth_provider = eth_provider.clone();

        let mut stream = tokio_stream::iter(deal_ids.iter().map(|deal_id| {
            let eth_provider = eth_provider.clone();
            let db_provider = db_provider.clone();
            let (deal_id, wakeup_block) = (*deal_id, *wakeup_block);
            tokio::spawn(async move {
                db_provider
                    .handle_deal_id(eth_provider, deal_id, wakeup_block)
                    .await
            })
        }));
        while let Some(v) = stream.next().await {
            // TODO improve error handling
            match v.await {
                Err(e) => error!("something is wrong with the runtime! {:?}", e),
                Ok(Err(e)) => warn!("something is wrong with the database or something! {:?}", e),
                Ok(Ok(())) => {}
            }
        }
    }
    Ok(())
}
