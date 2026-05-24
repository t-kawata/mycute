---
ticket_id: 66
title: M1.5-R6: VirtualClock 再定義 — EventBus commit clock への制限
slug: m15-r6-virtualclock-eventbus-commit-clock
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0066-m15-r6-virtualclock-eventbus-commit-clock/observation-20260524-120930.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0066-m15-r6-virtualclock-eventbus-commit-clock/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0066-m15-r6-virtualclock-eventbus-commit-clock/review.md
---
# M1.5-R6: VirtualClock 再定義 — EventBus commit clock への制限

## Summary

`VirtualClock` を「commit 済み DarviumEvent 列の順序番号」として再定義する。EventBus がクロック進行の唯一の authority であり、外部からの直接 `advance` 呼び出しをコンパイル時に禁止する。既存の時間計測用 `Clock` トレイト（`now_ms()` / `advance()`）と EventBus の commit clock（`u64` 順序番号）を明確に分離する。

## Background

v2.3-g Darvium Event Architecture（RFC §12C.6）は `VirtualClock` を「commit 済み DarviumEvent 列の順序番号」として再定義した。しかし現状のコードベースには以下の問題がある：

1. **名前の衝突**: `src/clock/mod.rs` に時間計測用の `VirtualClock` 構造体が存在するが、これは RFC で再定義された EventBus commit clock とは異なる概念（ミリ秒単位の仮想時間カウンタ）である。
2. **advance の可視性漏洩**: `Clock` トレイトが `fn advance(&mut self, delta_ms: u64)` を公開メソッドとして定義しており、EventBus 以外からも直接呼び出せる状態にある。RFC §12C.6 MUST #4 は `advance_virtual_clock` を EventBus 内部実装のみに制限することを要求している。
3. **概念混同のリスク**: 既存の `VirtualClock` 構造体（時間計測）と EventBus の commit clock（順序番号）が同名であるため、将来の開発者が誤った clock 概念を使用する可能性がある。
4. **EventBus commit clock のトレイト不在**: 現在 `DarviumEventBus` の `current_clock() -> u64` のみが commit clock 値を提供しているが、VirtualClock の読み取り専用トレイトとしての定義が存在しない。

### 参照観察レポート

- `tickets/context/0065-m15-r5-darviumeventbus-fakeeventbus/observation-20260524-115247.md` — FakeEventBus の publish→replay 完全性 (n=1000) および並行アクセス下 clock 単調増加性 (n=64) が PASS 確認済み。FakeEventBus の内部 clock 実装は正しく動作しており、R6 のテスト基盤として利用可能。

## Scope

1. **新しい `VirtualClock` トレイトの定義（`src/event.rs`）**: `fn now(&self) -> u64` のみを持つ読み取り専用トレイト。EventBus の commit clock を表現する。
2. **`FakeEventBus` への `VirtualClock` トレイト実装追加**: 内部の `Arc<Mutex<u64>>` の値を返す。
3. **`DarviumEventBus` トレイトに `VirtualClock` 境界追加**: すべての EventBus 実装が VirtualClock 読み取りを保証する。
4. **時間計測用 `VirtualClock` 構造体のリネーム**: `src/clock/mod.rs` の `VirtualClock` 構造体を `ManualClock` に改名し、概念混同を防止する。`Clock` トレイトは時間計測用として維持。
5. **`advance` の可視性制限**: `Clock` トレイトの `advance()` に doc comment で EventBus 専用である制約を明記する（Rust のトレイトメソッドは可視性を pub 未満にできないため、ドキュメントによる運用制限）。
6. **既存コードの互換性維持**: M-2-1.8 の `Clock` トレイト利用コード、M1.75-1 の `should_update_position` 等が変更なしでコンパイルを通ることを確認。
7. **テスト追加**: 以下「Test Plan」の全テストケース。

## Non-scope

- `ConcreteEventBus` の実装（将来チケット）
- `SystemClock` / `FrozenClock` の動作変更
- `Clock` トレイトの削除
- EventBus 以外の commit clock 導入（複数 EventBus 調整プロトコルは OQ-17 として将来）
- clock overflow 対策（OQ-16 として将来検討）

## Investigation

### ソースコード調査結果（物理的証拠）

#### 証拠1: 時間計測用 `VirtualClock` 構造体（src/clock/mod.rs:36-73）

`advance()` は `pub` であり、任意の外部コードから呼び出し可能。RFC §12C.6 MUST #4（advance は EventBus 内部のみ）に違反する状態。

#### 証拠2: `VirtualClock` 構造体の参照範囲

`grep` の結果、`VirtualClock` 構造体への参照は `src/clock/mod.rs` 内のテスト（T4-T9, T16）に限定。外部モジュールからの参照は存在しない。リネームによる影響範囲は最小限。

#### 証拠3: FakeEventBus の内部 clock（src/event.rs:562-566）

FakeEventBus は `Arc<Mutex<u64>>` で commit clock を管理しており、`VirtualClock` 構造体を一切使用していない。`publish()`, `open()`, `resolve()`, `reconnect()` 内で排他的に `*clock += 1` を実行。外部からの advance 経路は存在しない（実装としては既に正しい）。

#### 証拠4: `DarviumEventBus` トレイトの current_clock（src/event.rs:543-544）

`fn current_clock(&self) -> u64;` — 読み取り専用で `&self`。新しい `VirtualClock::now(&self)` と同一セマンティクス。

