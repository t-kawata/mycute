---
ticket_id: 56
title: M1-2: 管理者 AdminFastTrack 発動時における信頼値強制更新と TrustAuditLog 生成不変条件の検証
slug: m1-2-adminfasttrack-trustauditlog
status: reviewed
created_at: 2026-05-23
updated_at: 2026-05-23
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0056-m1-2-adminfasttrack-trustauditlog/observation-20260523-182912.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0056-m1-2-adminfasttrack-trustauditlog/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0056-m1-2-adminfasttrack-trustauditlog/review.md
---

# M1-2: 管理者 AdminFastTrack 発動時における信頼値強制更新と TrustAuditLog 生成不変条件の検証

## Summary

管理者が `AdminFastTrack` を発動した際に、`HumanTrustLogistic.score` を 0.80 に強制固定し、キャッシュ無効化フラグを立て、`TrustAuditLog` 配列に監査レコードを追加する `apply_admin_fast_track` 関数を実装する。併せて `TrustAuditLog` / `TrustAuditEvent` の型定義を具体化し、RFC §8.2 が求める監査ログ要件（SHOULD）を満たす。

## Background

RFC §8.2 は以下の2点を規範として定めている：

1. **HumanTrust の Fast-track**: 権限を持つ管理者が明示的に承認した場合、`HumanTrustLogistic.score` を `TRUST_ADMIN_FAST_TRACK (0.80)` に設定することができる (MAY)。これは B2B 環境で人間フィードバックが 50 件蓄積されるまでの過渡期に活用する。
2. **監査ログ要件 (v1.2 追加)**: 管理者 fast-track を適用した場合、その操作を `TrustAuditLog` に記録しなければならない (SHOULD)。B2B 環境では MUST に引き上げることを推奨する。

現在のコードベースにおいて：

- `TrustAuditLog` は空の構造体（`pub struct TrustAuditLog;`）であり、フィールドが一切未実装（`src/types.rs:4477`）
- `TrustAuditEvent` enum は未定義（RFC §8.2 では 10 バリアントが規定されている）
- `apply_admin_fast_track` 関数は未実装
- `TRUST_ADMIN_FAST_TRACK` 定数は未定義
- `HumanTrustLogistic` 構造体（score / k / scale / count）は未実装（RFC §10.3 で定義）
- `invalidate_applicability_cache()` メソッドは未実装
- `MetadataStore` には `store_trust_audit_log` / `load_trust_audit_logs` のトレイトメソッドが既に定義されており、`InMemoryMetadataStore` / `JsonMetadataStore` 双方で実装済み

本チケットは上記のギャップを埋め、管理者 fast-track の完全な実装とその不変条件の検証を行う。

## Scope

以下の実装を含む：

1. **定数追加** (`src/constants.rs`):
   - `TRUST_ADMIN_FAST_TRACK: f64 = 0.80` の定義追加（RFC §8.2 指定値）
   - `HUMAN_TRUST_SCALE: f64 = 0.30` の定義追加（RFC §10.3 指定値、HumanTrustLogistic のロジスティックスケール）
   - `HUMAN_TRUST_COLD_START: f64 = 0.50` の定義追加（RFC §10.3 指定値、HumanTrustLogistic 初期値）

2. **型定義の具体化** (`src/types.rs`):
   - `HumanTrustLogistic` 構造体の実装（`score: f64`, `k: f64`, `scale: f64`, `count: u32` + `default()` + `update(outcome)`）
   - `TrustAuditEvent` enum の実装（10 バリアント: `AdminFastTrack`, `ManualOverride`, `AbstractionRequested`, `AbstractionApplied`, `AbstractionRejected`, `HumanReviewApproved`, `HumanReviewRejected`, `HumanReviewNeedsRevision`, `HumanReviewIrrelevant`, `HumanReviewUnsafe`）
   - `TrustAuditLog` 構造体のフィールド具体化（`graph_id: String`, `event_type: TrustAuditEvent`, `actor_id: String`, `old_value: f64`, `new_value: f64`, `timestamp: SystemTime`, `reason: Option<String>`）

