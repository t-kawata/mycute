# 実装サマリ: チケット#139 — HELPイベント発行のプロダクション配線

## 変更ファイル一覧

| ファイル | 種別 | 変更内容 |
|----------|------|----------|
| `src/simulation.rs` | 修正 | SimulationContext に event_bus フィールド追加、phase3_help_protocol の直接代入4箇所を transition_to() 経由に修正、コンストラクタ・設定構造体に event_bus 注入箇所追加、T3/T4 テスト追加 |
| `src/event.rs` | 修正 | DarviumEventBus トレイトに Debug スーパートレイト追加、FakeEventBus に #[derive(Debug)] 追加 |
| `src/help.rs` | 修正 | T2 テスト追加（transition_to(None) 互換性確認） |
| `tickets/specs/0139-help.md` | 修正 | 行番号・遷移記述の実態との乖離を修正、Offered→Executing 非合法遷移のドキュメント化 |

## 実装の要点

### 1. SimulationContext + Config への event_bus 注入
- `SimulationContext` に `event_bus: Option<Arc<dyn DarviumEventBus + Send + Sync>>` フィールド追加
- `ReciprocitySimulatorConfig` に同フィールド追加（Default は None）
- 3箇所のシミュレーションエントリポイントで config→context への転送を追加

### 2. phase3_help_protocol の transition_to 経由化
- `let event_bus = ctx.event_bus.clone()` をループ前に抽出（借用回避）
- Proposal→Offered: `transition_to(Offered, ...)` に置換
- Offered→Executing（非合法）→ Offered→Accepted→Executing の2段階合法遷移に分割
- 非Accept枝に Offered→Rejected 遷移を追加
- Executing→Succeeded/Failed: `transition_to(Succeeded/Failed, ...)` に置換
- エラーハンドリングは `.expect()`（プログラミングバグとしてパニック）

### 3. DarviumEventBus + FakeEventBus の Debug 対応
- `DarviumEventBus` トレイトに `Debug` スーパートレイト追加（SimulationContext が Debug を derive するため）
- `FakeEventBus` に `#[derive(Debug)]` 追加
- `event_bus: None` の全既存コンストラクタ修正（18箇所）

### 4. テスト追加（T2, T3, T4）
- **T2**: `transition_to(None)` でイベント発行なし、互換性確認
- **T3**: シミュレーション全tick実行後に FakeEventBus にイベント蓄積確認
- **T4**: イベント→ReciprocityEvent 変換→ReciprocityEventStore 蓄積経路確認

## 検証結果
- `cargo test`: 1358 passed, 0 failed, 63 ignored
- 既存テスト全件通過、回帰なし
