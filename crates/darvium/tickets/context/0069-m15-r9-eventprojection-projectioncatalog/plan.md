# 実装計画: M1.5-R9: EventProjection フレームワーク + ProjectionCatalog 実装

## RFC §12E 既存実装状態検証

| RFC 型 | 現行コード | 状態 |
|--------|-----------|------|
| `EventProjection` trait | (未実装) | ❌ 型未定義 |
| `ProjectionError` struct | (未実装) | ❌ 型未定義（`DarviumError::Projection` で代替） |
| `ProjectionErrorKind` enum | (未実装) | ❌ 型未定義（本チケットでは実装せず、`DarviumError` 文字列 variant で対応） |
| `ProjectionEngine` | (未実装) | ❌ 型未定義（チケット仕様により `ProjectionCatalog` として実装） |

**評価サマリ**: 全型が未実装（新規実装）。RFC との乖離はなく additive な追加。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/error.rs` | 編集 | `DarviumError::Projection(String)` variant 追加（+1行） |
| `src/event.rs` | 編集 | 既存コードに additive 追加。EventProjection トレイト、ProjectionEventFilter、ProjectionCatalog トレイト、FakeProjection、FakeProjectionCatalog、mod tests (TC-1〜TC-8) |
| `src/lib.rs` | 編集 | re-export 行に新規型を追加 |

## 実装手順

1. `src/error.rs`: DarviumError::Projection(String) を EventChannel セクション直後に追加
2. `src/event.rs`: EventProjection トレイト、ProjectionEventFilter、ProjectionCatalog トレイト、FakeProjection、FakeProjectionCatalog、mod tests を追加
3. `src/lib.rs`: 新規型を re-export に追加
4. 検証: cargo test --lib event:: -- --nocapture

## 物理的レビュー方法

```bash
node "$_R/scripts/tickets/review/run-quality-checks.js" src/event.rs src/error.rs src/lib.rs
```

## リスク

- event.rs が既に ~2300行。新コードの挿入位置を誤ると既存テストに影響。
- 新規型の命名が既存と衝突しないよう注意。
