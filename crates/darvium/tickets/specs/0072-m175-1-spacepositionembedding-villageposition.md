---
ticket_id: 72
title: M1.75-1: SpacePositionEmbedding / VillagePosition 型定義および位置更新ダイナミクスの実装
slug: m175-1-spacepositionembedding-villageposition
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0072-m175-1-spacepositionembedding-villageposition/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0072-m175-1-spacepositionembedding-villageposition/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0072-m175-1-spacepositionembedding-villageposition/observation-20260524-140442.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0072-m175-1-spacepositionembedding-villageposition/review.md
---
# M1.75-1: SpacePositionEmbedding / VillagePosition 型定義および位置更新ダイナミクスの実装

## Summary

v2.3-e Child Support Villages / HELP Consensus の基盤として、ワークフローの生態学的位置（ecological position）を表現する `SpacePositionEmbedding` / `VillagePosition` 型を定義し、観測位置に対する指数平滑化 (exponential smoothing) による位置更新ダイナミクスを実装する。また位置更新を EventBus と結合し、`DarviumEventKind::System` イベントとして publish する機能を提供する。

## Background

RFC §41B.2 では、各 `MemoizedGraph` が低次元連続空間における現在の生態学的位置を表す `spacepositionembedding: Option<[f32; 3]>` フィールドを持つことが規定されている。この位置情報は以下を支える基盤である：

1. **局所性形成 (locality formation)**: 類似したワークフローが近接する位置に集まることで、自然なクラスタリングが生まれる
2. **近傍観測 (neighborhood observation)**: チャイルドが自身の近くにいるアダルトヘルパーを発見する
3. **チャイルドサポートルーティング**: HELP プロトコルにおける helper 選択の距離指標
4. **較正**: α（平滑化率）が calibration candidate として実験的に調整される

本チケットは M1.75 マイルストーン全体の基礎となる。後続チケット（M1.75-2: 成熟判定器、M1.75-3: HELP プロトコル状態機械など）は本チケットで定義される位置型・更新関数に依存する。

すでに完了している M1.5-R 系列（Event Architecture）の `DarviumEventBus`・`VirtualClock`・`DarviumEventKind::System` を位置更新イベントの publish 先として利用する。

## Scope

### 実装するもの

1. **`SpacePositionEmbedding` 型**: RFC §41B.2 に従い `Option<[f32; 3]>` の newtype ラッパーとして定義。3次元連続空間の位置ベクトルを表現する。
2. **`VillagePosition` 構造体**: 位置 + 更新時刻情報を保持するランタイム型。
3. **`VillageObservation` 構造体**: 観測位置の分解要素（ミッション局所性・ヘルパー局所性・知識局所性）を保持する。
4. **`PositionUpdatePolicy` 列挙型**: 更新条件（定期更新・イベント駆動更新・最小変化量閾値）を規定する。
5. **`update_space_position(prev, obs, alpha) -> VillagePosition`**: 指数平滑化による位置更新純粋関数（式 41B-1）。
6. **`should_update_position(last_updated_vt, now_vt, policy) -> bool`**: VirtualClock ベースの更新判定関数。
7. **`l2_distance(a, b) -> f64`**: 固定点収束テストのための L2 距離補助関数。
8. **位置更新 → `DarviumEventKind::System(SpacePositionUpdated)` publish 機能**。
9. **`SpacePositionUpdated` ペイロード型**: 更新前位置・更新後位置・観測・alpha を保持する。

### 実装ファイル

RFC §41B.17 の推奨実装分割に従い、本チケットのコードは `src/spaceposition.rs` に配置する。これにより `src/village.rs`（M1.75-2）や `src/help.rs`（M1.75-3）と明確に責務分離される。

### 機能的結合

- `DarviumEventBus`（event.rs）: `publish()` 経由で位置更新イベントを発行
- `VirtualClock`（clock/）: `current_clock()` 経由で更新タイミング制御
- `DarviumEventKind::System`（event.rs）: `SpacePositionUpdated` ペイロードを持つ

## Non-scope

- MemoizedGraph 構造体自体への `spacepositionembedding` フィールド追加（該当構造体がまだ実装されていない、もしくは別チケットで管理する場合）
- Child/Adult 成熟度判定器（M1.75-2）
- HELP プロトコル状態機械（M1.75-3）
- 近傍選択・village 構成ロジック（M1.75-2）
- チャイルドサポートミッションオーケストレーション（M1.75-5 以降）
- 村の安定性メトリクス（M1.75-7）
- 位置埋め込みの永続化（MetadataStore 統合）

## Investigation

### RFC 交叉参照結果

#### RFC §41B.2「空間位置埋め込み」

