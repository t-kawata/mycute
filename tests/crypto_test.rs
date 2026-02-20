use ed448_goldilocks::curve::ExtendedPoint;
use mycute::utils::crypto::{verify_signature, Ed448KeyValuePair, Ed448Signature};

#[test]
fn test_ed448_linkage() {
    let identity = ExtendedPoint::identity();
    assert_eq!(
        identity.compress().0,
        ExtendedPoint::identity().compress().0
    );
}

#[test]
fn test_keygen_sign_verify_flow() {
    // 1. Generate KeyPair
    let pair = Ed448KeyValuePair::generate().expect("Keygen failed");

    // 2. Sign
    let message = b"Hello Ed448 World";
    let sig = pair.sign(message).expect("Sign failed");

    assert_eq!(sig.signature.len(), 114);

    // 3. Verify (Good)
    let valid = verify_signature(&pair.public, message, &sig).expect("Verify function failed");
    assert!(valid, "Signature should be valid");

    // 4. Verify (Bad Message)
    let bad_msg = b"Hello Hacker";
    let valid_bad_msg =
        verify_signature(&pair.public, bad_msg, &sig).expect("Verify function failed");
    assert!(
        !valid_bad_msg,
        "Signature should be invalid for different message"
    );

    // 5. Verify (Bad Key)
    let pair2 = Ed448KeyValuePair::generate().expect("Keygen2 failed");
    let valid_bad_key =
        verify_signature(&pair2.public, message, &sig).expect("Verify function failed");
    assert!(
        !valid_bad_key,
        "Signature should be invalid for different key"
    );

    // 6. Verify (Tampered Signature)
    let mut bad_sig_bytes = sig.signature;
    bad_sig_bytes[0] ^= 0xFF; // Flip first byte of R
    let bad_sig = Ed448Signature {
        signature: bad_sig_bytes,
    };
    let valid_tamper =
        verify_signature(&pair.public, message, &bad_sig).expect("Verify function failed");
    assert!(!valid_tamper, "Signature should be invalid if tampered");
}

#[test]
fn test_derive_public_key() {
    let pair = Ed448KeyValuePair::generate().expect("Keygen failed");
    let recovered = Ed448KeyValuePair::from_secret(pair.secret);

    assert_eq!(pair.secret, recovered.secret);
    assert_eq!(
        pair.public, recovered.public,
        "Public key should be deterministically derived from secret"
    );

    // Cross check: ensure signatures match or at least are valid
    let msg = b"Derivation Check";
    let sig = recovered.sign(msg).expect("Sign failed");
    assert!(
        verify_signature(&pair.public, msg, &sig).expect("Verify failed"),
        "Signature from recovered key should be valid for original public key"
    );
}

#[test]
fn test_file_sign_verify() {
    use mycute::utils::crypto::{sign_file, verify_file};
    use std::io::Write;

    // Create a temporary file
    let dir = std::env::temp_dir();
    let file_path = dir.join("test_sig_ed448.txt");
    {
        let mut f = std::fs::File::create(&file_path).expect("Failed to create temp file");
        f.write_all(b"File content to sign")
            .expect("Failed to write to temp file");
    }

    let pair = Ed448KeyValuePair::generate().expect("Keygen failed");

    // Sign file
    let sig = sign_file(&file_path, &pair).expect("Sign file failed");

    // Verify file
    let valid = verify_file(&file_path, &pair.public, &sig).expect("Verify file failed");
    assert!(valid, "File signature should be valid");

    // Cleanup
    let _ = std::fs::remove_file(file_path);
}

#[test]
fn test_key_save_load() {
    use mycute::utils::crypto::{load_keypair, save_keypair};

    let dir = std::env::temp_dir();
    let file_path = dir.join("test_key_ed448.json");

    let pair = Ed448KeyValuePair::generate().expect("Keygen failed");

    // Save
    save_keypair(&file_path, &pair).expect("Save failed");

    // Check permission (Unix only check logic hard to do portably in unit test without cfg, but implementation has it)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(&file_path).expect("Metadata failed");
        let mode = meta.permissions().mode();
        // 0o600 usually means last 3 octals are 600. mode includes file type bits.
        assert_eq!(mode & 0o777, 0o600, "Permissions should be 600");
    }

    // Load
    let loaded_pair = load_keypair(&file_path).expect("Load failed");

    assert_eq!(pair.secret, loaded_pair.secret);
    assert_eq!(pair.public, loaded_pair.public);

    // Cleanup
    let _ = std::fs::remove_file(file_path);
}
