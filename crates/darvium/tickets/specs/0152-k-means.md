---
ticket_id: 152
title: k-means実行間隔の設定可能化
slug: k-means
status: done
created_at: 2026-06-01
updated_at: 2026-06-01
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0152-k-means/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0152-k-means/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0152-k-means/observation-20260601-142517.md
---

# k-means実行間隔の設定可能化

## Summary

`phase2_village_clustering`（k-means）が毎 tick O(n × k × iter) の距離計算を実行し、支配的ボトルネックとなっている。k-means の実行頻度を間引いて高速化する。間隔はフロントエンドから設定可能にする。

## Background

#149-151 の最適化は O(n) の定数倍削減にとどまった。真の支配項は Phase 2 の k-means:
- k-means++ 初期化: O(n × k)
- Lloyd 反復 × 最大20回: O(n × k × iter)
- k ≈ n/50 なので実質 O(n²/50)
- 人口 2488 時点で ~620K 距離計算/tick

## Scope

1. **`ReciprocitySimulatorConfig.village_recluster_interval` 追加**: k-means 実行間隔 (tick数), デフォルト 1
2. **`phase2_village_clustering` 改造**: 間引き tick は簡易 Voronoi 割り当て
3. **Voronoi 割り当て関数追加**: 既存 centroid から各個人を最寄りに割り当て
4. **フロントエンド連携**: スライダー + start コマンド + server.rs config parsing
5. **`SimulationContext.last_centroids` 追加**: 前回 k-means の centroid を保持するフィールド

## Non-scope

- Voronoi による村融合・分裂は対象外
- k-means 以外のクラスタリング手法への置き換え
- `kind_world.rs` の `assign_village_ids` の改造（独立した別機能）

## Investigation

### 物理的証拠

**証拠1**: `phase2_village_clustering`（`simulation.rs:2718`）の構造
- k-means++ 初期化（`simulation.rs:2765-2797`）: 固定 seed 42 の PRNG で centroids を選択
- Lloyd 反復（`simulation.rs:2801-2842`）: 最大 50 イテレーション、収束時 break
- 割り当て結果を `ctx.population[id].village_assignment` に書き戻し（`simulation.rs:2845-2854`）
- **centroids は関数ローカル変数であり、関数終了時に破棄される**

**証拠2**: `phase2_village_clustering` は **4箇所** のシミュレーションループから呼ばれている
- `simulation.rs:1861` — `run_simulation`（標準シミュレーション）
- `simulation.rs:2068` — `run_kw_real_simulation`（KW-REAL）
- `simulation.rs:2236` — `run_evaluation_simulation_with_channel`（評価用、チャネル付き）
- `simulation.rs:8102` — `run_evaluation_simulation`（評価用）
- すべて `config.target_village_size` を直接渡している

**証拠3**: `ReciprocitySimulatorConfig`（`simulation.rs:121-130`）に `village_recluster_interval` フィールドは存在しない
- `Default` 実装（`simulation.rs:132-158`）で全フィールドを初期化
- `target_village_size: Option<f64>` は既存（デフォルト `None` = 定数 `TARGET_VILLAGE_SIZE` 使用）

**証拠4**: 明示的フィールド初期化を行うテスト設定が9箇所存在
- `simulation.rs:4784`, `5071`, `5201`, `5263`, `5362`, `5431`, `5510`, `6850`, `6967`
- これらは `ReciprocitySimulatorConfig { field: ..., ..Default::default() }` に変更する必要がある

**証拠5**: サーバー設定パース（`server.rs:240-260`）
- `if let Some(val) = obj.get("field").and_then(|v| v.as_f64()) { cfg.field = val; }` のパターン
- `village_recluster_interval` 用のパース行を追加する

**証拠6**: フロントエンド start コマンド（`web/cube/observation/script.js:621-630`）
- `sendCommand("start", { population_size, max_ticks, target_village_size })` の形式
- `village_recluster_interval` を追加する

**証拠7**: Voronoi 割り当て関数は未実装
- centroid を保持する機構がない（`SimulationContext` に `last_centroids` フィールドが必要）
- `phase2_village_clustering` は centroids を返さない

