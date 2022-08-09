use crate::proof_utils;
use anyhow::Result;
use cid::Cid;
use std::fs::File;
use std::io::BufReader;

pub async fn get_handle_for_cid(_cid: Cid) -> Result<BufReader<File>> {
    unimplemented!("https://open.spotify.com/track/2enPRFda84VE2wtI8c86Uf?si=714947276bc3400b")
}

pub async fn _do_we_have_this_cid_locally(_cid: Cid) -> Result<bool> {
    unimplemented!("https://open.spotify.com/track/4vjvx7Zxkb4AltGcZ0BBvI?si=3c7928800a1f4f3b")
}

pub async fn download_file_from_ipfs(_cid: Cid, _length: u64) -> Result<()> {
    unimplemented!("https://open.spotify.com/track/0oxYB9GoOIDrdzniNdKC44?si=71f88a0b1afa47a4")
}

/// returns Ok(Cid) of the obao if things succeeded, Error if not
pub async fn validate_file_gen_obao(cid: Cid, blake3_hash: bao::Hash) -> Result<Cid> {
    let handle = get_handle_for_cid(cid).await?;
    let (obao_cid, obao_digest) = proof_utils::gen_obao::<BufReader<File>>(handle).await?;
    if obao_digest != blake3_hash {
        Err(anyhow::anyhow!("obao does not match blake3 hash"))?
    };
    Ok(obao_cid)
}
