use anyhow::Result;
use bao::encode::SliceExtractor;
use cid::Cid;
use ethers::abi::ethereum_types::BigEndianHash;
use std::io::Read;
use crate::talk_to_ipfs::get_handles_for_file_and_obao;
use crate::talk_to_vitalik;
use crate::types::*;

// 1024 bytes per bao chunk
const CHUNK_SIZE: u64 = 1024;

// TODO: check this for correctness it's from copilot...
fn get_num_chunks(size: u64) -> u64 {
    (size as f32 / CHUNK_SIZE as f32).ceil() as u64
}

pub async fn gen_proof(block_number: BlockNum, file_to_prove: Cid, file_length: u64) -> Result<Proof> {
    let (source, obao) = get_handles_for_file_and_obao(file_to_prove).await?;
    let block_hash = talk_to_vitalik::get_block_hash_from_num(block_number).await?;

    let chunk_number = (block_hash.into_uint() % get_num_chunks(file_length)).as_u64();
    let chunk_offset = chunk_number * CHUNK_SIZE;
    let chunk_size = if chunk_number == get_num_chunks(file_length) - 1 {
        file_length - chunk_offset
    } else {
        CHUNK_SIZE
    };

    let mut bao_proof_data = vec![];
    let _ = SliceExtractor::new_outboard(
        source,
        obao,
        chunk_offset,
        chunk_size,
    )
    .read_to_end(&mut bao_proof_data)?;

    // TODO: should we check the proof locally at all...?
    Ok(Proof {
        block_number,
        bao_proof_data,
    })
}
