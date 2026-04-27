use aes_gcm::aead::{rand_core::RngCore as RngCore_v06, Aead, KeyInit, Nonce, OsRng};
use aes_gcm::{Aes256Gcm, Key};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use bcrypt::{hash, verify};
use rand::{Rng, RngCore};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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
    let bytes = encrypt_bytes(plain_text.as_bytes(), key.as_bytes())?;
    Ok(hex::encode(bytes))
}

pub fn decrypt(encrypted_hex: &str, key: &str) -> Result<String> {
    let bytes = decrypt_bytes(&hex::decode(encrypted_hex)?, key.as_bytes())?;
    String::from_utf8(bytes).context("Failed to convert decrypted data to string.")
}

/// バイナリデータの暗号化 (AES-256-GCM)
/// Returns: Nonce(12) + Ciphertext
pub fn encrypt_bytes(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    RngCore_v06::fill_bytes(&mut OsRng, &mut nonce_bytes);
    let nonce = Nonce::<Aes256Gcm>::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// バイナリデータの復号 (AES-256-GCM)
/// Input: Nonce(12) + Ciphertext
pub fn decrypt_bytes(encrypted_data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    if encrypted_data.len() < 12 {
        anyhow::bail!("Invalid encrypted data length.");
    }

    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::<Aes256Gcm>::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))
}

// ============================================================
// Ed448 Utils
// ============================================================
use crate::constants::{ED448_KEY_BYTES_LEN, ED448_SIGNATURE_BYTES_LEN};
use ed448_goldilocks::curve::ExtendedPoint;
use ed448_goldilocks::Scalar; // Trying this path
use sha3::{
    digest::{ExtendableOutput, Update, XofReader},
    Shake256,
};

/// Ed448 公開鍵・秘密鍵のペア
pub struct Ed448KeyValuePair {
    pub secret: [u8; ED448_KEY_BYTES_LEN],
    pub public: [u8; ED448_KEY_BYTES_LEN],
}

/// Ed448 署名
#[derive(Debug, Clone, Copy)]
pub struct Ed448Signature {
    pub signature: [u8; ED448_SIGNATURE_BYTES_LEN],
}

impl Default for Ed448Signature {
    fn default() -> Self {
        Self {
            signature: [0u8; ED448_SIGNATURE_BYTES_LEN],
        }
    }
}

const DOM4_PREFIX: &[u8] = b"SigEd448\x00\x00";

impl Ed448KeyValuePair {
    /// 新しいキーペアを生成する
    pub fn generate() -> Result<Self> {
        // Updated to use rand::rng() as thread_rng is deprecated in newer versions if applicable,
        // but if rand is 0.8, thread_rng is fine.
        // Cargo.toml says rand = "0.9.2" -> usually rand::rng().
        let mut rng = rand::rng();
        let mut secret = [0u8; ED448_KEY_BYTES_LEN];
        rng.fill(&mut secret);

        let mut hasher = Shake256::default();
        hasher.update(&secret);
        let mut output = [0u8; ED448_SIGNATURE_BYTES_LEN];
        let mut reader = hasher.finalize_xof();
        XofReader::read(&mut reader, &mut output);

        // Pruning (RFC 8032)
        let mut s_bytes = [0u8; ED448_KEY_BYTES_LEN];
        s_bytes.copy_from_slice(&output[0..ED448_KEY_BYTES_LEN]);
        s_bytes[0] &= 0xfc;
        s_bytes[55] |= 0x80;
        s_bytes[56] = 0;

        let mut scalar_bytes_56 = [0u8; 56];
        scalar_bytes_56.copy_from_slice(&s_bytes[0..56]);

        let s = Scalar::from_bytes(scalar_bytes_56);
        let public_point = ExtendedPoint::generator() * s;
        let public_compressed = public_point.compress();

        Ok(Self {
            secret,
            public: public_compressed.0,
        })
    }

    /// 秘密鍵からキーペア（公開鍵）を復元する
    pub fn from_secret(secret: [u8; ED448_KEY_BYTES_LEN]) -> Self {
        let mut hasher = Shake256::default();
        hasher.update(&secret);
        let mut output = [0u8; ED448_SIGNATURE_BYTES_LEN];
        let mut reader = hasher.finalize_xof();
        XofReader::read(&mut reader, &mut output);

        // Pruning (RFC 8032)
        let mut s_bytes = [0u8; ED448_KEY_BYTES_LEN];
        s_bytes.copy_from_slice(&output[0..ED448_KEY_BYTES_LEN]);
        s_bytes[0] &= 0xfc;
        s_bytes[55] |= 0x80;
        s_bytes[56] = 0;

        let mut scalar_bytes_56 = [0u8; 56];
        scalar_bytes_56.copy_from_slice(&s_bytes[0..56]);

        let s = Scalar::from_bytes(scalar_bytes_56);
        let public_point = ExtendedPoint::generator() * s;
        let public_compressed = public_point.compress();

        Self {
            secret,
            public: public_compressed.0,
        }
    }

