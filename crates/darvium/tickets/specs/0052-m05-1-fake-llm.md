---
ticket_id: 52
title: M0.5-1: スクリプト化された壊れたフォーマット出力 Fake LLM クライアントの実装
slug: m05-1-fake-llm
status: reviewed
created_at: 2026-05-23
updated_at: 2026-05-23
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0052-m05-1-fake-llm/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0052-m05-1-fake-llm/observation-20260523-145926.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0052-m05-1-fake-llm/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0052-m05-1-fake-llm/review.md
---
# M0.5-1: スクリプト化された壊れたフォーマット出力 Fake LLM クライアントの実装

## Summary

PRNG 駆動のスクリプト指示に従い、JSON 構造を意図的に破壊した不正文字列を返す `ScriptedFakeLlmClient` を実装する。これにより LLM 出力のパース箇所に不正フォーマットが注入された際、システムがクラッシュせず `DarviumError::LlmMalformedJson` として安全に捕捉可能であることを、観測ベース検証で実証する。

## Background

### 対象不変条件 / 規範

- **RFC §14.2**: 構造化出力・JSON パースエラーハンドリング — LLM が返す JSON が不正形式でもパニックせず、`LlmMalformedJson` エラーとして上位レイヤに伝播すること。
- **RFC §12.2**: `GraphPatchGenerator::generate` 内で `llm_generate()` の結果をデシリアライズする箇所 — 不正フォーマットが `PatchError::LowConfidence` 等へ安全に変換される経路。
- **チケット M-2-1.6**: `LLMClient` 抽象トレイトが `src/llm/mod.rs:55-59` で定義済み。
- **チケット M-2-3**: `FakeLlmClient` が `src/llm/mod.rs:73-160` で実装済み。ハッシュベースの簡易不正フォーマット（3種類固定）をサポート。

### 現状のコード

- `FakeLlmClient`（`src/llm/mod.rs:73-160`）はハッシュ値ベースの確率的制御で、3種類の固定パターン（空文字列、`{"invalid": "json"`、`UNEXPECTED_FORMAT`）のみを生成する。
- `FakeLlmClient::should_be_malformed()`（`src/llm/mod.rs:124-134`）は PRNG ではなく `prev_count.wrapping_mul(2_654_435_761)` のハッシュ値を用いるため、外部からのエントロピー制御が不可能。
- `FakeLlmClient::generate_malformed()`（`src/llm/mod.rs:137-143`）は `count % 3` でパターンを切り替えるのみで、スクリプトによる任意の不正フォーマット注入が不可能。
- `LLMClient::generate_structured()`（`src/llm/mod.rs:57-59`）は `Result<String, DarviumError>` を返すが、現状の `FakeLlmClient` は常に `Ok(...)` を返し、不正フォーマットをエラーとして通知しない。これはテスト二重としての責務の範囲の問題であり、M0.5-1 ではこの設計を踏襲する（不正文字列を `Ok` で返し、JSON パース側でエラー検出させる設計が RFC §14.2 の想定）。
- `StdRng::seed_from_u64(TEST_PRNG_SEED)` パターンは `src/search/mock_proposer.rs` で既に確立済み。
- `DarviumError::LlmMalformedJson(String)` は `src/error.rs:100-101` で定義済みだが、現状どのコードからも発行されていない（到達不能に近い）。

### なぜ既存の FakeLlmClient では不十分か

既存の `FakeLlmClient` は以下の制約を持ち、M0.5-1 の要件を満たせない：

1. **固定3パターンのみ**: 任意の JSON スキーマに対するフォーマット破壊をシミュレートできない。例えばキー名の置換、ネストの破壊、型の不一致など、実際の LLM が生成しうる多様な不正パターンをカバーできない。
2. **PRNG 非対応**: エントロピー制御がハッシュベースで、連続的な p_m 制御が不可能。相転移プロファイルの観測に必要な精密制御（p_m を 0.0→1.0 に sweep）が行えない。
3. **スクリプト不能**: 外部から不正パターンの種類・出現順序・頻度を制御するインターフェースがない。

