use std::fs::File;
use std::io::Read;
use anyhow::{anyhow, Result};
use bao::encode::SliceExtractor;
use ethers::{
    core::types::{BlockNumber, H256, U256},
    providers::{Http, Middleware, Provider},
};
use cid::Cid;
use ethers::abi::ethereum_types::BigEndianHash;

// 1024 bytes per bao chunk
const CHUNK_SIZE: u64 = 1024;

struct IPFSFileReader {
    cid: Cid,
    size: u64,
    source: File,
    obao: File
}

struct Proof {
    // TODO: may not need block hash, probably will just get that locally on the verifier.
    block_number: BlockNumber,
    block_hash: H256,
    bao_proof_data: Vec<u8>,
}

async fn get_block_hash_from_num(block_number: BlockNumber) -> Result<H256> {
    let provider = Provider::<Http>::try_from("https://mainnet.infura.io/v3/YOUR_API_KEY")?;

    let block = provider
        .get_block(block_number)
        .await?
        .ok_or_else(|| anyhow!("block not found"))?;
    block.hash.ok_or_else(|| anyhow!("block hash not found"))
}

// TODO: check this for correctness it's from copilot...
fn get_num_chunks(size: u64) -> u64 {
    (size as f32 / CHUNK_SIZE as f32).ceil() as u64
}

async fn gen_proof(block_number: BlockNumber, file_to_prove: IPFSFileReader) -> Result<Proof> {
    let block_hash = get_block_hash_from_num(block_number).await?;

    let chunk_number = (block_hash.into_uint() % get_num_chunks(file_to_prove.size)).as_u64();
    let chunk_offset = chunk_number * CHUNK_SIZE;
    let chunk_size = if chunk_number == get_num_chunks(file_to_prove.size) - 1 {
        file_to_prove.size - chunk_offset
    } else {
        CHUNK_SIZE
    };

    let mut bao_proof_data = vec![];
    let _ = SliceExtractor::new_outboard(file_to_prove.source, file_to_prove.obao, chunk_offset, chunk_size).read_to_end(&mut bao_proof_data)?;

    // TODO: should we check the proof locally at all...?
    Ok(Proof { block_number, block_hash, bao_proof_data })
}
