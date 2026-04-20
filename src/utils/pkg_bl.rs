use crate::constants::{
    APP_BUILD_ZSTD_LEVEL, APP_MANIFEST_FILENAME, APP_PACKAGE_KEY_SALT, ED448_KEY_BYTES_LEN,
    ED448_SIGNATURE_BYTES_LEN,
};
use crate::mode::rt::rtbl::identities_bl::{self, AppVerificationDetail, IdentityLayer};
use crate::utils::crypto::{verify_signature, Ed448Signature};
use crate::utils::time;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha3::{
    digest::{ExtendableOutput, Update, XofReader},
    Shake256,
};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use utoipa::ToSchema;
use zstd::stream::read::Decoder as ZstdDecoder;
use zstd::stream::write::Encoder as ZstdEncoder;

// ============================================================
// 定数と構造体
// ============================================================
// ヘッダー: MAGIC(6) + VERSION(2) + META_LEN(8)
const MAGIC: &[u8; 6] = b"MYCUTE";
const PKG_VERSION: u16 = 1;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppVerification {
    pub ca_public_key: String,
    pub signature: String,
    pub ca_token: String,
    pub expire_at: u64,
}

// ============================================================
// App Verification Results (共通の検証結果)
// ============================================================
#[derive(Serialize, Deserialize, Debug, ToSchema, Clone)]
pub struct AppVerificationResults {
    /// パッケージの物理構造（ヘッダー、マジックバイト等）が正しいか
    pub ok_structure: bool,
    /// パッケージのバージョンがシステムでサポートされているものか
    pub ok_version: bool,
    /// マニフェストファイル（mycute.json）が正しくパース可能か
    pub ok_manifest: bool,
    /// ペイロードデータが破損なく読み込めるか
    pub ok_payload: bool,
    /// 暗号化（難読化）されたペイロードが正しく復号できるか
    pub ok_decryption: bool,
    /// ターゲットディレクトリへのファイル展開が正常に完了したか
    pub ok_extraction: bool,

    /// 各証明書（verifications配列の各要素）に対する個別の検証結果
    pub verifications: Vec<AppVerificationDetail>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema, Clone)]
pub struct AppTrustInfo {
    pub global_app_id: String,
    pub global_app_version: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub dev_public_key: Option<String>,
    pub manifest_data: Option<serde_json::Value>,
    /// [証拠]: 開発者が提供した生の検証情報のリスト
    pub verifications: Option<serde_json::Value>,
    /// [結果キャッシュ]: ノードが検証した結果の詳細リスト
    pub verification_results_cache: Option<serde_json::Value>,
    pub app_verification: Option<AppVerificationResults>,
}

