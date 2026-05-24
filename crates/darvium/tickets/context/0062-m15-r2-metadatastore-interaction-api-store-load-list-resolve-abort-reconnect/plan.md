# 実装計画: M1.5-R2 MetadataStore 汎用 Interaction API 拡張

## 要件

MetadataStore トレイトに6つの汎用 Interaction メソッド（store_interaction / load_interaction / list_interactions / resolve_interaction / abort_interaction / reconnect_interaction）を追加する。InteractionFilter 構造体を新規定義。既存4つの HITL 特化メソッドを汎用メソッドのラッパーとして再実装。

## 変更ファイル

| ファイル | 種別 | 内容 |
|---|---|---|
| src/types.rs | 追加 | InteractionFilter 構造体 |
| src/store/metadata_store.rs | 変更 | トレイト + InMemory 実装 + デフォルト実装 + テスト |
| src/store/json_metadata_store.rs | 変更 | JsonMetadataStore 実装 + デフォルト実装委譲 |

## 実装手順

1. InteractionFilter を types.rs に定義
2. MetadataStore トレイトに6汎用メソッド追加、既存4メソッドをデフォルト実装化
3. InMemoryMetadataStore に6汎用メソッド実装、既存4メソッドの明示的実装削除
4. JsonMetadataStore に6汎用メソッド実装、既存4メソッドの明示的実装削除
5. テスト T1〜T8 + OTS-1 追加
6. cargo test 全件通過確認

## レビュー方法

- run-quality-checks.js で変更ファイルチェック
- 翻訳可能性 grep（名詞始まり関数、ハードコード値）
- cargo test 全件通過
