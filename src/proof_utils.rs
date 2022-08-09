use crate::types::*;
use anyhow::Result;
use bao::encode::SliceExtractor;
use cid::Cid;
use ethers::abi::ethereum_types::BigEndianHash;
use ethers::prelude::H256;
use std::io::{Read, Seek};

// TODO move this info a config file maybe?
/// 1024 bytes per bao chunk
const CHUNK_SIZE: u64 = 1024;

// TODO: check this for correctness it's from copilot...
fn get_num_chunks(size: u64) -> u64 {
    (size as f32 / CHUNK_SIZE as f32).ceil() as u64
}

pub async fn compute_blake3_digest<R: Read>(mut reader: R) -> Result<[u8; 32]> {
    let mut buf = [0u8; CHUNK_SIZE as usize];
    let mut hasher = blake3::Hasher::new();
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(*hasher.finalize().as_bytes())
}

/// returns tuple (chunk_offset, chunk_size) for the Nth bao hash that you need to grab :)
fn compute_random_block_choice_from_hash(block_hash: H256, file_length: u64) -> (u64, u64) {
    let chunk_number = (block_hash.into_uint() % get_num_chunks(file_length)).as_u64();
    let chunk_offset = chunk_number * CHUNK_SIZE;
    let chunk_size = if chunk_number == get_num_chunks(file_length) - 1 {
        file_length - chunk_offset
    } else {
        CHUNK_SIZE
    };
    (chunk_offset, chunk_size)
}

/// returns the cid where the obao is stored, as well as the root hash of the obao.
pub async fn gen_obao<R: Read + Seek>(_reader: R) -> Result<(Cid, bao::Hash)> {
    unimplemented!("need to wire things up to ipfs first");
}

pub async fn gen_proof<R: Read + Seek>(
    block_number: BlockNum,
    block_hash: H256,
    file_handle: R,
    obao_handle: R,
    file_length: u64,
) -> Result<Proof> {
    let (chunk_offset, chunk_size) = compute_random_block_choice_from_hash(block_hash, file_length);

    let mut bao_proof_data = vec![];
    let _ = SliceExtractor::new_outboard(file_handle, obao_handle, chunk_offset, chunk_size)
        .read_to_end(&mut bao_proof_data)?;

    // TODO: should we check the proof locally at all...?
    Ok(Proof {
        block_number,
        bao_proof_data,
    })
}
