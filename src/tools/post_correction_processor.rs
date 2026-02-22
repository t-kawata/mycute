use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// 補正バックエンドの抽象インターフェース
/// 音声認識機能は持たず、テキスト補正のみを行う
#[async_trait]
pub trait PostCorrectionBackend: Send + Sync {
    /// テキストを受け取り、補正されたテキストを返す
    async fn post_correct(&self, text: &str) -> Result<String>;
}

// ============================================================================
// SttModelType: エンジン特性を明示的に区分する列挙型
// ============================================================================

/// 音声認識モデルの特性を区分する列挙型
///
/// この区分により、補正プロセッサが「届いたテキストのセマンティクス」を正しく理解できます。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SttModelType {
    /// オフラインモデル（OpenAI Whisper 等）
    ///
    /// - 届くデータのセマンティクス: 「これは新しく増えた分（Delta / 増分パケット）です」
    /// - バッファ操作: 既存バッファの末尾に「追記（Append）」する
    /// - 用途: PseudoAsrStreamer（VAD で区切られた短い音声をバッチ推論する形態）
    #[default]
    UseOfflineModel,

    /// オンラインモデル（Apple Tahoe, Windows OS ディクテーション等）
    ///
    /// - 届くデータのセマンティクス: 「これが未確定区間の最新の状態（Live State）です」
    /// - バッファ操作: 未確定区間を「上書き（Overwrite / Replace）」する
    /// - 用途: MacSpeechBackend, WinSpeechBackend（OS がセッション全文をリアルタイムで送り続ける形態）
    UseOnlineModel,
}

/// 補正プロセッサの設定
#[derive(Clone, Debug)]
pub struct PostCorrectionConfig {
    /// 補正を行う文の数（閾値）
    pub sentence_count_threshold: usize,
    /// 補正を行う最小文字数（補助条件）
    pub min_text_length: usize,
    /// 補正を実行する最小間隔（ミリ秒）
    pub interval_ms: u64,
}

impl Default for PostCorrectionConfig {
    fn default() -> Self {
        Self {
            sentence_count_threshold: 3,
            min_text_length: 10,
            interval_ms: 2000,
        }
    }
}

/// プロセッサからの出力イベント
#[derive(Debug, Clone)]
pub enum ProcessorOutput {
    /// 途中経過（補正なし、または簡易補正）
    Partial(String),
    /// 確定結果（補正済み）
    Final(String),
}

/// 内部バッファの状態
#[derive(Debug, Default)]
struct ProcessorBuffer {
    /// 現在蓄積中の「未確定」テキスト (target_text)
    /// ポスト補正の対象となる
    target_text: String,

    /// 既に補正・確定済みだが、一つの大きな塊として管理しているテキスト (completed_text)
    /// 複数の補正単位を結合したもの
    completed_text: String,

    /// 外部に見せるための表示用テキスト (org_text)
    /// completed_text + target_text の状態になることが多い
    org_text: String,
}

impl ProcessorBuffer {
    fn clear(&mut self) {
        self.target_text.clear();
        self.completed_text.clear();
        self.org_text.clear();
    }
}

/// 最終補正レイヤープロセッサ
/// 入力テキストをバッファリングし、条件に応じてバックエンドによる補正を行い、
/// 確定（Final）と未確定（Partial）の出力を制御する。
pub struct PostCorrectionProcessor {
    pub backend: Arc<dyn PostCorrectionBackend>,
    config: PostCorrectionConfig,
    buffer: ProcessorBuffer,
    last_correction_time: Instant,
    /// エンジンの特性（オンライン/オフライン）
    model_type: SttModelType,
    /// 文字列置換リスト (from, to)
    replaces: Vec<(String, String)>,
    /// 発話状態（外部の VAD プロセッサから更新される）
    is_speaking: Arc<AtomicBool>,
    /// 補正の実行条件を満たしたかどうかの保留フラグ
    is_pending_correction: bool,
    /// 最後に沈黙が検知された（is_speaking が false になった）時刻
    last_silence_start: Option<Instant>,
}

impl PostCorrectionProcessor {
    pub fn new(
        backend: Arc<dyn PostCorrectionBackend>,
        config: PostCorrectionConfig,
        replaces: Vec<(String, String)>,
        is_speaking: Arc<AtomicBool>,
    ) -> Self {
        Self::with_model_type(
            backend,
            config,
            SttModelType::UseOfflineModel,
            replaces,
            is_speaking,
        )
    }

