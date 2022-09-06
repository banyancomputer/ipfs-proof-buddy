use crate::database_types::{DealTodo, LocalDealInfo};
use anyhow::Result;
use banyan_shared::{eth::VitalikProvider, ipfs, proofs::gen_proof, proofs::window, types::*};
use std::sync::Arc;

impl LocalDealInfo {
    // TODO check this logic
    pub(crate) async fn submit_proof(
        &self,
        deal_id: DealID,
        eth_provider: Arc<VitalikProvider>,
        on_chain_current_block: BlockNum,
    ) -> Result<(Option<BlockNum>, LocalDealInfo)> {
        let mut deal_info = self.clone();

        match window::get_the_current_window(&deal_info.on_chain, on_chain_current_block) {
            // case 1: we get the right window, we're in it, time to prove that window.
            Ok(proof_window_start) => {
                // TODO handle case where this window's proof was already submitted
                // TODO handle case where there is already a pending transaction to submit this proof
                // TODO handle case where the deal was actually cancelled

                // get the block hash of the window start block
                let block_hash = eth_provider
                    .get_block_hash_from_num(proof_window_start)
                    .await?;
                let proof_to_post = gen_proof(
                    proof_window_start,
                    block_hash,
                    ipfs::get_handle_for_cid(deal_info.on_chain.ipfs_file_cid).await?,
                    ipfs::get_handle_for_cid(deal_info.obao_cid).await?,
                    deal_info.on_chain.file_size,
                )
                .await?;

                let submission_blocknum = eth_provider.post_proof(&deal_id, proof_to_post).await?;
                deal_info.last_successful_proof_submission = Some(submission_blocknum);

                if let Some(next_window) =
                    window::get_the_next_window(&deal_info.on_chain, on_chain_current_block)
                {
                    deal_info.deal_todo = DealTodo::SubmitProof;
                    Ok((Some(next_window), deal_info))
                } else {
                    deal_info.deal_todo = DealTodo::InitiateChainlinkFinalization;
                    Ok((Some(deal_info.on_chain.get_final_block()), deal_info))
                }
            }
            Err(window::DealStatusError::Future) => {
                error!("deal woke up before the deal started. this should never ever happen.");
                panic!("how did we wake up before the deal started!! this is definitely a bug");
            }
            Err(window::DealStatusError::Past) => {
                // woke up with a proof as the dealtodo, but after the deal ended!
                // schedule it back as "waitingtofinalize". we missed our chance.
                // TODO change this to just get the finalization done rn instead of scheduling it.
                deal_info.deal_todo = DealTodo::InitiateChainlinkFinalization;
                // and wake back up ASAP :)
                let wakeup_block = BlockNum(0);
                Ok((Some(wakeup_block), deal_info))
            }
        }
    }

    pub(crate) async fn submit_finalization(
        &self,
        _deal_id: DealID,
        _eth_provider: Arc<VitalikProvider>,
        _on_chain_current_block: BlockNum,
    ) -> Result<(Option<BlockNum>, LocalDealInfo)> {
        // TODO check if already finalized
        // TODO submit finalization
        unimplemented!();
    }
}
