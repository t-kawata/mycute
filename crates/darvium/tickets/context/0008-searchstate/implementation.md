# 実装サマリ: M-1.5-1 SearchState 合法状態遷移マトリクス

## 変更ファイル
| ファイル | 変更内容 |
|---------|---------|
| src/types.rs | SearchState Enum 追加、is_legal_transition 関数追加、全テスト追加 |
| src/lib.rs | pub use types::SearchState を追加（1行） |

## 実装内容
1. SearchState Enum: 8 バリアント（Init/Retrieve/Evaluate/Refine/Compose/ProposeNew/Finalize/Abort）
2. is_legal_transition: RFC §13.5 の 16 合法遷移を match パターンマッチで判定
3. テスト: T1（16 個別合法遷移）+ T2（7 グループ違法遷移）+ T3（総当たり 64 ペア）+ T4（メモリサイズ）
4. 観測テスト: OTS-1（CSV完全性）+ OTS-2（スペクトル半径 ρ<1）+ OTS-3（平均吸収時間）

## 検証結果
- cargo test: 149 passed, 0 failed
- cargo clippy: 通過
- cargo fmt: 通過
- 品質チェック: 単一文字変数を改善（n→state_count, q→transition_matrix, v→eigenvector）
