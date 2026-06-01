# チケット #143 実装計画: HELP 成功時ワークフロー伝搬

## 要件
phase5_capability_diffusion に helper のグラフを helpee に条件付きでコピーする処理を追加する。
コピー条件: helper.node_count >= helpee.node_count の場合のみ。不成立時はスキップ。

## RFC 既存実装状態検証
- RFC §4A.5 Mechanism 25-26 (HELP Execution / HELP Success)
- RFC §41B.9 (HelpSuccess)
- MemoizedGraph の graph フィールドは既存実装に存在。RFC との矛盾なし。
- 現状は graph の伝搬が欠落している。これを補う。

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
| src/simulation.rs | 修正 | phase5_capability_diffusion に条件付きグラフコピー追加 |
| src/simulation.rs | 追加（テスト） | T1-T5 不変条件テスト |

新規定数なし。

## 実装内容
1. phase5_capability_diffusion: trust/reputation 継承後、node_count 比較 → 条件成立時のみ graph コピー
2. copy_graph_if_more_complex ヘルパー関数に抽出（Boy Scout）
3. 計装: println! で helper/helpee node_count とコピー有無を出力
4. TODO コメントで本来の本物実装（セマンティックマージ等）への拡張ポイントを記載

## テスト
- T1: helper複雑 > helpee単純 → コピーされる
- T2: helper単純 < helpee複雑 → コピーされない
- T3: 等ノード数 → コピーされる
- T4: trust/reputation/experience 継承維持
- T5: cargo test 全パス

## レビュー方法
run-quality-checks.js + 翻訳可能性 grep

## リスク
- 既存の trust/reputation 継承を壊す → T4 で防止
- 境界値バグ → T3 で等値ケース検証
