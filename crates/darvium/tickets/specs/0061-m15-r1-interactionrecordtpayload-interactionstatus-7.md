---
ticket_id: 61
title: M1.5-R1: InteractionRecord<TPayload> ジェネリック型 + InteractionStatus 7状態列挙型の定義
slug: m15-r1-interactionrecordtpayload-interactionstatus-7
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0061-m15-r1-interactionrecordtpayload-interactionstatus-7/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0061-m15-r1-interactionrecordtpayload-interactionstatus-7/observation-20260524-110006.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0061-m15-r1-interactionrecordtpayload-interactionstatus-7/review.md
---
# M1.5-R1: InteractionRecord<TPayload> ジェネリック型 + InteractionStatus 7状態列挙型の定義

## Summary

既存の `StoredInteraction` 具象構造体を `InteractionRecord<HitlPayload>` の型エイリアスに移行し、`InteractionStatus` を現行の2状態から RFC §12C で定義された7状態へ拡張する。このチケットは v2.3-g Event Architecture 統合の基盤となる第一歩である。

## Background

v2.3-g では DarviumEventBus を導入し、TwoWay インタラクション（HITL・HELP・外部チャネル等）を統一的に扱うアーキテクチャへ移行する。その基盤として、ペイロード型に依存しないジェネリック `InteractionRecord<TPayload>` が必要である。現行の `StoredInteraction` 構造体は HITL に特化しており、他のドメイン（HelpPayload 等）で再利用できない。

RFC §12C では以下の型定義が規範として記載されているが、コード上は未実装である：
- `InteractionPayload` トレイト（associated type `Outcome`）
- `InteractionRecord<TPayload>` ジェネリック構造体
- `HitlPayload` HITL ドメインペイロード
- `InteractionStatus` 7状態列挙型（`Aborted` 追加）
- `type StoredInteraction = InteractionRecord<HitlPayload>;` エイリアス

## Scope

1. `InteractionPayload` トレイトの定義（`Clone + Serialize + Deserialize` 境界、associated type `Outcome: Clone + Serialize + Deserialize`）
2. `InteractionRecord<TPayload: InteractionPayload>` ジェネリック構造体の定義（フィールド: `interaction_id`, `payload`, `outcome`, `status`, `created_at`, `updated_at`）
3. `HitlPayload { request: HumanRequest }` 構造体の定義と `InteractionPayload for HitlPayload { type Outcome = HumanOutcome }` 実装
4. `InteractionStatus` 列挙型の拡張: `Pending`, `AwaitingExternal`, `Resolved`, `TimedOut`, `Unreachable`, `ChannelClosed`, `Aborted`
5. `pub type StoredInteraction = InteractionRecord<HitlPayload>;` エイリアス定義
6. `StoredInteraction` 後方互換アクセサ: `fn request(&self) -> &HumanRequest`, `fn outcome(&self) -> &Option<HumanOutcome>`
7. 既存コードの `StoredInteraction { request, ... }` → `InteractionRecord { payload: HitlPayload { request }, ... }` の参照をエイリアス経由で透過的に解決

## Non-scope

- MetadataStore 汎用 Interaction API の実装（別チケット M1.5-R2）
- DarviumEventBus トレイトの定義（別チケット M1.5-R5）
- FakeEventBus の実装（別チケット M1.5-R5）
- EventChannel トレイト・外部チャネル実装（別チケット M1.5-R8）
- 遷移則の検証（状態機械としての完全検証は本チケットでは行わず、既存6状態との差分のみ確認）

## Investigation

### 物理的証拠

**現行の実装（src/types.rs:4886-4906）:**

1. `StoredInteraction` 構造体 (src/types.rs:4886-4900):
```rust
pub struct StoredInteraction {
    pub interaction_id: String,
    pub request: HumanRequest,
    pub outcome: Option<HumanOutcome>,
    pub status: InteractionStatus,
    pub created_at: u64,
    pub updated_at: u64,
}
```

