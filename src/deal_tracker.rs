// DataBase? more like DaBaby! https://www.youtube.com/watch?v=mxFstYSbBmc

use anyhow::{anyhow, Result};
use cid::Cid;
use ethers::providers::{Middleware, Provider, Http};
use ethers::types::RewardType::Block;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sled::IVec;
use proof_gen::gen_proof;

// TODO: ensure this is safe if it falls over in the middle of a transaction. you've done half a job...

const SLED_FILE: &str = "deal_tracker.sled";
const DEAL_DB_IDENT: &str = "deal_db";
const SCHEDULE_DB_IDENT: &str = "schedule_db";
const CURRENT_THROUGH_BLOCK_N_KEY: &str = "current_through_block_n";

lazy_static::lazy_static! {
    static ref DB: ProofScheduleDb = {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
struct DealID(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
struct BlockNum(u64);

#[derive(Serialize, Deserialize)]
struct DealParams {
    on_chain_deal_info: OnChainDealInfo,
    next_proof_window_start_blocknumber: BlockNum,
    last_proof_submission_blocknumber: BlockNum,
}

struct ProofScheduleDb {
    db: sled::Db,
    /// on_chain deal id --mapped_to--> DealParams
    deal_tree: typed_sled::Tree<DealID, DealParams>,
    /// window --mapped_to--> on_chain deal id
    schedule_tree: typed_sled::Tree<BlockNum, Vec<DealID>>,
}

fn serialize_cid<S: Serializer>(cid: &Cid, s: S) -> Result<S::Ok, S::Error> {
    let cid_bytes = cid.to_bytes();
    s.serialize_bytes(&cid_bytes)
}

// fn<'de, D>(D) -> Result<T, D::Error> where D: Deserializer<'de>
fn deserialize_cid<'de, D>(deserializer: D) -> Result<Cid, D::Error>
where
    D: Deserializer<'de>,
{
    let cid_bytes = <&[u8]>::deserialize(deserializer)?;
    Cid::read_bytes(cid_bytes).map_err(|e| Error::custom(e.to_string()))
}

#[derive(Serialize, Deserialize, Clone, Copy)]
struct OnChainDealInfo {
    deal_start_block: BlockNum,
    deal_length_in_blocks: BlockNum,
    proof_frequency_in_blocks: BlockNum,
    #[serde(serialize_with = "serialize_cid", deserialize_with = "deserialize_cid")]
    ipfs_file_cid: Cid,
    ipfs_file_size: u64,
}

fn get_on_chain_deal_info(_deal_id: DealID, _cid: Cid) -> Result<OnChainDealInfo> {
    unimplemented!("write meee")
}

impl Into<IVec> for DealID {
    fn into(self) -> IVec {
        IVec::from(&self.0.to_le_bytes())
    }
}

impl From<IVec> for DealID {
    fn from(iv: IVec) -> Self {
        let bytes = iv.as_ref();
        let mut deal_id_bytes = [0u8; 8];
        deal_id_bytes.copy_from_slice(&bytes[..8]);
        DealID(u64::from_le_bytes(deal_id_bytes))
    }
}

impl ProofScheduleDb {
    fn schedule(&self, blocknum: BlockNum, deal_id: DealID) -> Result<()> {
        let _ = self.schedule_tree.fetch_and_update(
            &blocknum,
            |maybe_deal_ids| match maybe_deal_ids {
                Some(deal_ids) => {
                    let mut deal_ids = deal_ids.clone();
                    deal_ids.push(deal_id);
                    Some(deal_ids)
                }
                None => Some(vec![deal_id]),
            },
        )?;
        Ok(())
    }
    // TODO: claudia, you are a trainwreck, and so is this code. use me: https://github.com/spacejam/sled/blob/main/examples/structured.rs
    // relate the on-chain ID to the DealParams struct.
    fn add_a_deal_to_db(&self, deal_id: DealID, cid: Cid) -> Result<()> {
        // TODO: check whether we have this CID locally in the IPFS cluster?
        // TODO: check that we've accepted it on-chain, check the parameters?

        // add to deals database with correct information.
        let on_chain_deal_info = get_on_chain_deal_info(deal_id, cid)?;
        let deal_params = DealParams {
            on_chain_deal_info,
            // TODO: figure out what if we only hear about the deal really late?
            next_proof_window_start_blocknumber: on_chain_deal_info.deal_start_block,
            last_proof_submission_blocknumber: BlockNum(0),
        };
        self.deal_tree
            .insert(&deal_id, &deal_params)?;

        // TODO: put into scheduler!
        self.schedule(on_chain_deal_info.deal_start_block, deal_id)?;
        Ok(())
    }

    // TODO: make DB stuff atomic i think
    async fn wake_up(&self) -> Result<()> {
        // what's the last block that we proved our deals up through?
        let current_through_block_n = match self.db.get(CURRENT_THROUGH_BLOCK_N_KEY)? {
            Some(current_through_block_n_vec) => {
                let mut current_through_block_n_bytes = [0u8; 8];
                current_through_block_n_bytes.copy_from_slice(&*current_through_block_n_vec);
                BlockNum(u64::from_le_bytes(current_through_block_n_bytes))
            },
            None => BlockNum(0),
        };

        // TODO: lazystatic this provider
        // construct an ethers provider
        let provider = Provider::<Http>::try_from(
            "https://mainnet.infura.io/v3/idk hee hee",
        )?;
        // call get_block_number() on the provider to get the current block number.
        let current_block_n = BlockNum(provider.get_block_number().await?.as_u64());

        // TODO: do the proofs, submit them, and move them around in the scheduler as needed.
        for block_and_deals in self.schedule_tree.range(current_through_block_n..current_block_n) {
            let (block, deal_ids) = block_and_deals?;
            for deal_id in deal_ids.iter() {
                let deal_params = self.deal_tree.get(&deal_id)?.ok_or(anyhow!("no deal params found for deal id {:?}", deal_id))?;
                let proof_to_post = gen_proof(block, deal_params.on_chain_deal_info.ipfs_file_cid, deal_params.on_chain_deal_info.ipfs_file_size)?;
                post_proof(deal_id, proof_to_post)?;
            }
            self.schedule_tree.remove(&block)?;
        }

        // update the last block we proved up to.
        let _ = self.db.insert(
            CURRENT_THROUGH_BLOCK_N_KEY,
            &(current_block_n.0.to_le_bytes())
        )?;
        Ok(())
    }
}

// // insert and get, similar to std's BTreeMap
// let old_value = tree.insert("key", "value")?;
//
// assert_eq!(
//     tree.get(&"key")?,
//     Some(sled::IVec::from("value")),
// );
//
// // range queries
// for kv_result in tree.range("key_1".."key_9") {}
//
// // deletion
// let old_value = tree.remove(&"key")?;
//
// // atomic compare and swap
// tree.compare_and_swap(
// "key",
// Some("current_value"),
// Some("new_value"),
// )?;
//
// // block until all operations are stable on disk
// // (flush_async also available to get a Future)
// tree.flush()?;
