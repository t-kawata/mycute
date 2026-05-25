---
ticket_id: 87
title: M1.76-2: ReciprocityLifecyclePolicy 構造体 + ReputationProfile 拡張フィールド定義
slug: m176-2-reciprocitylifecyclepolicy-reputationprofile
status: reviewed
created_at: 2026-05-25
updated_at: 2026-05-25
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0087-m176-2-reciprocitylifecyclepolicy-reputationprofile/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0087-m176-2-reciprocitylifecyclepolicy-reputationprofile/observation-20260525-183806.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0087-m176-2-reciprocitylifecyclepolicy-reputationprofile/review.md
---
# M1.76-2: ReciprocityLifecyclePolicy 構造体 + ReputationProfile 拡張フィールド定義

## Summary

本チケットは v2.3-f Reciprocity-Aware Survival のデータ型基盤を実装する。既存 RFC 定義の ReputationProfile 構造体を v2.3-f 用に拡張し、ReciprocityLifecyclePolicy 構造体（全パラメータを保持する versioned policy object）を新規定義する。さらに、これらの構造体で使用する較正定数 16 種を constants.rs に追加する。

## Background

Darvium-Tickets-v2.3.md 記載のとおり、本チケットは RFC §15.10.7 Lifecycle calibration parameter object、§15.10.3 Extended ReputationProfile、および RFC §41C.3 の M0.x に先行するデータ型基盤である。

M1.76-1 (チケット 86) で ReciprocityEvent / ReciprocityEventKind のデータ型定義が完了した。これにより互恵性イベントを受信・記録する基盤が整い、本チケットではそのイベントを集約・解釈するためのポリシー構造体（ReciprocityLifecyclePolicy）と評判プロファイル（ReputationProfile）を定義する。

全パラメータは versioned policy object として記録されなければならない (MUST)。v2.3-f 追加フィールドを永続カラムとして保存しない場合でも、ReciprocityEvent から recompute 時に導出可能でなければならない (MUST)。

## Scope

1. `ReciprocityLifecyclePolicy` 構造体の新規定義
2. `ReputationProfile` 構造体の拡張（v2.3-e 既存 8 フィールド + v2.3-f 追加 8 フィールド = 全 16 フィールド）
3. 較正定数 16 種を src/constants.rs に追加（推奨初期値付き）
4. 公開 API への re-export（lib.rs）
5. テスト 5 ケースの実装

## Non-scope

- compute_direct_reciprocity (F-1) などの純粋関数実装（M1.76-3 以降）
- 永続ストアとの統合
- ReciprocityEvent インジェスションパイプライン
- 既存 ReputationProfile の破壊的変更（既存フィールドの型・名は維持する）

## Investigation

### 物理的証拠

#### 1. ReputationProfile は現時点で Rust ソースコード上に未実装

- grep `struct ReputationProfile` in `src/` → **該当なし**
- RFC §15.10.3 (Darvium-RFC-0001-Unified-v2.3-final.md:4352-4371) に拡張後の構造体定義あり:
  - 既存 8 フィールド: `direct_score`, `indirect_score`, `experience_score`, `inherited_score`, `final_score`, `alpha_positive`, `beta_negative`, `last_recomputed_at`
  - v2.3-f 追加 8 フィールド: `direct_help_count`, `direct_success_count`, `direct_reject_count`, `harm_event_count`, `accepted_offer_rate`, `help_success_rate`, `village_centrality`, `benevolence_score`

#### 2. ReciprocityLifecyclePolicy は現時点で Rust ソースコード上に未実装

- grep `ReciprocityLifecyclePolicy` in `src/` → **該当なし**
- RFC §15.10.7 (line 4459-4475) に 15 フィールド定義あり
- チケット仕様 (Darvium-Tickets-v2.3.md:1290) により `policy_version: String` を追加して全 16 フィールド

#### 3. 較正定数は constants.rs に未存在

- `RECIPROCITY_ALPHA_HELP` など 16 種の定数が全て未定義（確認済み）

#### 4. 関連既存型

- `ReciprocityEvent` / `ReciprocityEventKind`: `src/event.rs:303-391` — M1.76-1 で実装済み
- lib.rs の re-export に ReputationProfile / ReciprocityLifecyclePolicy は未追加

### 参照観察レポート

