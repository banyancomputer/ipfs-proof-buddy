use banyan_shared::types;
use banyan_shared::types::BlockNum;
use serde::{Deserialize, Serialize};

/// what's the next job we need to get done for the deal?
/// the options
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DealTodo {
    SubmitProof,
    Cancel,
    InitiateChainlinkFinalization,
    CheckChainlinkFinalization,
    WithdrawEarnings,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LocalDealInfo {
    pub on_chain: types::OnChainDealInfo,
    #[serde(
        serialize_with = "types::serialize_cid",
        deserialize_with = "types::deserialize_cid"
    )]
    pub obao_cid: cid::Cid,
    pub last_successful_proof_submission: Option<BlockNum>,
    pub deal_todo: DealTodo,
}
