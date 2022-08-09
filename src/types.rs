use cid::Cid;
use ethers::prelude::Address;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sled::IVec;
use std::ops::Add;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct DealID(pub u64);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct BlockNum(pub u64);

impl Add for BlockNum {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        BlockNum(self.0 + other.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct TokenAmount(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token(pub Address);

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct OnChainDealInfo {
    pub deal_id: DealID,
    pub deal_start_block: BlockNum,
    pub deal_length_in_blocks: BlockNum,
    pub proof_frequency_in_blocks: BlockNum,
    pub price: TokenAmount,
    pub collateral: Amount,
    pub erc20_token_denomination: Token,
    #[serde(serialize_with = "serialize_cid", deserialize_with = "deserialize_cid")]
    pub ipfs_file_cid: Cid,
    pub file_size: u64,
    pub blake3_file_checksum: [u8; 32],
}

pub struct Proof {
    pub block_number: BlockNum,
    pub bao_proof_data: Vec<u8>,
}