    /// メッセージに署名する
    pub fn sign(&self, message: &[u8]) -> Result<Ed448Signature> {
        // 1. Hash secret to get s (scalar) and prefix
        let mut hasher = Shake256::default();
        hasher.update(&self.secret);
        let mut output = [0u8; ED448_SIGNATURE_BYTES_LEN];
        let mut reader = hasher.finalize_xof();
        XofReader::read(&mut reader, &mut output);

        let mut s_bytes = [0u8; ED448_KEY_BYTES_LEN];
        s_bytes.copy_from_slice(&output[0..ED448_KEY_BYTES_LEN]);
        s_bytes[0] &= 0xfc;
        s_bytes[55] |= 0x80;
        s_bytes[56] = 0;

        let mut scalar_bytes_56 = [0u8; 56];
        scalar_bytes_56.copy_from_slice(&s_bytes[0..56]);
        let s = Scalar::from_bytes(scalar_bytes_56);

        let prefix = &output[ED448_KEY_BYTES_LEN..ED448_SIGNATURE_BYTES_LEN];

        // 2. Derive r
        let mut hasher_r = Shake256::default();
        hasher_r.update(DOM4_PREFIX);
        hasher_r.update(prefix);
        hasher_r.update(message);
        let mut r_out = [0u8; ED448_SIGNATURE_BYTES_LEN];
        let mut reader_r = hasher_r.finalize_xof();
        XofReader::read(&mut reader_r, &mut r_out);

        let r = Scalar::from_bytes_mod_order_wide(&r_out);

        // 3. R = r * B
        let r_point = ExtendedPoint::generator() * r;
        let r_comp = r_point.compress();

        // 4. k = Shake256(dom4 || R || A || M)
        let mut hasher_k = Shake256::default();
        hasher_k.update(DOM4_PREFIX);
        hasher_k.update(&r_comp.0);
        hasher_k.update(&self.public);
        hasher_k.update(message);
        let mut k_out = [0u8; ED448_SIGNATURE_BYTES_LEN];
        let mut reader_k = hasher_k.finalize_xof();
        XofReader::read(&mut reader_k, &mut k_out);

        let k = Scalar::from_bytes_mod_order_wide(&k_out);

        // 5. S = (r + k * s)
        let s_final = r + k * s;

        // Encode Signature: R (ED448_KEY_BYTES_LEN) || S (ED448_KEY_BYTES_LEN)
        let mut sig_bytes = [0u8; ED448_SIGNATURE_BYTES_LEN];
        sig_bytes[0..ED448_KEY_BYTES_LEN].copy_from_slice(&r_comp.0);

        // S to bytes
        let s_final_bytes_56 = s_final.to_bytes();
        sig_bytes[ED448_KEY_BYTES_LEN..ED448_SIGNATURE_BYTES_LEN - 1]
            .copy_from_slice(&s_final_bytes_56);
        sig_bytes[ED448_SIGNATURE_BYTES_LEN - 1] = 0; // Padding

        Ok(Ed448Signature {
            signature: sig_bytes,
        })
    }
}

/// Ed448 Raw Key Pair (Scalar based, not Seed based)
/// Used for Owner Keys generated via `og` mode.
#[derive(Clone)]
pub struct Ed448RawKeyPair {
    pub secret: Scalar,
    pub public: [u8; ED448_KEY_BYTES_LEN],
}

