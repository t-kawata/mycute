---
ticket_id: 63
title: M1.5-R3: StoredInteraction → InteractionRecord<HitlPayload> 型エイリアス移行
slug: m15-r3-storedinteraction-interactionrecordhitlpayload
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0063-m15-r3-storedinteraction-interactionrecordhitlpayload/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0063-m15-r3-storedinteraction-interactionrecordhitlpayload/observation-20260524-112843.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0063-m15-r3-storedinteraction-interactionrecordhitlpayload/review.md
---

# M1.5-R3: StoredInteraction → InteractionRecord<HitlPayload> 型エイリアス移行

## Summary

`StoredInteraction` を `InteractionRecord<HitlPayload>` の型エイリアスとして再定義し、既存コードの変更をゼロに抑えつつ、v2.3-g の汎用 Interaction API との整合を取る。シリアライズ互換性を維持する。

## Background

M1.5-R1 で `InteractionRecord<TPayload>` ジェネリック型と `InteractionStatus` 7状態列挙型を定義し、M1.5-R2 で MetadataStore に汎用 Interaction API 6メソッドを追加した。既存コードは `StoredInteraction` という型名でこれらを参照しているため、この型を `InteractionRecord<HitlPayload>` のエイリアスとして再定義し、段階的移行を可能にする。後方互換性を MUST とする。

## Scope

- `type StoredInteraction = InteractionRecord<HitlPayload>` エイリアス定義
- 後方互換アクセサ（`request()` / `outcome()` メソッド）の提供
- JSON シリアライズ/デシリアライズ互換性の確認（ラウンドトリップ n >= 1000）
- 既存コメントの更新（「StoredInteraction」参照の最新化）

## Non-scope

- `StoredInteraction` という名前自体の変更（エイリアスとして維持）
- 既存の `struct StoredInteraction` 独立定義（既に存在しない）
- 既存 API シグネチャの変更（`load_interaction` 等はそのまま）

## Investigation

### 物理的証拠

#### 1. 型エイリアスの現状

`src/types.rs:5088` にて既に定義済み：

```
pub type StoredInteraction = InteractionRecord<HitlPayload>;
```

さらに後方互換アクセサが `src/types.rs:5091-5101` に実装済み：

```
impl StoredInteraction {
    pub fn request(&self) -> &HumanRequest { &self.payload.request }
    pub fn outcome(&self) -> &Option<HumanOutcome> { &self.outcome }
}
```

#### 2. 独立した `struct StoredInteraction` の有無

`struct StoredInteraction` の独立定義はコードベースのどこにも存在しない。型エイリアスのみ。

#### 3. コンパイル確認（MUST）

`cargo check` 成功。`StoredInteraction` を参照する全箇所（metadata_store.rs、json_metadata_store.rs、human_channel.rs、coordinator.rs、recovery.rs）がエイリアス解決により透過的にコンパイルされている。

#### 4. InteractionRecord の構造

`src/types.rs:5060-5073`:

```
pub struct InteractionRecord<TPayload: InteractionPayload> {
    pub interaction_id: String,
    pub payload: TPayload,
    pub outcome: Option<TPayload::Outcome>,
    pub status: InteractionStatus,
    pub created_at: u64,
    pub updated_at: u64,
}
```

**注意**: `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]` — `Default` は derive されていない。したがって `StoredInteraction::default()` はコンパイル不可。

#### 5. 既存のラウンドトリップテスト

- `src/types.rs:4131` `json_roundtrip_n1000()`: `InteractionRecord<HitlPayload>` として n=1000 ラウンドトリップ（100% success）
- `src/human_channel.rs:1300-1350` OTS-2: `StoredInteraction` として n=1000 ラウンドトリップ（全成功）

両テストとも固定シード `StdRng::seed_from_u64(12345)` を使用。

### 参照観察レポート

- tickets/context/0062-m15-r2-metadatastore-interaction-api-store-load-list-resolve-abort-reconnect/observation-20260524-111627.md — M1.5-R2 6メソッドのスループット計測完了（14734 calls/sec）
- tickets/context/0061-m15-r1-interactionrecordtpayload-interactionstatus-7/observation-20260524-110006.md — M1.5-R1 状態遷移行列 + ラウンドトリップ検証完了

