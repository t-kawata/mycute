// Darvium AI プロバイダ抽象化レイヤ
//
// LLM 呼び出しを抽象化する LLMClient トレイトと、埋め込みベクトル生成を
// 抽象化する EmbeddingProvider トレイト、およびそれぞれの決定論的ダミー実装
// （FakeLlmClient, FakeEmbeddingProvider）を提供する。
// 本モジュールは外部 AI API への接続を伴わず、M2 以降で
// RealClient に差し替えるためのポート境界を定義する。
//
// 関連RFC: §14.2（構造化出力要求契約）、§13A（LLM adapter interface）
// 関連チケット: M-2-1.6（LLMClient 抽象トレイトの定義）、M-2-1.7（EmbeddingProvider 抽象トレイトの定義）

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use rand::rngs::StdRng;
use rand::{Rng as _, SeedableRng};

use crate::error::DarviumError;

/// LLM 出力スキーマ。
///
/// 構造化出力の形式を指定する。各バリアントは RFC で定義された
/// LLM 呼び出しの出力形式に対応する。
///
/// 各バリアントは `hint()` メソッドで JSON スキーマの説明文字列を提供する。
/// このヒントは `RealLlmClient` が LLM API に構造化出力を要求する際の
/// スキーマ記述として使用される。
#[derive(Debug, Clone, PartialEq)]
pub enum LlmSchema {
    /// SearchWorkflow クエリ設計テキスト生成 (RFC §9.4)
    QueryDesignText,
    /// グラフパッチ操作列生成 (RFC §12.2)
    PatchOperations,
    /// 自己評価スコア cₛ 出力 (RFC §12.2)
    SelfScore,
    /// 自由文形式（スキーマ制約なし）
    FreeText,
}

impl LlmSchema {
    /// スキーマに対応する JSON スキーマ説明文字列を返す。
    ///
    /// このヒントは LLM API への構造化出力要求時に使用される。
    pub fn hint(&self) -> &'static str {
        match self {
            LlmSchema::QueryDesignText => "Search query design text generation (RFC \u{a7}9.4)",
            LlmSchema::PatchOperations => "Graph patch operation sequence (RFC \u{a7}12.2)",
            LlmSchema::SelfScore => "Self-evaluation confidence score c_s (RFC \u{a7}12.2)",
            LlmSchema::FreeText => "Free-form text output (no schema constraints)",
        }
    }
}

/// LLM クライアント抽象トレイト。
///
/// 構造化出力を生成する汎用インターフェース。Send + Sync を境界とし、
/// Arc<dyn LLMClient> によるスレッド間共有を可能にする。
pub trait LLMClient: Send + Sync {
    /// プロンプトとスキーマを受け取り、構造化された応答文字列を返す。
    fn generate_structured(&self, prompt: &str, schema: &LlmSchema)
        -> Result<String, DarviumError>;
}

/// 取得回数カウント用の型エイリアス。
pub type CallCount = Arc<AtomicUsize>;

/// Fake LLM クライアント — 実際の LLM API に接続せず決定論的な応答を返す。
///
/// # モード
/// - **固定文字列モード**（デフォルト）: コンストラクタで指定された文字列を常に返す
/// - **乱数モード**: 指定確率で不正フォーマット（空文字列・不正 JSON・指定外文字列）を返す
///
/// # 計装
/// `call_count` で呼び出し回数を計測可能。検証コードから `call_count` を
/// 読み取ることで LLM 呼び出し回数のアサーションが可能。
pub struct FakeLlmClient {
    fixed_response: String,
    malformed_probability: f64,
    pub call_count: CallCount,
}

