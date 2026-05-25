# 実装計画: チケット #86 M1.76-1 ReciprocityEvent / ReciprocityEventKind データ型定義

## 要件

RFC §15.10.6 の `ReciprocityEvent` 構造体（9 フィールド）と `ReciprocityEventKind` 列挙型（8 variant）を実装する。既存の `ReciprocityEvent` 列挙型を `ReciprocityEventKind` にリネームし、新規に構造体を定義。TryFrom<DarviumEvent> 変換を実装し、DarviumError に ReciprocityError を追加。

## 変更ファイル一覧

| ファイル | 種別 | 変更内容 |
|---|---|---|
| src/error.rs | 追加 | DarviumError::ReciprocityError(String) バリアント追加 |
| src/event.rs | 編集 | 列挙型リネーム、構造体追加、TryFrom 実装、DomainProjection 更新、テスト更新 |
| src/help.rs | 編集 | 全 12 箇所の ReciprocityEvent::* → ReciprocityEventKind::* |
| src/event_channel.rs | 編集 | 2 箇所の ReciprocityEvent::* → ReciprocityEventKind::* |
| src/lib.rs | 編集 | ReciprocityEvent(struct) + ReciprocityEventKind(enum) 両方を export |

## 実装手順

1. error.rs: ReciprocityError(String) 追加
2. event.rs: リネーム + 構造体追加 + TryFrom + テスト更新
3. help.rs: 型参照更新
4. event_channel.rs: 型参照更新
5. lib.rs: export 更新
6. cargo build / cargo test / cargo clippy / cargo fmt

## 計装・観測

- TC-6: StdRng::seed_from_u64(12345), n=1000, println! + --nocapture
- 観測対象: 往復変換成功率（期待値 100%）、非 Reciprocity kind ブロック率（100%）

## Boy Scout 改善

- src/event.rs:303 — コメント修正 (ReciprocityEvent → ReciprocityEventKind)
- src/help.rs:546 — transition_to_event → transition_to_event_kind 改名

## 物理的レビュー方法

- run-quality-checks.js + cargo build + cargo test -- --nocapture + cargo clippy -- -D warnings + cargo fmt --check
