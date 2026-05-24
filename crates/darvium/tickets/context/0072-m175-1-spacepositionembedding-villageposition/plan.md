# 計画: チケット #72 M1.75-1

## 要件の再確認

RFC §41B.2 に基づき、ワークフローの生態学的位置を表現する型定義と指数平滑化による位置更新ダイナミクスを実装する。Event Architecture（完了済み M1.5-R 系列）と結合し、位置更新イベントを DarviumEventKind::System として publish する。

## RFC 既存実装状態検証結果

### SystemEvent (event.rs:153-162)
| フィールド | RFC §41B.2 | 現行コード | 状態 |
|---|---|---|---|
| ClockAdvanced | (暗黙) | ✅ 存在 | ✅ |
| SnapshotTaken | (暗黙) | ✅ 存在 | ✅ |
| ReplayCompleted | (暗黙) | ✅ 存在 | ✅ |
| StartupCompleted | (暗黙) | ✅ 存在 | ✅ |
| SpacePositionUpdated | (本チケットで必要) | ❌ 欠落 | ❌ 追加必要 |

**評価サマリ**: SystemEvent に SpacePositionUpdated variant を追加する必要あり。

### VirtualClock trait (event.rs:512-515)
| 要素 | RFC §12C.6 | 現行コード | 状態 |
|---|---|---|---|
| now() -> u64 | ✅ 必須 | fn now(&self) -> u64 | ✅ 一致 |

### DarviumEventBus trait (event.rs:527-557)
| 要素 | RFC §12C.5 | 現行コード | 状態 |
|---|---|---|---|
| publish() | ✅ 必須 | fn publish(&self, event: DarviumEvent) -> Result<EventId, DarviumError> | ✅ 一致 |
| current_clock() -> u64 | ✅ 必須 | fn current_clock(&self) -> u64 | ✅ 一致 |

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| src/spaceposition.rs | NEW | 型定義・純粋関数・EventBus publish ロジック・ユニットテスト (T-1〜T-8) |
| src/constants.rs | 追加 | SPACE_POSITION_UPDATE_ALPHA, SPACE_POSITION_UPDATE_MIN_INTERVAL, SPACE_POSITION_L2_EPSILON |
| src/event.rs | 追加 | SystemEvent::SpacePositionUpdated variant + ペイロード型 |
| src/lib.rs | 追加 | pub mod spaceposition; + 公開型の re-export |
| tests/ | 追加 | 観測テスト O-1, O-2, O-3 |

## 計装・観測の実装計画

| テスト | 種類 | ファイル | 観測出力 | サンプルサイズ |
|---|---|---|---|---|
| T-1〜T-8 | ユニットテスト (assert) | src/spaceposition.rs | — | — |
| O-1 | 観測テスト | tests/ | MSD(t) 時系列、Γ(α) テーブル | n=10,000 |
| O-2 | 観測テスト | tests/ | 発火密度 ρ(Δt) | 窓幅4水準×500 |
| O-3 | 観測テスト | tests/ | publish 完全性 η | n=1,000 |

- 観測テストは固定シード StdRng::seed_from_u64(12345) を使用
- --nocapture 経由で CSV/JSON 形式の構造化データを出力

## 較正対象

| 定数 | 初期値 | ファイル |
|---|---|---|
| SPACE_POSITION_UPDATE_ALPHA (Calibration Candidate) | 0.30 | constants.rs |
| SPACE_POSITION_UPDATE_MIN_INTERVAL (Calibration Candidate) | 5 | constants.rs |
| SPACE_POSITION_L2_EPSILON (Safety Invariant) | 1e-6 | constants.rs |

## Boy Scout 改善

スコープ外の既存コードに翻訳可能性を損なう箇所は現時点では確認されていない。

## 実装手順

1. constants.rs: 3つの定数を追加
2. event.rs: SystemEvent に SpacePositionUpdated variant + ペイロード型を追加
3. src/spaceposition.rs 作成: 型定義 → 純粋関数 → EventBus publish → ユニットテスト (T-1〜T-8)
4. lib.rs: mod + re-export 追加
5. tests/: 観測テスト (O-1, O-2, O-3)
6. 検証: cargo test + cargo clippy

## 物理的レビュー方法

1. run-quality-checks.js で変更全ファイルを検査
2. cargo test 全テストパス確認
3. 翻訳可能性 grep チェック: 関数名が動詞句、ハードコード数値なし、unwrap() 適切使用
4. cargo clippy -- -D warnings 通過確認
5. 観測テスト --nocapture 出力確認

## リスク

- 低: 既存コードへの影響は最小（enum variant + module 追加のみ）
- 低: 純粋関数（数学演算のみ）のため不具合混入リスクは極めて低い