本チケットでは既存の `FakeLlmClient` を改変せず、新しい `ScriptedFakeLlmClient` を追加することで上記を解決する。

## Investigation

### 既存コードの解析結果

**LLMClient トレイト（src/llm/mod.rs:55-59）**:

```rust
pub trait LLMClient: Send + Sync {
    fn generate_structured(&self, prompt: &str, schema: &LlmSchema)
        -> Result<String, DarviumError>;
}
```

新しく実装する `ScriptedFakeLlmClient` もこのトレイトを実装する。

**エラー型（src/error.rs:96-101）**:

```rust
#[error("LLM error: {0}")]
Llm(String),

#[error("LLM malformed JSON: {0}")]
LlmMalformedJson(String),
```

`LlmMalformedJson` バリアントは定義済みだが、`generate_structured` から直接返されることは現状ない。本チケットではスクリプトの指示に応じて `Err(DarviumError::LlmMalformedJson(...))` を返すモードもサポートする。

**PRNG 使用パターン（src/search/mock_proposer.rs:14,47,63）**:

```rust
use rand::rngs::StdRng;
// ...
struct MockProposer {
    rng: StdRng,
    // ...
}
// ...
rng: StdRng::seed_from_u64(seed),
```

このパターンを再利用し、`ScriptedFakeLlmClient` も内部に `StdRng` を持つ。

**定数（src/constants.rs:67-69）**:

```rust
pub const TEST_PRNG_SEED: u64 = 12345;
pub const FAKE_LLM_DEFAULT_MALFORMED_PROB: f64 = 0.0;
```

`FAKE_LLM_DEFAULT_MALFORMED_PROB` は既存の `FakeLlmClient` 用。新規に `SCRIPTED_FAKE_LLM_DEFAULT_MALFORMED_PROB` 等を追加する必要はない（デフォルトは 0.0 で十分）。

**rand 依存（Cargo.toml 確認必要）**: `StdRng` を使用するには `rand` クレートが依存に含まれている必要がある。`mock_proposer.rs` で既に使用されているため、依存は解決済みと推定される。

### 参照観察レポート

現在の実験系列では M0-3（0051）まで完了しており、M0.5 フェーズは未着手。過去の観察レポートは M0 以前のものが対象であり、本チケットの設計に直接利用可能な観測データは存在しない。

## Scope

### 実装スコープ

1. **`ScriptedFakeLlmClient` 構造体の実装（src/llm/mod.rs に追記）**:
   - 内部に `StdRng`（固定シード PRNG）を持つ
   - 不正フォーマットのスクリプト（`Vec<MalformationScript>`）を保持
   - `LLMClient` トレイトを実装
   - 各呼び出しで PRNG の確率 `p_m` に従い、スクリプトから不正フォーマットを選択・適用

2. **`MalformationType` 列挙型の定義**:
   - 不正フォーマットの種類を列挙する列挙型
   - 各バリアント: `MissingClosingBrace`, `WrongKeyName(String)`, `BitFlip(usize)`, `EmptyObject`, `ExtraField`, `TypeMismatch`, `NestedBraceDestruction`, `RawError`

3. **PRNG 駆動の確率的制御**:
   - `p_m ∈ [0.0, 1.0]` の連続制御パラメータ
   - 固定シード `StdRng::seed_from_u64(12345)` による完全再現性
   - `p_m` はコンストラクタまたはビルダーパターンで設定可能

4. **不正フォーマット適用ロジック**:
   - 基本となる正常応答文字列を受け取り、選択された `MalformationType` に従って破壊的変換を施す
   - `MissingClosingBrace`: 末尾の `}` を削除（JSON がオブジェクトの場合）
   - `WrongKeyName`: 指定されたキー名を別の名前に置換
   - `BitFlip`: 指定位置の文字をビット反転
   - `EmptyObject`: `{}` を返す
   - `ExtraField`: 予期しないフィールドを追加
   - `TypeMismatch`: フィールドの値の型を変える（文字列→数値など）
   - `NestedBraceDestruction`: ネストした括弧構造を破壊
   - `RawError`: 直接 `Err(DarviumError::LlmMalformedJson(...))` を返す

