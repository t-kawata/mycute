# 変更したファイル一覧と実装内容の概要

## 変更ファイル

### 1. src/lib.rs （1行追加）
- `pub mod simulation;` を line 40 に追加（search と spaceposition の間）

### 2. src/simulation.rs （新規作成、~980行）
- **8 データ型定義**: ReciprocitySimulatorConfig, SimWorkflowState, SimMission, HelpSessionStatus, SimHelpSession, SimulationTickSnapshot, ReciprocitySimulationResult
- **9 公開関数/非公開関数**:
  - `generate_population`: 子/成人ワークフロー合成生成（child_ratio 制御、E_ADULT_THRESHOLD/T_ADULT_THRESHOLD/R_ADULT_THRESHOLD 参照）
  - `generate_mission_stream`: 各 tick のミッション生成（mission_rate 制御）
  - `offer_help_sessions`: 慈悲スコア biased 支援申し出
  - `advance_help_sessions`: セッション状態遷移（5状態、慈悲バイアス確率）
  - `build_events_for_workflow`: ReciprocityEvent 構築（F-1 compute_direct_reciprocity 入力用）
  - `recompute_trust_reputation`: F-1〜F-5 信頼評判再計算
  - `run_lifecycle_gc`: F-7〜F-8 GC hazard + 生存判定（gc_interval 制御）
  - `observe_tick`: パーセンタイル metrics + 生存率差観測
  - `run_simulation`: メインシミュレーションループ（public、決定論的）
- **9 テスト** (T1〜T9): 決定論的リプレイ、child_ratio=0、max_ticks=0、慈悲的生存優位、GC低減、metrics範囲、境界パラメータ、実験ID形式、CSV観測出力
- **既存関数の統合**: compute_direct_reciprocity, compute_indirect_reciprocity, compute_benevolence_score, recompute_reputation, compute_gc_hazard, compute_survival_probability（全 F-1〜F-8 関数）
- **コード品質**: unwrap() 不使用（expect("reason") のみ）、println! は観測テスト出力のみ、翻訳可能性準拠

## 実装詳細

### アーキテクチャ
```
run_simulation(config)
  ├── generate_population(config, rng)        ─ Phase 1
  ├── loop over ticks:
  │   ├── generate_mission_stream(...)         ─ Phase 2
  │   ├── offer_help_sessions(...)             ─ Phase 3
  │   ├── advance_help_sessions(...)           ─ Phase 3
  │   ├── recompute_trust_reputation(...)      ─ Phase 4
  │   ├── run_lifecycle_gc(...)               ─ Phase 4
  │   └── observe_tick(...)                   ─ Phase 5
  └── ReciprocitySimulationResult
```

### Kind World 創発メカニズム
1. 慈悲スコアが高い workflows は支援申し出確率が高い
2. 支援セッションの成功確率が慈悲スコアに比例
3. 成功体験 → 評判スコア向上 → GC hazard 低減
4. hazard 低減 → 生存確率向上
5. 初期 tick で survival_advantage ≈ 0.186 を確認

## 検証結果
- 既存 1053 テスト + 新規 9 テスト = 1062 全 PASS
- 決定論的リプレイ確認済み（同一 seed → ビットレベル一致）
- 品質チェック通過（unrwap 不使用、expect のみ）
