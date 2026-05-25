use crate::constants::{
    MODEL_FILENAME_GTCRN, MODEL_FILENAME_SILERO_VAD, MODEL_FILENAME_SILERO_VAD_INT8,
    MODEL_FILENAME_TEN_VAD, MODEL_FILENAME_TEN_VAD_INT8, MODEL_FILENAME_TOKENS,
};
use crate::mycute_settings::ConfigManager;
use anyhow::{Context, Result};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::Path;

// モデル定義
struct ModelDef {
    filename: &'static str,
    url: &'static str,
    description: &'static str,
    // hex エンコードされた SHA256 チェックサム。空文字列の場合は検証をスキップする。
    sha256: &'static str,
}

// Makefile から移植されたモデルリスト
const MODELS: &[ModelDef] = &[
    ModelDef {
        filename: MODEL_FILENAME_SILERO_VAD,
        url: "https://huggingface.co/t-kawata/mycute/resolve/main/silero_vad.onnx",
        description: "Silero VAD Model",
        sha256: "",
    },
    ModelDef {
        filename: MODEL_FILENAME_SILERO_VAD_INT8,
        url: "https://huggingface.co/t-kawata/mycute/resolve/main/silero_vad.int8.onnx",
        description: "Silero VAD Int8 Model",
        sha256: "",
    },
    ModelDef {
        filename: MODEL_FILENAME_TEN_VAD,
        url: "https://huggingface.co/t-kawata/mycute/resolve/main/ten_vad.onnx",
        description: "TEN VAD Model",
        sha256: "",
    },
    ModelDef {
        filename: MODEL_FILENAME_TEN_VAD_INT8,
        url: "https://huggingface.co/t-kawata/mycute/resolve/main/ten-vad.int8.onnx",
        description: "TEN VAD Int8 Model",
        sha256: "",
    },
    ModelDef {
        filename: MODEL_FILENAME_TOKENS,
        url: "https://huggingface.co/t-kawata/mycute/resolve/main/tokens.txt",
        description: "Tokens file",
        sha256: "",
    },
    ModelDef {
        filename: MODEL_FILENAME_GTCRN,
        url: "https://huggingface.co/t-kawata/mycute/resolve/main/gtcrn.onnx",
        description: "GTCRN Denoiser Model",
        sha256: "",
    },
];

/// 必要なモデルが全て揃っているか確認し、不足していればダウンロードする
pub async fn ensure_models(config_manager: &ConfigManager) -> Result<()> {
    // mycute_settings.rs の修正により、ConfigManager::new で model_dir が必ず解決されている前提
    // Settings 内の model_dir を参照
    let model_dir_str = {
        let settings = config_manager.settings.read();
        settings.stt.model_dir.clone().unwrap_or_default()
    };

    if model_dir_str.is_empty() {
        return Err(anyhow::anyhow!(
            "CRITICAL: model_dir is not set in ConfigManager."
        ));
    }

    let model_dir = Path::new(&model_dir_str);
    let client = Client::new();

    for model in MODELS {
        let file_path = model_dir.join(model.filename);
        if !file_path.exists() {
            log::info!("Downloading {} ({}) ...", model.description, model.url);
            download_file(&client, model.url, &file_path).await?;
        } else if !model.sha256.is_empty() {
            // sha256 が設定されている場合、既存ファイルの整合性を検証する
            match compute_sha256(&file_path) {
                Ok(actual) if actual == model.sha256 => {
                    log::debug!("Model integrity verified: {}", model.filename);
                }
                Ok(actual) => {
                    log::warn!(
                        "Model corrupted, re-downloading: {} (expected={}, actual={})",
                        model.filename, model.sha256, actual
                    );
                    fs::remove_file(&file_path)?;
                    download_file(&client, model.url, &file_path).await?;
                }
                Err(e) => {
                    log::warn!("Could not verify model {}: {}", model.filename, e);
                }
            }
        } else {
            // sha256 未設定（既存モデルとの互換性維持）
            log::debug!("Model exists: {}", model.filename);
        }
    }

    Ok(())
}

async fn download_file(client: &Client, url: &str, path: &Path) -> Result<()> {
    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to send request")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to download file: status {}",
            response.status()
        ));
    }

    let content = response
        .bytes()
        .await
        .context("Failed to read response body")?;

    // 一時ファイルに書き込んでからリネームする（原子性を保つため）
    let temp_path = path.with_extension("downloading");
    {
        let mut file = fs::File::create(&temp_path).context("Failed to create temp file")?;
        file.write_all(&content)
            .context("Failed to write to temp file")?;
    }

    fs::rename(&temp_path, path).context("Failed to rename temp file to target")?;

    // ダウンロード完了後に SHA256 を計算しログ出力する。
    // 将来、sha256 フィールドに既知ハッシュを設定するための情報として利用する。
    if let Ok(hash) = compute_sha256(path) {
        log::info!(
            "Downloaded {} (SHA256: {})",
            path.file_name().unwrap_or_default().to_string_lossy(),
            hash
        );
    }

    Ok(())
}

/// ファイルの SHA256 ハッシュ（16進数文字列）を計算する
fn compute_sha256(path: &Path) -> Result<String> {
    let content = fs::read(path).context("Failed to read file for SHA256")?;
    let hash = Sha256::digest(&content);
    Ok(hex::encode(hash))
}
