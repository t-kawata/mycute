---
ticket_id: 57
title: M1-3: 人間フィードバック非同期連続注入に対する Debounce（キャッシュ無効化抑制）ロジックの検証
slug: m1-3-debounce
status: reviewed
created_at: 2026-05-23
updated_at: 2026-05-23
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0057-m1-3-debounce/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0057-m1-3-debounce/observation-20260523-191416.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0057-m1-3-debounce/review.md
---

# M1-3: 人間フィードバック非同期連続注入に対する Debounce（キャッシュ無効化抑制）ロジックの検証

## Summary

人間フィードバック (`TrustUpdate::Human`) が非同期に連続注入される状況において、複合信頼スコアの変動デルタが閾値 `TRUST_DEBOUNCE_DELTA (0.05)` 未満の場合は `invalidate_applicability_cache()` をスキップする Debounce ロジックを実装する。これにより、微小なフィードバック（例: thumbs-up でスコアが 0.01 しか変動しない場合）による不必要なキャッシュ破棄を防止する。

## Background

RFC §10.5 は `TrustUpdate` 状態機械を以下のように規定している:

- `TrustUpdate::Operational`: 常にキャッシュ無効化
- `TrustUpdate::Semantic`: 常にキャッシュ無効化
- **`TrustUpdate::Human`**: 複合スコアの変動デルタが `TRUST_DEBOUNCE_DELTA (0.05)` **以上**の場合のみキャッシュ無効化。未満の場合はスキップ

この Debounce は OQ-11 で「非同期フィードバックのバッチ更新パターンに依存する」と評価されており、`0.05` の妥当性は今後の較正対象 (Calibration Candidate) である。

現在のコードベースにおいて:

- `MemoizedGraph` は `trust.rs` に試験用縮約実装済み (M1-2)。`cache_invalidated` フラグと `invalidate_applicability_cache()` を持ち、追跡可能
- `HumanTrustLogistic` は `types.rs` に RFC §10.3 準拠で実装済み。`update(outcome)` メソッド持有
- `TrustProfile` は `types.rs` に実装済みだが、`composite()` メソッドは未実装
- `TrustUpdate` enum は未定義
- `update_trust()` メソッドは `MemoizedGraph` に未実装
- `TRUST_DEBOUNCE_DELTA` 定数は `constants.rs` に未定義

## Scope

以下の実装を含む:

1. **定数追加** (`src/constants.rs`):
   - `TRUST_DEBOUNCE_DELTA: f64 = 0.05` — Calibration Candidate として定義。RFC §10.5 指定値

2. **`TrustUpdate` enum の定義** (`src/types.rs`):
   ```rust
   pub enum TrustUpdate {
       Operational(bool),  // true = success, false = failure
       Human(f64),         // outcome ∈ {0.0, 0.25, 0.5, 0.75, 1.0}
       Semantic(f64),      // semantic_deviation ∈ [0.0, 1.0]
   }
   ```

3. **`TrustProfile::composite()` メソッドの追加** (`src/types.rs`):
   - RFC §10.4 の重み付き複合スコア計算: `0.35 * operational + 0.25 * semantic + 0.20 * temporal + 0.20 * human.score`
   - M1-3 では `Provenance` / `TimeDecayProfile` の完全実装は不要。temporal は現状の `f64` 値をそのまま加算 (時間減衰の完全実装は後続チケット)
   - シグネチャ: `pub fn composite(&self) -> f64`

4. **`MemoizedGraph::update_trust()` メソッドの実装** (`src/trust.rs`):
   - シグネチャ: `pub fn update_trust(&mut self, update: TrustUpdate)`
   - 処理内容:
     - `TrustUpdate::Operational(success)` → `update_operational_trust()` 相当の簡易更新 + `invalidate_applicability_cache()` (常時)
     - `TrustUpdate::Human(outcome)` → 複合スコア前後比較、デルタ < TRUST_DEBOUNCE_DELTA ならキャッシュ無効化スキップ
     - `TrustUpdate::Semantic(score)` → `update_semantic_ema()` 相当の簡易更新 + `invalidate_applicability_cache()` (常時)

5. **ユニットテスト** (`src/trust.rs` 内 `#[cfg(test)] mod tests`):
   - T1〜T8: 不変条件テスト
   - OTS-1〜OTS-3: 観測テスト

## Non-scope

- `Provenance` / `TimeDecayProfile` の完全実装 (後続チケット)
- `MemoizedGraph` の完全実装 (WorkflowRepository 統合・GraphVersion CAS 等は M2/M3 以降)
- `TrustProfile` の完全な時間減衰計算
- Elo 昇格 (RFC-0003 委譲)
- Operational / Semantic の完全な更新ロジック (M1-3 では簡易更新で十分)

## Investigation

### 参照観察レポート

