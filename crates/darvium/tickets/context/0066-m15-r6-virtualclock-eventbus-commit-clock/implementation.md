# 変更したファイル一覧と実装内容の概要

## src/event.rs

1. **VirtualClock トレイト追加**: `fn now(&self) -> u64` のみを持つ読み取り専用トレイト。Send + Sync 境界付き。
2. **DarviumEventBus トレイト更新**: VirtualClock を supertrait として要求 (pub trait DarviumEventBus: VirtualClock)。
3. **FakeEventBus への VirtualClock 実装追加**: `fn now(&self) -> u64 { self.current_clock() }`
4. **テスト追加 (TC-1〜TC-8)**:
   - TC-1: VirtualClock トレイトのコンパイル時検証、Send + Sync 確認
   - TC-2: FakeEventBus が VirtualClock を実装、now() == current_clock() の一致確認
   - TC-3: DarviumEventBus が VirtualClock を supertrait として要求
   - TC-4: publish/open/resolve/reconnect 操作後の clock 増加確認
   - TC-5: replay 後の clock 不変確認 (MUST NOT #3)
   - TC-6: ManualClock (旧 VirtualClock) が Clock トレイトを実装
   - TC-7: 既存の時間計測用テスト (T4-T9, T13-T15) がそのまま通過
   - TC-8: n=1000 観測テスト — 一意性・単調増加性の統計検証

## src/clock/mod.rs

1. **VirtualClock 構造体 → ManualClock に改名**: 同名異概念の解消。時間計測用クロックを ManualClock（手動操作で進行する決定論的クロック）として再定義。
2. **テスト関数名リネーム**: test_virtual_clock_* → test_manual_clock_*
3. **advance() ドキュメント改善**: RFC §12C.6 MUST #4 の制約を明記。
4. **コメント更新**: 日本語コメント内の「仮想クロック」→「手動操作で進行する決定論的クロック」に修正。セクションコメントも更新。

## src/lib.rs

1. **VirtualClock を公開 API に追加**: `pub use event::{ ..., VirtualClock, ... };`

## テスト結果

- 既存 637 テスト: 全て PASS
- 新規 12 テスト: 全て PASS
- clippy: -D warnings でクリーン
- TC-8 観測テスト: n=1000, clock 一意性 100%, 重複 0, 単調増加違反 0