2. `InteractionStatus` 列挙型 (src/types.rs:4902-4906): 2状態のみ
```rust
pub enum InteractionStatus {
    Pending,
    Resolved,
}
```

3. `HumanRequest` 構造体 (src/types.rs:4815-4824):
```rust
pub struct HumanRequest {
    pub subject: String,
    pub body: String,
    pub context: serde_json::Value,
    pub timeout: Option<std::time::Duration>,
}
```

4. `HumanOutcome` 列挙型 (src/types.rs:4828-4832):
```rust
pub enum HumanOutcome {
    Responded(HumanResponse),
    TimedOut,
    Unreachable(String),
}
```

**RFC §12C の目標定義 (Darvium-RFC-0001-Unified-v2.3-final.md:1917-1963):**
```rust
pub trait InteractionPayload: Clone + Serialize + Deserialize {
    type Outcome: Clone + Serialize + Deserialize;
}

pub struct InteractionRecord<TPayload: InteractionPayload> {
    pub interaction_id: String,
    pub payload: TPayload,
    pub outcome: Option<TPayload::Outcome>,
    pub status: InteractionStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

pub struct HitlPayload {
    pub request: HumanRequest,
}

impl InteractionPayload for HitlPayload {
    type Outcome = HumanOutcome;
}

pub type StoredInteraction = InteractionRecord<HitlPayload>;

impl StoredInteraction {
    pub fn request(&self) -> &HumanRequest { &self.payload.request }
    pub fn outcome(&self) -> &Option<HumanOutcome> { &self.outcome }
}

pub enum InteractionStatus {
    Pending,
    AwaitingExternal,
    Resolved,
    TimedOut,
    Unreachable,
    ChannelClosed,
    Aborted,
}
```

**StoredInteraction 使用箇所（物理ファイル一覧）:**

| ファイル | 行番号 | 使用方法 |
|---------|--------|---------|
| src/store/json_metadata_store.rs | 18,25,35,51,190-430 | インポート・永続化 HashMap・CRUD |
| src/human_channel.rs | 15,101,117,144,154-171,214,231-258,1290-1340 | インポート・export_interactions・観測テスト |
| src/recovery.rs | 128 | 回復ループテストでレコード作成 |
| src/store/metadata_store.rs | トレイト定義シグネチャ | 型参照 |

**後方互換性の検証条件:**
- 既存テストコードは全て `StoredInteraction { interaction_id, request, outcome, status, created_at, updated_at }` のフィールドアクセスパターンを使用している（src/store/json_metadata_store.rs:257-270, 301-310, 349-358, 407-427 等）
- `InteractionStatus::Pending` / `InteractionStatus::Resolved` のパターンマッチが既存コード全体で行われている
- 変更後もこれらのパターンが透過的にコンパイル通過する必要がある（型エイリアス + アクセサによる担保）

## Test Plan

### 1. `InteractionRecord<HitlPayload>` のフィールド完全保存テスト
- 既存 `StoredInteraction` の全フィールドを `InteractionRecord<HitlPayload>` として構築し、互換アクセサ（`request()`, `outcome()`）経由で全フィールドにアクセス可能であることを確認
- 新旧の全フィールドを人手照合: `interaction_id` ↔ `interaction_id`, `request` ↔ `payload.request`, `outcome` ↔ `outcome`, `status` ↔ `status`, `created_at` ↔ `created_at`, `updated_at` ↔ `updated_at`
- `serde_json` ラウンドトリップで新旧構造体の JSON 互換性を確認

### 2. `InteractionStatus` 7状態への拡張確認
- 全7 variant が Debug, Clone, PartialEq, Serialize, Deserialize を実装することのコンパイル確認
- 旧2状態（Pending, Resolved）の既存参照が変更なしにコンパイル通過すること