impl Ed448RawKeyPair {
    /// Sign message using Randomized EdDSA (since we don't have the deterministic prefix)
    pub fn sign(&self, message: &[u8]) -> Result<Ed448Signature> {
        // 1. Generate random r
        let mut rng = rand::rng();
        let mut r_bytes = [0u8; ED448_SIGNATURE_BYTES_LEN];
        rng.fill(&mut r_bytes);
        let r = Scalar::from_bytes_mod_order_wide(&r_bytes);

        // 2. R = r * B
        let r_point = ExtendedPoint::generator() * r;
        let r_comp = r_point.compress();

        // 3. k = Shake256(dom4 || R || A || M)
        let mut hasher_k = Shake256::default();
        hasher_k.update(DOM4_PREFIX);
        hasher_k.update(&r_comp.0);
        hasher_k.update(&self.public);
        hasher_k.update(message);
        let mut k_out = [0u8; ED448_SIGNATURE_BYTES_LEN];
        let mut reader_k = hasher_k.finalize_xof();
        XofReader::read(&mut reader_k, &mut k_out);

        let k = Scalar::from_bytes_mod_order_wide(&k_out);

        // 4. S = (r + k * s)
        let s_final = r + k * self.secret;

        // Encode Signature: R (ED448_KEY_BYTES_LEN) || S (ED448_KEY_BYTES_LEN)
        let mut sig_bytes = [0u8; ED448_SIGNATURE_BYTES_LEN];
        sig_bytes[0..ED448_KEY_BYTES_LEN].copy_from_slice(&r_comp.0);

        // S to bytes
        let s_final_bytes_56 = s_final.to_bytes();
        sig_bytes[ED448_KEY_BYTES_LEN..ED448_SIGNATURE_BYTES_LEN - 1]
            .copy_from_slice(&s_final_bytes_56);
        sig_bytes[ED448_SIGNATURE_BYTES_LEN - 1] = 0; // Padding

        Ok(Ed448Signature {
            signature: sig_bytes,
        })
    }
}

pub fn verify_signature(
    public_key: &[u8; ED448_KEY_BYTES_LEN],
    signed_payload: &[u8],
    signature: &Ed448Signature,
) -> Result<bool> {
    if signature.signature.len() != ED448_SIGNATURE_BYTES_LEN {
        return Ok(false);
    }

    // Parse R and S
    let r_bytes_slice = &signature.signature[0..ED448_KEY_BYTES_LEN]; // Point (compressed)
    let s_bytes_slice = &signature.signature[ED448_KEY_BYTES_LEN..ED448_SIGNATURE_BYTES_LEN]; // Scalar (ED448_KEY_BYTES_LEN bytes)

    // Check S < L?
    if s_bytes_slice[56] != 0 {
        return Ok(false);
    }
    let mut s_bytes_56 = [0u8; 56];
    s_bytes_56.copy_from_slice(&s_bytes_slice[0..56]);
    let s_scalar = Scalar::from_bytes(s_bytes_56);

    // Parse A (public key)
    use ed448_goldilocks::curve::edwards::CompressedEdwardsY;

    // Attempt to decompress A
    let a_comp = CompressedEdwardsY(public_key.clone());
    let a_point = match a_comp.decompress() {
        Some(p) => p,
        None => return Ok(false),
    };

    // Attempt to decompress R
    let mut r_arr = [0u8; ED448_KEY_BYTES_LEN];
    r_arr.copy_from_slice(r_bytes_slice);
    let r_comp = CompressedEdwardsY(r_arr);
    let r_point = match r_comp.decompress() {
        Some(p) => p,
        None => return Ok(false),
    };

    // Calculate k = Shake256(dom4 || R || A || M)
    let mut hasher_k = Shake256::default();
    hasher_k.update(DOM4_PREFIX);
    hasher_k.update(r_bytes_slice);
    hasher_k.update(public_key);
    hasher_k.update(signed_payload);
    let mut k_out = [0u8; ED448_SIGNATURE_BYTES_LEN];
    let mut reader_k = hasher_k.finalize_xof();
    XofReader::read(&mut reader_k, &mut k_out);

    let k = Scalar::from_bytes_mod_order_wide(&k_out);

    // Check S * B == R + k * A
    let lhs = ExtendedPoint::generator() * s_scalar;
    let rhs = r_point + a_point * k;

    Ok(lhs.compress().0 == rhs.compress().0)
}

/// ファイルに署名する
pub fn sign_file<P: AsRef<Path>>(path: P, key_pair: &Ed448KeyValuePair) -> Result<Ed448Signature> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    key_pair.sign(&buffer)
}

/// ファイルの署名を検証する
pub fn verify_file<P: AsRef<Path>>(
    path: P,
    public_key: &[u8; ED448_KEY_BYTES_LEN],
    signature: &Ed448Signature,
) -> Result<bool> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    verify_signature(public_key, &buffer, signature)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct KeyPairJson {
    secret: String,
    public: String,
}

