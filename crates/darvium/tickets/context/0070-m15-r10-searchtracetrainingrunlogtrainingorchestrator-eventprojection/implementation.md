# 変更したファイル一覧と実装内容の概要

## 変更ファイル

### 1. src/event.rs (+~400 lines)
- `DomainProjection` 構造体: 全てのドメイン特化 Projection を兼ねる単一ジェネリック構造体
  - `inner: Arc<Mutex<InnerDomainProjection>>` — 内部 state（name, events）
  - `filter: ProjectionEventFilter` — kind フィルタ
- 4 つのコンストラクタ関数:
  - `DomainProjection::search_trace()` — SearchEvent 全5種
  - `DomainProjection::training_run_log()` — TrainingEvent 全9種
  - `DomainProjection::reciprocity_event()` — ReciprocityEvent 全8種
  - `DomainProjection::search_run_log()` — SearchEvent subset（Started 除外の4種）
- `EventProjection` トレイト実装: name(), interested_kinds(), project(), snapshot(), clear()
- `initialize_domain_projections(catalog: &dyn ProjectionCatalog)` — 4 Projection を一括登録
- 9 テストケース追加（TC-1〜TC-9）
  - TC-8 計装: n=1000 ランダムイベント配送、フィルタ精度 100% 確認

### 2. src/lib.rs (+2 symbols)
- `DomainProjection` と `initialize_domain_projections` を pub use event に追加

### 3. src/store/coordinator.rs (~+60 lines)
- `event_bus: Option<Arc<dyn DarviumEventBus + Send + Sync>>` フィールド追加
- `with_event_bus()` ビルダーメソッド追加
- `commit_dual_store_update()` 内に EventBus publish 経路追加（2箇所: 通常 commit と repair retry）
- 既存の MetadataStore 経路は変更せず、Optional な publish として追加

## テスト結果
- 687 unit tests + 17 integration tests = 704 tests 全て PASS
- clippy -D warnings PASS
- フィルタリング精度: 100% (n=1000)
- クロスプロジェクション汚染: 0
- 既存テストに影響なし
