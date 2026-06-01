# 実装サマリ: ワークフロー複雑化メカニズム完全活性化

## 変更内容

### 1. GMR 常時有効化（設定経由）
- `ReciprocitySimulatorConfig` に `use_gmr: bool` フィールドを追加（デフォルト `true`）
- 全3プロダクションシミュレーションパスを `ctx.use_gmr = config.use_gmr` に変更:
  - `run_kw_real_simulation`
  - `run_evaluation_simulation`
  - `run_evaluation_simulation_with_channel`

### 2. DeterminismScore のグラフ構造ベース化
- `try_gmr_diffusion` 内の乱数ベース DeterminismScore 計算をグラフ構造ベースに置き換え
- AgentStep ノードの out-degree から決定論値を算出（`propose_subgraph_and_accept` と同一方式）
- TODO コメントでセマンティック差分への拡張ポイントを記載

### 3. phase5_capability_diffusion 戻り値の活用
- `_diffusions` → `diffusions` にリネーム（2箇所）
- Phase5 観測出力 `println!("Phase5: diffusions={}", diffusions)` を `run_evaluation_simulation` と `run_evaluation_simulation_with_channel` に追加

### 変更ファイル
- `src/simulation.rs` — 主要修正箇所
