---
ticket_id: 148
title: 空間移動力学 — 首長レジストリと引力・斥力による個体移動
slug: untitled-4
status: reviewed
created_at: 2026-05-29
updated_at: 2026-05-29
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0148-untitled-4/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0148-untitled-4/observation-20260529-150743.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0148-untitled-4/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0148-untitled-4/review.md
---
# 空間移動力学 — 首長レジストリと引力・斥力による個体移動

## Summary

チケット #147 で導入した首長性スコアを基盤に、首長レジストリと空間移動力学を実装する。首長を中心とした引力・斥力により、Darvium 空間内に秩序と流動を同時に生み出す。

## Background

現在、各個人の Darvium 空間位置は出生時のランダム初期化と摂動コピーのみで決定され、連続的な移動力学が存在しない。チケット #147 で首長（chiefdom_score 最大の個体）が各村に選出されるようになったが、首長と非首長の間の空間的関係は未定義である。

首長を空間的な「引力点」として機能させることで、以下の効果を期待する：
- 高い能力を持つ個体（首長）の周囲に非首長が集積する秩序
- 首長同士の斥力による散らばり（単一地点への過度な集中を防止）
- 非首長の「最近距離」到達後の2番目首長へのスイッチによる流動（滞在の固定化を防止）
- 主首長（最優秀首長）を不動点とした座標系の安定

## Scope

- ChiefRegistry 構造体の作成（Arc RwLock によるスレッドセーフシングルトン）
- MOVEMENT_DISTANCE, MIN_APPROACH_DISTANCE 定数の追加
- 首長移動ベクトル計算（主首長への引力 + 他首長からの斥力）
- 非首長移動ベクトル計算（最寄り首長への引力、最近距離到達後の2番目首長スイッチ）
- 単一首長時のフォールバック（近傍ランダム点）
- Phase 3.9 としてシミュレーションループに移動フェーズを追加
- compute_chief_movements 関数（一括移動計算 + 位置更新）
- フロントエンド: 主首長の視覚的識別（任意、首長との差別化）

## Non-scope

- 位置の指数平滑化（既存の update_space_position 関数は別用途）
- 村クラスタリングロジックの変更（Phase 2 は既存のまま）
- 首長選出ロジックの変更（Phase 3.8 は既存のまま）
- 位置情報の永続化（DB保存）

## Investigation

### 現在の位置管理

src/trust.rs (line 55):
```
pub position: SpacePositionEmbedding,
```

src/spaceposition.rs (lines 24-36):
SpacePositionEmbedding は Option[f32; 3] の newtype。3次元位置を [f32; 3] で保持。初期値は各軸 uniform [0,1)。

src/simulation.rs (line 416):
出生時位置: [rng.random::f32, rng.random::f32, rng.random::f32]

src/simulation.rs (lines 2437-2444):
子個体の位置摂動（親位置に [-0.05, +0.05] の一様ノイズ）。これが唯一の「移動」であり、連続的な移動力学は存在しない。

### Phase ループの構造 (simulation.rs lines 1561-1654)

```
Phase 1:     人口増加
Phase 2:     村クラスタリング
Phase 2.5:   村中心性算出
Phase 3:     HELP プロトコル
Phase 3.5:   評判再計算
Phase 3.7:   首長性スコア計算 (by #147)
Phase 3.8:   首長選出 (by #147)
Phase 3.6:   自己抽象化
Phase 4:     GC（生存判定）
Phase 5:     能力拡散
Phase 6:     J_kw 測定
```

### 諸元

- 位置は [f32; 3] の連続値、全軸概ね [0,1] の範囲（摂動により逸脱可）
- village_chiefs: HashMap VillageId, PersonId は既存（Phase 3.8 の出力）
- フロントエンドは payload.nodes の position フィールドを読んで描画
- 移動のみではイベントペイロードの変更は不要（位置は自動反映）

### 参照観察レポート

- tickets/context/0147-untitled-3/observation-20260529-140858.md — 首長性スコア導入、首長選出の動作確認

## 移動力学の定義

### 首長レジストリ

```
pub struct ChiefRegistry {
    chiefs: HashMap,PersonId, ChiefEntry,,
}

pub struct ChiefEntry {
    pub person_id: PersonId,
    pub position: [f32; 3],
    pub chiefdom_score: f32,
    pub village_id: VillageId,
}
```

シングルトン: Arc RwLock ChiefRegistry として SimulationContext に保持。
Phase 3.8（首長選出）完了後に registry を全更新（clear + rebuild from village_chiefs）。

### 主首長の決定

ChiefRegistry 内で chiefdom_score が最大の chief を主首長とする。同点の場合は最初に見つかった方を採用。

### 移動規則

移動距離: MOVEMENT_DISTANCE（定数、全移動で共通）

#### 主首長
- 移動しない（全軸 0）

#### 副首長（主首長以外の首長）
- 引力成分: normalized(main_chief_pos - self_pos) → 主首長方向
- 斥力成分: sum(normalized(self_pos - other_chief_pos)) over all other chiefs → 総合離散方向
- 合力方向: normalized(attract_vector + repulsion_vector)
- 最小接近距離: 主首長との距離が MIN_APPROACH_DISTANCE 未満の場合、引力成分を 0（斥力のみ）
- 移動: self_pos += MOVEMENT_DISTANCE * resultant_direction

#### 非首長
- 第1優先: 最も近い首長を探す
- 距離 > MIN_APPROACH_DISTANCE: その首長に向かって移動
- 距離 <= MIN_APPROACH_DISTANCE:
  - 2番目に近い首長が存在する → その首長にターゲットを切り替え、以降はその首長に向かい続ける
  - 2番目に近い首長が存在しない（首長が1人のみ） → 首長の近傍ランダム点に向かう
