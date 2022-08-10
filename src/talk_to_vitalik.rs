use std::future::Future;
use crate::types::*;
use anyhow::{anyhow, Result};
use ethers::prelude::H256;
use tokio::time::{timeout, Duration};
use ethers::providers::{Http, Middleware, Provider};

pub struct VitalikProvider{
    provider: Provider<Http>,
    timeout: Duration,
}

impl VitalikProvider {

    pub fn new(url: String, timeout_seconds: u64) -> Result<Self> {
        Ok(Self{
            provider: Provider::<Http>::try_from(url)?,
            timeout: Duration::from_secs(timeout_seconds),
        })
    }

    pub async fn get_latest_block_num(&self) -> Result<BlockNum> {
        let block = timeout(self.timeout, self.provider.get_block_number()).await??;
        Ok(BlockNum(block.as_u64()))
    }

    pub async fn get_block_hash_from_num(&self, block_number: BlockNum) -> Result<H256> {
        let block = timeout(self.timeout, self
            .provider
            .get_block(block_number.0))
            .await??
            .ok_or_else(|| anyhow!("block not found"))?;
        block.hash.ok_or_else(|| anyhow!("block hash not found"))
    }

    pub async fn get_on_chain_deal_info(&self, _deal_id: DealID) -> Result<OnChainDealInfo> {
        unimplemented!("write me ;)")
    }

    pub async fn post_proof(&self, _deal_id: &DealID, _proof: Proof) -> Result<()> {
        unimplemented!("write me :)")
    }

    pub async fn accept_deal_on_chain(&self) -> Result<OnChainDealInfo> {
        unimplemented!("https://open.spotify.com/track/0oxYB9GoOIDrdzniNdKC44?si=71f88a0b1afa47a4")
    }
}