3. **`apply_admin_fast_track` 関数の実装**（`src/trust.rs` として新規ファイルを作成）:
   - シグネチャ: `pub fn apply_admin_fast_track(graph: &mut MemoizedGraph, actor_id: String, audit_log: &mut Vec<TrustAuditLog>, reason: Option<String>)`
   - 処理内容:
     1. `graph.trust.human.score` の旧値を保存
     2. `graph.trust.human.score` を `TRUST_ADMIN_FAST_TRACK` に強制設定
     3. `graph.invalidate_applicability_cache()` を呼び出し
     4. `TrustAuditLog` レコードを `audit_log` に追加（`event_type = AdminFastTrack`）

4. **MemoizedGraph の試験用縮約実装**（`src/trust.rs` に試験用構造体として実装）:
   - M1-2 のスコープは AdminFastTrack の検証に限定するため、`MemoizedGraph` 全体ではなく、`apply_admin_fast_track` の動作検証に必要な最小限のフィールド（`id: String`, `trust: TrustProfile`）と `invalidate_applicability_cache()` メソッドを持つ試験用実装とする。
   - `invalidate_applicability_cache()` は呼び出し追跡用のフラグ（`cache_invalidated: AtomicBool`）を持つ簡易実装とする。
   - `TrustProfile` も必要最小限のフィールド（`human: HumanTrustLogistic`）から開始し、他の軸（`operational`, `semantic`, `temporal`）は `f64` のダミーフィールドとする。

5. **モジュール登録** (`src/lib.rs`):
   - `pub mod trust;` の追加

6. **ユニットテスト** (`src/trust.rs` 内 `#[cfg(test)] mod tests`):
   - T1〜T7 の不変条件テスト
   - OTS-1 / OTS-2 の観測テスト

## Non-scope

- `MemoizedGraph` の完全実装（WorkflowRepository 統合・GraphVersion CAS 等は M2/M3 以降のスコープ）
- `MetadataStore` トレイトの拡張（`store_trust_audit_log` / `load_trust_audit_logs` は既に実装済みであり、本チケットではメモリ内 `Vec<TrustAuditLog>` で検証）
- SQLite / LadybugDB への TrustAuditLog 永続化パス
- `TrustUpdate` 状態機械の完全実装（§10.5 の `update_trust()` は M1-3 で扱う）
- `HumanTrustLogistic` の Elo 昇格（RFC-0003 委譲）

## Investigation

### 参照観察レポート

- `tickets/context/0055-m1-1-human-review-queue/observation-20260523-181239.md` — M1-1 HumanReviewQueue の線形成長・スレッド競合・情報リーク率の観測が完了。AdminFastTrack はこのキューに対する管理者介入経路として位置づけられる。
- `tickets/context/0047-m-05-4-notifier/observation-20260523-123238.md` — HumanChannel 抽象トレイトの定義と FakeHumanChannel の call_count 検証が完了。AdminFastTrack は通常の HumanChannel 経路をバイパスする管理者専用操作である。

### コードベース調査結果

1. **TrustAuditLog の現状**（`src/types.rs:4477`）:
   ```rust
   pub struct TrustAuditLog;  // 空構造体 — フィールド未実装
   ```

2. **TrustAuditEvent の現状**: 未定義。RFC §8.2 で以下の 10 バリアントが規定されている:
   ```rust
   enum TrustAuditEvent {
       AdminFastTrack,
       ManualOverride,
       AbstractionRequested,
       AbstractionApplied,
       AbstractionRejected,
       HumanReviewApproved,
       HumanReviewRejected,
       HumanReviewNeedsRevision,
       HumanReviewIrrelevant,
       HumanReviewUnsafe,
   }
   ```

3. **apply_admin_fast_track の現状**: 未実装。RFC §8.2 で以下の完全な擬似コードが規定:
   ```rust
   fn apply_admin_fast_track(
       graph: &mut MemoizedGraph,
       actor_id: String,
       audit_log: &mut Vec<TrustAuditLog>,
       reason: Option<String>,
   ) {
       let old_value = graph.trust.human.score;
       graph.trust.human.score = TRUST_ADMIN_FAST_TRACK;
       graph.invalidate_applicability_cache();
       audit_log.push(TrustAuditLog {
           graph_id:   graph.id.clone(),
           event_type: TrustAuditEvent::AdminFastTrack,
           actor_id,
           old_value,
           new_value:  TRUST_ADMIN_FAST_TRACK,
           timestamp:  SystemTime::now(),
           reason,
       });
   }
   ```