impl AppTrustInfo {
    pub fn from_manifest(
        manifest: MyCuteManifest,
        app_verification: AppVerificationResults,
    ) -> Self {
        let manifest_json = serde_json::to_value(&manifest).ok();
        let verifications_json = serde_json::to_value(&manifest.verifications).ok();
        let results_cache_json = serde_json::to_value(&app_verification.verifications).ok();

        Self {
            global_app_id: manifest.global_app_id,
            global_app_version: manifest.global_app_version,
            name: manifest.name,
            author: manifest.author,
            description: manifest.description,
            dev_public_key: manifest.dev_public_key,
            manifest_data: manifest_json,
            verifications: verifications_json,
            verification_results_cache: results_cache_json,
            app_verification: Some(app_verification),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MyCuteManifest {
    pub global_app_id: String,      // UUID v4 形式
    pub global_app_version: String, // 00000.00.00 形式
    pub name: String,
    pub author: String,
    #[serde(default)]
    pub description: String,

    // マルチCA検証情報
    #[serde(default)]
    pub verifications: Vec<AppVerification>,

    // 開発者公開鍵 (L3の検証に必要)
    pub dev_public_key: Option<String>,

    // [声明]: ビルド時に開発者自身が検証した結果のレポート
    #[serde(default)]
    pub verification_report: Option<Vec<AppVerificationDetail>>,

    // 新しい検証フィールド
    pub dev_certificate: Option<String>,    // 証明書本体 (Hex)
    pub delegate_signature: Option<String>, // オプションの委譲署名

    // 依存関係 (将来用)
    #[serde(default)]
    pub dependencies: Vec<String>,
}

pub struct IdentityCredentials {
    pub key_pair: crate::utils::crypto::Ed448KeyValuePair,
    pub verifications: Vec<AppVerification>,
}

/// 開発者の公開鍵とシステムソルトからペイロード用 AES 鍵 (32 bytes) を派生させる
fn derive_payload_key(pubkey_hex: &str) -> Result<[u8; 32]> {
    use sha3::Digest;
    let mut hasher = sha3::Sha3_256::new();
    Digest::update(&mut hasher, pubkey_hex.as_bytes());
    Digest::update(&mut hasher, APP_PACKAGE_KEY_SALT.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    Ok(key)
}

// ============================================================
// Build Logic
// ============================================================
pub fn build_package<P: AsRef<Path>, Q: AsRef<Path>>(
    config_manager: &crate::mycute_settings::ConfigManager,
    src_dir: P,
    output_path: Q,
    creds: Option<IdentityCredentials>,
) -> Result<()> {
    let src_dir = src_dir.as_ref();
    let output_path = output_path.as_ref();

    // 1. src から mycute.json を読み込む
    let manifest_path = src_dir.join(APP_MANIFEST_FILENAME);
    if !manifest_path.exists() {
        bail!("{} not found in source directory.", APP_MANIFEST_FILENAME);
    }
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .context(format!("Failed to read {}", APP_MANIFEST_FILENAME))?;
    let mut manifest: MyCuteManifest = serde_json::from_str(&manifest_content)
        .context(format!("Failed to parse {}", APP_MANIFEST_FILENAME))?;

    // 2. マニフェストの検証（およびサニタイズ）
    validate_manifest(&mut manifest)?;

    let c_ref = creds; // 所有権の管理

    // 3. 資格情報を注入
    manifest.dev_public_key = c_ref.as_ref().map(|c| hex::encode(c.key_pair.public));
    if let Some(c) = &c_ref {
        manifest.verifications = c.verifications.clone();
    } else {
        log::warn!("Building package without credentials (Unverified App).");
    }

    // 4. 出力ファイルとバッファの作成
    let file = File::create(output_path).context("Failed to create output file")?;
    let mut writer = BufWriter::new(file);

    // 5. 一時ペイロードの書き込み (ハッシュ計算用)
    // 構成: [HEADER][META_LEN][META][SIG][PAYLOAD]
    // 署名は (Meta + Hash(Payload)) に対するものであるため、先にペイロードを生成しハッシュを特定する必要がある。

    let temp_payload_path = output_path.with_extension("tmp.payload");
    let temp_file = File::create(&temp_payload_path)?;
    let mut temp_writer = BufWriter::new(temp_file);

    // Zstd エンコーダー (配布効率最大化のため最高レベルを使用。展開速度への影響は軽微)
    let mut encoder = ZstdEncoder::new(&mut temp_writer, APP_BUILD_ZSTD_LEVEL)?;

    // Tar ビルダー (借用を解除するためのスコープ)
    {
        let mut tar = tar::Builder::new(&mut encoder);
        tar.follow_symlinks(false); // セキュリティ: 外部へのシンボリックリンクを許可しない
                                    // ディレクトリの内容を追加
        tar.append_dir_all(".", src_dir)?;
        tar.finish()?;
    }
    encoder.finish()?; // エンコーダーを消費し、temp_writer の借用を解除
    temp_writer.flush()?;
    drop(temp_writer); // ファイルを閉じる

    // 6. ペイロードのハッシュを計算
    let mut payload_reader = File::open(&temp_payload_path)?;
    let mut hasher = Shake256::default();
    let mut buffer = [0u8; 8192];
    loop {
        let n = payload_reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let mut hash_output = [0u8; 64];
    let mut xof = hasher.finalize_xof();
    XofReader::read(&mut xof, &mut hash_output);

    // 7. ペイロードの暗号化 (AES-256-GCM による難読化)
    // 開発者の公開鍵が利用可能な場合のみ暗号化（難読化）を行う。
    // 公開鍵とシステムソルトから派生した鍵を使用する。
    let dev_pub_hex = c_ref
        .as_ref()
        .map(|c| hex::encode(c.key_pair.public))
        .unwrap_or_default();
    let (final_payload_bytes, _is_encrypted) = if !dev_pub_hex.is_empty() {
        let key = derive_payload_key(&dev_pub_hex)?;
        // 一時ファイルを読み込み、AES 暗号化
        let mut raw_payload = Vec::new();
        File::open(&temp_payload_path)?.read_to_end(&mut raw_payload)?;
        let encrypted = crate::utils::crypto::encrypt_bytes(&raw_payload, &key)
            .context("Failed to encrypt payload")?;
        (encrypted, true)
    } else {
        let mut raw_payload = Vec::new();
        File::open(&temp_payload_path)?.read_to_end(&mut raw_payload)?;
        (raw_payload, false)
    };

    // 8. パッケージ署名と検証レポートの生成 (2-Pass プロセス)
    // .mycute パッケージは [META_JSON] + [PAYLOAD_HASH] を開発者の秘密鍵で署名し、整合性を保証する。
    // マニフェスト自体のなかに「この時点での検証結果」を記録するため、2段階で処理を行う。

    let signature_bytes = if let Some(c) = &c_ref {
        // --- Pass 1: 仮署名による検証レポートの生成 ---
        // まだレポートがない状態で一度シリアライズ
        let meta_json_tmp = serde_json::to_string(&manifest)?;
        let meta_bytes_tmp = meta_json_tmp.as_bytes();

        let mut sign_msg_tmp = Vec::new();
        sign_msg_tmp.extend_from_slice(meta_bytes_tmp);
        sign_msg_tmp.extend_from_slice(&hash_output);
        let sig_tmp = c
            .key_pair
            .sign(&sign_msg_tmp)
            .context("Failed to produce temporary signature")?;

        // 検証実行
        let mut manifest_mut = manifest.clone();
        let details = verify_trust_chain(
            config_manager,
            &mut manifest_mut,
            &sig_tmp.signature,
            &hash_output,
            meta_bytes_tmp,
        )?;

        // レポートをマニフェストに充填
        manifest.verification_report = Some(details);

        // --- Pass 2: レポートを含めた状態での本署名 ---
        let meta_json_final = serde_json::to_string(&manifest)?;
        let meta_bytes_final = meta_json_final.as_bytes();

        let mut sign_msg_final = Vec::new();
        sign_msg_final.extend_from_slice(meta_bytes_final);
        sign_msg_final.extend_from_slice(&hash_output);
        let sig_final = c
            .key_pair
            .sign(&sign_msg_final)
            .context("Failed to sign final package")?;

        sig_final.signature
    } else {
        [0u8; ED448_SIGNATURE_BYTES_LEN]
    };

    // 9. 最終的なメタデータのシリアライズ
    let meta_json = serde_json::to_string(&manifest)?;
    let meta_bytes = meta_json.as_bytes();
    let meta_len = meta_bytes.len() as u64;

    // 10. 全データをファイルに書き込む
    // [META_LEN]
    writer.write_all(&meta_len.to_be_bytes())?;
    // [METADATA]
    writer.write_all(meta_bytes)?;
    // [SIGNATURE]
    writer.write_all(&signature_bytes)?;
    // [PAYLOAD]
    writer.write_all(&final_payload_bytes)?;
    writer.flush()?;

    // クリーンアップ
    std::fs::remove_file(temp_payload_path)?;

    Ok(())
}

fn validate_manifest(m: &mut MyCuteManifest) -> Result<()> {
    // 1. 基本形式のチェック
    // バージョンチェック (00000.00.00)
    let ver_regex = match regex::Regex::new(r"^\d{5}\.\d{2}\.\d{2}$") {
        Ok(re) => re,
        Err(_) => return Err(anyhow::anyhow!("Internal Error: Version regex is invalid")),
    };
    if !ver_regex.is_match(&m.global_app_version) {
        bail!("Invalid version format. Expected 00000.00.00");
    }
    // UUID チェック
    if uuid::Uuid::parse_str(&m.global_app_id).is_err() {
        bail!("Invalid global_app_id. Expected UUID.");
    }

    // 2. 必須項目と文字列のサニタイズ（制御文字の自動除去）とバリデーション
    let sanitize_and_validate = |val: &mut String, field: &str, max_len: usize| -> Result<()> {
        // 制御文字（改行、タブ等）をすべて除去
        let sanitized: String = val.chars().filter(|c| !c.is_control()).collect();
        let trimmed = sanitized.trim().to_string();

        if trimmed.is_empty() {
            bail!("Manifest validation failed: '{}' is required and cannot be empty (after sanitizing control characters).", field);
        }
        if trimmed.len() > max_len {
            bail!(
                "Manifest validation failed: '{}' exceeds maximum length of {} characters.",
                field,
                max_len
            );
        }

        *val = trimmed;
        Ok(())
    };

    sanitize_and_validate(&mut m.name, "name", 255)?;
    sanitize_and_validate(&mut m.author, "author", 255)?;
    sanitize_and_validate(&mut m.description, "description", 1024)?;

    // 3. 依存関係のバリデーション
    for dep_id in &m.dependencies {
        if uuid::Uuid::parse_str(dep_id).is_err() {
            bail!(
                "Manifest validation failed: Dependency ID '{}' is not a valid UUID.",
                dep_id
            );
        }
    }

    Ok(())
}

pub fn extract_package<P: AsRef<Path>, Q: AsRef<Path>>(
    config_manager: &crate::mycute_settings::ConfigManager,
    pkg_path: P,
    target_dir: Q,
) -> Result<(MyCuteManifest, AppVerificationResults)> {
    let pkg_path = pkg_path.as_ref();
    let target_dir = target_dir.as_ref();

    let file = File::open(pkg_path)?;
    let mut reader = BufReader::new(file);

    // 1. ヘッダー
    let mut magic = [0u8; 6];
    reader.read_exact(&mut magic)?;
    if &magic != MAGIC {
        bail!("Invalid Magic Bytes");
    }
    let mut ver_bytes = [0u8; 2];
    reader.read_exact(&mut ver_bytes)?;
    let version = u16::from_be_bytes(ver_bytes);
    if version != PKG_VERSION {
        bail!("Unsupported Package Version: {}", version);
    }

    // 2. メタデータの長さ
    let mut meta_len_bytes = [0u8; 8];
    reader.read_exact(&mut meta_len_bytes)?;
    let meta_len = u64::from_be_bytes(meta_len_bytes) as usize;

    // 3. メタデータ
    let mut meta_buf = vec![0u8; meta_len];
    reader.read_exact(&mut meta_buf)?;
    let manifest: MyCuteManifest = serde_json::from_slice(&meta_buf)?;

    // Hard Error 以外はここまでで合格
    let ok_structure = true;
    let ok_version = true;
    let ok_manifest = true;

    // 4. 署名
    let mut sig_buf = [0u8; ED448_SIGNATURE_BYTES_LEN];
    reader.read_exact(&mut sig_buf)?;

    // 5. ペイロードの読み込み
    let mut encrypted_payload = Vec::new();
    reader
        .read_to_end(&mut encrypted_payload)
        .context("Failed to read payload")?;
    let ok_payload = true;

    // 6. ペイロードの復号 (難読化の解除)
    // 開発者の公開鍵が存在する場合のみ復号を行う
    let payload_bytes = if let Some(pubkey_hex) = &manifest.dev_public_key {
        let key = derive_payload_key(pubkey_hex)?;
        crate::utils::crypto::decrypt_bytes(&encrypted_payload, &key)
            .context("Failed to decrypt payload (Invalid key or corrupted data)")?
    } else {
        encrypted_payload
    };
    let ok_decryption = true;

    // 7. 復号されたペイロードのハッシュ計算 (L3検証用)
    let mut hasher = Shake256::default();
    hasher.update(&payload_bytes);
    let mut hash_output = [0u8; 64];
    let mut xof = hasher.finalize_xof();
    XofReader::read(&mut xof, &mut hash_output);

    // 8. 署名 (L3) と信頼チェーンの検証
    let mut manifest = manifest; // ミュータブルにする
    let verification_details = verify_trust_chain(
        config_manager,
        &mut manifest,
        &sig_buf,
        &hash_output,
        &meta_buf,
    )?;

    // 【点検】ビルド時の声明（Report）と現在の検証結果を突き合わせる
    if let Some(report) = &manifest.verification_report {
        if !are_verification_details_equal(report, &verification_details) {
            log::error!("CRITICAL: Trust Chain Inconsistency! Current verification results differ from the developer's signed build-time report.");
            log::debug!("Report (Build-time): {:?}", report);
            log::debug!("Result (Current): {:?}", verification_details);
            bail!("Trust Chain Inconsistency: Build-time report and current results do not match. Possible tampering or reporting fraud.");
        } else {
            log::info!("Trust Chain Consistency: Checked. Current results perfectly match the developer's signed report.");
        }
    } else {
        log::info!("No build-time verification report found in manifest.");
    }

    let valid_count = verification_details
        .iter()
        .filter(|d| d.ok_ca_until.is_some() && d.ok_dev_until.is_some() && d.ok_app_sig)
        .count();
    log::info!(
        "Package Trust Chain: {} valid out of {} verifications",
        valid_count,
        verification_details.len()
    );

    // 9. ペイロードの展開 (Zstd + Tar)
    let payload_cursor = std::io::Cursor::new(payload_bytes);
    let decoder = ZstdDecoder::new(payload_cursor)?;
    let mut tar = tar::Archive::new(decoder);

    // 安全な展開
    tar.unpack(target_dir)?;
    let ok_extraction = true;

    let app_verification = AppVerificationResults {
        ok_structure,
        ok_version,
        ok_manifest,
        ok_payload,
        ok_decryption,
        verifications: verification_details,
        ok_extraction,
    };

    Ok((manifest, app_verification))
}

pub fn verify_trust_chain(
    config_manager: &crate::mycute_settings::ConfigManager,
    manifest: &mut MyCuteManifest,
    app_sig: &[u8; ED448_SIGNATURE_BYTES_LEN],
    payload_hash: &[u8; 64], // ペイロードハッシュの生バイト
    meta_bytes: &[u8],
) -> Result<Vec<AppVerificationDetail>> {
    let mut details = Vec::new();
    let dev_pub_hex = match manifest.dev_public_key.as_ref() {
        Some(s) => s,
        // 開発者公開鍵がない場合、検証は不可能
        None => return Ok(vec![]),
    };

    let now_ts = time::now_ts_ms() as u64;

    for v in &manifest.verifications {
        let layer = identities_bl::determine_layer(
            config_manager,
            dev_pub_hex,
            &v.ca_public_key,
            // ca_base_url はログ出力用にのみ使用され、署名検証ロジック（暗号学的強度）には一切影響しないため、
            // ここではダミー値 ("PackageVerifier") を渡してもセキュリティ上の問題はない。
            "PackageVerifier", // ca_base_url dummy for logging
            Some(&v.signature),
            Some(&v.ca_token),
            Some(v.expire_at),
            now_ts,
        );

        let mut detail = AppVerificationDetail {
            ca_public_key: v.ca_public_key.clone(),
            ok_ca_until: None,
            ok_dev_until: None,
            ok_app_sig: false,
        };

        // determine_layer の結果を AppVerificationDetail にマッピング
        match layer {
            IdentityLayer::L3 => {
                // L3 の場合、L1, L2, L3 すべてクリア
                detail.ok_ca_until = Some(0); // 簡略化：L3判定済みなら有効とする
                detail.ok_dev_until = Some(v.expire_at);
                detail.ok_app_sig = true; // パッケージ署名は L3 判定の前提ではないが、pkg_bl では別途検証が必要
            }
            IdentityLayer::L2 => {
                detail.ok_dev_until = Some(v.expire_at);
            }
            IdentityLayer::L1 => {}
        }

        // --- L3: アプリ署名の検証 (開発者による署名) ---
        // identities_bl::determine_layer はアイデンティティ (CA->Dev) の検証のみを行うため、
        // パッケージ本体の整合性 (Dev->Pkg) はここで別途確認する。
        let mut app_l3_msg = Vec::new();
        app_l3_msg.extend_from_slice(meta_bytes);
        app_l3_msg.extend_from_slice(payload_hash);
        let app_sig_struct = Ed448Signature {
            signature: *app_sig,
        };
        if let Ok(dev_pub_bytes) = hex::decode(dev_pub_hex) {
            let mut dev_pub_arr = [0u8; ED448_KEY_BYTES_LEN];
            dev_pub_arr.copy_from_slice(&dev_pub_bytes);
            if verify_signature(&dev_pub_arr, &app_l3_msg, &app_sig_struct).unwrap_or(false) {
                detail.ok_app_sig = true;
            } else {
                detail.ok_app_sig = false; // 明示的に false
                                           // パッケージ署名に失敗した場合、レイヤーに関わらず信頼できない
                detail.ok_ca_until = None;
                detail.ok_dev_until = None;
            }
        }

        details.push(detail);
    }

    Ok(details)
}

/// 2つの検証結果配列が「論理的に同一」であるかを厳格に判定する。
/// 順序の違いを許容するため、公開鍵でソートしてから比較を行う。
pub fn are_verification_details_equal(
    a: &[AppVerificationDetail],
    b: &[AppVerificationDetail],
) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut sorted_a = a.to_vec();
    let mut sorted_b = b.to_vec();

    // 公開鍵でソート（同じ CA の検証結果を並べる）
    sorted_a.sort_by(|x, y| x.ca_public_key.cmp(&y.ca_public_key));
    sorted_b.sort_by(|x, y| x.ca_public_key.cmp(&y.ca_public_key));

    sorted_a == sorted_b
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_manifest() -> MyCuteManifest {
        MyCuteManifest {
            global_app_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            global_app_version: "00001.00.00".to_string(),
            name: "Test App".to_string(),
            author: "Test Author".to_string(),
            description: "This is a test description.".to_string(),
            verifications: vec![],
            dev_public_key: None,
            verification_report: None,
            dev_certificate: None,
            delegate_signature: None,
            dependencies: vec!["550e8400-e29b-41d4-a716-446655440001".to_string()],
        }
    }

    #[test]
    fn test_validate_manifest_valid() {
        let mut m = create_valid_manifest();
        assert!(validate_manifest(&mut m).is_ok());
    }

    #[test]
    fn test_validate_manifest_sanitize_and_require_desc() {
        let mut m = create_valid_manifest();
        m.name = "Test\nApp\tWith\rControl".to_string();
        m.description = "Line1\nLine2\rLine3".to_string();

        assert!(validate_manifest(&mut m).is_ok());
        assert_eq!(m.name, "TestAppWithControl");
        assert_eq!(m.description, "Line1Line2Line3");

        // 空の説明文は拒否
        m.description = "  ".to_string();
        assert!(validate_manifest(&mut m).is_err());
    }

    #[test]
    fn test_validate_manifest_invalid_version() {
        let mut m = create_valid_manifest();
        m.global_app_version = "1.0.0".to_string();
        assert!(validate_manifest(&mut m).is_err());
    }

    #[test]
    fn test_validate_manifest_empty_name() {
        let mut m = create_valid_manifest();
        m.name = "  ".to_string();
        assert!(validate_manifest(&mut m).is_err());
    }

    #[test]
    fn test_validate_manifest_invalid_dependency_uuid() {
        let mut m = create_valid_manifest();
        m.dependencies = vec!["not-a-uuid".to_string()];
        assert!(validate_manifest(&mut m).is_err());
    }

    #[test]
    fn test_validate_manifest_long_description() {
        let mut m = create_valid_manifest();
        m.description = "a".repeat(1025);
        assert!(validate_manifest(&mut m).is_err());
    }

    #[test]
    fn test_derive_payload_key() -> Result<()> {
        let pubkey = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde";
        let key1 = derive_payload_key(pubkey)?;
        let key2 = derive_payload_key(pubkey)?;
        assert_eq!(key1, key2);

        let key3 = derive_payload_key("ffff")?;
        assert_ne!(key1, key3);
        Ok(())
    }
}