- tickets/context/0086-reciprocityevent-reciprocityeventkind/observation-20260525-182113.md — ReciprocityEvent / ReciprocityEventKind 完了。本チケットの ReputationProfile 拡張で source_graph_id / target_graph_id を使用した評判計算の基礎が完成。

## Test Plan

### TC-1: ReciprocityLifecyclePolicy の全フィールドデフォルト初期化

- `ReciprocityLifecyclePolicy::default()` で全フィールドが初期化可能
- 全数値フィールドが f32 または u32 として定義され、NaN でないことのアサーション
- `policy_version` が空文字列 "" で初期化されること
- `policy_version` が明示的に設定・更新可能であること

### TC-2: 拡張後の ReputationProfile フィールド完全性

- 既存 8 フィールドが全て保持されていること
- v2.3-f 追加 8 フィールドが正しく追加されていること
- フィールド数が合計 16 であること（コンパイル時検証）
- `last_recomputed_at` が `SystemTime` 型として定義されていること

### TC-3: ReputationProfile のシリアライズ完全性

- serde Serialize / Deserialize が正しく derive されていること
- 全 16 フィールドを含む JSON ラウンドトリップが成功すること
- n=100 のランダム値による往復変換完全性

### TC-4: ReciprocityLifecyclePolicy のシリアライズ完全性

- serde Serialize / Deserialize が正しく derive されていること
- 全 16 フィールドを含む JSON ラウンドトリップが成功すること

### TC-5: 全定数の定義確認

- 16 種全ての定数が `pub const` として定義されていること
- 全定数が `f32` または `u32` 型であること
- 全定数に適切な JSDoc コメントと分類 (Calibration Candidate) が付与されていること

## 計装方法・観測対象

### 計装方法

- 構造体のメモリレイアウト（フィールド数・型サイズ）をコンパイル時に確認
- println! + --nocapture で定数一覧と RFC 付録 E の対応表を出力
- 固定シード PRNG (StdRng::seed_from_u64(12345)) を使用したラウンドトリップテスト

### 観測対象

- フィールド数: ReputationProfile = 16, ReciprocityLifecyclePolicy = 16
- 全定数が NaN でないことのアサーション通過率 100%
- 定数命名一覧と RFC 付録 E の v2.3-f calibration candidates との対応テーブル（過不足なく網羅されていること）

### 較正計画

本チケットはデータ型定義のみであり、較正対象の定数は定義するが較正ループは実施しない。較正は M1.76-3 以降で行う。

## Boy Scout Rule — 翻訳可能性計画

- ReciprocityLifecyclePolicy のフィールド名は RFC の数式変数名 (theta_dir, theta_ind 等) を直接使用。RFC とのトレーサビリティを優先。
- lib.rs の re-export 行はアルファベット順を維持。
- 定数名は `SCREAMING_SNAKE_CASE` で統一し、`///` JSDoc で分類を明記。

## Acceptance Criteria

- [ ] ReciprocityLifecyclePolicy 構造体（16 フィールド）が正しく定義され、Default トレイト実装を持つ
- [ ] ReputationProfile 構造体が既存 8 フィールドを保持し、v2.3-f の 8 フィールドが追加されている（合計 16 フィールド）
- [ ] 両構造体に Debug, Clone, PartialEq, Serialize, Deserialize が derive されている
- [ ] 16 種の較正定数が constants.rs に推奨初期値付きで定義されている
- [ ] 全定数が f32 または u32 型で、NaN / 異常値でないことのアサーション
- [ ] テスト 5 ケースが全て通過
- [ ] lib.rs の re-export が正しく更新されている
- [ ] 既存テストの回帰がないこと

## Notes

### 実装先ファイル

- 構造体定義: `src/event.rs`
- 定数定義: `src/constants.rs`
- 公開 API: `src/lib.rs`
- テスト: 各構造体定義ファイル内の `mod tests`

### policy_version の設計判断

`String` 型。semantic versioning ("v2.3-f.1") を想定。Default では空文字列 ""。

### 成果物

- 計画: context/0087-m176-2-reciprocitylifecyclepolicy-reputationprofile/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0087-m176-2-reciprocitylifecyclepolicy-reputationprofile/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0087-m176-2-reciprocitylifecyclepolicy-reputationprofile/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0087-m176-2-reciprocitylifecyclepolicy-reputationprofile/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