4. **TRUST_ADMIN_FAST_TRACK 定数**: 未定義（`src/constants.rs` に不在）。RFC §8.2 指定値 = 0.80。

5. **HumanTrustLogistic 構造体**: 未実装。RFC §10.3 で以下が規定:
   ```rust
   struct HumanTrustLogistic {
       score: f64,  // 初期値 0.50
       k:     f64,  // 学習率 HUMAN_TRUST_K (0.08)
       scale: f64,  // ロジスティックスケール 0.30
       count: u32,
   }
   ```
   `HUMAN_TRUST_K = 0.08` は既に `src/constants.rs:24` で定義済み。`HUMAN_TRUST_SCALE` と `HUMAN_TRUST_COLD_START` は未定義。

6. **MetadataStore トレイト**: 既に `store_trust_audit_log` / `load_trust_audit_logs` が定義済み（`src/store/metadata_store.rs:27-31`）。`InMemoryMetadataStore` と `JsonMetadataStore` の双方で実装済み。

7. **MemoizedGraph の現状**: 完全な構造体は未実装（`src/types.rs` 全域に定義なし）。RFC §8.1 で全フィールドが規定されているが、このチケットでは試験用縮約実装で十分。

8. **invalidate_applicability_cache の現状**: 未実装。RFC §8.2 で `apply_admin_fast_track` 内から呼ばれることが規定。

## Test Plan

### 不変条件テスト（T1〜T7）

| ID | テスト内容 | 確認項目 |
|----|-----------|---------|
| T1 | `apply_admin_fast_track` 呼び出し後、`graph.trust.human.score == TRUST_ADMIN_FAST_TRACK`（0.80） | 信頼値強制固定 |
| T2 | 監査ログ配列の末尾要素の `event_type == TrustAuditEvent::AdminFastTrack` | 監査ログ生成 |
| T3 | 監査ログの `old_value` が呼び出し前の `graph.trust.human.score` と一致 | 旧値正しさ |
| T4 | 監査ログの `new_value` が `TRUST_ADMIN_FAST_TRACK`（0.80）と一致 | 新値正しさ |
| T5 | 呼び出し後、`graph.cache_invalidated == true` であること | キャッシュ無効化フラグ |
| T6 | `actor_id`（例: "admin-001"）と `reason`（例: Some("B2B contract approval")）が正確に監査ログに反映される | メタデータ正しさ |
| T7 | スコア初期値 0.50 と 0.30 の双方で呼び出し、`old_value` がそれぞれ正しく記録される | 任意初期値対応 |

### 観測テスト（OTS）

| ID | 観測対象 | 手法 | n |
|----|---------|------|---|
| OTS-1 | キャッシュ無効化の完全性: ダミーキャッシュコンテキスト配列に対する一斉無効化 | N 個のキャッシュエントリを保持するラッパーを作成し、`apply_admin_fast_track` 呼び出し後に全エントリの無効化フラグが立っていることを確認。無効化レイテンシも計測し、エントリ数に対して線形であること（O(N)）を観測 | 1,000 |
| OTS-2 | TrustAuditLog 記録完全性: 連続発動でのレコード数一致および全レコードの event_type 一致 | 10,000 回ループで `apply_admin_fast_track` を呼び出し、毎回の監査ログ長が期待値と一致すること、および全レコードの `event_type == AdminFastTrack` を事後検証 | 10,000 |

## 計装方法・観測対象

### 計装方法

- `src/trust.rs` 内 `#[cfg(test)] mod tests` にユニットテストとして実装
- 固定シード非使用（本テストは決定論的動作の検証が主目的のため）
- `std::time::Instant` を用いた高分解能レイテンシ計測
- `println!` による構造化出力（`--nocapture` 経由で観測）

### 観測対象

**OTS-1: キャッシュ無効化の完全性**
- N = 1,000 個のダミーキャッシュエントリ（`CacheEntry { id: String, valid: AtomicBool }`）を `Vec` で保持
- `apply_admin_fast_track` 呼び出し後、全エントリの `valid` が `false` に設定されているかを確認
- レイテンシ測定: `Instant::now()` で前後を挟み、`println!("OTS-1,n={},dt_ns={},all_invalidated={}", n, dt.as_nanos(), all_invalid)` で出力
- 期待: `all_invalidated == true`。レイテンシは O(N) でスケール

