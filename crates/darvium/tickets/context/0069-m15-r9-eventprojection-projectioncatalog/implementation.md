# 実装サマリー: M1.5-R9: EventProjection フレームワーク + ProjectionCatalog 実装

## 変更ファイル一覧

| ファイル | 種別 | 変更内容 |
|---------|------|---------|
| `src/error.rs` | 編集 (+2行) | `DarviumError::Projection(String)` variant 追加 |
| `src/event.rs` | 編集 (~250行追加) | EventProjection トレイト、ProjectionEventFilter、ProjectionCatalog、FakeProjection、FakeProjectionCatalog、TC-1〜TC-8 の tests |
| `src/lib.rs` | 編集 (re-export) | 新規5型の公開 API 追加 |

## 新增された型

1. **EventProjection trait** — `name()`, `interested_kinds()`, `project()`, `snapshot()`, `clear()`
2. **ProjectionEventFilter** — `all()`, `from_kinds()`, `matches()`
3. **ProjectionCatalog trait** — `register()`, `get()`, `project_all()`
4. **FakeProjection** — メモリ内 EventProjection 実装（filter 対応）
5. **FakeProjectionCatalog** — メモリ内 ProjectionCatalog 実装（Arc<Mutex<HashMap>>）

## テスト結果

- TC-1: EventProjection トレイト境界 — ✅ PASS
- TC-2: project + snapshot ラウンドトリップ (count=2) — ✅ PASS
- TC-3: 複数 projection 同時配送 (2 projections) — ✅ PASS
- TC-4: ProjectionEventFilter フィルタリング — ✅ PASS
- TC-5: clear() 後 snapshot リセット — ✅ PASS
- TC-6: クロスプロジェクション汚染ゼロ — ✅ PASS
- TC-7: register / get / registered_names — ✅ PASS
- TC-8: n=1000 一括配送 独立完全性 (filter_accuracy=100%) — ✅ PASS

## 既存テスト影響

全 695 テスト PASS（678 unit + 5 integration + 12 doc）。既存テストへの影響なし。