### 残ギャップ分析

| 要件 | 状態 | 根拠 |
|------|------|------|
| 型エイリアス定義 | ✅ 完了 | `types.rs:5088` |
| 後方互換アクセサ | ✅ 完了 | `types.rs:5091` |
| cargo check | ✅ PASS | ビルド確認済み |
| JSON roundtrip (StoredInteraction) | ✅ 完了 | human_channel.rs OTS-2 |
| JSON roundtrip (InteractionRecord\<HitlPayload\>) | ✅ 完了 | types.rs `json_roundtrip_n1000` |
| `StoredInteraction::default()` テスト | ⚠️ 非対応 | InteractionRecord に Default 無し。テスト不要 |
| クロス型シリアライズ互換性テスト | ⚠️ 未実装 | 型エイリアスなので実質同じだが、明示的テストが不足 |
| コメントの更新 | ⚠️ 未確認 | 点検が必要 |

## Test Plan

### TC-1: 双方向型代入コンパイル確認 (MUST)

```rust
let record = InteractionRecord::<HitlPayload> { ... };
let typed: StoredInteraction = record;
let stored = StoredInteraction { ... };
let deref: InteractionRecord<HitlPayload> = stored;
```

### TC-2: 後方互換アクセサ確認 (MUST)

```rust
let s = make_interaction("test", InteractionStatus::Pending, 100);
let _req: &HumanRequest = s.request();
let _out: &Option<HumanOutcome> = s.outcome();
```

### TC-3: 既存ラウンドトリップテスト継続確認 (MUST)

`json_roundtrip_n1000` (types.rs) と OTS-2 (human_channel.rs) が変更なしに n=1000 100% PASS することを確認。

### TC-4: クロス型シリアライズ互換性 (MUST)

`InteractionRecord::<HitlPayload>` としてシリアライズした JSON が `StoredInteraction` としてデシリアライズ可能であること。

## 計装方法・観測対象

### 計装方法

- TC-4: 新規テスト。固定シード `StdRng::seed_from_u64(12345)` を使用、n=1000
- TC-3: 既存テストが計装済み

### 観測対象

- ラウンドトリップ成功率: n=1000 で 100% であること（不変条件）

### 較正計画

本チケットは較正対象の定数は存在しない。

## Boy Scout Rule — 翻訳可能性計画

- `types.rs` の `StoredInteraction` 関連コメントが「v2.3-d 互換」「v2.3-g」と RFC を正しく参照していることを確認する
- 既存の全 `StoredInteraction` コンストラクト箇所のコメントが型エイリアス化を反映しているか点検する

## Acceptance Criteria

- [ ] TC-1: 双方向代入がコンパイル可能であること
- [ ] TC-2: 後方互換アクセサがコンパイル可能であること
- [ ] TC-3: 既存ラウンドトリップテストが変更なしに n=1000 100% PASS すること
- [ ] TC-4: クロス型ラウンドトリップが n=1000 100% PASS すること
- [ ] `cargo test` 全テスト通過
- [ ] 翻訳可能性の検証が通っている

## Notes

### 関連チケット実装状況

本チケットの実体（型エイリアス）は M1.5-R1 の実装段階で既に定義済み。本 spec は事後的な追跡と、残るクロス型互換性テストの追加およびコメント監査のための正式なチケット化である。

### 成果物

- 計画: context/0063-m15-r3-storedinteraction-interactionrecordhitlpayload/plan.md（未作成）
- 実装サマリ: context/0063-m15-r3-storedinteraction-interactionrecordhitlpayload/implementation.md（未作成）
- レビュー報告書: context/0063-m15-r3-storedinteraction-interactionrecordhitlpayload/review.md（未作成）
- 観察レポート: context/0063-m15-r3-storedinteraction-interactionrecordhitlpayload/observation-YYYYMMDD-HHmmss.md（未作成）