- 移動: self_pos += MOVEMENT_DISTANCE * normalized(target_pos - self_pos)

### 数学的不変条件

1. MOVEMENT_DISTANCE < MIN_APPROACH_DISTANCE: 1 tick で最小接近距離を飛び越さない（オーバーシュート防止）
2. 生存個体のみ移動: 死亡個体は移動計算から除外
3. 位置は常に有効: 移動後も [f32; 3] として有効（NaN/Inf 禁止）
4. 首長は村に属する: village_assignment.is_some の個体のみ首長として registry に登録

## Test Plan

### T1: 首長レジストリ基本操作
- register, unregister, get_paramount, get_nearest, get_second_nearest
- 正常系: 複数首長の登録・主首長の特定・最寄り首長の距離順取得
- 境界値: 空レジストリからの get_paramount → None、1件のみからの get_second_nearest → None
- 同点 chiefdom_score → 最初に登録された方を主首長

### T2: 主首長の不動性
- 主首長が設定された状態で move_chiefs を実行 → 主首長の位置が変化しない

### T3: 副首長の引力
- 2首長（主 + 副1）で副首長の引力方向が正しいことを確認
- MIN_APPROACH_DISTANCE 未満接近時の引力停止を確認

### T4: 副首長の斥力
- 3首長以上で斥力成分が正しく合成されることを確認
- 全ての他首長から離れる方向成分を含むこと

### T5: 非首長の最寄り首長への移動
- 単一首長 + 非首長1: 非首長が首長に向かう方向を確認
- MIN_APPROACH_DISTANCE 到達後のターゲット切り替えを確認

### T6: 非首長の2番目首長スイッチ
- 2首長 + 非首長1: 最近距離到達後、2番目首長への移動方向を確認
- スイッチ後も首長間距離が変化しないことを確認

### T7: 単一首長時のフォールバック
- 首長1 + 非首長1: 最近距離到達後、首長近傍ランダム点への移動を確認
- 移動後の位置が首長位置と同一でないことのみ確認（ランダム性のため）

### T8: MOVEMENT_DISTANCE 不変条件
- MOVEMENT_DISTANCE < MIN_APPROACH_DISTANCE のアサート

### T9: 死亡個体の除外
- 死亡個体が移動計算に影響しないことを確認

### 観測テスト O1: 空間分布の時系列観測
- 固定シードで 50 tick 実行し、tick ごとに以下を CSV 出力:
  - 各個人の位置 (x, y, z)
  - 首長と非首長の分類
  - 主首長の位置
  - 各首長の周辺集中度（半径 MIN_APPROACH_DISTANCE 内の非首長数）

### 観測テスト O2: 定常分布の統計
- 100 tick 後の定常状態で:
  - 首長間の平均距離
  - 非首長の最近接首長距離の分布
  - 2番目首長追跡中の個体数

### 観測テスト O3: 主首長交代時の過渡応答
- 主首長を強制的に死亡させた際の再収束時間を計測

## 計装方法・観測対象

### 計装方法

- simulation.rs の tests モジュールに T1-T9（不変条件テスト、assert / assert_eq）
- 同 tests モジュールに O1-O3（観測テスト、println + --nocapture）
- 固定シード: StdRng::seed_from_u64(12345)
- テストは同一ファイルの mod tests に追加（既存の simulation.rs テストパターンに従う）

### 観測対象

| 観測量 | 期待される傾向 | 分析方法 |
|---|---|---|
| 首長間平均距離 | 定常状態で安定（斥力平衡） | 時系列プロット |
| 非首長の首長距離分布 | MIN_APPROACH_DISTANCE 付近にピーク | ヒストグラム |
| 2番目首長追跡率 | 定常状態で一定割合 | 割合の時系列 |
| 主首長交代収束時間 | 数 tick 以内 | ステップ応答 |

### 較正計画

| 定数 | 分類 | 初期値 | 調整範囲 |
|---|---|---|---|
| MOVEMENT_DISTANCE | Calibration Candidate | 0.02 | 0.005-0.10 |
| MIN_APPROACH_DISTANCE | Calibration Candidate | 0.05 | 0.01-0.20 |

目的関数 J(θ): 空間エントロピー（全個人位置の一様性からの乖離）+ 首長集中度の逆数。高い秩序（首長周辺への集中）と高い流動（集中の固定化防止）のバランスを評価。

## Boy Scout Rule — 翻訳可能性計画

- 首長・非首長の移動ロジックは別関数に分離（move_chiefs, move_non_chiefs）
- 合力ベクトル計算も関数抽出（compute_attraction_vector, compute_repulsion_vector）
- 位置定数は constants.rs に MOVEMENT_DISTANCE, MIN_APPROACH_DISTANCE として定義
- ハードコードされた位置摂動 0.1, 0.05 は既存定数があれば流用、なければ残存

## Acceptance Criteria

- [ ] ChiefRegistry がスレッドセーフに動作する
- [ ] 主首長が不動である
- [ ] 副首長が主首長方向に移動し、かつ他首長から離散する
- [ ] 非首長が最寄り首長方向に移動し、MIN_APPROACH_DISTANCE 到達後にターゲットを切り替える
- [ ] 単一首長時のフォールバックが動作する
- [ ] MOVEMENT_DISTANCE < MIN_APPROACH_DISTANCE の不変条件が維持される
- [ ] T1-T9 の全テストが通過する
- [ ] 既存テストが全て通過する（回帰なし）
- [ ] 翻訳可能性の検証が通っている

## Notes

### 成果物

- 計画: context/0148-untitled-4/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0148-untitled-4/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0148-untitled-4/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0148-untitled-4/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