### RFC 既存実装状態検証（plan-time: 2026-06-01）

#### 該当 RFC セクション

| セクション | 内容 | 関連性 |
|-----------|------|--------|
| §4A.0.9 | 村形成系（41B-6・41B-7）— 2 定数 | `VILLAGE_DISTANCE_THRESHOLD`, `VILLAGE_MIN_SIZE` |
| §41B.3 | Child, adult, and local village | 村クラスタリングの理論的基礎（TopK / radius） |
| §41B.14 | Stability and dynamicity discipline | 村チャーン指標（41B-22, 41B-23）、動特性の評価方法 |
| §41B.15 | Operational metrics and calibration candidates | 運用指標一覧、較正候補 |
| §41B.17 | Recommended implementation decomposition | `src/village.rs` への分割推奨 |

#### RFC §4A.0.9 村形成系定数

| フィールド | RFC の型 | 現行コードの型 | 状態 |
|-----------|----------|---------------|------|
| `VILLAGE_DISTANCE_THRESHOLD` | f64 (0.2) | (欠落) | ⚠️ RFC定義だが未実装 (Unused分類のため許容) |
| `VILLAGE_MIN_SIZE` | usize (3) | (欠落) | ⚠️ RFC定義だが未実装 (Unused分類のため許容) |

**評価サマリ**: 本チケットのスコープ（`village_recluster_interval` 追加）は RFC に定義がない新規最適化である。RFC の §41B.14（安定性と動特性の規律）は村構造が毎 tick 変化すべきでないことを示唆しており、本変更は RFC と矛盾しない。

### 参照観察レポート

- `tickets/context/0137-untitled/observation-20260528-154420.md` — 評判再計算パイプライン実装後のシミュレーション完全性確認。村中心性算出が導入された状態での動作確認結果。
- `tickets/context/0115-m176-kw-real-p4-6/observation-20260526-201837.md` — KW-REAL シミュレーションでの村クラスタリング動作確認。k-means が毎 tick 実行されていることの観測証拠を含む。

### 参照チケット
- #149 - #151: 前段の性能最適化

## Test Plan

### T1: `village_recluster_interval=1` で後方互換性
- `village_recluster_interval=1`（従来動作）でシミュレーションを実行
- 村割り当て結果が `interval=1` なし（従来コード）と同一であることを検証
- **正常系**: 同一 seed → 同一の village_assignment 履歴

### T2: `village_recluster_interval=N` で N tick ごとに村再編成
- `village_recluster_interval=5` でシミュレーションを実行
- tick 1, 6, 11, ... でのみ k-means 実行、それ以外は Voronoi 割り当て
- **正常系**: 間引き tick でも全生存者がいずれかの村に所属する
- **正常系**: 間引き tick における村割り当てが変化しない（centroid からの最近傍割り当て）

### T3: Voronoi 割り当ての完全性
- 全生存者が Voronoi 割り当て後もいずれかの村に所属する
- 空の村が発生しない（centroid ごとに少なくとも1人所属する）
- **異常系**: 人口 0 の場合は 0 村を返す
- **境界値**: 人口 < k の場合の動作（各個人が独立した村）

### T4: `village_recluster_interval=0` のエラー処理
- 0 を指定した場合の挙動を定義（デフォルト値 1 にフォールバック or エラー）
- **異常系**: clamp で [1, u64::MAX] に制限することを確認

### T5: 距離計算回数の削減確認（観測テスト）
- k-means 実行時と Voronoi 割り当て時の距離計算回数を計測
- Voronoi 割り当てが k-means++ 初期化 + Lloyd 反復より有意に少ないことを確認
- **観測対象**: 距離計算回数, 実行時間

## 計装方法・観測対象

### 計装方法
- `println!` + `--nocapture` で距離計算回数・実行時間を標準出力に出力
- 固定シード `StdRng::seed_from_u64(12345)` で決定論的実行
- カウンタ変数 `ctx.kmeans_distance_computations` を `SimulationContext` に追加

