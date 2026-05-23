# 実装サマリ: チケット #46 — AG-06/AG-07 ハードゲートの全弾ブロックテスト

## 変更したファイル

### 新規ファイル
- `src/search/applicability.rs` — AG-06/AG-07 ハードゲートモジュール
  - `EmbeddingChannelVersion` 構造体（model_version, template_version）
  - `EmbeddingVersions` 構造体（task, design）
  - `check_ag06()` — semantic channel model version 完全一致比較
  - `check_ag07()` — structural channel model + template version 完全一致比較
  - 単体テスト T1〜T12（全12テスト）
- `tests/ag_06_07_test.rs` — 統合テスト + 観測テスト
  - T13: QueryRepresentation 後方互換性テスト
  - T14: EmbeddingVersions 構築テスト
  - OTS-1: 偽陽性率ゼロ検証（10,000 回走査）
  - OTS-2: 一致時通過率 1.0 検証（10,000 回走査）
  - OTS-3: 階段関数マッピング実測（E=0〜10, 各 1,000 回走査）

### 変更ファイル
- `src/search/mod.rs` — `pub mod applicability;` 追加
- `src/types.rs` — QueryRepresentation に task_embedding_version, design_embedding_version フィールド追加
- `src/constants.rs` — AG_HARD_GATE_DEFAULT_MODEL_VERSION, AG_HARD_GATE_DEFAULT_TEMPLATE_VERSION 定数追加
- `src/lib.rs` — check_ag06, check_ag07, EmbeddingChannelVersion, EmbeddingVersions を pub use で再公開

## 検証結果
- cargo build: ✅ OK
- cargo test (全306テスト): ✅ OK（既存289 + 新規17）
- 観測テスト出力: OTS-1 pass_rate=0.0000, OTS-2 pass_rate=1.0000, OTS-3 階段関数確認
