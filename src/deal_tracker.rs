// DataBase? more like DaBaby! https://www.youtube.com/watch?v=mxFstYSbBmc

// TODO: claudia, you are a trainwreck, and so is this code. use me: https://github.com/spacejam/sled/blob/main/examples/structured.rs
use anyhow::Result;
use cid::Cid;

const DEAL_DB_IDENT: &str = "deal_db";
const SCHEDULE_DB_IDENT: &str = "schedule_db";

#[derive(Serialize, Deserialize)]
struct DealParams {
    ipfs_file_cid: Cid,
    ipfs_file_size: u64,
    deal_start_timestamp: u64,
    deal_length: u64,
    proof_frequency: u64,
    next_proof_window_start_timestamp: u64,
    last_proof_submission_timestamp: u64,
    last_proof_submission_blocknumber: u64,
}

struct ProofScheduleDb {
    /// on_chain deal id --mapped_to--> DealParams
    deal_db: sled::Tree,
    /// window --mapped_to--> on_chain deal id
    schedule_db: sled::Tree,
}

impl ProofScheduleDb {
    // relate the on-chain ID to the DealParams struct.
    fn init_db(sled_file_loc: String) -> Result<&Self> {
        let db = sled::open(sled_file_loc)?;
        Ok(&Self {
            deal_db: db.insert(DEAL_DB_IDENT)?,
            schedule_db: db.insert(SCHEDULE_DB_IDENT)?,
        })
    }
    fn add_a_deal_to_db(&self, deal_id: u64, cid: Cid) -> Result<()> {
        // TODO: check whether we have this CID locally!
        // TODO: check that we've accepted it on-chain, check the parameters?
        // TODO: put into scheduler!
        unimplemented!("yeehaw cowboi")
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
