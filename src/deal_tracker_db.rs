use std::collections::HashSet;
use std::sync::Arc;
// DataBase? more like DaBaby! https://www.youtube.com/watch?v=mxFstYSbBmc
use anyhow::{anyhow, Result};
use banyan_shared::{eth::VitalikProvider, ipfs, proofs::gen_proof, proofs::window, types::*};
use cid::Cid;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use typed_sled::TransactionalTree;

// TODO: ensure this is safe if it falls over in the middle of a transaction. you've done half a job...
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

    // TODO not sure this is entirely atomic lol?? it's not on the database...
    // TODO make getters and setters for these hashset members
    // the deal_params never get removed from the database. perhaps we should implement it later.
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
            if let Some(new_block_num) = new_block_num{
                if let Some(deal_id_set) = db.get(&new_block_num)? {
                    let _ = db.insert(&new_block_num, &{
                        let mut deal_id_set = deal_id_set;
                        deal_id_set.insert(deal_id);
                        deal_id_set
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

    // TODO handle finality time!!!
    /// relate the on-chain ID to the LocalDealInfo struct.
    /// BEFORE YOU CALL THIS!: have accepted the deal on chain, have received and validated the file, and have generated and stored the obao.
    /// this schedules the deal for the deal_start_block and sets the status to future. on the next DB wakeup, it'll get scheduled correctly
    pub(crate) async fn add_a_deal_to_db(
        &self,
        deal_params: OnChainDealInfo,
        obao_cid: Cid,
    ) -> Result<()> {
        let local_deal_info = LocalDealInfo {
            onchain: deal_params,
            obao_cid,
            last_submission: BlockNum(0),
            deal_todo: LocalDealStatus::Active,
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

    // TODO this error ought to be a bit more descriptive ideally
    async fn handle_deal_id(
        &self,
        eth_provider: Arc<VitalikProvider>,
        deal_id: DealID,
        wakeup_block: BlockNum,
    ) -> Result<()> {
        let current_block_n = eth_provider.get_latest_block_num().await?;
        let mut deal_params = self
            .deal_tree
            .read()
            .await
            .get(&deal_id)?
            .ok_or_else(|| anyhow!("no deal params found for deal id {:?}", deal_id))?;

        // TODO handle cancellation situation
        // TODO handle deal finalization situation
        // figure out the start of the window we're currently in for this deal
        match window::get_the_current_window(&deal_params.onchain, current_block_n) {
            // case 1: we get the right window, we're in it, time to prove that window.
            Ok(proof_window_start) => {
                // get the block hash of the window start block
                let block_hash = eth_provider
                    .get_block_hash_from_num(proof_window_start)
                    .await?;
                let proof_to_post = gen_proof(
                    proof_window_start,
                    block_hash,
                    ipfs::get_handle_for_cid(deal_params.onchain.ipfs_file_cid).await?,
                    ipfs::get_handle_for_cid(deal_params.obao_cid).await?,
                    deal_params.onchain.file_size,
                )
                .await?;
                // TODO posting proof to chain may take a while, should probably wait until we're sure the transaction succeeded to update the database.
                // TODO implement retries somewhere.
                let submission_blocknum = eth_provider.post_proof(&deal_id, proof_to_post).await?;
                deal_params.last_submission = submission_blocknum;

                // was this our last proof? byeee if so... else figure out the next proof window.
                let new_block_num = window::get_the_next_window(&deal_params);
                if new_block_num == None {
                    deal_params.deal_todo = LocalDealStatus::WaitingToFinalize;
                    // TODO schedule calling finalization?
                };

                self.unschedule_and_reschedule_atomic(
                    deal_id,
                    Some(wakeup_block),
                    new_block_num,
                    Some(deal_params),
                )
                .await?;
            }
            Err(window::DealStatusError::Future) => {
                // TODO this is totally wrong
                info!("deal {:?} is scheduled for the future (this shouldn't happen?? something's wrong... the wakeup window was {:?}: {:?}", deal_id,  wakeup_block, deal_params);
                // this should be the case otherwise invariants are wrong!!
                assert_eq!(deal_params.deal_todo, LocalDealStatus::Active);
                // reschedule the deal into the future... this shouldn't ever happen, so log it.
                self.unschedule_and_reschedule_atomic(
                    deal_id,
                    Some(wakeup_block),
                    Some(deal_params.onchain.deal_start_block),
                    Some(deal_params),
                )
                .await?;
            }
            Err(window::DealStatusError::Past) => {
                // TODO this is totally wrong
                match deal_params.deal_todo {
                    LocalDealStatus::Active => {
                        deal_params.deal_todo = LocalDealStatus::WaitingToFinalize
                    }
                    LocalDealStatus::WaitingToFinalize | LocalDealStatus::Cancelled => {
                        warn!("deal {:?} was still in the scheduler with a status where it shouldn't have been. your assumed invariants are wrong. info: {:?}", deal_id, deal_params);
                    }
                    _ => unimplemented!("trainwreck codebase"),
                }
                // remove the deal from the scheduling database
                self.unschedule_and_reschedule_atomic(
                    deal_id,
                    Some(wakeup_block),
                    None,
                    Some(deal_params),
                )
                .await?;
            }
        };
        Ok(())
    }
}

// TODO: add hella timeouts to DB tasks
// TODO: holy shit clean this up please
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
