use ethers::providers::{Middleware, Provider, Http};
use crate::types::*;
use anyhow::{anyhow, Result};
use ethers::prelude::H256;

lazy_static::lazy_static! {
    static ref PROVIDER: Provider<Http> = Provider::<Http>::try_from("https://mainnet.infura.io/v3/YOUR_API_KEY").unwrap();
}

pub async fn get_latest_block_num() -> Result<BlockNum> {
    let block = PROVIDER.get_block_number().await?;
    Ok(BlockNum(block.as_u64()))
}

pub async fn get_block_hash_from_num(block_number: BlockNum) -> Result<H256> {
    let block = PROVIDER.get_block(block_number.0).await?.ok_or_else(|| anyhow!("block not found"))?;
    block.hash.ok_or_else(|| anyhow!("block hash not found"))
}

pub async fn get_on_chain_deal_info(_deal_id: DealID) -> Result<OnChainDealInfo> {
    unimplemented!("write me ;)")
}

pub async fn post_proof(_deal_id: &DealID, _proof: Proof) -> Result<()> {
    unimplemented!("write me :)")
}
