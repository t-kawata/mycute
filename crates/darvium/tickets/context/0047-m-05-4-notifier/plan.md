# 計画: M-0.5-4 HITL HumanChannel トレイト定義

## 要件
- `HumanChannel` トレイト（RFC §13B, TableSpec §T-HumanChannel）
- `InteractionHandle` 応答ブロッキング待機機構（mpsc ベース）
- `FakeHumanChannel` テスト用二重化実装
- `StdinoutChannel<R, W>` 標準入出力による具象実装（JSON Lines プロトコル）
- データ型: HumanRequest / HumanOutcome / HumanResponse / HumanDecision / StoredInteraction / InteractionStatus
- MetadataStore HITL 永続化メソッド: store/load/list_pending/resolve
- エラー型: HumanChannelIo / HumanChannelClosed
- テスト: T1〜T10 + OTS-1/2/3/4

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/error.rs | 変更 | HumanChannelIo / HumanChannelClosed 追加 |
| src/types.rs | 変更 | 6 データ型追加 |
| src/store/metadata_store.rs | 変更 | 4 HITL 永続化メソッド追加 |
| src/human_channel.rs | 新規 | HumanChannel トレイト + 全実装 + テスト |
| src/lib.rs | 変更 | pub mod human_channel 追加 |
| Cargo.toml | 変更 | uuid = "1" 依存追加 |

## 実装手順
1. error.rs: エラーバリアント追加
2. types.rs: データ型定義
3. metadata_store.rs: HITL 永続化メソッド追加
4. human_channel.rs: トレイト + 実装 + テスト
5. lib.rs: モジュール公開 + re-export
6. Cargo.toml: uuid 依存追加
7. cargo test → cargo clippy → cargo fmt

## 物理的レビュー方法
- cargo test -- --nocapture
- cargo clippy -- -D warnings
- cargo fmt -- --check
- 確認: InteractionHandle のタイムアウト動作
- 確認: StdinoutChannel のスレッド安全性
- 確認: MetadataStore 永続化の整合性

## リスク
- uuid クレートの新規依存追加（Cargo.toml 変更）
- スレッド使用によるテストの非決定論性（固定シード PRNG で緩和）
