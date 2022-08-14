use crate::types::{BlockNum, LocalDealInfo, OnChainDealInfo};

#[derive(Debug)]
pub enum DealStatusError {
    Future,
    Past,
}

// TODO: check this it might not be correct. write some tests. from copilot. suspect this is wrong.
pub fn get_the_right_window(
    deal_info: &OnChainDealInfo,
    current_block_num: BlockNum,
) -> Result<BlockNum, DealStatusError> {
    if current_block_num < deal_info.deal_start_block {
        return Err(DealStatusError::Future);
    };
    if current_block_num >= deal_info.deal_start_block + deal_info.deal_length_in_blocks {
        return Err(DealStatusError::Past);
    };
    // this should return the window start of the current block
    let window_number = (current_block_num - deal_info.deal_start_block)
        .0
        .div_euclid(deal_info.proof_frequency_in_blocks.0);
    Ok(deal_info.deal_start_block + deal_info.proof_frequency_in_blocks * window_number)
}

// TODO: check this it might not be correct. write some tests. from copilot. suspect this is wrong.
pub fn completed_last_proof(deal_info: &LocalDealInfo) -> bool {
    deal_info
        .onchain
        .deal_length_in_blocks
        .0
        .div_euclid(deal_info.onchain.proof_frequency_in_blocks.0)
        .eq(
            &(deal_info.last_submission - deal_info.onchain.deal_start_block)
                .0
                .div_euclid(deal_info.onchain.proof_frequency_in_blocks.0),
        )
}

// TODO: check this it might not be correct. write some tests. from copilot. suspect this is wrong.
pub fn get_the_next_window(deal_info: &LocalDealInfo) -> BlockNum {
    let window_number = (deal_info.last_submission - deal_info.onchain.deal_start_block)
        .0
        .div_euclid(deal_info.onchain.proof_frequency_in_blocks.0);
    deal_info.onchain.deal_start_block
        + deal_info.onchain.proof_frequency_in_blocks * (window_number + 1)
}