- `tickets/context/0056-m1-2-adminfasttrack-trustauditlog/observation-20260523-182912.md` — M1-2 AdminFastTrack の実装完了観測。`MemoizedGraph` 縮約実装、`HumanTrustLogistic`、`TrustProfile` が利用可能になった。本チケットはこれらの上に `update_trust()` を追加する。
- `tickets/context/0055-m1-1-human-review-queue/observation-20260523-181239.md` — M1-1 HumanReviewQueue の観測完了。人間フィードバックキューイング基盤が整った。

### コードベース調査結果

1. **TRUST_DEBOUNCE_DELTA 定数**: `src/constants.rs` に未定義。RFC §10.5 では 0.05 が指定値。OQ-11 で Calibration Candidate と評価済み。

2. **TrustUpdate enum**: `src/types.rs` に未定義。RFC §10.5 で以下の 3 バリアントが規定:
   ```rust
   pub enum TrustUpdate {
       Operational(bool),
       Human(f64),
       Semantic(f64),
   }
   ```

3. **TrustProfile::composite()**: 未実装。RFC §10.4 で重み付き複合スコアが規定済み:
   ```rust
   fn composite(&self) -> f64 {
       0.35 * self.operational
       + 0.25 * self.semantic
       + 0.20 * self.temporal
       + 0.20 * self.human.score
   }
   ```

4. **MemoizedGraph::update_trust()**: 未実装。RFC §10.5 で疑似コードが規定済み。現状の MemoizedGraph は M1-2 で実装された以下のフィールドを持つ:
   - `id: String`
   - `trust: TrustProfile` (operational, semantic, temporal, human 完備)
   - `cache_invalidated: bool`
   - `invalidate_applicability_cache()` — フラグを立てる
   - `new(id, human_score)` — コンストラクタ

5. **HumanTrustLogistic::update()**: `src/types.rs:4405` で実装済み。RFC §10.3 のロジスティック更新式に準拠:
   ```rust
   pub fn update(&mut self, outcome: f64) {
       let expected = 1.0 / (1.0 + (-(self.score - 0.5) / self.scale).exp());
       self.score = (self.score + self.k * (outcome - expected)).clamp(0.0, 1.0);
       self.count += 1;
   }
   ```

6. **test PRNG シード**: `src/constants.rs:106` で `TEST_PRNG_SEED = 12345` が定義済み。観測テストで使用可能。

## Test Plan

### 不変条件テスト (T1〜T8)

| ID | テスト内容 | 確認項目 |
|----|-----------|---------|
| T1 | `TrustUpdate::Human(0.5)` — outcome=0.5、1回更新後、キャッシュ無効化が発生するか検証 | Human 更新がキャッシュ無効化をトリガー |
| T2 | `TrustUpdate::Human(0.5)` の前後で `composite()` が正しく変化する | 複合スコア計算の正確性 |
| T3 | `TrustUpdate::Operational(true)` → 常にキャッシュ無効化 | Operational の常時無効化保証 |
| T4 | `TrustUpdate::Semantic(0.5)` → 常にキャッシュ無効化 | Semantic の常時無効化保証 |
| T5 | 連続10回の微量フィードバック (outcome=0.51, 1回あたりのスコア変動 < 0.05) → 全回でキャッシュ無効化スキップ | Debounce 閾値の基本的な機能 |
| T6 | 大フィードバック (outcome=1.0, 変動 ≥ 0.05) → キャッシュ無効化発生 | 閾値超過時の正しい動作 |
| T7 | コールドスタート時の composite() 値が正しい (0.35*0 + 0.25*0 + 0.20*0 + 0.20*0.50 = 0.10) | 初期複合スコア計算 |
| T8 | 3種類の TrustUpdate を連続で呼び出し、状態が正しく遷移することを確認 | 状態機械の逐次正確性 |

### 観測テスト (OTS)

| ID | 観測対象 | 手法 | n |
|----|---------|------|---|
| OTS-1 | ステップ関数応答: ΔT が 0.000〜0.100 を 0.001 刻みで変化させたときのキャッシュ無効化発動率 | ΔT ごとに同一条件で 100 回試行し、無効化フラグ発動率を計測。不感帯 (ΔT < 0.05) での発動率 0%、通過帯 (ΔT ≥ 0.05) での発動率 100% を検証 | 101 水準 × 100 = 10,100 |
| OTS-2 | 微小フィードバック連続注入による累積デルタ追跡 | outcome=0.51 を 100 回連続注入。累積スコア変動が 0.05 に達した回 (N_th) 以降のみキャッシュ無効化が発生することを観測 | 100 |
| OTS-3 | フィードバック注入レイテンシ分布 | `update_trust(TrustUpdate::Human(0.5))` を 10,000 回実行し、各呼び出しのレイテンシ平均・最大・最小・分位数を計測 | 10,000 |

## 計装方法・観測対象

### 計装方法