- `spacepositionembedding` は `Option<[f32; 3]>` 型（3次元に限定される根拠は RFC 本文の明示的な定義による）
- `spacepositionupdatedat` は `Option<SystemTime>` 型だが、本実装では EventBus の `VirtualClock`（u64 の単調増加カウンター）と統合するため `Option<u64>` として扱う
- 位置が存在しない場合（None）、局所性不明 (locality-unknown) として中立的動作にフォールバックする
- 更新則は指数平滑化: `x_{t+1} = (1-α)x_t + α·p_t` （式 41B-1）
- α は `0 < α ≤ 1` の calibration candidate
- 観測位置の分解: `p_t = λ_q·q_t + λ_h·h_t + λ_k·k_t` （式 41B-2）、λ_q + λ_h + λ_k = 1
- 本チケットでは観測位置の分解までは実装せず、外部から `VillageObservation` として与えられる前提とする

#### RFC §41B.17「推奨実装分割」

- `src/spaceposition.rs` が本チケットの担当ファイル
- 局所性距離（l2_distance）と位置更新を実装

#### RFC §12C（Event Architecture）

- 完了済み M1.5-R4 により `DarviumEventKind::System(SystemEvent)` が定義済み
- 完了済み M1.5-R5 により `DarviumEventBus` トレイトが利用可能
- 完了済み M1.5-R6 により `VirtualClock` が EventBus commit clock として再定義済み

### 既存コード調査

- `src/event.rs`: `DarviumEventKind::System(SystemEvent)` は variant として存在する（345行目）。SystemEvent の詳細を確認する必要あり。
- `src/constants.rs`: 村関連の定数は未定義。本チケットで位置更新率 α の初期値を追加する。
- `src/types.rs`: 空間位置関連の型は未定義。本チケットで `SpacePositionEmbedding` などを追加する。または、RFC §41B.17 の推奨に従い専用ファイル `src/spaceposition.rs` を作成する。
- `src/clock/`: VirtualClock トレイトが定義済み（EventBus の commit clock）。
- 既存の SystemEvent 列挙型が SpacePositionUpdated に対応しているか確認要。

### SystemEvent 列挙型の確認

```rust
// event.rs 内の SystemEvent 定義を確認する必要あり
```

## Test Plan

### ユニットテスト（`src/spaceposition.rs` の `mod tests` 内）

#### T-1: 位置更新の境界値（alpha = 0.0）
- `alpha = 0.0` で `update_space_position` を呼び出したとき、返される位置が `prev` と完全一致することを検証
- 任意の位置ベクトル、任意の観測値で成立すべき不変条件

#### T-2: 位置更新の境界値（alpha = 1.0）
- `alpha = 1.0` で `update_space_position` を呼び出したとき、返される位置が観測 `obs.delta` に完全一致することを検証

#### T-3: 指数固定点収束
- 同一観測を反復入力したとき、位置系列が指数的に固定点へ収束することを検証
- 各ステップの残差 `|x_n - x*|` が単調減少することを確認
- 収束後の固定点が理論値 `x* = p`（観測位置）と一致することを確認

#### T-4: L2 距離の正値性
- 任意の 2 点間の `l2_distance` が非負であること
- 同一位置間の距離が 0 であること
- 三角不等式が成立すること

#### T-5: should_update_position の更新窓制御
- `last_updated_vt == now_vt` のとき `should_update_position` が `false` を返す
- `now_vt - last_updated_vt >= policy.min_interval_ticks` のとき `true` を返す
- `policy == PositionUpdatePolicy::Always` のとき常に `true` を返す

#### T-6: SpacePositionUpdate イベント publish
- 位置更新後にダミー EventBus 経由で `DarviumEventKind::System(SpacePositionUpdated {..})` イベントが publish されていることを検証
- publish されたイベントのペイロードに更新前位置・更新後位置・観測・alpha が正しく格納されていることを確認

#### T-7: SpacePositionEmbedding newtype
- `SpacePositionEmbedding::from([x, y, z])` で生成できること
- `inner()` または `Deref` で内部の `[f32; 3]` にアクセスできること
- `Option::None` が locality-unknown 状態を表現できること

#### T-8: VillageObservation の構築
- `VillageObservation::new(delta)` で最小構成が構築できること
- 全フィールド（mission_locality, helper_locality, knowledge_locality, delta）が正しく設定されること

### 観測テスト（`tests/` の統合テスト）

#### 観測 O-1: 位置更新系列の統計的観測
- 固定シード PRNG（`StdRng::seed_from_u64(12345)`）で観測ノイズを注入した位置更新系列を n=10,000 本生成
- 平均二乗変位 `⟨‖x(t)-x(0)‖²⟩` の時間発展を計測
- 緩和率 Γ を α の関数として観測（α ∈ {0.1, 0.3, 0.5, 0.7, 0.9}）
- 軌道が発散せず、有限分散に拘束されることを確認

