use bcrypt::{hash, verify};
use anyhow::{Context, Result};

pub fn get_hash_with_cost(bd: &str, cost: u32) -> Result<String> {
    if bd.is_empty() {
        anyhow::bail!("BD is empty.");
    }
    hash(bd, cost).context("Failed to generate hash.")
}

/// ハッシュ検証関数
/// 入力された平文 `bd` と `hashed` が一致するか検証する
pub fn verify_hash(bd: &str, hashed: &str) -> Result<bool> {
    if bd.is_empty() || hashed.is_empty() {
        return Ok(false);
    }
    // verify は平文とハッシュを受け取り、Result<bool, BcryptError> を返す
    verify(bd, hashed).context("Failed to verify hash.")
}

pub fn encrypt(plain_text: &str, key: &str) -> Result<String> {
    use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit, aead::Aead};
    use aes_gcm::aead::OsRng;
    use aes_gcm::aead::rand_core::RngCore;

    let key = Key::<Aes256Gcm>::from_slice(key.as_bytes());
    let cipher = Aes256Gcm::new(key);
    
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher.encrypt(nonce, plain_text.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
    
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    
    Ok(hex::encode(result))
}

pub fn decrypt(encrypted_hex: &str, key: &str) -> Result<String> {
    use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit, aead::Aead};

    let data = hex::decode(encrypted_hex).context("Failed to decode hex.")?;
    if data.len() < 12 {
        anyhow::bail!("Invalid encrypted data length.");
    }
    
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let key = Key::<Aes256Gcm>::from_slice(key.as_bytes());
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);
    
    let plaintext_bytes = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;
    
    String::from_utf8(plaintext_bytes).context("Failed to convert decrypted data to string.")
}