    /// 新しいプロセッサを作成（モデル種別を明示的に指定）
    pub fn with_model_type(
        backend: Arc<dyn PostCorrectionBackend>,
        config: PostCorrectionConfig,
        model_type: SttModelType,
        replaces: Vec<(String, String)>,
        is_speaking: Arc<AtomicBool>,
    ) -> Self {
        log::debug!(
            "[PostCorrectionProcessor] Initialized with model_type: {:?}, replaces_count: {}",
            model_type,
            replaces.len()
        );
        Self {
            backend,
            config,
            buffer: ProcessorBuffer::default(),
            last_correction_time: Instant::now(),
            model_type,
            replaces,
            is_speaking,
            is_pending_correction: false,
            last_silence_start: None,
        }
    }

    /// 入力テキストを処理する
    ///
    /// ## UseOfflineModel (従来動作)
    /// incoming_text は「新しく増えた分（差分）」として扱われ、バッファの末尾に**追記**されます。
    ///
    /// ## UseOnlineModel (オンラインモード)
    /// incoming_text は「未確定区間の最新状態（全体）」として扱われ、target_text を**上書き（置換）**します。
    /// これにより、OS 側でのバックトラック（過去の書き換え）にも正しく同期されます。
    pub fn process_input(&mut self, incoming_text: &str) -> Option<ProcessorOutput> {
        if incoming_text.trim().is_empty() {
            return None;
        }

        // 1. 文字列置換を適用 (スライスされる前の全文が揃った段階で行う)
        let processed_text = self.apply_replaces(incoming_text);

        match self.model_type {
            SttModelType::UseOfflineModel => {
                // ========================================
                // [Offline Path] 従来の追記動作（OpenAI用）
                // ========================================
                self.buffer.org_text.push_str(&processed_text);
                self.buffer.target_text.push_str(&processed_text);
            }
            SttModelType::UseOnlineModel => {
                // ========================================
                // [Online Path] 上書き動作（Tahoe等用）
                // processed_text は「未確定区間のLive State」なので、
                // target_text を丸ごと置換し、org_text も再構築する
                // ========================================
                self.buffer.target_text = processed_text;
                // org_text = 確定済み部分 + 最新の未確定部分
                self.buffer.org_text =
                    format!("{}{}", self.buffer.completed_text, self.buffer.target_text);
            }
        }

        // 補正条件のチェック (動的再評価)
        if self.should_trigger_correction(None) {
            if !self.is_pending_correction {
                log::debug!(
                    "[PostCorrectionProcessor] Correction threshold MET. Entering PENDING state."
                );
                self.is_pending_correction = true;
            }
        } else {
            if self.is_pending_correction {
                log::info!(
                    "[PostCorrectionProcessor] Correction threshold NO LONGER MET. Cancelling PENDING state."
                );
                self.is_pending_correction = false;
                self.last_silence_start = None; // 沈黙タイマーも破棄
            }
        }

        // 補正待機中に関わらず、表示用テキストとしては現在の累積を返す
        Some(ProcessorOutput::Partial(self.buffer.org_text.clone()))
    }

    pub fn check_and_start_silence_timer(&mut self) -> bool {
        if !self.is_pending_correction {
            return false;
        }

        let currently_speaking = self.is_speaking.load(Ordering::SeqCst);
        if currently_speaking {
            // 発話中なら猶予タイマーをリセットし続ける
            if self.last_silence_start.is_some() {
                log::debug!(
                    "[PostCorrectionProcessor] Speech detected again. Resetting silence timer."
                );
                self.last_silence_start = None;
            }
            return false;
        }

        // 沈黙の開始を検知
        if self.last_silence_start.is_none() {
            log::debug!("[PostCorrectionProcessor] Silence started. Starting grace period timer.");
            self.last_silence_start = Some(Instant::now());
        }

        // 猶予時間の判定
        if let Some(silence_start) = self.last_silence_start {
            use crate::constants::POST_CORRECTION_SILENCE_WAIT_MS;
            if silence_start.elapsed().as_millis() as u64 >= POST_CORRECTION_SILENCE_WAIT_MS {
                return true;
            }
        }

        false
    }

