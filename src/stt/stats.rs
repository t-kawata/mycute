use crate::utils::time;
use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

/// 統計データの内部構造 (日付 -> (時 -> (モデル名 -> 指標オブジェクト)))
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UsageData {
    /// 日付ごとの統計。キーは "YYYY-MM-DD" 形式。
    /// 内側のマップは 1時間ごとの統計（キーは "00" から "23"）。
    /// さらにその内側はモデル名に対し、各指標（audio_ms, input_tokens 等）をマップ。
    #[serde(flatten)]
    pub daily: BTreeMap<String, BTreeMap<String, BTreeMap<String, BTreeMap<String, u64>>>>,
}

static BASE_PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

pub struct UsageStats;

impl UsageStats {
    /// ベースパスを設定する
    pub fn set_base_path(path: PathBuf) {
        let mutex = BASE_PATH.get_or_init(|| Mutex::new(None));
        let mut guard = mutex.lock();
        *guard = Some(path);
    }

    /// ベースパスを取得する
    fn get_base_path() -> Result<PathBuf> {
        let mutex = BASE_PATH
            .get()
            .ok_or_else(|| anyhow!("UsageStats not initialized with base path"))?;
        let guard = mutex.lock();
        guard
            .clone()
            .ok_or_else(|| anyhow!("UsageStats base path is None"))
    }

    /// 統計機能の初期化（起動時の書き込み権限チェック）
    pub fn init() -> Result<()> {
        let stats_dir = Self::get_base_path()?;

        // ディレクトリが存在しない場合は作成
        if !stats_dir.exists() {
            fs::create_dir_all(&stats_dir)
                .with_context(|| format!("Failed to create stats directory: {:?}", stats_dir))?;
        }

        // 書き込み権限チェック用のダミーファイル
        let test_file = stats_dir.join(".write_test");
        fs::write(&test_file, "test")
            .with_context(|| format!("No write permission in stats directory: {:?}", stats_dir))?;
        let _ = fs::remove_file(&test_file);

        log::debug!("[UsageStats] Statistics initialized in {:?}", stats_dir);
        Ok(())
    }

    /// 音声認識時間 (ASR) を記録する
    pub fn record_asr(model_name: &str, duration_ms: u64) -> Result<()> {
        Self::update_metric(model_name, "audio_ms", duration_ms)
    }

    /// LLM のトークン使用量を記録する
    pub fn record_llm(model_name: &str, input_tokens: u64, output_tokens: u64) -> Result<()> {
        Self::update_metric(model_name, "input_tokens", input_tokens)?;
        Self::update_metric(model_name, "output_tokens", output_tokens)
    }

    /// 指定した指標の値を更新する内部メソッド
    fn update_metric(model_name: &str, metric_key: &str, delta: u64) -> Result<()> {
        if delta == 0 {
            return Ok(());
        }

        let now = time::now();
        let date_str = now.format("%Y%m%d").to_string();
        let hour_str = now.format("%H").to_string();

        let stats_dir = Self::get_base_path()?;
        let stats_path = stats_dir.join(format!("usage_{}.json", date_str));

        // ディレクトリ作成
        if let Some(parent) = stats_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).context("Failed to create stats directory")?;
            }
        }

        // 既存データの読み込み
        let mut data = if stats_path.exists() {
            let content = fs::read_to_string(&stats_path).context("Failed to read stats file")?;
            serde_json::from_str::<UsageData>(&content).unwrap_or_default()
        } else {
            UsageData::default()
        };

        // データの更新 (日付 -> 時間 -> モデル -> 指標)
        let formatted_date = now.format("%Y-%m-%d").to_string();
        let daily_stats = data.daily.entry(formatted_date).or_default();
        let hourly_stats = daily_stats.entry(hour_str).or_default();
        let metrics = hourly_stats.entry(model_name.to_string()).or_default();

        let current_val = metrics.get(metric_key).cloned().unwrap_or(0);
        metrics.insert(metric_key.to_string(), current_val + delta);

        // 保存
        let json = serde_json::to_string_pretty(&data).context("Failed to serialize stats data")?;
        fs::write(&stats_path, json).context("Failed to write stats file")?;

        Ok(())
    }

    /// 利用可能な統計ファイル (usage_*.json) をリストアップし、ソートして返す
    pub fn list_usage_files() -> Result<Vec<PathBuf>> {
        let stats_dir = Self::get_base_path()?;
        if !stats_dir.exists() {
            return Ok(vec![]);
        }

        let mut files = vec![];
        for entry in fs::read_dir(stats_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("usage_") && name.ends_with(".json") {
                        files.push(path);
                    }
                }
            }
        }
        files.sort();
        Ok(files)
    }

    /// 統計ファイルを削除する
    pub fn delete_usage_file(path: &PathBuf) -> Result<()> {
        if path.exists() {
            fs::remove_file(path)
                .with_context(|| format!("Failed to delete stats file: {:?}", path))?;
        }
        Ok(())
    }

    /// 指定したファイルの統計を集計する
    pub fn get_aggregated_stats(path: &PathBuf) -> Result<BTreeMap<String, AggregatedMetrics>> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read stats file: {:?}", path))?;
        let data: UsageData =
            serde_json::from_str(&content).context("Failed to parse stats data")?;

        let mut aggregated = BTreeMap::new();

        // data.daily: BTreeMap<String, BTreeMap<String, BTreeMap<String, BTreeMap<String, u64>>>>
        // Date -> Hour -> Model -> Metric -> Value
        for daily_stats in data.daily.values() {
            for hourly_stats in daily_stats.values() {
                for (model_name, metrics) in hourly_stats {
                    let entry = aggregated
                        .entry(model_name.clone())
                        .or_insert(AggregatedMetrics::default());
                    entry.audio_ms += metrics.get("audio_ms").cloned().unwrap_or(0);
                    entry.input_tokens += metrics.get("input_tokens").cloned().unwrap_or(0);
                    entry.output_tokens += metrics.get("output_tokens").cloned().unwrap_or(0);
                }
            }
        }

        Ok(aggregated)
    }

    pub fn get_summary() -> Result<String> {
        let files = Self::list_usage_files()?;
        let mut total = BTreeMap::new();

        for path in files {
            if let Ok(aggregated) = Self::get_aggregated_stats(&path) {
                for (model, metrics) in aggregated {
                    let entry = total.entry(model).or_insert(AggregatedMetrics::default());
                    entry.audio_ms += metrics.audio_ms;
                    entry.input_tokens += metrics.input_tokens;
                    entry.output_tokens += metrics.output_tokens;
                }
            }
        }

        let mut summary = String::from("Usage Statistics Summary:\n");
        if total.is_empty() {
            summary.push_str("No data available.");
        } else {
            for (model, metrics) in total {
                summary.push_str(&format!(
                    "\nModel: {}\n  ASR: {:.2}s\n  LLM Input: {} tokens\n  LLM Output: {} tokens\n",
                    model,
                    metrics.audio_ms as f64 / 1000.0,
                    metrics.input_tokens,
                    metrics.output_tokens
                ));
            }
        }
        Ok(summary)
    }
}

#[derive(Debug, Default, Clone)]
pub struct AggregatedMetrics {
    pub audio_ms: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}
