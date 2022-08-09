use anyhow::Result;
use cid::Cid;
use std::fs::File;
use std::io::BufReader;

pub async fn get_handle_for_cid(cid: Cid) -> Result<BufReader<File>> {
    unimplemented!("https://open.spotify.com/track/2enPRFda84VE2wtI8c86Uf?si=714947276bc3400b")
}

pub async fn do_we_have_this_cid_locally(_cid: Cid) -> Result<bool> {
    unimplemented!("https://open.spotify.com/track/4vjvx7Zxkb4AltGcZ0BBvI?si=3c7928800a1f4f3b")
}

pub async fn download_file_from_ipfs(_cid: Cid, _length: u64) -> Result<()> {
    unimplemented!("https://open.spotify.com/track/0oxYB9GoOIDrdzniNdKC44?si=71f88a0b1afa47a4")
}

/// returns Ok(Cid) of the obao if things succeeded, Error if not
pub async fn validate_file_and_gen_obao(_cid: Cid, _blake3_hash: [u8; 32]) -> Result<Cid> {
    unimplemented!("https://open.spotify.com/track/0oxYB9GoOIDrdzniNdKC44?si=71f88a0b1afa47a4")
}