#### 観測 O-2: 更新イベント発火密度
- EventBus `current_clock()` の更新窓幅（min_interval_ticks）を走査（1, 5, 10, 50）
- 過剰更新によるノイズ増幅が起きる臨界領域を同定
- 各設定における位置更新イベント発火密度を観測

#### 観測 O-3: publish 完全性
- 位置更新 1 件につき EventBus イベント 1 件の対応関係を n=1,000 で検証
- イベント消失率が 0% であることを確認

## 計装方法・観測対象

### 計装方法

- 計装コードは `src/spaceposition.rs` の `mod tests` 内および `tests/` の統合テストに実装
- `println!` + `--nocapture` 経由で構造化テキスト（CSV/JSON）を標準出力に書き出す
- 固定シード PRNG（`StdRng::seed_from_u64(12345)`）を使用し完全再現を保証
- 観測テスト O-1〜O-3 は `#[cfg(test)]` で隔離し、通常の `cargo test` で実行可能

### 観測対象

| 観測量 | シンボル | サンプルサイズ | 期待される性質 |
|---------|----------|---------------|----------------|
| 平均二乗変位 | MSD(t) | n=10,000 | 有限時間で飽和（発散しない） |
| 緩和率 | Γ(α) | α 5水準 × 1,000 | α の単調増加関数 |
| イベント発火密度 | ρ(Δt) | 窓幅4水準 × 500 | 窓幅の単調減少関数 |
| publish 完全性 | η | n=1,000 | η = 1.0（消失 0%） |

### 較正計画

本チケットでは以下の calibration candidate を導入する：

| 定数名 | 初期値 | 分類 | 備考 |
|--------|--------|------|------|
| `SPACE_POSITION_UPDATE_ALPHA` | 0.30 | Calibration Candidate | 指数平滑化率 α、RFC §41B.2 |
| `SPACE_POSITION_UPDATE_MIN_INTERVAL` | 5 | Calibration Candidate | 最小更新間隔（ticks） |
| `SPACE_POSITION_L2_EPSILON` | 1e-6 | Safety Invariant | L2 距離のゼロ判定閾値 |

目的関数 J(α) は本チケットでは定義せず（単独では村全体の挙動を評価できない）、M1.75-11 の J_village(θ) の構成要素として組み込まれる。

## Boy Scout Rule — 翻訳可能性計画

本チケットは新規ファイル `src/spaceposition.rs` を作成するため、翻訳可能性を最初から確保した設計とする：

- **関数名は動詞句**: `update_space_position`, `should_update_position`, `l2_distance`
- **構造体名は名詞**: `SpacePositionEmbedding`, `VillagePosition`, `VillageObservation`, `PositionUpdatePolicy`
- **一関数一責務**: 位置更新・更新判定・距離計算・イベント publish は別関数に分割
- **ハードコード値は名前付き定数**: α の初期値などは `constants.rs` に定義し、コード中に直書きしない
- **エラー握りつぶし禁止**: 位置更新結果の `Result` は上位に伝播する（`unwrap()` 不使用）
- **コメントは「なぜ」を説明**: 更新式の数学的背景（指数平滑化の選択理由）を日本語でコメントする

## Acceptance Criteria

- [ ] SpacePositionEmbedding / VillagePosition / VillageObservation / PositionUpdatePolicy の型定義が完了している
- [ ] update_space_position 純粋関数が指数平滑化（式 41B-1）を正しく実装している
- [ ] alpha=0.0 / 1.0 の境界値テストに合格する
- [ ] 同一観測反復入力時の指数収束テストに合格する
- [ ] should_update_position が VirtualClock ベースの更新窓制御を正しく行う
- [ ] l2_distance が数学的に正しい（正値性・同一性・三角不等式）
- [ ] 位置更新後に EventBus へ SpacePositionUpdated イベントが publish される
- [ ] 観測テスト O-1〜O-3 が統計的に意味のある結果を出力する
- [ ] 既存テストが全て通過する
- [ ] 翻訳可能性の検証が通っている

## Notes

- RFC 41B.2 の式 (41B-1) に基づく実装であること
- α は後続チケット M1.75-11 で較正されるため、初期値は RFC 推奨値を使用する
- EventBus との結合部分は完了済みの M1.5-R 系列（event.rs）に依存する
- 観測位置の分解（式 41B-2）自体は本チケットのスコープ外。`VillageObservation` の各成分は外部から与えられる前提とし、内部での λ 加重合成は行わない

### 参照観察レポート

- `tickets/context/0071-m15-r11-event-architecture/observation-20260524-134843.md` — Event Architecture 較正完了確認、DarviumEventBus および VirtualClock の安定動作確認済み

### 成果物

- 計画: context/0072-m175-1-spacepositionembedding-villageposition/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0072-m175-1-spacepositionembedding-villageposition/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0072-m175-1-spacepositionembedding-villageposition/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0072-m175-1-spacepositionembedding-villageposition/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