5. **観測テストの実装**:
   - `p_m` を sweep した際の不正フォーマット出現率の統計的検証
   - 各 `MalformationType` が指定確率通りに出現することの検証
   - 固定シード下での完全再現性の検証
   - 相転移プロファイルの観測（デシリアライザ成功/失敗率の曲線）

### 非スコープ

- 既存 `FakeLlmClient` の改変（後方互換性維持のために触らない）
- 実際の JSON デシリアライザの実装（実際のパース処理は M2 以降で LLM 出力の消費部に実装される。本チケットは不正文字列を生成する側の責務）
- E2E テストの作成（本チケットはユニットテスト + 観測テストで完結）
- M0.5-2（確率的パッチ操作インジェクション）の実装
- M0.5-3（未解決変数の確率的検出）の実装
- `rand` 以外の新規依存クレートの追加

## Test Plan

### ユニットテスト（src/llm/mod.rs の既存 tests モジュールに追記）

Test naming は既存の `T{number}` 命名に従い、新しい系列は S（Scripted）prefix で区別する。

| ID | テスト内容 | 種別 |
|------|-----------|------|
| S1 | `ScriptedFakeLlmClient` が `LLMClient` トレイトを実装していることのコンパイル時検証 | Compile-time |
| S2 | `Box<dyn LLMClient>` としてのオブジェクト安全性 | Type bound |
| S3 | `Send + Sync` 境界の充足 | Type bound |
| S4 | 確率 `p_m = 0.0` で常に正常応答（script template 通りの出力） | 正常系 |
| S5 | 確率 `p_m = 1.0` で常に不正フォーマット | 異常系 |
| S6 | 特定 `MalformationType`（例: `MissingClosingBrace`）を指定した場合の出力確認 | 正常系 |
| S7 | `WrongKeyName` によるキー名置換の正確性検証 | 正常系 |
| S8 | `BitFlip` によるビット反転の正確性検証 | 正常系 |
| S9 | `RawError` モードで `Err(DarviumError::LlmMalformedJson)` が返る検証 | 異常系 |
| S10 | 固定シード下での完全再現性（同一シード→同一出力系列） | 正常系 |
| S11 | 異なるシードで異なる出力系列が生成されること | 正常系 |
| S12 | `EmptyObject` マルフォーメーションの出力検証 | 正常系 |
| S13 | `ExtraField` マルフォーメーションの出力検証 | 正常系 |
| S14 | `TypeMismatch` マルフォーメーションの出力検証 | 正常系 |
| S15 | `NestedBraceDestruction` マルフォーメーションの出力検証 | 正常系 |
| S16 | `call_count` 追跡が正しく動作すること | 正常系 |
| S17 | 複数の `MalformationType` をスクリプトで指定し、出現分布が指定比率に従うことの統計的検証（n=10,000） | 観測 |
| S18 | マルフォーメーション適用後の文字列が `serde_json::from_str` で確実にエラーになること（各タイプについて検証） | 異常系 |

### 観測テスト

| ID | テスト内容 | 観測対象 |
|------|-----------|---------|
| OTS-S1 | `p_m` sweep テスト: 0.0→1.0 を 0.1 刻みで変化させ、各ポイントで不正フォーマット出現率を計測（各 n=1,000） | 出現率曲線の線形性（期待値 p_m との一致） |
| OTS-S2 | シャノンエントロピー一致性: `p_m=0.3` で出力カテゴリのエントロピーが期待値 ±10% に収まること（n=10,000） | エントロピー一致性 |
| OTS-S3 | `serde_json::from_str` デシリアライズ成功率の相転移: `p_m` 上昇に伴い、成功率が単調減少し `p_m ≈ 0.5` で急峻に低下するプロファイルを観測（n=1,000 × 11 points） | 相転移プロファイル |

## 計装方法・観測対象

### 計装方法

