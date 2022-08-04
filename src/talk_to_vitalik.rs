use crate::types::*;
use anyhow::{anyhow, Result};
use ethers::prelude::H256;
use ethers::providers::{Http, Middleware, Provider};

lazy_static::lazy_static! {
    static ref PROVIDER: Provider<Http> = Provider::<Http>::try_from("https://mainnet.infura.io/v3/YOUR_API_KEY").unwrap();
}

pub async fn get_latest_block_num() -> Result<BlockNum> {
    let block = PROVIDER.get_block_number().await?;
    Ok(BlockNum(block.as_u64()))
}

pub async fn get_block_hash_from_num(block_number: BlockNum) -> Result<H256> {
    let block = PROVIDER
        .get_block(block_number.0)
        .await?
        .ok_or_else(|| anyhow!("block not found"))?;
    block.hash.ok_or_else(|| anyhow!("block hash not found"))
}

pub async fn get_on_chain_deal_info(_deal_id: DealID) -> Result<OnChainDealInfo> {
    unimplemented!("write me ;)")
}

pub async fn post_proof(_deal_id: &DealID, _proof: Proof) -> Result<()> {
    unimplemented!("write me :)")
}

// TODO perhaps this is not the correct place for this code to go...
pub async fn check_incoming_deal_params(deal_id: DealID) -> Result<()> {
    let on_chain_deal_info = get_on_chain_deal_info(deal_id).await?;
    if on_chain_deal_info.deal_start_block + on_chain_deal_info.deal_length_in_blocks
        > get_latest_block_num().await?
    {
        return Err(anyhow!("deal ended"));
    }

    // TODO: you will need to check more than this... but this is a start. check on-chain state as you keep going
    unimplemented!("check the rest of the things you need to check for incoming deal parameters!")
}

pub async fn accept_deal_on_chain() -> Result<DealID> {
    unimplemented!("https://open.spotify.com/track/0oxYB9GoOIDrdzniNdKC44?si=71f88a0b1afa47a4")
}
