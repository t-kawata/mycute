# 実装計画: チケット #70 M1.5-R10

## 要件
R9 の EventProjection フレームワーク上に4つのドメイン特化 Projection を実装する：
1. SearchTraceProjection — Search イベント (5種) を蓄積
2. TrainingRunLogProjection — Training イベント (9種) を蓄積
3. ReciprocityEventProjection — Reciprocity イベント (8種) を蓄積
4. SearchRunLogProjection — Search の subset (StepCompleted/Completed/Failed/Aborted)
5. initialize_domain_projections() — 上記4つを一括登録
6. Dual-path: DualStoreCoordinator に EventBus publish 経路追加

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/event.rs | 追加 | 4 DomainProjection + initialize_domain_projections() + 9テストケース |
| src/store/coordinator.rs | 修正 | EventBus フィールド追加 + dual-path |
| src/types.rs | 追加 | TrainingRunLog 型エイリアス |
| src/lib.rs | 修正 | 新規公開型の re-export |

## 実装手順
1. SearchTraceProjection (TC-1)
2. TrainingRunLogProjection (TC-2)
3. ReciprocityEventProjection (TC-3)
4. SearchRunLogProjection (TC-4)
5. initialize_domain_projections() (TC-5)
6. Cross-domain contamination tests (TC-6, TC-7)
7. 計装テスト n=1000 (TC-8)
8. Dual-path + TC-9
9. lib.rs re-export

## 計装・観測
- 追加10 tests、src/event.rs の mod tests に additive
- StdRng::seed_from_u64(12345), n=1000
- println! + --nocapture で観測出力

## レビュー方法
1. cargo clippy -- -D warnings
2. cargo check
3. cargo test --lib event::tests -- --nocapture (既存39 + 新規)
4. cargo test (全テスト影響なし確認)
5. 翻訳可能性 grep

## リスク
- 低: additive 追加で既存影響なし
- 低: Dual-path は Option でラップ
- 低: DomainProjection は同一パターン