- `src/trust.rs` 内 `#[cfg(test)] mod tests` に実装
- 固定シード PRNG (`StdRng::seed_from_u64(constants::TEST_PRNG_SEED)`) を使用 (OTS-1 のノイズ注入等)
- `std::time::Instant` を用いた高分解能レイテンシ計測
- `println!` による構造化出力 (`--nocapture` 経由で観測)

### 観測対象

**OTS-1: ステップ関数応答**
- ΔT = 0.000 から 0.100 まで 0.001 刻みの 101 水準
- 各水準で 100 回試行し、キャッシュ無効化発動フラグの平均値を算出
- 出力: `println!("OTS-1,delta={:.3},inv_rate={:.4}", delta, rate)`
- 期待: `ΔT < 0.05 → inv_rate = 0.0`, `ΔT ≥ 0.05 → inv_rate = 1.0`

**OTS-2: 累積デルタ追跡**
- outcome=0.51 で 100 回連続更新
- 各ステップの composite スコア、累積変動デルタ、キャッシュ無効化フラグを出力
- 期待: 累積デルタが 0.05 に達した最初のステップでキャッシュ無効化が ON になり、以降維持

**OTS-3: レイテンシ分布**
- n = 10,000 回の `update_trust(TrustUpdate::Human(0.5))` をループ
- 平均・最小・最大・P50・P95・P99 レイテンシを観測
- 期待: レイテンシが実用的範囲 (μs オーダー) であること

### 較正計画

本チケットでは `TRUST_DEBOUNCE_DELTA = 0.05` を Calibration Candidate として導入する。初期値は RFC 指定値の 0.05 とし、OQ-11 の評価に基づいて将来の較正ループで調整可能とする。

較正目的関数 J(θ) の設計候補:
- `J = α * (1 - 不感帯精度) + β * (平均レイテンシ / 基準値) + γ * (通過帯誤差率)`
- α = 0.6, β = 0.2, γ = 0.2 (収束速度より精度重視)

## Boy Scout Rule — 翻訳可能性計画

このチケットで触るコードに対して以下の改善を行う:

1. **`TrustUpdate` enum の明確なドキュメント**: 各バリアントの意味と期待される値域をコメントで記述 (例: `// outcome ∈ {0.0, 0.25, 0.5, 0.75, 1.0} — 0.0=thumbs-down, 1.0=thumbs-up`)。

2. **`update_trust` は一関数一責務**: RFC の疑似コード通り、各マッチアームが1つの更新処理 + キャッシュ無効化判断を行う翻訳可能な構造を維持する。

3. **ハードコード値の排除**: `0.05` を `TRUST_DEBOUNCE_DELTA`、重み値 (0.35/0.25/0.20/0.20) をコメントで RFC §10.4 にトレースバックする。

4. **テスト関数名の散文性**: `fn t1_human_update_triggers_cache_invalidation()` のようにテスト名が検証内容を一文で語る命名とする。

5. **composite() の重み定数化**: 後続チケットで調整可能なように、複合スコアの重みは現状ハードコードとするが、Calibration Candidate としてのコメントを付記する。

## Acceptance Criteria

- [ ] `TRUST_DEBOUNCE_DELTA = 0.05` が `src/constants.rs` に Calibration Candidate として定義されている
- [ ] `TrustUpdate` enum (3 バリアント) が `src/types.rs` に定義されている
- [ ] `TrustProfile::composite()` が RFC §10.4 の重み計算で実装されている
- [ ] `MemoizedGraph::update_trust()` が RFC §10.5 の疑似コードに忠実に実装されている
- [ ] Human 更新時の Debounce ロジックが正しく動作する (変動 < 0.05 → スキップ、≥ 0.05 → 無効化)
- [ ] Operational / Semantic 更新時は常にキャッシュ無効化される
- [ ] T1〜T8 の全不変条件テストが通過
- [ ] OTS-1: 不感帯 (ΔT < 0.05) の無効化発動率 = 0%、通過帯 (ΔT ≥ 0.05) の発動率 = 100%
- [ ] OTS-2: 累積デルタが 0.05 を超えるまでキャッシュ無効化が発生しない
- [ ] `cargo test` が全て PASS (既存テスト含む)

## Notes

- plan_path: 未作成
- implementation_path: 未作成
- review_report_path: 未作成
- observation_report_path: 未作成

### 成果物

- 計画: `context/0057-m1-3-debounce/plan.md`（未作成、`/plan-ticket` 承認後に作成）
- 実装サマリ: `context/0057-m1-3-debounce/implementation.md`（未作成、`/start-ticket` 実装完了後に作成）
- レビュー報告書: `context/0057-m1-3-debounce/review.md`（未作成、`/review-ticket` 全チェック通過後に作成）
- 観察レポート: `context/0057-m1-3-debounce/observation-YYYYMMDD-HHmmss.md`（未作成、`/start-ticket` 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
