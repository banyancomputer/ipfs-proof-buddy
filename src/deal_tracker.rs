// DataBase? more like DaBaby! https://www.youtube.com/watch?v=mxFstYSbBmc

use anyhow::Result;
use cid::Cid;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sled::IVec;

const SLED_FILE: &str = "deal_tracker.sled";
const DEAL_DB_IDENT: &str = "deal_db";
const SCHEDULE_DB_IDENT: &str = "schedule_db";

lazy_static::lazy_static! {
    static ref DB: ProofScheduleDb = {
        let db = sled::open(SLED_FILE).unwrap();
        ProofScheduleDb {
            deal_tree: db.open_tree(DEAL_DB_IDENT).unwrap(),
            schedule_tree: db.open_tree(SCHEDULE_DB_IDENT).unwrap(),
        }
    };
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct DealID(u64);

#[derive(Serialize, Deserialize)]
struct DealParams {
    on_chain_deal_info: OnChainDealInfo,
    next_proof_window_start_timestamp: u64,
    last_proof_submission_timestamp: u64,
    last_proof_submission_blocknumber: u64,
}

struct ProofScheduleDb {
    /// on_chain deal id --mapped_to--> DealParams
    deal_tree: sled::Tree,
    /// window --mapped_to--> on_chain deal id
    schedule_tree: sled::Tree,
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
    deal_start_timestamp: u64,
    deal_length: u64,
    proof_frequency: u64,
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
            next_proof_window_start_timestamp: on_chain_deal_info.deal_start_timestamp,
            last_proof_submission_timestamp: 0,
            last_proof_submission_blocknumber: 0,
        };
        let deal_params_bytes = serde_json::to_string(&deal_params)?;
        self.deal_tree
            .insert(deal_id.0.to_le_bytes(), deal_params_bytes.as_str())?;

        // TODO: put into scheduler!
        self.schedule_tree.insert(
            deal_params.next_proof_window_start_timestamp.to_le_bytes(),
            deal_id,
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
