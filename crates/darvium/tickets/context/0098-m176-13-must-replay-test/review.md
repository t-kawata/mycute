# レビュー報告書: M1.76-13 決定論的リプレイテスト（MUST replay test）

## 1. 静的品質チェック結果

### run-quality-checks.js 出力

- unwrap()/expect(): 1 finding (line 2984) — 既存コード（M1.76-11 PerturbationSuite）、本チケット未変更
- 1文字変数: 51 findings — うち49件は既存コード。新規追加コード内の2件（`m`, `t`）を実装者修正済み
- debug出力（println!）: 既存の観測テストパターンとして意図的な計装出力。全て `--nocapture` 使用想定
- 多引数関数: 2 findings (lines 406, 754) — 既存コード

### 修正内容
- `let mut m` → `let mut scores` (test utility function)
- `let t` → `let trace` (closure variable)

### 観測検証（validate-observation.js）
- ✅ valid=true, issuesCount=0

### 構造整合性（validate-structure.js）
- ✅ valid=true, issuesCount=0

## 2. チケット仕様交叉参照（Darvium-Tickets-v2.3.md）

lines 1485-1501:
- ✅ `ReciprocityReplayScenario` / `ReciprocityReplayTrace` / `run_reciprocity_replay` 実装済み
- ✅ 4仕様テストケース（実装では6テストに拡張、全条件カバー）
- ✅ golden trace + trace_hash 機構実装済み
- ✅ n=100 決定論的再現性確認

## 3. RFC 理論交叉参照（Darvium-RFC-0001-Unified-v2.3-final.md）

### §41B.20.8 Testing discipline — Replay test (MUST)
- RFC要件: 「同一 event stream、同一 policy version、同一 VirtualClock なら ReputationProfile と GC hazard の再計算結果は一致すること」
- ✅ 全6テストで立証。n=100独立実行でも全て一致。

### §41C.3 M1.x milestone
- RFC要件: 「replayable reputation/hazard recompute (ReciprocityEvent ingestion, policy-versioned recompute, snapshot comparison)」
- ✅ run_reciprocity_replay がイベント取り込み→再計算→スナップショット取得を順次実行
- ✅ policy version 変更テストで versioned recompute 検証
- ✅ snapshot comparison は ReciprocityReplaySnapshot + compute_replay_comparison で既存実装

## 4. 翻訳可能性チェック（grep）

- 関数定義: 全関数動詞句始まり（compute_, run_, assert_）✅
- 1文字変数: 新規コード内の2件を修正済み ✅
- デバッグ出力: 観測テストとして意図的な println! ✅
- マジックナンバー: テストパラメータ（イベント数、seed値等）は定数または明確な文脈値 ✅

## 5. Acceptance Criteria 検証

- [x] `ReciprocityReplayScenario` / `ReciprocityReplayTrace` / `ReplayTraceComparator` 実装済み、テストパス
- [x] 完全同一シナリオの2回実行でビットレベル一致（T1）
- [x] policy version 変更で限定差分（T2）
- [x] clock_schedule 変更で時刻依存項のみ差分（T3）
- [x] イベント順序維持の再実行で完全一致（T4）
- [x] n=100回の独立実行で最大差分量0（T5）
- [x] Golden trace 保存機構（T6）
- [x] 既存テスト全通過（1015 lib tests, 0 failed）
- [x] RFC 該当セクションとの無矛盾確認
- [x] 翻訳可能性計画に沿ったコード記述（Boy Scout Rule適用）

## 6. 計装・観測検証結果

- [x] spec「計装方法・観測対象」が全て実装されている
- [x] 観測テストが実行可能である（--nocapture で観測出力確認済み）
- [x] 較正ループが実行されている（本チケットは較不要のためスキップ）
- [x] 観察レポートが保存されている（observation-20260526-110021.md）

## 7. 実験系列サマリ

- M1.76-12 → M1.76-13: 単調性テスト完了によりリプレイテスト前提条件成立。全 MUST 単調性条件 PASS。
- M1.76-13 → M1.76-14: 決定論的再現性確認により摂動テストスイートの前提条件成立。
- M1.76-13 → M1.76-16: 較正フェーズで run_reciprocity_replay をシナリオ基盤として再利用可能。

## 所見

- trace_hash 計算における HashMap iteration 順序非依存性への対応（DefaultHasher不使用＋キーソート）は、Rust の非決定論的 iteration + ランダム化ハッシャーへの正しい対処。
- 全テストが固定シードの決定論的環境で動作し、実行ごとに結果が一致する。
- レビュー中に発見した2件の1文字変数は Boy Scout Rule に基づき修正済み。
- 全テスト通過（1015 lib + 2 doc = 1017 tests, 0 failed）。

## 総評

**PASS** — 全チェック通過。ステータスを reviewed に遷移可能。