#### 証拠5: `Clock` トレイト（src/clock/mod.rs:18-27）

`now_ms()` は UTC 起点のミリ秒。EventBus commit clock の `now()` は単純な `u64` 順序番号。単位と意味が異なる概念。

#### 証拠6: `VirtualClock` / `Clock` の公開状況（src/lib.rs）

`clock` モジュールは `pub mod clock;` として公開されているが、`VirtualClock` や `Clock` は `pub use` で再公開されていない。MYCUTE からの利用想定ではこれらの型を直接使用しない。

### 結論

- **リネーム安全**: 時間計測用 `VirtualClock` → `ManualClock` は外部への影響ゼロで可能。
- **advance 制限**: トレイトメソッドの可視性は `pub` 固定のため、doc comment による制約明記で対応。
- **新トレイト**: `VirtualClock` トレイトを `event.rs` に追加し、読み取り専用インタフェースを提供する。

## Test Plan

### TC-1: 新しい `VirtualClock` トレイトのコンパイル時検証
- `VirtualClock` トレイトが `fn now(&self) -> u64` のみを持つこと
- `&self`（不変参照）で宣言されていること（読み取り専用の確認）
- `Send + Sync` 境界を満たすこと

### TC-2: `FakeEventBus` が `VirtualClock` トレイトを実装可能であること
- `FakeEventBus` が `VirtualClock` トレイトを実装していることのコンパイル時確認
- `bus.now()` が `bus.current_clock()` と同一値を返すこと

### TC-3: `DarviumEventBus` トレイトが `VirtualClock` を supertrait として要求すること
- `fn assert_virtual_clock<T: DarviumEventBus>(_t: &T) {}` で検証

### TC-4: EventBus 操作（publish/open/resolve）後に `now()` が増加すること
- publish → clock が +1 されること
- open → clock が +1 されること
- open → resolve → clock が +2（open + resolve）されること
- reconnect → clock が +1 されること

### TC-5: `replay` 後に clock が増加しないこと（MUST NOT #3）
- replay 呼び出し前後で `now()` が同一値であること

### TC-6: 時間計測用 `ManualClock`（旧 VirtualClock）が `Clock` トレイトを実装していること
- リネーム後も `ManualClock::new()` が使用可能
- `ManualClock::now_ms()` が期待値を返す
- 外部からの `advance` 呼び出しは doc comment で禁止されていること

### TC-7: 既存 Clock テストが変更なしで通過すること
- リネーム後の `ManualClock` に対する既存テスト（T4-T9, T16）が全て PASS すること

### TC-8: 計装 — EventBus 操作と VirtualClock 値の相関観測（n=1000）
- 1000 回の publish/open/resolve 混合操作を実行
- 操作ごとに clock が単調増加することを確認
- clock 値の範囲・一意性・欠落を統計出力

## 計装方法・観測対象

### 計装方法
- `test_*` 関数内で `println!` による構造化テキスト出力を `--nocapture` 経由で表示
- 固定シード PRNG（`StdRng::seed_from_u64(12345)`）をランダム操作に使用
- 観測テストは `event.rs` 内の `#[cfg(test)] mod tests` に追加（既存パターンに準拠）

### 観測対象
- **TC-8 計装テスト**: n=1000 の EventBus 操作（publish/open/resolve 混合）を実行し、以下の統計量を出力：
  - 操作総数 / clock 最大値
  - clock の単調増加性（全操作を通じて真であること）
  - clock 値の一意性（重複ゼロ）
  - clock 値の範囲（`min..max`）
  - 欠損値の有無

### 較正計画
本チケットは純粋なトレイト再定義 + 可視性制限であり、較正すべき定数は存在しない。

## Boy Scout Rule — 翻訳可能性計画

### `VirtualClock` → `ManualClock` リネーム
- **変更理由**: 同名異概念の解消。`VirtualClock` という名前が時間計測と commit sequence の両方に使われているため、時間計測側を `ManualClock`（手動操作で進行するクロック）に改名する。
- **影響範囲**: `src/clock/mod.rs` のみ（外部からの参照なし）。

### advance メソッドのドキュメント改善
- `Clock` トレイトの `advance()` に RFC §12C.6 の参照を追加し、EventBus 実装以外からの呼び出しが禁止であることを明確に記載する。
- 新しい `VirtualClock` トレイトの `now()` には「読み取り専用」であることを明記。

### 関数名の翻訳可能性
- `VirtualClock::now()` → commit clock の「現在値を取得する」。動詞句として翻訳可能。
- `ManualClock::advance()` → 時間を進める。内部的にのみ使用されるべき操作。

## Acceptance Criteria

- [ ] 新しい `VirtualClock` トレイト（読み取り専用）が `src/event.rs` に定義されている
- [ ] `FakeEventBus` が `VirtualClock` トレイトを実装している
- [ ] 時間計測用の旧 `VirtualClock` 構造体が `ManualClock` に改名されている
- [ ] `Clock::advance()` のドキュメントに EventBus 専用である制約が明記されている
- [ ] TC-1〜TC-8 の全テストが通過している
- [ ] 既存の全テストが通過している
- [ ] 翻訳可能性の検証が通っている

## Notes

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

### 成果物

- 計画: context/0066-m15-r6-virtualclock-eventbus-commit-clock/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0066-m15-r6-virtualclock-eventbus-commit-clock/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0066-m15-r6-virtualclock-eventbus-commit-clock/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0066-m15-r6-virtualclock-eventbus-commit-clock/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
