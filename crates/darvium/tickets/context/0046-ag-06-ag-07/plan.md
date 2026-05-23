# 実装計画: チケット #46 — AG-06/AG-07 ハードゲートの全弾ブロックテスト

## 要件
AG-06（semantic channel バージョン不整合）および AG-07（structural channel バージョン不整合）のハードゲートを実装し、バージョン不一致の候補が 100% ブロックされることを T1〜T14 + OTS-1〜OTS-3 で検証する。

## RFC 既存実装状態検証
- `EmbeddingChannelVersion`: 未実装
- `EmbeddingVersions`: 未実装
- `check_ag06()` / `check_ag07()`: 未実装
- `DarviumError::ApplicabilityRejected`: 実装済み (src/error.rs:62-63)

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/search/applicability.rs` | 新規 | EmbeddingChannelVersion, EmbeddingVersions, check_ag06(), check_ag07() |
| `src/search/mod.rs` | 変更 | pub mod applicability; 追加 |
| `src/types.rs` | 変更 | QueryRepresentation にバージョンフィールド追加 |
| `src/constants.rs` | 変更 | デフォルトバージョン定数追加 |
| `src/lib.rs` | 変更 | search::applicability の型を再公開 |
| `tests/m_minus0_5/ag_06_07_test.rs` | 新規 | T1〜T14 + OTS-1〜OTS-3 |

## 実装手順
1. src/search/applicability.rs 作成
2. src/search/mod.rs 変更
3. src/types.rs に QueryRepresentation フィールド追加
4. src/constants.rs に定数追加
5. src/lib.rs に再公開追加
6. tests/m_minus0_5/ag_06_07_test.rs 作成
7. cargo build → cargo test 確認

## 計装・観測
- OTS-1: 10,000 回偽陽性率ゼロ検証 (StdRng::seed_from_u64(12345))
- OTS-2: 10,000 回一致時通過率 1.0 検証
- OTS-3: ハミング距離 E=0〜10 階段関数マッピング
- 較正対象パラメータなし

## 物理的レビュー方法
cargo build → cargo test (全テスト + --nocapture) → cargo clippy

## リスク
低: 新モジュール追加による既存コードへの干渉なし