    /// 補正対象のテキストを取得する
    pub fn get_text_to_correct(&self) -> String {
        self.buffer.target_text.clone()
    }

    /// 補正結果を反映する（同期）
    pub fn commit_correction(&mut self, corrected_text: &str) -> ProcessorOutput {
        self.is_pending_correction = false;
        self.last_silence_start = None;
        self.last_correction_time = Instant::now();

        // 確定済みのバッファを更新
        self.buffer.completed_text.push_str(corrected_text);
        self.buffer.completed_text.push(' '); // 文の区切り

        // 未確定バッファをクリア
        self.buffer.target_text.clear();

        // 表示用バッファを同期
        self.buffer.org_text = self.buffer.completed_text.clone();

        log::debug!(
            "[PostCorrectionProcessor] Correction committed. New org_text len: {}",
            self.buffer.org_text.len()
        );

        ProcessorOutput::Final(self.buffer.org_text.clone())
    }

    /// 現在の状態で try_execute_pending_correction を呼んだ場合に、
    /// 実際に補正処理（LLM）が実行されるかどうかを判定します。
    pub fn will_execute_now(&self) -> bool {
        if !self.is_pending_correction {
            return false;
        }

        let currently_speaking = self.is_speaking.load(Ordering::SeqCst);
        if currently_speaking {
            return false;
        }

        if let Some(silence_start) = self.last_silence_start {
            use crate::constants::POST_CORRECTION_SILENCE_WAIT_MS;
            return silence_start.elapsed().as_millis() as u64 >= POST_CORRECTION_SILENCE_WAIT_MS;
        }

        false
    }

    /// 文字列置換を適用する内部メソッド
    fn apply_replaces(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (from, to) in &self.replaces {
            if !from.is_empty() {
                result = result.replace(from, to);
            }
        }
        result
    }

    /// 強制的に補正を実行して確定させる（無音検知時など）
    ///
    /// ## 挙動
    /// - 設定値（最小文字数、文数、経過時間）を満たしている場合: LLMで補正し、Final を返す
    /// - 設定値を満たしていない場合: LLMを呼ばず、現在のバッファ内容を Partial として返す
    ///   （バッファはクリアせず、ウォーターマークも進めない）
    ///
    /// これにより「即時表示・累積補正」が実現される。短い発話でもOSが確定させた瞬間に
    /// 画面には表示されるが、LLM補正は閾値に達するまで待機する。
    pub async fn force_commit(&mut self) -> Option<ProcessorOutput> {
        if self.buffer.target_text.trim().is_empty() {
            return None;
        }

        // 閾値チェック: 満たしていればLLM補正、満たしていなければ生のままPartialを返す
        if self.should_trigger_correction(None) {
            // 条件を満たした: LLM補正を実行し、Finalとして確定
            log::debug!(
                "[PostCorrectionProcessor] force_commit: Threshold MET, performing correction."
            );
            self.perform_correction().await
        } else {
            // 条件を満たさない: LLMを呼ばず、現在のバッファ内容をPartialとして返す
            // バッファはクリアしない（次の入力と合算されて閾値判定に再度かけられる）
            log::debug!(
                "[PostCorrectionProcessor] force_commit: Threshold NOT met (text_len={}, sentences={}). Returning Partial without LLM.",
                self.buffer.target_text.chars().count(),
                self.count_sentences()
            );
            Some(ProcessorOutput::Partial(self.buffer.org_text.clone()))
        }
    }

    /// 文の数をカウント (。！？.!?)
    fn count_sentences_in_text(text: &str) -> usize {
        text.matches('。').count()
            + text.matches('？').count()
            + text.matches('！').count()
            + text.matches('!').count()
            + text.matches('?').count()
            + text.matches('.').count()
    }

    fn count_sentences(&self) -> usize {
        Self::count_sentences_in_text(&self.buffer.target_text)
    }