- 全テストは `StdRng::seed_from_u64(TEST_PRNG_SEED)` を使用し、完全再現性を保証する
- 観測テストは `println!` + `--nocapture` で構造化テキスト（JSON/CSV）を標準出力に書き出す
- `p_m` sweep では 0.0, 0.1, 0.2, ..., 1.0 の 11 ポイントで各 n=1,000 のサンプルを取得
- エントロピー観測では n=10,000、4カテゴリ（正常/欠落/キー誤り/型誤り）の分布

### 観測対象

| 観測量 | 統計手法 | サンプルサイズ | 期待値 |
|--------|---------|---------------|--------|
| 不正フォーマット出現率 | 比率の95%信頼区間 | n=10,000/p_m | 指定 p_m ± 1.96√(p(1-p)/n) |
| 出力カテゴリ分布エントロピー H | H = -Σp(x)log₂p(x) | n=10,000 | 期待エントロピーの90-110% |
| デシリアライズ成功率 R(p_m) | 比率測定 | n=1,000/point × 11 | p_m=0 で 100%、p_m→1 で 0% への単調減少 |
| 相転移勾配 dR/dp_m | 隣接ポイント差分 | 同上 | p_m=0.5 付近で最大勾配 |

### 較正計画

本チケットは新規の計装であり、既存定数の調整は行わない。ただし以下の新規定数を `constants.rs` に追加する：

```rust
/// ScriptedFakeLlmClient のデフォルト不正フォーマット確率
pub const SCRIPTED_FAKE_LLM_DEFAULT_MALFORMED_PROB: f64 = 0.0;
```

較正ループは適用せず（本チケットはテスト基盤の整備のみ）。

## Boy Scout Rule — 翻訳可能性計画

### 新規コード（src/llm/mod.rs 追記部分）

- **型名はドメイン概念を直接表現**: `ScriptedFakeLlmClient`（責務が明確）、`MalformationType`（不正フォーマットの種類）
- **関数名は動詞句**: `apply_malformation()`、`select_malformation()`、`build_normal_response()`
- **一関数一責務**: スクリプト選択と文字列破壊は別関数に分割
- **ハードコード値の定数化**: マルフォーメーション適用時のマジックナンバーは全て名前付き定数
- **エラー握りつぶし禁止**: 内部エラーは `DarviumError` として上位に伝播（`unwrap()` 不使用）

### 既存コードの改善

既存 `FakeLlmClient`（`src/llm/mod.rs:73-160`）のコードは本チケットで触らない。しかし `generate_malformed()`（line 137-143）の `count % 3` によるパターン分岐は「翻訳不可能」に該当する（何が何%3なのかが自明でない）。Boy Scout Rule として、もしこの関数を通る場合はリファクタリングを検討するが、本チケットでは変更しない（既存テストに影響を与えるリスクを避ける）。

## Acceptance Criteria

- [ ] `ScriptedFakeLlmClient` が `LLMClient` トレイトを実装し、`Box<dyn LLMClient>` + `Send + Sync` 境界を満たす
- [ ] 全8種類の `MalformationType` が実装され、それぞれ意図通りの不正文字列を生成する
- [ ] `p_m=0.0` → 常に正常応答、`p_m=1.0` → 常に不正フォーマット
- [ ] 固定シード下で完全再現性があり、異なるシードで異なる系列が生成される
- [ ] 全ユニットテスト S1〜S18 が通過
- [ ] 全観測テスト OTS-S1〜OTS-S3 が通過
- [ ] `RawError` モードで `Err(DarviumError::LlmMalformedJson(...))` が正しく返る
- [ ] 既存の `FakeLlmClient` テストが全て通過（回帰なし）
- [ ] 全テスト通過後、`cargo clippy` が警告ゼロ
- [ ] `cargo fmt` が通過
- [ ] 翻訳可能性の検証が通っている（関数名・変数名が散文として読める）
- [ ] 新規定数が `constants.rs` に適切に定義されている

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0052-m05-1-fake-llm/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0052-m05-1-fake-llm/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0052-m05-1-fake-llm/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0052-m05-1-fake-llm/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
