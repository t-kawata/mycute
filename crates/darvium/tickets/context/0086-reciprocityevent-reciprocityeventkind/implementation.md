# 変更したファイル一覧と実装内容の概要

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/error.rs | 編集 | DarviumError に ReciprocityError(String) バリアントを追加 |
| src/event.rs | 編集・追加 | ReciprocityEventKind へのリネーム、ReciprocityEvent 構造体・TryFrom 実装・7 TC 追加 |
| src/help.rs | 編集 | transition_to_event → transition_to_event_kind 改名、全 variant 参照更新 |
| src/event_channel.rs | 編集 | test 内 import・variant 参照を ReciprocityEventKind に更新 |
| src/lib.rs | 編集 | ReciprocityEventKind の re-export 追加、transition_to_event_kind に更新 |

## 実装内容の概要

### 1. src/error.rs
- `DarviumError::ReciprocityError(String)` を HELP Protocol セクション直後に追加

### 2. src/event.rs (主対象)
- **ReciprocityEventKind** 列挙型: 既存 8 variant を維持し、コメントを RFC 正確な記述に修正
- **ReciprocityEvent** 構造体: 9 フィールド（event_id, mission_id, source_graph_id, target_graph_id, event_kind, weight, created_at, virtual_clock, trace_ref）を新規定義
- **TryFrom<DarviumEvent>**: Reciprocity kind 時のみ変換成功、それ以外は ReciprocityError
- **DarviumEventKind::Reciprocity**: variant 型を ReciprocityEventKind に更新
- **DomainProjection**: reciprocity_event() フィルタ条件を ReciprocityEventKind に更新
- **全テストコード**: 30 箇所以上の参照を ReciprocityEventKind に更新

### 3. src/help.rs
- transition_to_event → transition_to_event_kind に改名
- 戻り値型を Option<ReciprocityEventKind> に変更
- 全 14 箇所の variant 参照を更新

### 4. src/event_channel.rs
- test 内部の import と variant 参照を ReciprocityEventKind に更新

### 5. src/lib.rs
- pub use に ReciprocityEventKind を追加（ReciprocityEvent 構造体と併記）
- transition_to_event → transition_to_event_kind に更新

## テスト結果
- 全 916 ユニットテスト通過（既存 909 + 新規 7）
- 全統合テスト通過
- cargo clippy -- -D warnings: 通過
- cargo fmt --check: 通過