    /// 補正を実行すべきかどうかを判定
    ///
    /// ## 引数
    /// - `incoming`: これから投入しようとしている新規テキスト（予測モード）。
    ///   None の場合は現在のバッファ状態のみで判定する。
    pub fn should_trigger_correction(&self, incoming: Option<&str>) -> bool {
        let text_len;
        let sentence_count;

        if let Some(text) = incoming {
            // 予測モード: incoming を適用した場合の状態をシミュレート
            let processed = self.apply_replaces(text);
            match self.model_type {
                SttModelType::UseOfflineModel => {
                    // 増分パケットなので単純加算
                    text_len = self.buffer.target_text.chars().count() + processed.chars().count();
                    sentence_count =
                        self.count_sentences() + Self::count_sentences_in_text(&processed);
                }
                SttModelType::UseOnlineModel => {
                    // 最新状態での上書きなので、引数のテキストのみを評価
                    text_len = processed.chars().count();
                    sentence_count = Self::count_sentences_in_text(&processed);
                }
            }
        } else {
            // 現状確認モード
            text_len = self.buffer.target_text.chars().count();
            sentence_count = self.count_sentences();
        }

        let len_ok = text_len >= self.config.min_text_length;

        // 経過時間チェック
        let elapsed_ms = self.last_correction_time.elapsed().as_millis() as u64;
        let time_ok = elapsed_ms >= self.config.interval_ms;

        // 文数が閾値に達している場合のみ補正を実行
        let sentence_ok = sentence_count >= self.config.sentence_count_threshold;

        let result = len_ok && time_ok && sentence_ok;

        log::debug!(
            "[PostCorrectionProcessor] should_trigger_correction? {} (mode: {}, len: {}/{} {}, time: {}/{} {}, sentence: {}/{} {})",
            result,
            if incoming.is_some() { "PREDICT" } else { "CURRENT" },
            text_len, self.config.min_text_length, if len_ok { "OK" } else { "SKIP" },
            elapsed_ms, self.config.interval_ms, if time_ok { "OK" } else { "SKIP" },
            sentence_count, self.config.sentence_count_threshold, if sentence_ok { "OK" } else { "SKIP" }
        );

        result
    }

    /// 補正を実行する内部メソッド
    async fn perform_correction(&mut self) -> Option<ProcessorOutput> {
        let text_to_correct = self.buffer.target_text.clone();

        match self.backend.post_correct(&text_to_correct).await {
            Ok(corrected) => {
                log::debug!(
                    "[PostCorrectionProcessor] Corrected: '{}' -> '{}'",
                    text_to_correct,
                    corrected
                );

                // 確定済みバッファに追記
                self.buffer.completed_text.push_str(&corrected);

                // org_text も確定済みのものと同期
                self.buffer.org_text = self.buffer.completed_text.clone();

                // 出力用テキスト（確定）
                let final_output = self.buffer.org_text.clone();

                // =================================================================
                // [CRITICAL FIX] 重複防止のためのリセット処理
                // FinalResult を出力するということは、そこまでのテキストは確定済みとなる。
                // したがって、内部バッファをクリアして「次の文」に備える。
                // 以前のセッションのテキストが残っていると、次のASR結果と結合されて重複が発生するため。
                // =================================================================
                self.buffer.clear();
                self.last_correction_time = Instant::now();

                log::debug!(
                    "[PostCorrectionProcessor] Corrected: '{}' -> '{}'. Buffer cleared.",
                    text_to_correct,
                    corrected
                );

                Some(ProcessorOutput::Final(final_output))
            }
            Err(e) => {
                log::error!("[PostCorrectionProcessor] Correction failed: {}", e);
                // 失敗時は補正なしで Partial として返すか、あるいは何もしないか。
                // 安全側に倒して、現状の org_text を Partial として返しておく（前回の状態維持）
                // ただし、失敗したからといってバッファクリアはしない（リトライの機会を残すため）
                Some(ProcessorOutput::Partial(self.buffer.org_text.clone()))
            }
        }
    }

    /// 強制的にリセットする（エラー時やストップ時など）
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.last_correction_time = Instant::now();
        self.is_pending_correction = false;
        self.last_silence_start = None;
    }

    /// 現在の表示用テキスト（org_text）を取得する
    /// UIのちらつき防止等のために、現在の内部バッファの状態を取得したい場合に使用する
    pub fn get_display_text(&self) -> String {
        self.buffer.org_text.clone()
    }

    /// 確定済みテキストの長さを返す（外部からのウォーターマーク追跡用）
    /// OnlineModel で「どこまでがキーボードで打ち込み済みか」を同期するために使用
    pub fn get_confirmed_len(&self) -> usize {
        self.buffer.completed_text.chars().count()
    }
}