### 3. 異種ペイロード型のインスタンス化
- `InteractionRecord<HitlPayload>` がコンパイル可能であること（既存 HITL パス）
- `InteractionRecord<HelpPayload>` 等の別ペイロード型がコンパイル可能であること（将来拡張）

### 4. 既存コードの下位互換性
- 既存 `StoredInteraction` を参照する全テストコードが変更なしにコンパイル・通過すること
- 具体的に確認すべきテスト箇所:
  - src/store/json_metadata_store.rs 内の ~10 テスト関数
  - src/human_channel.rs 内の OTS-2 Serde Roundtrip テスト
  - src/recovery.rs 内の回復ループテスト

### 5. 状態遷移マトリクス差分確認
- 旧5状態遷移行列 T_old（RFC 既存定義由来）と新7状態遷移行列 T_new の差分 ΔT = T_new - T_old がゼロであることを確認
- 特に既存遷移（Pending→Resolved, Pending→TimedOut等）が一切変更されていないことを検証

## 計装方法・観測対象

### 計装方法
- ジェネリック型のフィールド構成を現行 `StoredInteraction` のフィールド一覧と人手照合し、全フィールドの完全保存を確認する
- テストコードは `#[cfg(test)] mod tests { ... }` モジュール内に記述（src/types.rs または tests/ 配下）
- コンパイル時の型解決により、全 `StoredInteraction` 参照が `InteractionRecord<HitlPayload>` に透過的に解決されることを確認する

### 観測対象
- `Aborted` 状態を加えた7状態間の全遷移可能性行列 T ∈ {0,1}^{7×7} を列挙し、既存5状態の遷移が一切変更されていないことを行列差分 ΔT = T_new - T_old のゼロ確認により検証する
- JSON シリアライズ互換性テストのラウンドトリップ成功率を n = 1000 で計測し、100% 互換であることを確認する

### 較正計画
本チケットは型定義のみであり、較正対象の定数はない。

## Boy Scout Rule — 翻訳可能性計画

このチケットで触るコードは `src/types.rs` の型定義が中心であり、以下の翻訳可能性を確保する：

1. `InteractionRecord<TPayload>` は「インタラクションレコード」として名詞的に読める。フィールド名 `interaction_id` / `payload` / `outcome` / `status` / `created_at` / `updated_at` は自明で、散文として読める。
2. `HitlPayload` は「HITL ペイロード」で、`InteractionPayload` の実装としての責務を名前に表現する。
3. `InteractionStatus` の7 variant はそれぞれ単一の状態を表し、直感的に理解できる。
4. 既存 `StoredInteraction` のフィールドアクセスパターン（`record.request`、`record.outcome`）は互換アクセサで維持される。
5. 既存コードにおける `InteractionStatus::Pending` / `InteractionStatus::Resolved` のパターンマッチは、7状態拡張後も変更なしに動作する。

## Acceptance Criteria

- [x] `InteractionPayload` トレイトが `src/types.rs` に定義され、`Clone + Serialize + Deserialize` 境界を課している
- [ ] `InteractionRecord<TPayload>` ジェネリック構造体が RFC §12C 通りに定義されている
- [ ] `HitlPayload { request: HumanRequest }` が定義され、`InteractionPayload for HitlPayload` が実装されている
- [ ] `InteractionStatus` が7状態（Pending, AwaitingExternal, Resolved, TimedOut, Unreachable, ChannelClosed, Aborted）に拡張されている
- [ ] `type StoredInteraction = InteractionRecord<HitlPayload>;` エイリアスが定義され、既存コードの参照が変更なしにコンパイル通過する
- [ ] `StoredInteraction` 後方互換アクセサ（`request()`, `outcome()`）が実装されている
- [ ] 既存テストが全て通過している
- [ ] 翻訳可能性の検証が通っている
- [ ] 全7 variant のシリアライズ/デシリアライズが n = 1000 ラウンドトリップで安定している
