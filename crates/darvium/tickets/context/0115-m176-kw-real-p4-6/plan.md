# M1.76-KW-REAL-P4: 6 フェーズシミュレーションループ

## 要件の再確認

現行の run_simulation（4 相ループ、平坦な SimWorkflowState）に代えて、P1 の SimulationContext と P5 のライフサイクル機構を活用した 6 フェーズ tick ループ run_kw_real_simulation を新規追加する。P2/P3 の不足機能はスタブで代用。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| src/simulation.rs | 変更 | run_kw_real_simulation 追加 + 6 フェーズ関数 + 8 テスト |
| src/lib.rs | 変更（最小） | run_kw_real_simulation の pub re-export（必要な場合） |

## 計装・観測の実装計画

- 各フェーズに println! マーカーを埋め込み、--nocapture で出力確認
- CSV 形式の tick 別統計量を出力
- 固定シード StdRng::seed_from_u64(12345)
- サンプルサイズ: TC5 で 100 tick

## 実装手順

1. run_kw_real_simulation 関数を simulation.rs に追加（run_simulation の直後）
2. 6 つの内部ヘルパー関数を実装
3. SimulationContext を生成し tick ループで各フェーズを逐次実行
4. Death tracking は HashSet<NodeId>（petgraph コンパクション回避）
5. 8 テスト（TC1-TC8）を mod tests 内に追加
6. cargo test で全テスト通過確認
7. cargo test -- --nocapture で観測出力確認

## 物理的レビュー方法

- run-quality-checks.js で simulation.rs をチェック
- 翻訳可能性 grep（関数名が動詞句、1文字変数なし）

## リスク

- petgraph の指数コンパクション（逆順削除で対応）
- help.rs の状態遷移（transition_to 不使用、直接代入）
- 既存 run_simulation との互換性維持
