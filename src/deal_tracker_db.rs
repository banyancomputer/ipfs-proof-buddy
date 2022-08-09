// DataBase? more like DaBaby! https://www.youtube.com/watch?v=mxFstYSbBmc
use crate::proof_utils::gen_proof;
use crate::talk_to_ipfs::do_we_have_this_cid_locally;
use crate::{talk_to_ipfs, talk_to_vitalik};
use crate::types::*;
use anyhow::{anyhow, Result};
use cid::Cid;
use ethers::providers::{Http, Middleware, Provider};
use serde::{Deserialize, Serialize};

// TODO: ensure this is safe if it falls over in the middle of a transaction. you've done half a job...
const SLED_FILE: &str = "deal_tracker.sled";
const DEAL_DB_IDENT: &str = "deal_db";
const SCHEDULE_DB_IDENT: &str = "schedule_db";

#[derive(Serialize, Deserialize)]
struct DealParams {
    on_chain_deal_info: OnChainDealInfo,
    obao_cid: Cid,
    next_proof_window_start_block_num: BlockNum,
    last_proof_submission_block_num: BlockNum,
}

pub struct ProofScheduleDb {
    db: sled::Db,
    /// on_chain deal id --mapped_to--> DealParams
    deal_tree: typed_sled::Tree<DealID, DealParams>,
    /// window --mapped_to--> on_chain deal id
    schedule_tree: typed_sled::Tree<BlockNum, Vec<DealID>>,
}

lazy_static::lazy_static! {
    pub static ref DB: ProofScheduleDb = {
        let db = sled::open(SLED_FILE).unwrap();
        let deal_tree = typed_sled::Tree::open(&db, DEAL_DB_IDENT);
        let schedule_tree = typed_sled::Tree::open(&db, SCHEDULE_DB_IDENT);
        ProofScheduleDb {
            db,
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
        self.deal_tree.insert(&deal_params.on_chain_deal_info.deal_id, &deal_params)?;

        // put into scheduler!
        // TODO: this is wrong!!!
        self.schedule(deal_params.on_chain_deal_info.deal_start_block, deal_id)
    }

    // TODO: make DB stuff atomic i think
    pub(crate) async fn wake_up(&self) -> Result<()> {
        // TODO: lazy static this provider
        // construct an ethers provider
        let provider = Provider::<Http>::try_from("https://mainnet.infura.io/v3/idk hee hee")?;
        // call get_block_number() on the provider to get the current block number.
        let current_block_n = BlockNum(provider.get_block_number().await?.as_u64());

        // TODO: do the proofs, submit them, and move them around in the scheduler as needed.
        for block_and_deals in self
            .schedule_tree
            .range(0..current_block_n)
        {
            let (block, deal_ids) = block_and_deals?;
            let block_hash = talk_to_vitalik::get_block_hash(&block).await?;
            for deal_id in deal_ids.iter() {
                // TODO use sled compare_and_swap to atomically update the deal_params.
                let deal_params = self
                    .deal_tree
                    .get(deal_id)?
                    .ok_or_else(|| anyhow!("no deal params found for deal id {:?}", deal_id))?;
                let proof_to_post = gen_proof(
                    block,
                    block_hash,
                    talk_to_ipfs::get_handle_for_cid(deal_params.on_chain_deal_info.ipfs_file_cid).await?,
                    talk_to_ipfs::get_handle_for_cid(deal_params.obao_cid).await?,
                    deal_params.on_chain_deal_info.file_size,
                )
                .await?;
                talk_to_vitalik::post_proof(deal_id, proof_to_post).await?;
            }
            self.schedule_tree.remove(&block)?;
        }

        Ok(())
    }
}
