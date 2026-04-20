use crate::constants::ED448_SIGNATURE_BYTES_LEN;
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Context, Result};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use ed448_goldilocks::curve::ExtendedPoint;
use ed448_goldilocks::Scalar;
use rand::RngCore;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::utils::init::{CommonFlgs, HasCommonFlgs};
use clap::Parser;
use serde::Serialize;

#[derive(Debug, Parser, Serialize)]
#[command(override_usage = "mycute og [OPTIONS]")]
pub struct OGFlgs {
    #[command(flatten)]
    pub common: CommonFlgs,

    #[arg(
        short = 'f',
        long = "file",
        required = true,
        help = "Path to the passphrases file (must contain exactly 15 lines)"
    )]
    pub file: String,
}

impl HasCommonFlgs for OGFlgs {
    fn common_flgs(&self) -> &CommonFlgs {
        &self.common
    }
}

pub fn main_of_og(flgs: OGFlgs) -> Result<()> {
    let file_path = Path::new(&flgs.file);

    // 2. Read Passphrases
    println!("Reading passphrases from {:?}", file_path);
    let file = File::open(file_path).context(format!("Failed to open file: {:?}", file_path))?;
    let reader = BufReader::new(file);
    let passphrases: Vec<String> = reader
        .lines()
        .collect::<Result<Vec<String>, _>>()?
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .collect();

    if passphrases.len() != 15 {
        return Err(anyhow!(
            "Invalid number of passphrases. Expected 15, found {}",
            passphrases.len()
        ));
    }

    // 3. Generate Ed448 Keypair
    println!("Generating Anchor Keypair (Ed448)...");
    // Generate random secret key (114 bytes for wide reduction)
    let mut secret_bytes = [0u8; ED448_SIGNATURE_BYTES_LEN];
    let mut rng = rand::rng();
    rng.fill_bytes(&mut secret_bytes);
    let secret = Scalar::from_bytes_mod_order_wide(&secret_bytes);

    // Calculate public key
    let public = ExtendedPoint::generator() * &secret;
    let public_bytes = public.compress();
    let public_hex = hex::encode(public_bytes.0);

    println!("Generated Public Key: {}", public_hex);

    // 4. Encrypt Private Key with each passphrase
    let mut blobs = Vec::new();
    let argon2 = Argon2::default();

    println!("Encrypting private key with 15 passphrases...");
    for (i, pass) in passphrases.iter().enumerate() {
        // A. Generate Salt for Argon2
        let salt = SaltString::generate(argon2::password_hash::rand_core::OsRng);

        // B. KDF (Argon2id) -> 32 bytes Key for AES-256
        let password_hash = argon2
            .hash_password(pass.as_bytes(), &salt)
            .map_err(|e| anyhow!("Argon2 error: {}", e))?;

        // Use the raw hash output as the AES key. argon2 crate output includes robust formatting,
        // but we need raw bytes. hash_password returns PasswordHash which has 'hash' field if successful.
        let key_bytes = password_hash.hash.context("Argon2 hash missing")?;
        // Ensure 32 bytes. Argon2 default output length is 32.
        let key_array: [u8; 32] = key_bytes
            .as_bytes()
            .try_into()
            .map_err(|_| anyhow!("Derived key length mismatch"))?;

        let cipher = Aes256Gcm::new(&key_array.into());

        // C. Encrypt (AES-256-GCM)
        let mut nonce_bytes = [0u8; 12];
        rng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Payload: secret key bytes
        let ciphertext = cipher
            .encrypt(
                nonce,
                Payload {
                    msg: &secret_bytes,
                    aad: &[],
                },
            )
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

        // D. Format Blob: [Salt(string bytes) | Nonce(12) | Ciphertext(Var)]
        // To make it simpler for decryption, we serialize:
        // Length of Salt string (1 byte) + Salt string bytes + Nonce (12) + Ciphertext

        let salt_str = salt.as_str();
        let salt_bytes = salt_str.as_bytes();
        let salt_len = salt_bytes.len();
        if salt_len > 255 {
            return Err(anyhow!("Salt string too long"));
        }

        let mut blob = Vec::new();
        blob.push(salt_len as u8);
        blob.extend_from_slice(salt_bytes);
        blob.extend_from_slice(&nonce_bytes);
        blob.extend_from_slice(&ciphertext);

        let blob_len = blob.len();
        blobs.push(blob);
        log::debug!(
            "Passphrase #{}: Blob size {} bytes",
            i + 1,
            blob_len
        );
    }

    // 5. Verify Decryption (Self-Check)
    println!("Verifying decryption for all 15 passphrases...");
    for (i, pass) in passphrases.iter().enumerate() {
        let blob = &blobs[i];

        // Parse Blob
        let salt_len = blob[0] as usize;
        if blob.len() < 1 + salt_len + 12 {
            return Err(anyhow!(
                "Verification failed: Blob too short for index {}",
                i
            ));
        }
        let salt_bytes = &blob[1..1 + salt_len];
        let nonce_bytes = &blob[1 + salt_len..1 + salt_len + 12];
        let ciphertext = &blob[1 + salt_len + 12..];

        // Derive Key
        let salt_str = std::str::from_utf8(salt_bytes).map_err(|_| anyhow!("Invalid salt utf8"))?;
        let salt =
            SaltString::from_b64(salt_str).map_err(|e| anyhow!("Salt parse error: {}", e))?;

        let password_hash = argon2
            .hash_password(pass.as_bytes(), &salt)
            .map_err(|e| anyhow!("Argon2 error during verification: {}", e))?;

        let key_bytes = password_hash.hash.context("Argon2 hash missing")?;
        let key_array: [u8; 32] = key_bytes
            .as_bytes()
            .try_into()
            .map_err(|_| anyhow!("Derived key length mismatch"))?;

        let cipher = Aes256Gcm::new(&key_array.into());
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed for passphrase #{}: {}", i + 1, e))?;

        // Verify
        if plaintext != secret_bytes {
            return Err(anyhow!(
                "Verification failed for passphrase #{}: Content mismatch",
                i + 1
            ));
        }
    }
    println!("[OK] All 15 passphrases verified successfully.");

    // 6. Generate Rust Code
    println!("Generating rust code...");
    let mut code = String::new();
    code.push_str("// =======================================================\n");
    code.push_str("// AUTO-GENERATED BY `cargo run -- og`\n");
    code.push_str("// DO NOT EDIT MANUALLY\n");
    code.push_str("// =======================================================\n\n");

    code.push_str("/// The Anchor Public Key (Ed448) of this system.\n");
    code.push_str(&format!(
        "pub const OWNER_PUB_KEY_HEX: &str = \"{}\";\n\n",
        public_hex
    ));

    code.push_str("/// 15 Encrypted Blobs of the Owner Secret Key.\n");
    code.push_str("/// Format: [SaltLen(1) | Salt(...) | Nonce(12) | Ciphertext(...)]\n");
    code.push_str("pub const OWNER_SECRET_BLOBS: &[&[u8]; 15] = &[\n");

    for (i, blob) in blobs.iter().enumerate() {
        code.push_str(&format!("    // Blob #{}\n", i + 1));
        code.push_str("    &[");
        for (j, byte) in blob.iter().enumerate() {
            if j % 16 == 0 {
                code.push_str("\n        ");
            }
            code.push_str(&format!("0x{:02x}, ", byte));
        }
        code.push_str("\n    ],\n");
    }
    code.push_str("];\n");

    // 7. Write to file
    let output_path = Path::new("src/mode/rt/owner_secrets.rs");
    let mut outfile = File::create(output_path).context("Failed to create output file")?;
    outfile.write_all(code.as_bytes())?;

    println!("Successfully generated {:?}", output_path);

    Ok(())
}
