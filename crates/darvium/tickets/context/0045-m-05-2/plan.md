# M-0.5-2: ランクドリフト頑健性シミュレーションテスト — 実装計画

## 要件の再確認

ガウスノイズ注入環境下で上位選択アルゴリズムの頑健性を検証するテスト基盤を実装する。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| Cargo.toml | 依存追加 | cargo add rand_distr |
| src/constants.rs | 定数追加 | NOISE_SIMULATION_SIGMA = 0.05 |
| src/lib.rs | モジュール登録 | pub mod search; |
| src/search/mod.rs | 新規作成 | pub mod simulated_ranker; |
| src/search/simulated_ranker.rs | 新規作成 | ノイズ注入関数 + 全テスト(T1-T7, OTS-1/2) |

## 実装手順

1. cargo add rand_distr
2. src/search/mod.rs 作成
3. src/lib.rs 編集
4. src/constants.rs 編集
5. src/search/simulated_ranker.rs 実装
6. cargo test 実行
7. cargo clippy / cargo fmt

## レビュー方法

- cargo test (--nocapture 含む)
- cargo clippy -- -D warnings
- cargo fmt
- 翻訳可能性 grep

## リスク

- rand_distr のバージョン非互換 → Box-Muller 代替
- 既存テストへの影響は都度確認