### 観測対象
- **統計量**: k-means 実行 tick vs Voronoi 割り当て tick の距離計算回数比
- **サンプルサイズ**: 同一設定で 3 回実行（決定論的のため 1 回で十分だが、一貫性確認用）
- **期待される現象**: `village_recluster_interval=10` で距離計算回数が約 1/5 〜 1/10 に削減される

### 較正計画
- **調整する定数**: `village_recluster_interval`（constants.rs ではなく Config フィールドとして管理）
- **目的関数 J(θ)**: `J = -computations + w * stability_penalty`
  - `computations`: シミュレーション全体の距離計算回数（対数スケール）
  - `stability_penalty`: 村割り当ての churn rate（変化が激しすぎないか）
  - `w`: 重み係数（デフォルト 0.1）
- **較正ループの停止条件**: フロントエンドから設定可能なので、ユーザーが適宜調整する

## Implementation Plan

### Step 1: `SimulationContext` に `last_centroids` 追加
- `simulation.rs` の `SimulationContext` 構造体に `last_centroids: Vec<[f32; 3]>` フィールドを追加
- `new()` で `Vec::new()` に初期化

### Step 2: `ReciprocitySimulatorConfig` に `village_recluster_interval` 追加
- 型: `u64`、デフォルト: `1`（毎 tick = 従来動作）
- `Default` impl に `village_recluster_interval: 1` を追加

### Step 3: Voronoi 割り当て関数 `phase2_voronoi_assign` 追加
```rust
fn phase2_voronoi_assign(
    ctx: &mut SimulationContext,
    alive_ids: &[PersonId],
    centroids: &[[f32; 3]],
) -> usize {
    // centroids から各個人を最寄りに割り当てる
    // 空の村が発生しないことを保証
    // 村数を返す
}
```

### Step 4: `phase2_village_clustering` 改造
- k-means 実行後に `ctx.last_centroids = centroids.clone()` で保存
- 戻り値変更なし（依然として村数）

### Step 5: 4箇所のシミュレーションループで分岐
- `if tick % config.village_recluster_interval == 0` → k-means
- `else if !ctx.last_centroids.is_empty()` → Voronoi 割り当て
- `else` → 割り当て変更なし（初回 tick の直前など）

### Step 6: テスト設定更新
- 9箇所の明示的フィールド初期化に `village_recluster_interval: 1` または `..Default::default()` を追加

### Step 7: サーバー設定パース追加 (`server.rs`)
```rust
if let Some(val) = obj.get("village_recluster_interval").and_then(|v| v.as_u64()) {
    cfg.village_recluster_interval = val;
}
```

### Step 8: フロントエンド UI 追加
- index.html: スライダー `id="villageReclusterInterval"`, min=1, max=50, value=1
- script.js: start コマンドに `village_recluster_interval` を含める

## Boy Scout Rule — 翻訳可能性計画

- `phase2_village_clustering` から Voronoi 割り当てを分離し `phase2_voronoi_assign` として独立した関数にする
- k-means++ 初期化の seed 値 `42` を名前付き定数 `KMEANS_PLUSPLUS_SEED` に抽出（`constants.rs`）
- Lloyd 反復の最大値 `50` を名前付き定数 `KMEANS_MAX_ITERATIONS` に抽出（`constants.rs`）
- 距離計算クロージャ `distance` を `phase2_village_clustering` と `phase2_voronoi_assign` で共有するため、ヘルパー関数 `fn euclidean_distance_3d(a: &[f32; 3], b: &[f32; 3]) -> f64` に抽出
- `k = max(1, round(n / target))` の計算ロジックをヘルパー関数 `fn compute_village_count(n: usize, target: f64) -> usize` に抽出（`phase2_village_clustering` と `phase2_voronoi_assign` で共有）

## Acceptance Criteria

- [ ] k-means 実行間隔をフロントエンドのスライダーで設定できる
- [ ] 間引き tick では簡易 Voronoi 割り当てのみ行う
- [ ] 村数が変化しない（村の大構造は保持）
- [ ] 既存テストがすべて通過する
- [ ] 距離計算回数が間引き率に比例して削減される

## Notes

### 成果物

- 計画: context/0152-k-means/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0152-k-means/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0152-k-means/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0152-k-means/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