**OTS-2: TrustAuditLog 記録完全性**
- n = 10,000 回の `apply_admin_fast_track` をループ実行
- 毎回の監査ログ配列長が `call_count` と一致することを確認
- ループ終了後、全レコードを走査し `event_type == AdminFastTrack` かつ `new_value == TRUST_ADMIN_FAST_TRACK` であることを確認
- 期待: `records_added == n_calls`、`records_mismatch == 0`

### 較正計画

本チケットでは新規較正パラメータは導入しない。`TRUST_ADMIN_FAST_TRACK = 0.80` は RFC §8.2 の指定値であり、Safety Invariant として扱う。`HUMAN_TRUST_SCALE = 0.30` および `HUMAN_TRUST_COLD_START = 0.50` も RFC §10.3 の指定値であり変更不可。

## Boy Scout Rule — 翻訳可能性計画

このチケットで触るコードに対して以下の改善を行う：

1. **`TrustAuditLog` 空構造体の具体化**: 現在 `pub struct TrustAuditLog;` は空殻である。RFC 準拠のフィールドを持つ構造体として具体化し、かつ各フィールド名は RFC の命名に忠実に従う（`event_type`、`actor_id` 等）。コメントは「なぜ」に専念させる（例: `// 管理者等の操作者識別子。認証システムのユーザーIDを想定`）。

2. **`apply_admin_fast_track` は一関数一責務**: RFC の疑似コード通り、「旧値保存 → スコア強制 → キャッシュ無効化 → 監査ログ追記」の逐次制御フローをそのまま翻訳可能な関数として実装する。1行1処理が自然言語の文として読めることを確認する。

3. **ハードコード値の排除**: `0.80` を `TRUST_ADMIN_FAST_TRACK`、`0.30` を `HUMAN_TRUST_SCALE`、`0.50` を `HUMAN_TRUST_COLD_START` として定数化。

4. **HumanTrustLogistic の update() メソッド**: ロジスティック更新式 `h_{n+1} = h_n + k(outcome − σ((h−0.5)/s))` は RFC §10.3 の数式と一致していることをコメントで示す。

5. **テスト関数名**: `fn t1_score_forced_to_admin_fast_track()` のように、テスト名が検証内容を一文で語る命名とする。

## Acceptance Criteria

- [ ] `TRUST_ADMIN_FAST_TRACK = 0.80` が `src/constants.rs` に Safety Invariant として定義されている
- [ ] `HUMAN_TRUST_SCALE = 0.30` および `HUMAN_TRUST_COLD_START = 0.50` が `src/constants.rs` に定義されている
- [ ] `HumanTrustLogistic` 構造体が RFC §10.3 通りに実装されている（`score: f64`, `k: f64`, `scale: f64`, `count: u32` + `default()` + `update(outcome)`）
- [ ] `TrustAuditEvent` enum が RFC §8.2 通りの 10 バリアントで実装されている
- [ ] `TrustAuditLog` 構造体が RFC §8.2 通りの全フィールド（`graph_id`, `event_type`, `actor_id`, `old_value`, `new_value`, `timestamp`, `reason`）を持つ
- [ ] `apply_admin_fast_track` 関数が `src/trust.rs` に実装され、RFC §8.2 の疑似コードに忠実である
- [ ] T1〜T7 の全不変条件テストが通過
- [ ] OTS-1: 全キャッシュエントリ無効化率 = 100%
- [ ] OTS-2: 記録漏れ率 = 0%、全レコード event_type 一致
- [ ] `cargo test` が全て PASS（既存テスト含む）

## Notes

- plan_path: 未作成
- implementation_path: 未作成
- review_report_path: 未作成
- observation_report_path: 未作成

### 成果物

- 計画: `context/0056-m1-2-adminfasttrack-trustauditlog/plan.md`（未作成、`/plan-ticket` 承認後に作成）
- 実装サマリ: `context/0056-m1-2-adminfasttrack-trustauditlog/implementation.md`（未作成、`/start-ticket` 実装完了後に作成）
- レビュー報告書: `context/0056-m1-2-adminfasttrack-trustauditlog/review.md`（未作成、`/review-ticket` 全チェック通過後に作成）
- 観察レポート: `context/0056-m1-2-adminfasttrack-trustauditlog/observation-YYYYMMDD-HHmmss.md`（未作成、`/start-ticket` 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