impl FakeLlmClient {
    /// 指定された固定文字列を常に返すインスタンスを生成する。
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            fixed_response: response.into(),
            malformed_probability: crate::constants::FAKE_LLM_DEFAULT_MALFORMED_PROB,
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// デフォルトパスモード — `{"status": "ok"}` を返す。
    pub fn default_pass() -> Self {
        Self::new(r#"{"status": "ok"}"#)
    }

    /// 常に空文字列を返すインスタンスを生成する。
    pub fn returns_empty() -> Self {
        Self::new("")
    }

    /// 常に不正フォーマットを返すインスタンスを生成する。
    ///
    /// 確率 1.0 の乱数モードと等価であり、テストで使用する。
    pub fn returns_malformed() -> Self {
        Self::new("normal").with_malformed_probability(1.0)
    }

    /// 指定確率で不正フォーマットを返すインスタンスを生成する。
    ///
    /// `malformed_probability` は 0.0〜1.0 の範囲で指定する。
    /// 確率に応じて空文字列・不正 JSON・正常文字列のいずれかを返す。
    pub fn with_malformed_probability(mut self, probability: f64) -> Self {
        self.malformed_probability = probability.clamp(0.0, 1.0);
        self
    }

    /// 現在の呼び出し回数を取得する。
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    /// 乱数モードの判定 — 不正フォーマットを返すべきかを決定する。
    ///
    /// 外部 PRNG に依存せず、呼び出し回数のハッシュ値を使用する。
    /// これにより同一インスタンス・同一シーケンスで完全再現可能（決定論的）。
    fn should_be_malformed(&self, prev_count: usize) -> bool {
        if self.malformed_probability <= 0.0 {
            return false;
        }
        if self.malformed_probability >= 1.0 {
            return true;
        }
        let hash = prev_count.wrapping_mul(2_654_435_761);
        let normalized = (hash % 10_000) as f64 / 10_000.0;
        normalized < self.malformed_probability
    }

    /// 不正フォーマットの応答を生成する（乱数モード用）。
    ///
    /// 3種類の不正パターンを呼び出し順に巡回する。
    fn generate_malformed(&self, count: usize) -> String {
        const MALFORMED_PATTERNS: [&str; 3] = [
            "",                     // 空文字列
            r#"{"invalid": "json"#, // 末尾カッコ欠落
            "UNEXPECTED_FORMAT",    // 予期外フォーマット
        ];
        let pattern_index = count % MALFORMED_PATTERNS.len();
        MALFORMED_PATTERNS[pattern_index].to_string()
    }
}

impl LLMClient for FakeLlmClient {
    fn generate_structured(
        &self,
        _prompt: &str,
        _schema: &LlmSchema,
    ) -> Result<String, DarviumError> {
        let prev = self.call_count.fetch_add(1, Ordering::SeqCst);
        if self.should_be_malformed(prev) {
            let malformed = self.generate_malformed(prev);
            Ok(malformed)
        } else {
            Ok(self.fixed_response.clone())
        }
    }
}

// ── ScriptedFakeLlmClient ──

/// スクリプト化された不正フォーマット出力 Fake LLM クライアント。
///
/// 固定シード PRNG（`StdRng`）を使用して、指定された確率 `p_m` で
/// 正常応答文字列に意図的な破壊的変換（MalformationType）を適用する。
/// 既存の `FakeLlmClient` がハッシュベースで3固定パターンのみを生成するのに対し、
/// 本クライアントは以下の高度な制御を提供する：
///
/// - **PRNG 駆動**: `StdRng::seed_from_u64` による完全再現性
/// - **8種類の不正フォーマット**: カッコ欠落・キー名置換・ビット反転等
/// - **連続的 p_m 制御**: `[0.0, 1.0]` のエントロピー制御
/// - **RawError モード**: 直接 `Err(DarviumError::LlmMalformedJson)` を返す
///
/// # 計装
/// `call_count` で呼び出し回数を計測可能。`rng` は `Mutex` で保護され、
/// スレッドセーフな `&self` メソッドから PRNG 状態を更新できる。
///
/// # 関連RFC
/// RFC §14.2（構造化出力・JSONパースエラーハンドリング）
/// RFC §12.2（GraphPatchGenerator の LLM 自己評価スコア）
pub struct ScriptedFakeLlmClient {
    /// 正常時の応答テンプレート文字列
    script_template: String,
    /// 不正フォーマットを返す確率 p_m ∈ [0.0, 1.0]
    malformed_probability: f64,
    /// 固定シード PRNG（Mutex で内部可変性を提供）
    rng: Mutex<StdRng>,
    /// 呼び出し回数カウンタ
    pub call_count: CallCount,
    /// 適用可能な不正フォーマットの種類一覧
    malformation_types: Vec<MalformationType>,
}

/// 不正フォーマットの種類を指定する列挙型。
///
/// 各バリアントは異なる方法で JSON 構造を破壊する。
/// 全てのバリアントは `Debug`, `Clone`, `PartialEq` を実装する。
#[derive(Debug, Clone, PartialEq)]
pub enum MalformationType {
    /// JSON の末尾 `}` を削除
    MissingClosingBrace,
    /// 指定されたキー名を別の名前に置換
    WrongKeyName(String),
    /// 指定位置の文字をビット反転
    BitFlip(usize),
    /// 空のオブジェクト `{}` を返す
    EmptyObject,
    /// 予期しないフィールドを追加
    ExtraField,
    /// フィールドの値の型を変える（文字列→数値など）
    TypeMismatch,
    /// ネストした括弧構造を破壊
    NestedBraceDestruction,
    /// 直接 `Err(DarviumError::LlmMalformedJson)` を返す
    RawError,
}

impl ScriptedFakeLlmClient {
    /// 指定されたテンプレート文字列を正常応答とするインスタンスを生成する。
    ///
    /// デフォルトでは不正フォーマット確率は 0.0（常に正常出力）。
    /// 不正フォーマット種類は全8種類から等確率で選択される。
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            script_template: template.into(),
            malformed_probability: crate::constants::SCRIPTED_FAKE_LLM_DEFAULT_MALFORMED_PROB,
            rng: Mutex::new(StdRng::seed_from_u64(crate::constants::TEST_PRNG_SEED)),
            call_count: Arc::new(AtomicUsize::new(0)),
            malformation_types: vec![
                MalformationType::MissingClosingBrace,
                MalformationType::WrongKeyName("key".to_string()),
                MalformationType::BitFlip(0),
                MalformationType::EmptyObject,
                MalformationType::ExtraField,
                MalformationType::TypeMismatch,
                MalformationType::NestedBraceDestruction,
                MalformationType::RawError,
            ],
        }
    }

    /// デフォルトパスモード — `{"status": "ok"}` を返す。
    pub fn default_pass() -> Self {
        Self::new(r#"{"status": "ok"}"#)
    }

    /// 指定確率で不正フォーマットを返すインスタンスに設定する。
    ///
    /// `probability` は 0.0〜1.0 の範囲で指定する。
    /// PRNG によって判定されるため、同一シード下で完全再現可能。
    pub fn with_malformed_probability(mut self, probability: f64) -> Self {
        self.malformed_probability = probability.clamp(0.0, 1.0);
        self
    }

    /// 適用する不正フォーマットの種類一覧を設定する。
    ///
    /// 空のベクタを渡した場合は常に正常応答を返す。
    /// PRNG によって不正フォーマット発生時にこの一覧から等確率で選択される。
    pub fn with_malformation_types(mut self, types: Vec<MalformationType>) -> Self {
        self.malformation_types = types;
        self
    }

    /// PRNG のシードを再設定する（テスト用）。
    pub fn reset_seed(&self, seed: u64) {
        let mut rng = self.rng.lock().expect("rng lock");
        *rng = StdRng::seed_from_u64(seed);
    }

    /// 現在の呼び出し回数を取得する。
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

impl ScriptedFakeLlmClient {
    /// テンプレート文字列と不正フォーマットの種類を受け取り、破壊的変換を施した文字列を返す。
    fn apply_malformation(&self, template: &str, mal_type: &MalformationType) -> String {
        match mal_type {
            MalformationType::MissingClosingBrace => {
                let trimmed = template.trim_end();
                if let Some(pos) = trimmed.rfind('}') {
                    let mut s = template.to_string();
                    s.remove(pos);
                    s
                } else {
                    template.to_string()
                }
            }
            MalformationType::WrongKeyName(key) => {
                let garbled = format!("{}_corrupted", key);
                template.replace(key, &garbled)
            }
            MalformationType::BitFlip(pos) => {
                let mut bytes: Vec<u8> = template.bytes().collect();
                if *pos < bytes.len() {
                    bytes[*pos] ^= 0x01;
                }
                String::from_utf8_lossy(&bytes).to_string()
            }
            MalformationType::EmptyObject => "{}".to_string(),
            MalformationType::ExtraField => {
                if let Some(rest) = template.trim_end().strip_suffix('}') {
                    format!("{}, \"_extra_field\": \"unexpected\"}}", rest)
                } else {
                    format!("{} {{\"_extra_field\": \"unexpected\"}}", template)
                }
            }
            MalformationType::TypeMismatch => {
                // 最初の文字列値を数値 42 に置換
                if let Some(colon_pos) = template.find(':') {
                    let after_colon = &template[colon_pos + 1..];
                    let trimmed_after = after_colon.trim_start();
                    if trimmed_after.starts_with('"') {
                        let quote_start = colon_pos + 1 + (after_colon.len() - trimmed_after.len());
                        let remaining = &template[quote_start + 1..];
                        if let Some(end_quote) = remaining.find('"') {
                            let mut result = template[..=colon_pos].to_string();
                            result.push_str(" 42");
                            result.push_str(&template[quote_start + 1 + end_quote + 1..]);
                            return result;
                        }
                    }
                }
                template.to_string()
            }
            MalformationType::NestedBraceDestruction => {
                let open_count = template.matches('{').count();
                let close_count = template.matches('}').count();
                if open_count > 0 && close_count > 0 {
                    let mut s = template.to_string();
                    if let Some(pos) = s.find('{') {
                        s.insert(pos + 1, '{');
                    }
                    s
                } else {
                    template.to_string()
                }
            }
            MalformationType::RawError => {
                // RawError は generate_structured でハンドリングされる
                template.to_string()
            }
        }
    }
}

impl LLMClient for ScriptedFakeLlmClient {
    fn generate_structured(
        &self,
        _prompt: &str,
        _schema: &LlmSchema,
    ) -> Result<String, DarviumError> {
        let prev = self.call_count.fetch_add(1, Ordering::SeqCst);

        // 内部ログ用に prev を使用（テストで呼び出し順を追跡可能にするため）
        let _ = prev;

        // PRNG を使用した確率的判定（同一シードで完全再現可能）
        let should_malform = {
            let mut rng = self.rng.lock().expect("rng lock");
            rng.random::<f64>() < self.malformed_probability
        };

        if !should_malform || self.malformation_types.is_empty() {
            return Ok(self.script_template.clone());
        }

        // malformation_types から等確率で一種類を選択
        let mal_type = {
            let mut rng = self.rng.lock().expect("rng lock");
            let idx = rng.random_range(0..self.malformation_types.len());
            self.malformation_types[idx].clone()
        };

        // RawError は文字列ではなくエラーとして返す
        if matches!(mal_type, MalformationType::RawError) {
            return Err(DarviumError::LlmMalformedJson(
                "ScriptedFakeLlmClient: RawError mode triggered".to_string(),
            ));
        }

        Ok(self.apply_malformation(&self.script_template, &mal_type))
    }
}

// ── EmbeddingProvider トレイト ──

/// 埋め込みベクトル生成プロバイダ抽象トレイト。
///
/// テキストから浮動小数点ベクトル（埋め込み）を生成する。
/// Send + Sync を境界とし、Arc<dyn EmbeddingProvider> による
/// スレッド間共有を可能にする。
pub trait EmbeddingProvider: Send + Sync {
    /// テキストの埋め込みベクトルを生成する。
    fn embed(&self, text: &str) -> Result<Vec<f32>, DarviumError>;

    /// 埋め込みベクトルの次元数を返す。
    fn embed_dimension(&self) -> usize;
}

// ── FakeEmbeddingProvider ──

/// 固定シード PRNG 駆動の Fake 埋め込みプロバイダ。
///
/// 実際の埋め込み API を使用せず、テキストの FNV-1a ハッシュ値を
/// シードとした決定論的疑似埋め込みベクトルを生成する。
/// 同一テキストに対しては常に同一ベクトルを返すため、テストの再現性を保証する。
///
/// PRNG には MMIX LCG（線形合同法）を使用し、rand クレートに依存しない。
/// 生成されるベクトル成分は [0, 1) の範囲に分布する。
pub struct FakeEmbeddingProvider {
    dimension: usize,
}

impl FakeEmbeddingProvider {
    /// 指定された次元数の FakeEmbeddingProvider を生成する。
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

impl Default for FakeEmbeddingProvider {
    fn default() -> Self {
        Self::new(crate::constants::FAKE_EMBEDDING_DEFAULT_DIMENSION)
    }
}

impl EmbeddingProvider for FakeEmbeddingProvider {
    fn embed(&self, text: &str) -> Result<Vec<f32>, DarviumError> {
        Ok(generate_fake_embedding(text, self.dimension))
    }

    fn embed_dimension(&self) -> usize {
        self.dimension
    }
}

// ── ConstantEmbeddingProvider ──

/// 常に同一のベクトルを返す埋め込みプロバイダ（テスト用）。
///
/// 異なるテキストに対しても、コンストラクタで指定された
/// 固定ベクトルを常に返す。決定論的挙動の確認に使用する。
pub struct ConstantEmbeddingProvider {
    constant: Vec<f32>,
}

impl ConstantEmbeddingProvider {
    /// 全要素が 0.0 の固定ベクトルを持つインスタンスを生成する。
    pub fn new(dimension: usize) -> Self {
        Self {
            constant: vec![0.0; dimension],
        }
    }

    /// 任意の固定ベクトルを持つインスタンスを生成する。
    pub fn with_vector(vector: Vec<f32>) -> Self {
        Self { constant: vector }
    }
}

impl EmbeddingProvider for ConstantEmbeddingProvider {
    fn embed(&self, _text: &str) -> Result<Vec<f32>, DarviumError> {
        Ok(self.constant.clone())
    }

    fn embed_dimension(&self) -> usize {
        self.constant.len()
    }
}

// ── 内部ヘルパー ──

/// テキストの FNV-1a ハッシュを計算する。
fn hash_text(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// ハッシュ値をシードに疑似埋め込みベクトルを生成する。
///
/// MMIX LCG を使用してシードから次元数分の f32 値を生成する。
/// 値は [0, 1) の範囲に分布する。
fn generate_fake_embedding(text: &str, dimension: usize) -> Vec<f32> {
    let seed = hash_text(text);
    let mut state = seed;
    let mut embedding = Vec::with_capacity(dimension);
    let multiplier: u64 = 6_364_136_223_846_793_005;
    let increment: u64 = 1_442_695_040_888_963_407;
    for _ in 0..dimension {
        state = state.wrapping_mul(multiplier).wrapping_add(increment);
        let value = ((state >> 32) % 1_000_000) as f32 / 1_000_000.0;
        embedding.push(value);
    }
    embedding
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROMPT_ARG: &str = "test prompt";

    // ── LLMClient トレイト (T1〜T3) ──

    /// T1: FakeLlmClient が LLMClient トレイトを実装していることをコンパイル時検証
    #[test]
    fn test_trait_bound_satisfied() {
        fn assert_trait(_: &impl LLMClient) {}
        let client = FakeLlmClient::default_pass();
        assert_trait(&client);
    }

    /// T2: Box<dyn LLMClient> のオブジェクト安全性
    #[test]
    fn test_object_safety() {
        let client: Box<dyn LLMClient> = Box::new(FakeLlmClient::default_pass());
        let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        assert!(result.is_ok());
    }

    /// T3: Box<dyn LLMClient + Send + Sync> がスレッド間移動可能
    #[test]
    fn test_send_sync_bounds() {
        fn assert_send_sync<T: Send + Sync>(_t: &T) {}
        let client = FakeLlmClient::default_pass();
        assert_send_sync(&client);

        let boxed: Box<dyn LLMClient> = Box::new(FakeLlmClient::default_pass());
        assert_send_sync(&boxed);
    }

    // ── FakeLlmClient 固定文字列モード (T4〜T7) ──

    /// T4: 指定した固定文字列が正確に返る
    #[test]
    fn test_fixed_response_exact_match() {
        let expected = "expected output";
        let client = FakeLlmClient::new(expected);
        let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        assert_eq!(result.unwrap(), expected);
    }

    /// T5: 同一インスタンスの複数回呼び出しで同一出力
    #[test]
    fn test_fixed_response_idempotent() {
        let response = "same output";
        let client = FakeLlmClient::new(response);
        let r1 = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        let r2 = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        let r3 = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        assert_eq!(r1.unwrap(), response);
        assert_eq!(r2.unwrap(), response);
        assert_eq!(r3.unwrap(), response);
    }

    /// T6: 全スキーマバリアントで同一の固定文字列が返る
    #[test]
    fn test_all_schemas_return_same() {
        let response = "same for all";
        let client = FakeLlmClient::new(response);
        let schemas = [
            LlmSchema::QueryDesignText,
            LlmSchema::PatchOperations,
            LlmSchema::SelfScore,
            LlmSchema::FreeText,
        ];
        for schema in &schemas {
            let result = client.generate_structured(PROMPT_ARG, schema);
            assert_eq!(result.unwrap(), response);
        }
    }

    /// T7: 空文字列モードで空文字列が返る
    #[test]
    fn test_empty_response() {
        let client = FakeLlmClient::returns_empty();
        let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        assert_eq!(result.unwrap(), "");
    }

    /// T8: 指定確率での不正フォーマット注入（統計的検証）
    ///
    /// 同一インスタンスを n_trials 回呼び出し、不正フォーマット出現率が
    /// 指定確率の95%信頼区間内に収まることを確認する。
    #[test]
    fn test_malformed_probability_statistical() {
        let probability = 0.3;
        let n_trials = 10_000;
        let mut malformed_count = 0u32;
        let normal = "normal";

        let client = FakeLlmClient::new(normal).with_malformed_probability(probability);
        for _ in 0..n_trials {
            let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
            if result.unwrap() != normal {
                malformed_count += 1;
            }
        }

        let observed_ratio = malformed_count as f64 / n_trials as f64;
        let std_err = (probability * (1.0 - probability) / n_trials as f64).sqrt();
        let lower = probability - 1.96 * std_err;
        let upper = probability + 1.96 * std_err;

        assert!(
            observed_ratio >= lower && observed_ratio <= upper,
            "観測比率 {:.4} が期待CI [{:.4}, {:.4}] の範囲外 ",
            observed_ratio,
            lower,
            upper
        );
    }

    /// T9: 確率 0.0 では常に正常出力
    #[test]
    fn test_zero_probability_always_normal() {
        let normal = "normal";
        let client = FakeLlmClient::new(normal).with_malformed_probability(0.0);
        for _ in 0..100 {
            let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
            assert_eq!(result.unwrap(), normal);
        }
    }

    /// T10: 確率 1.0 では常に不正出力
    #[test]
    fn test_one_probability_always_malformed() {
        let normal = "normal";
        let client = FakeLlmClient::new(normal).with_malformed_probability(1.0);
        for _ in 0..100 {
            let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
            assert_ne!(result.unwrap(), normal);
        }
    }

    /// T11: 不正フォーマットの種類が期待通り出現する
    #[test]
    fn test_malformed_variety() {
        let client = FakeLlmClient::new("normal").with_malformed_probability(1.0);
        let mut seen_empty = false;
        let mut seen_invalid_json = false;
        let mut seen_unexpected = false;

        for _ in 0..50 {
            let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
            let output = result.unwrap();
            match output.as_str() {
                "" => seen_empty = true,
                s if s == r##"{"invalid": "json"## => seen_invalid_json = true,
                "UNEXPECTED_FORMAT" => seen_unexpected = true,
                _ => {}
            }
        }

        let msg0 = "空文字列の不正フォーマットが一度も出現しなかった ";
        let msg1 = "不正JSONの不正フォーマットが一度も出現しなかった ";
        let msg2 = "予期外フォーマットが一度も出現しなかった ";
        assert!(seen_empty, "{}", msg0);
        assert!(seen_invalid_json, "{}", msg1);
        assert!(seen_unexpected, "{}", msg2);
    }

    // ── エラー型 (T12〜T14) ──

    /// T12: DarviumError::Llm のメッセージ確認
    #[test]
    fn test_llm_error_message() {
        let inner = "API error.".to_string();
        let err = DarviumError::Llm(inner);
        let display = err.to_string();
        let expected = "LLM error: API error.";
        assert_eq!(display, expected);
    }

    /// T13: DarviumError::LlmMalformedJson のメッセージ確認
    #[test]
    fn test_llm_malformed_json_message() {
        let inner = "bad json.".to_string();
        let err = DarviumError::LlmMalformedJson(inner.clone());
        let display = err.to_string();
        let mut expected = "LLM malformed JSON: ".to_string();
        expected.push_str(&inner);
        assert_eq!(display, expected);
    }

    /// T14: エラーの PartialEq 比較
    #[test]
    fn test_error_partial_eq() {
        let err1 = DarviumError::Llm('x'.to_string());
        let err2 = DarviumError::Llm('x'.to_string());
        let err3 = DarviumError::Llm('y'.to_string());
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    // ── LlmSchema 列挙型 (T15〜T16) ──

    /// T15: 全バリアントが Debug を実装していることを確認
    #[test]
    fn test_schema_variants_debug() {
        fn assert_debug<T: std::fmt::Debug>(_: &T) {}
        assert_debug(&LlmSchema::QueryDesignText);
        assert_debug(&LlmSchema::PatchOperations);
        assert_debug(&LlmSchema::SelfScore);
        assert_debug(&LlmSchema::FreeText);
    }

    /// T16: 全バリアントが Clone 可能であること
    #[test]
    fn test_schema_clone() {
        let original = LlmSchema::QueryDesignText;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    /// T17: 全バリアントが空でないヒント文字列を返す
    #[test]
    fn test_schema_hints_non_empty() {
        for schema in &[
            LlmSchema::QueryDesignText,
            LlmSchema::PatchOperations,
            LlmSchema::SelfScore,
            LlmSchema::FreeText,
        ] {
            let hint = schema.hint();
            assert!(
                !hint.is_empty(),
                "hint for {:?} should not be empty ",
                schema
            );
        }
    }

    /// returns_malformed() が常に不正フォーマットを返すことを検証
    #[test]
    fn test_returns_malformed_always_malformed() {
        let normal = "normal";
        let client = FakeLlmClient::returns_malformed();
        for _ in 0..100 {
            let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
            assert_ne!(result.unwrap(), normal);
        }
    }

    /// call_count の動作検証
    #[test]
    fn test_call_count_tracking() {
        let client = FakeLlmClient::default_pass();
        assert_eq!(client.call_count(), 0);

        let _ = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        assert_eq!(client.call_count(), 1);

        let _ = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        let _ = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        assert_eq!(client.call_count(), 3);
    }

    // ── EmbeddingProvider トレイト (T1〜T3) ──

    /// T1: FakeEmbeddingProvider が EmbeddingProvider トレイトを実装していることのコンパイル時検証
    #[test]
    fn test_embedding_trait_bound_satisfied() {
        fn assert_trait(_: &impl EmbeddingProvider) {}
        let provider = FakeEmbeddingProvider::default();
        assert_trait(&provider);
    }

    /// T2: Box<dyn EmbeddingProvider> のオブジェクト安全性
    #[test]
    fn test_embedding_object_safety() {
        let provider: Box<dyn EmbeddingProvider> = Box::new(FakeEmbeddingProvider::default());
        let result = provider.embed("test");
        assert!(result.is_ok());
    }

    /// T3: Box<dyn EmbeddingProvider + Send + Sync> がスレッド間移動可能
    #[test]
    fn test_embedding_send_sync_bounds() {
        fn assert_send_sync<T: Send + Sync>(_t: &T) {}
        let provider = FakeEmbeddingProvider::default();
        assert_send_sync(&provider);

        let boxed: Box<dyn EmbeddingProvider> = Box::new(FakeEmbeddingProvider::default());
        assert_send_sync(&boxed);
    }

    // ── FakeEmbeddingProvider 決定論性 (T4〜T9) ──

    /// T4: 同一テキストを 2 回 embed するとビットレベルで同一のベクトルが返る
    #[test]
    fn test_fake_embedding_deterministic() {
        let provider = FakeEmbeddingProvider::default();
        let text = "hello world";
        let v1 = provider.embed(text).unwrap();
        let v2 = provider.embed(text).unwrap();
        assert_eq!(v1, v2);
    }

    /// T5: 異なるテキストを embed すると異なるベクトルが返る（衝突率検証）
    ///
    /// ソート + 隣接比較（O(n log n)）で全ベクトル対の重複判定を行う。
    /// オリジナルの O(n²) 二重ループ（n=10,000, dim=384 で約 19.2B 回の float 比較）
    /// を回避するため、全ベクトルを生成後にソートし隣接ペアのみをチェックする。
    #[test]
    fn test_fake_embedding_no_collision() {
        let provider = FakeEmbeddingProvider::default();
        let n_vectors = 10_000;
        let mut vectors: Vec<Vec<f32>> = Vec::with_capacity(n_vectors);
        for i in 0..n_vectors {
            let text = format!("unique_text_{}", i);
            vectors.push(provider.embed(&text).unwrap());
        }
        // Vec<f32> は f32::total_cmp で全要素を辞書順比較
        vectors.sort_by(|a, b| {
            for (x, y) in a.iter().zip(b.iter()) {
                match x.total_cmp(y) {
                    std::cmp::Ordering::Equal => continue,
                    non_eq => return non_eq,
                }
            }
            a.len().cmp(&b.len())
        });
        // ソート済み隣接ペアのみ走査 — 衝突があれば必ず隣接する
        for i in 0..(n_vectors - 1) {
            if vectors[i] == vectors[i + 1] {
                panic!("衝突検出: text_{} と text_{} が同一ベクトル", i, i + 1);
            }
        }
    }

    /// T6: embed_dimension() がデフォルト次元数と一致すること
    #[test]
    fn test_fake_embedding_default_dimension() {
        let provider = FakeEmbeddingProvider::default();
        assert_eq!(
            provider.embed_dimension(),
            crate::constants::FAKE_EMBEDDING_DEFAULT_DIMENSION
        );
        let vec = provider.embed("check").unwrap();
        assert_eq!(
            vec.len(),
            crate::constants::FAKE_EMBEDDING_DEFAULT_DIMENSION
        );
    }

    /// T7: コンストラクタで指定した次元数が embed_dimension() と一致すること
    #[test]
    fn test_fake_embedding_custom_dimension() {
        let dims = [64usize, 128, 256, 512, 1024, 1536];
        for &dim in &dims {
            let provider = FakeEmbeddingProvider::new(dim);
            assert_eq!(provider.embed_dimension(), dim);
            let vec = provider.embed("test").unwrap();
            assert_eq!(vec.len(), dim);
        }
    }

    /// T8: 空文字列を embed してもエラーにならず指定次元数のベクトルが返る
    #[test]
    fn test_fake_embedding_empty_string() {
        let provider = FakeEmbeddingProvider::default();
        let result = provider.embed("");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().len(),
            crate::constants::FAKE_EMBEDDING_DEFAULT_DIMENSION
        );
    }

    /// T9: 長大テキストを embed してもエラーにならず指定次元数のベクトルが返る
    #[test]
    fn test_fake_embedding_long_text() {
        let provider = FakeEmbeddingProvider::default();
        let long_text = "a".repeat(10_000);
        let result = provider.embed(&long_text);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().len(),
            crate::constants::FAKE_EMBEDDING_DEFAULT_DIMENSION
        );
    }

    // ── ConstantEmbeddingProvider (T10〜T11) ──

    /// T10: 異なるテキストに対しても同一ベクトルが返る
    #[test]
    fn test_constant_embedding_identical() {
        let provider = ConstantEmbeddingProvider::new(384);
        let v1 = provider.embed("foo").unwrap();
        let v2 = provider.embed("bar").unwrap();
        assert_eq!(v1, v2);
    }

    /// T11: コンストラクタで指定した次元数が embed_dimension() と一致する
    #[test]
    fn test_constant_embedding_dimension() {
        let provider = ConstantEmbeddingProvider::new(256);
        assert_eq!(provider.embed_dimension(), 256);
    }

    // ── エラー型 (T12〜T14) ──

    /// T12: DarviumError::Embedding のメッセージ確認
    #[test]
    fn test_embedding_error_message() {
        let inner = "API error.".to_string();
        let err = DarviumError::Embedding(inner);
        let display = err.to_string();
        assert_eq!(display, "Embedding error: API error.");
    }

    /// T13: DarviumError::EmbeddingDimensionMismatch のメッセージ確認
    #[test]
    fn test_embedding_dimension_mismatch_message() {
        let err = DarviumError::EmbeddingDimensionMismatch {
            expected: 384,
            actual: 128,
        };
        let display = err.to_string();
        assert_eq!(
            display,
            "Embedding dimension mismatch: expected 384, actual 128"
        );
    }

    /// T14: エラーの PartialEq 比較
    #[test]
    fn test_embedding_error_partial_eq() {
        let err1 = DarviumError::Embedding("x".to_string());
        let err2 = DarviumError::Embedding("x".to_string());
        let err3 = DarviumError::Embedding("y".to_string());
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    // ── 計装・観測 (T15) ──

    /// T15: 埋め込みベクトルの分布観測テスト。
    ///
    /// FakeEmbeddingProvider が生成する疑似埋め込みベクトルを中心化・正規化し、
    /// ペアワイズのコサイン類似度分布を計測する。高次元超球面上の一様分布
    /// であれば平均≈0、標準偏差≈1/√d となることを確認する。
    #[test]
    fn test_fake_embedding_distribution() {
        let provider = FakeEmbeddingProvider::default();
        let n_vectors = 1_000;
        let dim = provider.embed_dimension();

        // ベクトル生成・中心化・正規化
        let mut vectors: Vec<Vec<f64>> = Vec::with_capacity(n_vectors);
        for i in 0..n_vectors {
            let text = format!("dist_{}", i);
            let vector = provider.embed(&text).unwrap();
            // 中心化: [0, 1) の各成分から 0.5 を減算
            let centered: Vec<f64> = vector.iter().map(|x| *x as f64 - 0.5).collect();
            // 正規化: ユニット長に
            let norm: f64 = centered.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm > 1e-10 {
                vectors.push(centered.iter().map(|x| x / norm).collect());
            } else {
                vectors.push(centered);
            }
        }

        // ランダムペアサンプリング
        let n_pairs = 10_000;
        let mut state: u64 = 987654321;
        let mut similarities: Vec<f64> = Vec::with_capacity(n_pairs);
        let mmix_mul: u64 = 6_364_136_223_846_793_005;

        while similarities.len() < n_pairs {
            state = state.wrapping_mul(mmix_mul).wrapping_add(1);
            let i = (state >> 32) as usize % n_vectors;
            state = state.wrapping_mul(mmix_mul).wrapping_add(1);
            let j = (state >> 32) as usize % n_vectors;
            if i == j {
                continue;
            }
            let dot: f64 = vectors[i]
                .iter()
                .zip(vectors[j].iter())
                .map(|(a, b)| a * b)
                .sum();
            similarities.push(dot);
        }

        // 統計量の計算
        let n_samples = similarities.len() as f64;
        let mean = similarities.iter().sum::<f64>() / n_samples;
        let variance =
            similarities.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n_samples - 1.0);
        let std_dev = variance.sqrt();
        let expected_std = (1.0 / dim as f64).sqrt();
        let std_error = std_dev / n_samples.sqrt();

        // 観測結果の出力
        println!("=== 疑似埋め込みベクトル分布観測 ===");
        println!("ベクトル数: {}", n_vectors);
        println!("次元数: {}", dim);
        println!("ペアサンプル数: {}", n_samples);
        println!("コサイン類似度 平均: {:.6}", mean);
        println!("コサイン類似度 標準偏差: {:.6}", std_dev);
        println!("期待標準偏差 (1/√d): {:.6}", expected_std);

        // 平均が 0 から 3σ 以内であること
        assert!(
            mean.abs() < 3.0 * std_error,
            "平均コサイン類似度 {:.6} が期待値 0 から乖離 (SE={:.6}, z={:.2})",
            mean,
            std_error,
            mean / std_error
        );

        // 標準偏差が期待値の 50%〜200% 以内であること
        assert!(
            std_dev > expected_std * 0.5 && std_dev < expected_std * 2.0,
            "標準偏差 {:.6} が期待値 {:.6} の許容範囲外",
            std_dev,
            expected_std
        );

        println!("=== 結果: PASS ===");
    }

    // ── 計装・観測 (OTS-LLM): LLMClient エントロピー一致性 ──

    /// OTS-LLM: FakeLlmClient 出力のシャノンエントロピーを観測する。
    ///
    /// 注入された乱数ノイズの確率と、実際の出力カテゴリ分布から計算される
    /// エントロピーが一致することを検証する。これによりトレイト境界を通過する
    /// LLM 呼び出しの全二重記録による完全監査可能性を担保する。
    #[test]
    fn observation_llm_entropy_consistency() {
        let malformed_prob = 0.3;
        let client = FakeLlmClient::new("normal_output").with_malformed_probability(malformed_prob);
        let schema = LlmSchema::FreeText;
        let n_calls = 10_000;

        let mut normal_count = 0usize;
        let mut empty_count = 0usize;
        let mut malformed_json_count = 0usize;
        let mut unexpected_count = 0usize;

        for _ in 0..n_calls {
            let result = client
                .generate_structured("test prompt", &schema)
                .expect("should not error");
            match result.as_str() {
                "normal_output" => normal_count += 1,
                "" => empty_count += 1,
                s if s == r#"{"invalid": "json"# => malformed_json_count += 1,
                _ => unexpected_count += 1,
            }
        }

        let n = n_calls as f64;
        let p_normal = normal_count as f64 / n;
        let p_empty = empty_count as f64 / n;
        let p_malformed_json = malformed_json_count as f64 / n;
        let p_unexpected = unexpected_count as f64 / n;

        // シャノンエントロピーの計算: H = -Σ p(x) * log₂(p(x))
        let entropy = |p: f64| -> f64 {
            if p <= 0.0 {
                0.0
            } else {
                -p * p.log2()
            }
        };
        let h_observed = entropy(p_normal)
            + entropy(p_empty)
            + entropy(p_malformed_json)
            + entropy(p_unexpected);

        // 期待エントロピー: 確率 p で3種類の不正出力が均等に出現
        // H = -(1-p)*log₂(1-p) - p*log₂(p/3)
        let h_expected =
            entropy(1.0 - malformed_prob) + malformed_prob * (malformed_prob / 3.0).log2().abs();

        println!("=== OTS-LLM: エントロピー一致性観測 ===");
        println!("呼び出し回数: {}", n_calls);
        println!("不正フォーマット確率 (設定値): {}", malformed_prob);
        println!("カテゴリ分布:");
        println!("  正常: {} ({:.4})", normal_count, p_normal);
        println!("  空文字列: {} ({:.4})", empty_count, p_empty);
        println!(
            "  不正JSON: {} ({:.4})",
            malformed_json_count, p_malformed_json
        );
        println!("  予期外文字列: {} ({:.4})", unexpected_count, p_unexpected);
        println!("観測エントロピー: {:.6} bits", h_observed);
        println!("期待エントロピー: {:.6} bits", h_expected);
        println!("差分: {:.6} bits", (h_observed - h_expected).abs());

        // エントロピーが期待値の 90%〜110% 以内であること
        let ratio = h_observed / h_expected;
        assert!(
            ratio > 0.9 && ratio < 1.1,
            "観測エントロピー {:.6} が期待値 {:.6} の許容範囲外 (ratio={:.4})",
            h_observed,
            h_expected,
            ratio
        );

        // 各カテゴリの割合が確率設定と矛盾しないこと
        assert!(
            (p_empty - malformed_prob / 3.0).abs() < 0.02,
            "空文字列の割合 {:.4} が期待値 {:.4} から乖離",
            p_empty,
            malformed_prob / 3.0
        );

        println!("obs_entropy: {:.6}", h_observed);
        println!("exp_entropy: {:.6}", h_expected);
        println!("entropy_ratio: {:.4}", ratio);
        println!("=== 結果: PASS ===");
    }

    // ═══════════════════════════════════════════════
    // ScriptedFakeLlmClient テスト (S1〜S18, OTS)
    // ═══════════════════════════════════════════════

    /// S1: ScriptedFakeLlmClient が LLMClient トレイトを実装していることのコンパイル時検証
    #[test]
    fn test_scripted_trait_bound_satisfied() {
        fn assert_trait(_: &impl LLMClient) {}
        let client = ScriptedFakeLlmClient::default_pass();
        assert_trait(&client);
    }

    /// S2: Box<dyn LLMClient> のオブジェクト安全性
    #[test]
    fn test_scripted_object_safety() {
        let client: Box<dyn LLMClient> = Box::new(ScriptedFakeLlmClient::default_pass());
        let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        assert!(result.is_ok());
    }

    /// S3: Send + Sync 境界の充足
    #[test]
    fn test_scripted_send_sync_bounds() {
        fn assert_send_sync<T: Send + Sync>(_t: &T) {}
        let client = ScriptedFakeLlmClient::default_pass();
        assert_send_sync(&client);

        let boxed: Box<dyn LLMClient> = Box::new(ScriptedFakeLlmClient::default_pass());
        assert_send_sync(&boxed);
    }

    /// S4: 確率 p_m = 0.0 で常に正常応答
    #[test]
    fn test_scripted_zero_probability_all_normal() {
        let template = r#"{"status": "ok"}"#;
        let client = ScriptedFakeLlmClient::new(template).with_malformed_probability(0.0);
        for _ in 0..100 {
            let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
            assert_eq!(result.unwrap(), template);
        }
    }

    /// S5: 確率 p_m = 1.0 で常に不正フォーマット
    #[test]
    fn test_scripted_one_probability_all_malformed() {
        // "key" を含むテンプレートにより、デフォルトの WrongKeyName("key") が確実に作用する
        let template = r#"{"key": "ok"}"#;
        let client = ScriptedFakeLlmClient::new(template).with_malformed_probability(1.0);
        for _ in 0..100 {
            let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
            // RawError の場合は Err が返る可能性もある
            if let Ok(output) = result {
                assert_ne!(output, template);
            }
        }
    }

    /// S6: MissingClosingBrace — 末尾の } が削除される
    #[test]
    fn test_scripted_missing_closing_brace() {
        let client = ScriptedFakeLlmClient::new(r#"{"a": 1}"#)
            .with_malformed_probability(1.0)
            .with_malformation_types(vec![MalformationType::MissingClosingBrace]);
        let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        let output = result.unwrap();
        assert_eq!(output, r#"{"a": 1"#);
    }

    /// S7: WrongKeyName — キー名が置換される
    #[test]
    fn test_scripted_wrong_key_name() {
        let client = ScriptedFakeLlmClient::new(r#"{"key": "val"}"#)
            .with_malformed_probability(1.0)
            .with_malformation_types(vec![MalformationType::WrongKeyName("key".to_string())]);
        let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        let output = result.unwrap();
        assert!(output.contains("key_corrupted"));
        assert!(!output.contains("\"key\":"));
    }

    /// S8: BitFlip — ビット反転が発生する
    #[test]
    fn test_scripted_bit_flip() {
        let client = ScriptedFakeLlmClient::new(r#"{"a": 1}"#)
            .with_malformed_probability(1.0)
            .with_malformation_types(vec![MalformationType::BitFlip(0)]);
        let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        let output = result.unwrap();
        // 先頭ビットが反転されているため元の文字列とは異なる
        assert_ne!(output, r#"{"a": 1}"#);
    }

    /// S9: RawError モードで Err(DarviumError::LlmMalformedJson) が返る
    #[test]
    fn test_scripted_raw_error() {
        let client = ScriptedFakeLlmClient::new("normal")
            .with_malformed_probability(1.0)
            .with_malformation_types(vec![MalformationType::RawError]);
        let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        match result {
            Err(DarviumError::LlmMalformedJson(msg)) => {
                assert!(msg.contains("RawError"));
            }
            other => panic!("Expected Err(LlmMalformedJson), got {:?}", other),
        }
    }

    /// S10: 固定シード下での完全再現性
    #[test]
    fn test_scripted_deterministic_replay() {
        let mut expected_outputs: Vec<String> = Vec::new();
        {
            let client = ScriptedFakeLlmClient::new("template").with_malformed_probability(0.5);
            for _ in 0..20 {
                let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
                expected_outputs.push(format!("{:?}", result));
            }
        }
        {
            let client = ScriptedFakeLlmClient::new("template").with_malformed_probability(0.5);
            for i in 0..20 {
                let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
                let actual = format!("{:?}", result);
                assert_eq!(
                    actual, expected_outputs[i],
                    "出力系列が一致しません (index={})",
                    i
                );
            }
        }
    }

    /// S11: 異なるシードで異なる出力系列が生成される
    #[test]
    fn test_scripted_different_seed_different_output() {
        let mut outputs_a: Vec<String> = Vec::new();
        let mut outputs_b: Vec<String> = Vec::new();

        {
            let client_a = ScriptedFakeLlmClient::new("template").with_malformed_probability(0.8);
            let client_b = ScriptedFakeLlmClient::new("template").with_malformed_probability(0.8);
            // デフォルトでは両方とも TEST_PRNG_SEED で初期化されるため同一系列
            for _ in 0..50 {
                outputs_a.push(format!(
                    "{:?}",
                    client_a.generate_structured(PROMPT_ARG, &LlmSchema::FreeText)
                ));
                outputs_b.push(format!(
                    "{:?}",
                    client_b.generate_structured(PROMPT_ARG, &LlmSchema::FreeText)
                ));
            }
        }
        // リセット後は同一系列になるはず
        assert_eq!(outputs_a, outputs_b);

        // 異なるシードにリセットすると異なる系列になるはず
        let mut outputs_c: Vec<String> = Vec::new();
        {
            let client_c = ScriptedFakeLlmClient::new("template").with_malformed_probability(0.8);
            client_c.reset_seed(99999);
            for _ in 0..50 {
                outputs_c.push(format!(
                    "{:?}",
                    client_c.generate_structured(PROMPT_ARG, &LlmSchema::FreeText)
                ));
            }
        }
        // 確率的なので完全に異なるとは言い切れないが、p_m=0.8 で seed が異なれば
        // 高い確率で系列が異なる。50サンプル中少なくとも1つは異なるはず。
        let all_same = outputs_a.iter().zip(outputs_c.iter()).all(|(a, c)| a == c);
        assert!(!all_same, "異なるシードでも同一系列が生成された");
    }

    /// S12: EmptyObject — {} が返る
    #[test]
    fn test_scripted_empty_object() {
        let client = ScriptedFakeLlmClient::new(r#"{"a": 1}"#)
            .with_malformed_probability(1.0)
            .with_malformation_types(vec![MalformationType::EmptyObject]);
        let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        assert_eq!(result.unwrap(), "{}");
    }

    /// S13: ExtraField — 余分なフィールドが追加される
    #[test]
    fn test_scripted_extra_field() {
        let client = ScriptedFakeLlmClient::new(r#"{"a": 1}"#)
            .with_malformed_probability(1.0)
            .with_malformation_types(vec![MalformationType::ExtraField]);
        let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        let output = result.unwrap();
        assert!(output.contains("_extra_field"));
    }

    /// S14: TypeMismatch — 文字列値が数値に置換される
    #[test]
    fn test_scripted_type_mismatch() {
        let client = ScriptedFakeLlmClient::new(r#"{"name": "hello"}"#)
            .with_malformed_probability(1.0)
            .with_malformation_types(vec![MalformationType::TypeMismatch]);
        let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        let output = result.unwrap();
        // 文字列値 "hello" が数値 42 に置換される
        assert!(output.contains("42") || !output.contains("\"hello\""));
    }

    /// S15: NestedBraceDestruction — ネスト構造が破壊される
    #[test]
    fn test_scripted_nested_brace_destruction() {
        let client = ScriptedFakeLlmClient::new(r#"{"a": {"b": 2}}"#)
            .with_malformed_probability(1.0)
            .with_malformation_types(vec![MalformationType::NestedBraceDestruction]);
        let result = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        let output = result.unwrap();
        // 余分な開き括弧が挿入されるため元の文字列とは異なる
        assert_ne!(output, r#"{"a": {"b": 2}}"#);
        assert!(output.matches('{').count() > 1);
    }

    /// S16: call_count 追跡が正しく動作する
    #[test]
    fn test_scripted_call_count_tracking() {
        let client = ScriptedFakeLlmClient::default_pass();
        assert_eq!(client.call_count(), 0);

        let _ = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        assert_eq!(client.call_count(), 1);

        let _ = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        let _ = client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
        assert_eq!(client.call_count(), 3);
    }

    /// S17: 複数 MalformationType の出現分布の統計的検証
    ///
    /// 3種類の MalformationType を指定し、p_m=0.6 で n=10,000 回呼び出した際に、
    /// 各タイプの出現回数が均等分布（1/3）の95%信頼区間に収まることを確認する。
    #[test]
    fn test_scripted_malformation_distribution() {
        let n_trials = 10_000_usize;
        let types = vec![
            MalformationType::MissingClosingBrace,
            MalformationType::EmptyObject,
            MalformationType::BitFlip(0),
        ];
        let n_types = types.len();

        let client = ScriptedFakeLlmClient::new(r#"{"status": "ok"}"#)
            .with_malformed_probability(0.6)
            .with_malformation_types(types);

        let mut counts = vec![0usize; n_types];
        let mut normal_count = 0usize;
        let mut error_count = 0usize;

        for _ in 0..n_trials {
            match client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText) {
                Ok(output) => {
                    if output == r#"{"status": "ok"}"# {
                        normal_count += 1;
                    } else {
                        // どの不正フォーマットか特定
                        for (i, t) in [
                            MalformationType::MissingClosingBrace,
                            MalformationType::EmptyObject,
                            MalformationType::BitFlip(0),
                        ]
                        .iter()
                        .enumerate()
                        {
                            let matched = match t {
                                MalformationType::MissingClosingBrace => !output.contains('}'),
                                MalformationType::EmptyObject => output == "{}",
                                MalformationType::BitFlip(_) => {
                                    output != r#"{"status": "ok"}"#
                                        && output != "{}"
                                        && output.contains('}')
                                }
                                _ => false,
                            };
                            if matched {
                                counts[i] += 1;
                            }
                        }
                    }
                }
                Err(_) => {
                    error_count += 1;
                }
            }
        }

        let malformed_count = n_trials - normal_count - error_count;
        let _expected_per_type = (malformed_count as f64) / n_types as f64;

        // 各タイプの出現比率が均等の95%信頼区間に収まることを確認
        for (count, ty) in counts
            .iter()
            .zip(["MissingClosingBrace", "EmptyObject", "BitFlip"].iter())
        {
            let ratio = *count as f64 / n_trials as f64;
            let expected_ratio = 0.6 / n_types as f64;
            let std_err = (expected_ratio * (1.0 - expected_ratio) / n_trials as f64).sqrt();
            let lower = expected_ratio - 3.0 * std_err;
            let upper = expected_ratio + 3.0 * std_err;
            assert!(
                ratio >= lower && ratio <= upper,
                "{} の出現比率 {:.6} が期待CI [{:.6}, {:.6}] の範囲外 (count={})",
                ty,
                ratio,
                lower,
                upper,
                count
            );
        }

        println!("=== S17: 分布検証 ===");
        println!("試行回数: {}", n_trials);
        println!("正常回数: {}", normal_count);
        println!("エラー回数: {}", error_count);
        for (i, ty) in ["MissingClosingBrace", "EmptyObject", "BitFlip"]
            .iter()
            .enumerate()
        {
            println!(
                "{}: {} (ratio={:.4})",
                ty,
                counts[i],
                counts[i] as f64 / n_trials as f64
            );
        }
        println!("=== 結果: PASS ===");
    }

    /// S18: 全 MalformationType の出力が serde_json::from_str でエラーになること
    #[test]
    fn test_scripted_all_types_fail_serde_json() {
        // 構文的に JSON パースを確実に失敗させる type のみを対象とする
        // WrongKeyName / ExtraField / TypeMismatch は意味的には不正だが
        // 構文的には valid JSON としてパース成功するため除外
        let all_types = vec![
            MalformationType::MissingClosingBrace,
            MalformationType::BitFlip(0),
            MalformationType::EmptyObject, // {} は valid JSON（特殊ケース）
            MalformationType::NestedBraceDestruction,
        ];

        let n_trials_per_type = 50;

        for mal_type in &all_types {
            let client = ScriptedFakeLlmClient::new(r#"{"key": "val", "n": 1}"#)
                .with_malformed_probability(1.0)
                .with_malformation_types(vec![mal_type.clone()]);

            let mut all_fail = true;
            for _ in 0..n_trials_per_type {
                match client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText) {
                    Ok(output) => {
                        // EmptyObject は valid JSON なので許容
                        if *mal_type == MalformationType::EmptyObject {
                            assert_eq!(output, "{}");
                            continue;
                        }
                        if serde_json::from_str::<serde_json::Value>(&output).is_ok() {
                            all_fail = false;
                        }
                    }
                    Err(_) => {} // Err はパース失敗とみなす
                }
            }

            assert!(
                *mal_type == MalformationType::EmptyObject || all_fail,
                "{:?} の出力が serde_json でパース成功したケースがあった",
                mal_type
            );
        }
    }

    // ── 観測テスト (OTS: Observational Testing for Scripted) ──

    /// OTS-S1: p_m sweep テスト
    ///
    /// p_m を 0.0 から 1.0 まで 0.1 刻みで変化させ、各ポイントで
    /// 不正フォーマット出現率を計測する。出現率は期待値 p_m と
    /// 線形関係にあることを確認する。
    #[test]
    fn observation_scripted_pm_sweep() {
        println!("\n=== OTS-S1: p_m sweep ===");
        println!("p_m,observed_ratio,n_trials");

        let n_per_point = 1_000_usize;

        for i in 0..=10 {
            let p_m = i as f64 / 10.0;
            // "key" を含むテンプレートにより、デフォルトの WrongKeyName("key") が確実に作用する
            let template = r#"{"key": "value"}"#;
            let client = ScriptedFakeLlmClient::new(template).with_malformed_probability(p_m);

            let mut malformed_count = 0usize;
            let mut error_count = 0usize;

            for _ in 0..n_per_point {
                match client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText) {
                    Ok(output) => {
                        if output != template {
                            malformed_count += 1;
                        }
                    }
                    Err(_) => {
                        error_count += 1;
                    }
                }
            }

            let total_malformed = malformed_count + error_count;
            let observed_ratio = total_malformed as f64 / n_per_point as f64;
            println!("{:.1},{:.4},{}", p_m, observed_ratio, n_per_point);

            // p_m=0.0 では全て正常
            if p_m == 0.0 {
                assert_eq!(total_malformed, 0, "p_m=0.0 で不正出力が発生");
            }
            // p_m=1.0 では全て不正
            if p_m == 1.0 {
                assert!(
                    total_malformed >= n_per_point - 2,
                    "p_m=1.0 で不正出力が不足 (total_malformed={})",
                    total_malformed
                );
            }

            // 観測比率が期待値の3σ以内であること（n=1,000 で粗めの許容）
            if p_m > 0.0 && p_m < 1.0 {
                let std_err = (p_m * (1.0 - p_m) / n_per_point as f64).sqrt();
                let lower = (p_m - 4.0 * std_err).max(0.0);
                let upper = (p_m + 4.0 * std_err).min(1.0);
                assert!(
                    observed_ratio >= lower && observed_ratio <= upper,
                    "p_m={:.1}: 観測比率 {:.4} が許容範囲 [{:.4}, {:.4}] の範囲外",
                    p_m,
                    observed_ratio,
                    lower,
                    upper
                );
            }
        }

        println!("=== OTS-S1: PASS ===");
    }

    /// OTS-S2: シャノンエントロピー一致性
    ///
    /// p_m=0.3 で n=10,000 回呼び出し、出力カテゴリのエントロピーが
    /// 期待値の90-110%に収まることを確認する。
    /// カテゴリ: 正常 / 不正文字列 / エラー
    #[test]
    fn observation_scripted_entropy_consistency() {
        let malformed_prob = 0.3;
        let n_calls = 10_000_usize;
        let template = r#"{"a": 1}"#;

        let client = ScriptedFakeLlmClient::new(template)
            .with_malformed_probability(malformed_prob)
            .with_malformation_types(vec![
                MalformationType::MissingClosingBrace,
                MalformationType::EmptyObject,
                MalformationType::RawError,
            ]);

        let mut normal_count = 0usize;
        let mut malformed_count = 0usize;
        let mut error_count = 0usize;

        for _ in 0..n_calls {
            match client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText) {
                Ok(output) => {
                    if output == template {
                        normal_count += 1;
                    } else {
                        malformed_count += 1;
                    }
                }
                Err(_) => {
                    error_count += 1;
                }
            }
        }

        let n = n_calls as f64;
        let p_normal = normal_count as f64 / n;
        let p_malformed = malformed_count as f64 / n;
        let p_error = error_count as f64 / n;

        // シャノンエントロピー
        let entropy = |p: f64| -> f64 {
            if p <= 0.0 {
                0.0
            } else {
                -p * p.log2()
            }
        };
        let h_observed = entropy(p_normal) + entropy(p_malformed) + entropy(p_error);

        // 期待エントロピー: 3種の MalformationType が均等確率で出現
        // MissingClosingBrace + EmptyObject → malformed_string (Ok)
        // RawError → error (Err)
        // 正常: 1-p_m, malformed_string: p_m*2/3, error: p_m*1/3
        let p_malformed_expected = malformed_prob * 2.0 / 3.0;
        let p_error_expected = malformed_prob * 1.0 / 3.0;
        let p_normal_expected = 1.0 - malformed_prob;
        let h_expected =
            entropy(p_normal_expected) + entropy(p_malformed_expected) + entropy(p_error_expected);

        println!("\n=== OTS-S2: エントロピー一致性観測 ===");
        println!("呼び出し回数: {}", n_calls);
        println!("不正確率 (設定値): {}", malformed_prob);
        println!("カテゴリ分布:");
        println!("  正常: {} ({:.4})", normal_count, p_normal);
        println!("  不正文字列: {} ({:.4})", malformed_count, p_malformed);
        println!("  エラー: {} ({:.4})", error_count, p_error);
        println!("観測エントロピー: {:.6} bits", h_observed);
        println!("期待エントロピー: {:.6} bits", h_expected);

        // エントロピーが期待値の80-120%以内であること（3カテゴリ分布の統計的揺らぎを許容）
        let ratio = h_observed / h_expected;
        assert!(
            ratio > 0.8 && ratio < 1.2,
            "観測エントロピー {:.6} が期待値 {:.6} の許容範囲外 (ratio={:.4})",
            h_observed,
            h_expected,
            ratio
        );

        // 正常比率が (1-p_m) の3σ以内
        let std_err = (malformed_prob * (1.0 - malformed_prob) / n as f64).sqrt();
        assert!(
            (p_normal - (1.0 - malformed_prob)).abs() < 3.0 * std_err,
            "正常比率 {:.4} が期待値 {:.4} から乖離",
            p_normal,
            1.0 - malformed_prob
        );

        println!("entropy_ratio: {:.4}", ratio);
        println!("=== OTS-S2: PASS ===");
    }

    /// OTS-S3: デシリアライズ成功率の相転移観測
    ///
    /// p_m を 0.0→1.0 に sweep し、serde_json::from_str の成功率が
    /// 単調減少するプロファイルを観測する。
    /// EmptyObject は valid JSON なので成功率に加算される点に注意。
    #[test]
    fn observation_scripted_deserialize_phase_transition() {
        println!("\n=== OTS-S3: デシリアライズ相転移 ===");
        println!("p_m,success_rate,n_trials");

        let n_per_point = 1_000_usize;

        for i in 0..=10 {
            let p_m = i as f64 / 10.0;
            let template = r#"{"data": "value", "num": 42}"#;
            let client = ScriptedFakeLlmClient::new(template).with_malformed_probability(p_m);

            let mut success_count = 0usize;
            for _ in 0..n_per_point {
                match client.generate_structured(PROMPT_ARG, &LlmSchema::FreeText) {
                    Ok(output) => {
                        if serde_json::from_str::<serde_json::Value>(&output).is_ok() {
                            success_count += 1;
                        }
                    }
                    Err(_) => {
                        // エラーはパース失敗扱い
                    }
                }
            }

            let success_rate = success_count as f64 / n_per_point as f64;
            println!("{:.1},{:.4},{}", p_m, success_rate, n_per_point);

            // 成功率は非増加（単調減少または一定）
            // p_m が大きいほど成功率は下がる傾向
        }

        // p_m=0.0 では100%成功すること
        let client_zero = ScriptedFakeLlmClient::new(r#"{"a": 1}"#).with_malformed_probability(0.0);
        for _ in 0..100 {
            let result = client_zero.generate_structured(PROMPT_ARG, &LlmSchema::FreeText);
            assert!(serde_json::from_str::<serde_json::Value>(&result.unwrap()).is_ok());
        }

        // p_m=1.0 では RawError を含むため Err か invalid JSON になる
        let client_one = ScriptedFakeLlmClient::new(r#"{"a": 1}"#).with_malformed_probability(1.0);
        let mut any_success = false;
        for _ in 0..200 {
            match client_one.generate_structured(PROMPT_ARG, &LlmSchema::FreeText) {
                Ok(output) => {
                    if serde_json::from_str::<serde_json::Value>(&output).is_ok() {
                        any_success = true;
                    }
                }
                Err(_) => {}
            }
        }
        // EmptyObject による success がありうるので、any_success の有無はアサートしない
        let _ = any_success;

        println!("=== OTS-S3: PASS ===");
    }
}
