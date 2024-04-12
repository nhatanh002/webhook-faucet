use data_encoding::BASE64;
use ring::hmac;
pub fn hmac_256_verify(key_bytes: &[u8], payload: &[u8], digest: &str) -> anyhow::Result<()> {
    let key = hmac::Key::new(hmac::HMAC_SHA256, key_bytes);
    let digest_decoded = BASE64.decode(digest.as_bytes())?;
    hmac::verify(&key, payload, digest_decoded.as_slice()).map_err(Into::into)
}