/// キーペアをファイルに保存する (JSON形式)
pub fn save_keypair<P: AsRef<Path>>(path: P, key_pair: &Ed448KeyValuePair) -> Result<()> {
    let json = KeyPairJson {
        secret: hex::encode(key_pair.secret),
        public: hex::encode(key_pair.public),
    };
    let content = serde_json::to_string_pretty(&json)?;

    let path = path.as_ref();
    let file = File::create(path)?;

    // Set permissions to 600 (Unix only)
    #[cfg(unix)]
    {
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600);
        file.set_permissions(perms)?;
    }

    let mut writer = file;
    writer.write_all(content.as_bytes())?;
    Ok(())
}

/// キーペアをファイルから読み込む
pub fn load_keypair<P: AsRef<Path>>(path: P) -> Result<Ed448KeyValuePair> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let json: KeyPairJson = serde_json::from_str(&content)?;

    let secret_vec = hex::decode(json.secret).context("Failed to decode secret key hex")?;
    let public_vec = hex::decode(json.public).context("Failed to decode public key hex")?;

    if secret_vec.len() != ED448_KEY_BYTES_LEN || public_vec.len() != ED448_KEY_BYTES_LEN {
        anyhow::bail!("Invalid key length in file");
    }

    let mut secret = [0u8; ED448_KEY_BYTES_LEN];
    secret.copy_from_slice(&secret_vec);

    let mut public = [0u8; ED448_KEY_BYTES_LEN];
    public.copy_from_slice(&public_vec);

    Ok(Ed448KeyValuePair { secret, public })
}

// ============================================================
// ランダム鍵生成ユーティリティ
// ============================================================

/// 32バイトの暗号論的に安全な乱数を生成し、Base64 (Standard) エンコードした文字列を返す。
///
/// # 用途
/// JWT 署名鍵 (`rt_skey`) の自動生成に使用する。
/// RFC 7518 の HS256 要件（256ビット以上）を満たす。
///
/// # 戻り値
/// 44文字の Base64 エンコード文字列（デコード後 32バイト）
pub fn generate_random_b64_key_32() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    BASE64_STANDARD.encode(bytes)
}

/// 指定された長さの暗号論的に安全な英数字ランダム文字列を生成する。
///
/// # 用途
/// データ暗号化鍵 (`rt_crypto_key`) の自動生成に使用する。
///
/// # 引数
/// - `len`: 生成する文字列の文字数
///
/// # 戻り値
/// `len` 文字の英数字文字列 (a-z, A-Z, 0-9)
pub fn generate_random_alphanumeric(len: usize) -> String {
    rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

    #[test]
    fn test_generate_random_b64_key_32_length() -> Result<()> {
        let key = generate_random_b64_key_32();
        // Base64 エンコードした 32バイトは 44文字になる（パディング含む）
        assert_eq!(key.len(), 44, "Base64 encoded 32-byte key must be 44 chars");
        Ok(())
    }

    #[test]
    fn test_generate_random_b64_key_32_decodes_to_32_bytes() -> Result<()> {
        let key = generate_random_b64_key_32();
        let decoded = BASE64_STANDARD
            .decode(&key)
            .map_err(|e| anyhow::anyhow!("Failed to decode Base64: {}", e))?;
        assert_eq!(decoded.len(), 32, "Decoded key must be exactly 32 bytes");
        Ok(())
    }

    #[test]
    fn test_generate_random_b64_key_32_is_unique() -> Result<()> {
        // 同じ関数を2回呼んで、異なる値が生成されることを確認（確率論的に同一になることは無視できる）
        let key1 = generate_random_b64_key_32();
        let key2 = generate_random_b64_key_32();
        assert_ne!(key1, key2, "Two generated keys must not be identical");
        Ok(())
    }

    #[test]
    fn test_generate_random_alphanumeric_length() -> Result<()> {
        let key = generate_random_alphanumeric(32);
        assert_eq!(key.len(), 32, "Generated alphanumeric key must be 32 chars");
        Ok(())
    }

    #[test]
    fn test_generate_random_alphanumeric_is_alphanumeric() -> Result<()> {
        let key = generate_random_alphanumeric(32);
        assert!(
            key.chars().all(|c| c.is_ascii_alphanumeric()),
            "All characters must be ASCII alphanumeric"
        );
        Ok(())
    }

    #[test]
    fn test_generate_random_alphanumeric_is_unique() -> Result<()> {
        let key1 = generate_random_alphanumeric(32);
        let key2 = generate_random_alphanumeric(32);
        assert_ne!(key1, key2, "Two generated keys must not be identical");
        Ok(())
    }
}
