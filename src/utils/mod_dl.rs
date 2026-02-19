use crate::stt_config::ConfigManager;
use std::fs;
use std::path::Path;
use reqwest::Client;
use std::io::Write;
use anyhow::{Context, Result};
use crate::constants::{
    MODEL_FILENAME_GTCRN,
    MODEL_FILENAME_SILERO_VAD,
    MODEL_FILENAME_SILERO_VAD_INT8,
    MODEL_FILENAME_TEN_VAD,
    MODEL_FILENAME_TEN_VAD_INT8,
    MODEL_FILENAME_TOKENS,
};

// モデル定義
struct ModelDef {
    filename: &'static str,
    url: &'static str,
    description: &'static str,
}

// Makefile から移植されたモデルリスト
const MODELS: &[ModelDef] = &[
    ModelDef {
        filename: MODEL_FILENAME_SILERO_VAD,
        url: "https://huggingface.co/t-kawata/mycute/resolve/main/silero_vad.onnx",
        description: "Silero VAD Model",
    },
    ModelDef {
        filename: MODEL_FILENAME_SILERO_VAD_INT8,
        url: "https://huggingface.co/t-kawata/mycute/resolve/main/silero_vad.int8.onnx",
        description: "Silero VAD Int8 Model",
    },
    ModelDef {
        filename: MODEL_FILENAME_TEN_VAD,
        url: "https://huggingface.co/t-kawata/mycute/resolve/main/ten_vad.onnx",
        description: "TEN VAD Model",
    },
    ModelDef {
        filename: MODEL_FILENAME_TEN_VAD_INT8,
        url: "https://huggingface.co/t-kawata/mycute/resolve/main/ten-vad.int8.onnx",
        description: "TEN VAD Int8 Model",
    },
    ModelDef {
        filename: MODEL_FILENAME_TOKENS,
        url: "https://huggingface.co/t-kawata/mycute/resolve/main/tokens.txt",
        description: "Tokens file",
    },
    ModelDef {
        filename: MODEL_FILENAME_GTCRN,
        url: "https://huggingface.co/t-kawata/mycute/resolve/main/gtcrn.onnx",
        description: "GTCRN Denoiser Model",
    },
];

/// 必要なモデルが全て揃っているか確認し、不足していればダウンロードする
pub async fn ensure_models(config_manager: &ConfigManager) -> Result<()> {
    // stt_config.rs の修正により、ConfigManager::new で model_dir が必ず解決されている前提
    // Settings 内の model_dir を参照
    let model_dir_str = {
        let settings = config_manager.settings.read();
        settings.stt.model_dir.clone().unwrap_or_default()
    };

    if model_dir_str.is_empty() {
        return Err(anyhow::anyhow!("CRITICAL: model_dir is not set in ConfigManager."));
    }

    let model_dir = Path::new(&model_dir_str);
    let client = Client::new();

    for model in MODELS {
        let file_path = model_dir.join(model.filename);
        if !file_path.exists() {
            log::info!("Downloading {} ({}) ...", model.description, model.url);
            download_file(&client, model.url, &file_path).await?;
            log::info!("Downloaded: {}", model.filename);
        } else {
            log::debug!("Model exists: {}", model.filename);
        }
    }

    Ok(())
}

async fn download_file(client: &Client, url: &str, path: &Path) -> Result<()> {
    let response = client.get(url).send().await.context("Failed to send request")?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to download file: status {}", response.status()));
    }

    let content = response.bytes().await.context("Failed to read response body")?;
    
    // 一時ファイルに書き込んでからリネームする（原子性を保つため）
    let temp_path = path.with_extension("downloading");
    {
        let mut file = fs::File::create(&temp_path).context("Failed to create temp file")?;
        file.write_all(&content).context("Failed to write to temp file")?;
    }

    fs::rename(&temp_path, path).context("Failed to rename temp file to target")?;

    Ok(())
}
