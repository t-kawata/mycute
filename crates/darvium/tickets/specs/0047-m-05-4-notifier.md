---
ticket_id: 47
title: M-0.5-4: HITL (Human-In-The-Loop) 抽象トレイト HumanChannel の定義
slug: m-05-4-notifier
status: reviewed
created_at: 2026-05-23
updated_at: 2026-05-23
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0047-m-05-4-notifier/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0047-m-05-4-notifier/observation-20260523-123238.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0047-m-05-4-notifier/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0047-m-05-4-notifier/review.md
---
# M-0.5-4: HITL (Human-In-The-Loop) 抽象トレイト HumanChannel の定義

## Summary

人間との双方向通信を抽象化する `HumanChannel` トレイトと、
`InteractionHandle`（応答ブロッキング待機機構）、`FakeHumanChannel`（テスト用）、
`StdinoutChannel`（標準入出力による具象実装）、および `MetadataStore` への
HITL インタラクション永続化メソッドを定義する。

**中核要件: HITL インタラクションはプロセスの再起動を超えて生存する。**
全 `HumanChannel` 実装は `reconnect()` を提供し、システム終了・再起動後も
未解決のインタラクションを回復可能でなければならない (MUST)。

## Background

### HITL の「命」としての位置づけ

Darvium における HITL は単なる通知機能ではない。ワークフロー実行中に人間の判断が必要な場合、
実行中のワークフローは**完全に待機状態に入り、人間からの応答が届くまで停止し続ける**。
この待機はポーリングスリープではなく OS スケジューラレベルでのブロッキングであり、
CPU リソースを一切消費しない。

RFC §13A Training Orchestrator が定義する human-guided training loop では、
以下の各ステップで HITL が必要となる：

- `HumanMissionReview`: 人間がミッションを確認・承認・編集・却下する
- `ResultReport`: ワークフロー実行結果を人間に報告する
- `HumanFeedback`: 人間が Good/Bad/NeedsRevision 等のフィードバックを付与する
- `PromotionReview`: 訓練成果のプロダクション昇格を人間が判断する

RFC §13B はこれらの人間とのインタラクションを formal object に対応づけることを規範とし、
RFC §16A.1 HumanReviewQueue はタイムアウト・エスカレーション・バッチ処理の運用方針を定める。

### プロセス再起動を超えた生存性 — 設計の核心

`InteractionHandle` は標準の `mpsc::Receiver` を内包する。これは OS プロセスの生存中のみ
有効であり、プロセスがクラッシュまたは再起動された場合、メモリ上のチャネルは消滅する。

しかし HITL インタラクションは「人間が応答を考えている間」にプロセスが落ちる可能性に
耐えなければならない。この要件は既存の `MetadataStore` 抽象化を利用して満たす：

1. **`communicate()` 呼び出し時に `MetadataStore` へ永続化**:
   - interaction_id（`Uuid::new_v4()`）を発行
   - `request`（`HumanRequest`）と共に `StoredInteraction` として保存
   - ステータス: `Pending`

2. **応答受信時に `MetadataStore` を更新**:
   - ステータスを `Resolved` に変更
   - `outcome`（`HumanOutcome`）を保存

3. **プロセス再起動後、`MetadataStore` から未解決行を読み出し**:
   - Orchestrator（将来の M1 チケット）が起動時回復ループを実行
   - 各行に対して `channel.reconnect(id, &request)` を呼ぶ
   - チャネル実装は request の内容を人間に再通知し、応答を待つ

### 回復不能になるシナリオの排除

以下のシナリオを全て潰すため、`MetadataStore` への保存は `HumanChannel` 実装の
内部責務ではなく、`HumanChannel` を利用する上位レイヤー（Orchestrator）の責務とする：

| シナリオ | HumanChannel 内部保存 | Orchestrator + MetadataStore |
|---------|---------------------|-----------------------------|
| プロセスが communicate 直後にクラッシュ | 保存前に死ぬので不可能 | 呼び出し元が保存済みなので回復可能 |
| StdinoutChannel で外部アプリもクラッシュ | 外部アプリ依存で不可能 | Darvium の DB にリクエスト全文が残っている |
| 再起動後に別のチャネルに差し替え | チャネル固有の保存形式では不整合 | MetadataStore 抽象化で透過 |

### Notifier から HumanChannel への進化

| 要件 | Notifier | HumanChannel |
|------|----------|-------------|
| 一方向通知（fire-and-forget） | ✅ | ✅ (`notify()`) |
| 応答を含む双方向通信 | ❌ | ✅ (`communicate()`) |
| 待機中のスレッドブロッキング | ❌ | ✅ (`InteractionHandle::wait()`) |
| タイムアウト付き待機 | ❌ | ✅ |
| クラッシュ後の再接続 | ❌ | ✅ (`reconnect()`) |
| MetadataStore による永続化 | ❌ | ✅ |
| 応答キューによる疑似セッション | ❌ | ✅ |
| 具象チャネル差し替え | △（送信のみ） | ✅（送受信一貫） |

## Scope

### 1. HumanChannel トレイト（`src/human_channel.rs`）

```rust
pub trait HumanChannel: Send + Sync {
    /// 一方向通知（fire-and-forget）。
    fn notify(&self, request: &HumanRequest) -> Result<(), DarviumError>;

    /// 双方向通信（応答待機）。
    /// interaction_id（Uuid::new_v4()）を発行し、自身の状態管理下で
    /// インタラクションを追跡可能にする。
    fn communicate(&self, request: &HumanRequest) -> Result<InteractionHandle, DarviumError>;

    /// 永続化された interaction_id とリクエストからインタラクションを再接続する。
    ///
    /// プロセス再起動後に呼ばれる。request は MetadataStore から復元された
    /// 元のリクエスト全文である。チャネル実装は request の内容を人間に再通知し、
    /// 応答待機可能な InteractionHandle を返す。
    ///
    /// 全実装がこのメソッドを提供しなければならない (MUST)。
    fn reconnect(&self, interaction_id: uuid::Uuid, request: &HumanRequest)
        -> Result<InteractionHandle, DarviumError>;
}
```

`reconnect()` が `request: &HumanRequest` を引数に取ることで、チャネル実装は
元のリクエスト内容を知らなくても再接続できる。MetadataStore から復元したリクエストを
Orchestrator が渡す。これによりチャネル実装はストレージに依存せず、transport だけに
専念できる。

### 2. データ型 — 全 HITL 型は `types.rs` に定義

既存パターン（`SearchTrace`, `TrustAuditLog`, `TrainingMetadata` 等は `types.rs`）に従い、
HITL 関連の全データ型は `crate::types` に定義する。

```rust
// ========== src/types.rs に追加 ==========

use serde::{Serialize, Deserialize};

/// 人間への依頼内容。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanRequest {
    /// 概要タイトル（Slack の見出し行、メールの件名）。
    pub subject: String,
    /// 詳細説明（ワークフロー名・成否・判断材料など）。
    pub body: String,
    /// 機械可読なコンテキスト情報。チャネル実装は透過的に通過させる。
    pub context: serde_json::Value,
    /// 応答待機の推奨最大時間。
    pub timeout: Option<std::time::Duration>,
}

/// 人間との双方向通信の結果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HumanOutcome {
    Responded(HumanResponse),
    TimedOut,
    Unreachable(String),
}

/// 人間からの応答内容。
/// RFC §13A 規範要件2（edit mission text）に従い revised_body を保持。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanResponse {
    pub decision: HumanDecision,
    pub comment: Option<String>,
    pub revised_body: Option<String>,
}

/// 人間の判断。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HumanDecision {
    Approved,
    Rejected,
    NeedsRevision,
    Irrelevant,
    Unsafe,
}

/// 永続化される HITL インタラクションのレコード。
/// MetadataStore 経由で SQLite / InMemory に保存される。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredInteraction {
    /// UUID v4。全プロセス再起動を超えて一意。
    /// String として保持することで MetadataStore 実装（SQLite の TEXT 等）との
    /// 相互運用性を確保。export_interactions() で Uuid→String 変換される。
    pub interaction_id: String,
    /// リクエスト全文。
    pub request: HumanRequest,
    /// 応答。Resolved 時のみ Some。
    pub outcome: Option<HumanOutcome>,
    /// 現在の状態。
    pub status: InteractionStatus,
    /// 作成時刻（Unix エポック秒）。
    pub created_at: u64,
    /// 最終更新時刻（Unix エポック秒）。
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InteractionStatus {
    Pending,
    Resolved,
}
```

`human_channel.rs` では `use crate::types::{...}` でこれらの型をインポートする。

### 3. MetadataStore への永続化メソッド追加（`src/store/metadata_store.rs`）

既存の `MetadataStore` トレイトに以下の 4 メソッドを追加する：

```rust
pub trait MetadataStore {
    // === 既存メソッド ===
    fn store_search_trace(&self, trace: &SearchTrace) -> Result<(), DarviumError>;
    // ...

    // === HumanChannel インタラクション永続化（M-0.5-4） ===
    /// HITL インタラクションを保存する（新規作成時: status=Pending）。
    fn store_human_interaction(&self, record: &StoredInteraction) -> Result<(), DarviumError>;

    /// interaction_id で HITL インタラクションを取得する。
    fn load_human_interaction(&self, interaction_id: &str) -> Result<StoredInteraction, DarviumError>;

    /// status=Pending の全 HITL インタラクションを取得する。
    /// プロセス再起動時の回復ループで使用する。
    fn list_pending_human_interactions(&self) -> Result<Vec<StoredInteraction>, DarviumError>;

    /// インタラクションの outcome と status を更新する（Pending→Resolved）。
    fn resolve_human_interaction(&self, interaction_id: &str, outcome: &HumanOutcome)
        -> Result<(), DarviumError>;
}
```

`InMemoryMetadataStore` は `HashMap<String, StoredInteraction>` でこれらを実装する。

**SQLite 実装差し替え時の DDL 定義（本チケットでは実装しないが、設計として確定する）:**

```sql
CREATE TABLE IF NOT EXISTS human_interactions (
    interaction_id TEXT PRIMARY KEY NOT NULL,   -- UUID v4
    request_json   TEXT NOT NULL,                -- HumanRequest を JSON シリアライズ
    outcome_json   TEXT,                         -- HumanOutcome を JSON シリアライズ（Resolved 時のみ）
    status         TEXT NOT NULL DEFAULT 'Pending',  -- 'Pending' | 'Resolved'
    created_at     INTEGER NOT NULL,             -- Unix エポック秒
    updated_at     INTEGER NOT NULL              -- 最終更新時刻
);

CREATE INDEX idx_human_interactions_status ON human_interactions(status);
```

`request_json` / `outcome_json` は `serde_json::to_string()` / `from_str()` で
やり取りする。`InMemoryMetadataStore` は `StoredInteraction` をそのまま保持するため
JSON シリアライズは発生しないが、`SqliteMetadataStore`（後段チケット）は
store/load の際に JSON 変換を行えばよい。

**SQLite / LadybugDB の住み分け:**

| ストア | 責務 | HITL との関係 |
|--------|------|-------------|
| `GraphStore`（LadybugDB 責務） | ワークフローグラフ、埋め込みベクトル、知識オブジェクト、リレーション、OriginTrace | 関係なし。HITL インタラクションは知識オブジェクトではない |
| `MetadataStore`（SQLite 責務） | メタデータ、信頼スコア、監査ログ、Training/Fusion メタデータ | **HITL インタラクションはここに属する**。リクエスト・応答・状態はメタデータであり、LadybugDB の対象ではない |

HITL インタラクションの永続化は MetadataStore（SQLite 責務）の範囲である。
LadybugDB（GraphStore）が HITL データを扱うことはない。
この住み分けは既存のデュアルストア抽象化の定義（`src/store/mod.rs:1-5`）と完全に整合する。

**実装差し替え時の動作保証:**

```
InMemoryMetadataStore（M-0.5-4 で実装）
  ↓ store/load/list_pending/resolve の全テスト（T10）が通過
  ↓
SqliteMetadataStore（後段チケットで実装）
  └─ 上記 DDL でテーブル作成
  └─ request_json / outcome_json は serde_json で変換
  └─ interaction_id を PRIMARY KEY に設定
  └─ 全テスト（T10）が InMemory と同一の振る舞いを示すことを確認
  ↓
将来的な別 DB 実装
  └─ MetadataStore トレイトを実装するだけで自動的に HITL 永続化を継承
```

`MetadataStore` トレイトの 4 メソッドへのプログラミングである限り、
ストア実装の差し替えだけで HITL 永続化は完全に動作する。
これを保証するため、`InMemoryMetadataStore` のテスト（T10）は
`SqliteMetadataStore` が同じテストスイートで再実行できる形にする
（後段チケットの責務だが、設計として確定する）。

### 4. InteractionHandle（`src/human_channel.rs`）

reader スレッドの I/O エラー（不正 JSON、EOF 等）を呼び出し元に伝播できるよう、
内部チャネルは `Result<HumanOutcome, DarviumError>` を運ぶ。

```rust
pub struct InteractionHandle {
    pub(crate) interaction_id: uuid::Uuid,
    rx: std::sync::mpsc::Receiver<Result<HumanOutcome, DarviumError>>,
}

impl InteractionHandle {
    pub fn interaction_id(&self) -> &uuid::Uuid;

    /// 応答をブロッキング待機する。
    /// - Some(dur): recv_timeout(dur) を使用。超過で Ok(TimedOut)。
    /// - None: recv() を使用。無制限待機。
    /// - チャネルが Err(DarviumError) を運んだ場合、そのエラーを呼び出し元に伝播する。
    pub fn wait(self, timeout: Option<std::time::Duration>)
        -> Result<HumanOutcome, DarviumError>
    {
        match timeout {
            Some(dur) => match self.rx.recv_timeout(dur) {
                Ok(result) => result,
                Err(mpsc::RecvTimeoutError::Timeout) => Ok(HumanOutcome::TimedOut),
                Err(mpsc::RecvTimeoutError::Disconnected) => Err(DarviumError::HumanChannelClosed),
            },
            None => match self.rx.recv() {
                Ok(result) => result,
                Err(_) => Err(DarviumError::HumanChannelClosed),
            },
        }
    }
}
```

これにより、StdinoutChannel の reader スレッドは不正 JSON や EOF を
`Err(HumanChannelIo(...))` として送信し、wait() がそのエラーを呼び出し元に伝播する。
`TimedOut` は通常の値 (`Ok(TimedOut)`) としてチャネルを経由しない（recv_timeout の
タイムアウト機構で検出される）。切断は `Err(HumanChannelClosed)` として伝播される。
```

### 5. FakeHumanChannel（`src/human_channel.rs`）

```rust
/// FakeHumanChannel が管理する個別インタラクションの内部レコード。
/// - communicate() の呼び出しで Pending として登録され、応答消費後に Resolved に遷移する。
/// - reconnect() はこのレコードを参照して既存インタラクションを再開する。
enum InteractionRecord {
    Pending { request: HumanRequest },
    Resolved(HumanOutcome),
}

pub struct FakeHumanChannel {
    sent_count: std::sync::atomic::AtomicU64,
    requests_sent: std::sync::Mutex<Vec<HumanRequest>>,
    preloaded: std::sync::Mutex<std::collections::VecDeque<HumanOutcome>>,
    interactions: std::sync::Mutex<
        std::collections::HashMap<uuid::Uuid, InteractionRecord>
    >,
}

/// テスト時に FakeHumanChannel が管理している全インタラクションを
/// StoredInteraction の Vec としてエクスポートする。
impl FakeHumanChannel {
    /// 現在の全インタラクションを StoredInteraction の Vec として取得する。
    /// MetadataStore への永続化のテストで使用する。
    pub fn export_interactions(&self) -> Vec<StoredInteraction>;

    /// 全内部状態を初期状態にリセットする。
    /// - sent_count = 0
    /// - requests_sent = 空Vec
    /// - preloaded = 空VecDeque
    /// - interactions = 空HashMap
    pub fn reset(&self);
}
```

動作仕様:

| メソッド | 動作 |
|---------|------|
| `notify()` | 常に `Ok(())`。`requests_sent` + `sent_count` を更新。<br>**注**: `notify()` は fire-and-forget であり `HashMap` にインタラクションを追加しない。`interaction_id` を発行しないため、`export_interactions()` の対象外。 |
| `communicate()` | 1. `interaction_id = Uuid::new_v4()`<br>2. `HashMap` に `Pending` で保存<br>3. プリロードキューから取り出し。空なら panic<br>4. `Resolved` に更新。tx に `Ok(outcome)` を即時送信<br>5. `InteractionHandle` を返す |
| `reconnect(id, request)` | 1. HashMap から id を検索<br>2. 見つかった → `Pending` → 新規 handle（tx に `Ok(TimedOut)` を即時送信）<br>3. 見つかった → `Resolved` → 新規 handle（tx に `Ok(outcome)` を即時送信）<br>4. **見つからなかった（別インスタンス＝クラッシュ後）** → プリロードキューから取り出し<br>5. キューも空 → `Err(HumanChannelIo(...))`<br><br>**設計意図**: `reconnect()` は同一インスタンス内の復旧（HashMap 参照）と<br>プロセス再起動後の復旧（新インスタンス + プリロードキュー代替応答）の<br>両方に対応する。これにより T10-7（新インスタンスで reconnect）が成立する。 |
| `export_interactions()` | 全 `(id, record)` を `Vec<StoredInteraction>` に変換。<br>`interaction_id` は `Uuid` → `String`（`to_string()`）に変換される。<br>`created_at` / `updated_at` は `std::time::SystemTime::now()` で<br>動的に生成する（`InteractionRecord` はタイムスタンプを保持しないため）。<br>`Pending` レコードは `outcome: None` / `status: Pending` に、<br>`Resolved` レコードは `outcome: Some(outcome)` / `status: Resolved` に変換される。 |

### 6. StdinoutChannel（`src/human_channel.rs`）

reader は `communicate()` / `reconnect()` 内で別スレッドに委譲するため `Arc` で包む。
これにより write（同期的）→ handle 即時返却 → read（別スレッド）→ mpsc 経由で解決、
という非同期読み取りパターンを実現する。

`session: Mutex<()>` は複数の `communicate()` / `reconnect()` が同時に呼ばれた場合の
write-read 系列を直列化する。実装内部では、セッションロックを取得したスレッドのみが
write + read の全サイクルを実行する。これにより、同時呼び出し時の応答の取り違えを防止する。

```rust
pub struct StdinoutChannel<R, W> {
    reader: std::sync::Arc<std::sync::Mutex<R>>,
    writer: std::sync::Mutex<W>,
    session: std::sync::Mutex<()>,  // 同時呼び出し直列化（write + read 全体）
}

impl<R: std::io::BufRead + Send, W: std::io::Write + Send> HumanChannel for StdinoutChannel<R, W> {
    fn notify(&self, request: &HumanRequest) -> Result<(), DarviumError> {
        let id = uuid::Uuid::new_v4();
        let mut writer = self.writer.lock().map_err(|e| HumanChannelIo(e.to_string()))?;
        write_json_line(&mut *writer, "notify", id, request)
    }

    fn communicate(&self, request: &HumanRequest) -> Result<InteractionHandle, DarviumError> {
        let id = uuid::Uuid::new_v4();
        let (tx, rx) = std::sync::mpsc::channel();
        // セッションロック確保（_session がドロップされるまで次の呼び出しはブロック）
        let _session = self.session.lock().map_err(|e| HumanChannelIo(e.to_string()))?;
        // 1. リクエスト送信（同期的）
        {
            let mut writer = self.writer.lock().map_err(|e| HumanChannelIo(e.to_string()))?;
            write_json_line(&mut *writer, "communicate", id, request)?;
            writer.flush().map_err(|e| HumanChannelIo(e.to_string()))?;
        }
        // 2. 応答読み取りスレッドを起動（非同期的）
        let reader = self.reader.clone();
        std::thread::spawn(move || {
            let mut line = String::new();
            match reader.lock() {
                Ok(mut r) => {
                    match r.read_line(&mut line) {
                        Ok(0) => {
                            // EOF → エラーとして伝播
                            let _ = tx.send(Err(DarviumError::HumanChannelIo("reader EOF: response line expected".into())));
                            return;
                        }
                        Ok(_) => {
                            // 1行読み取り成功 → JSON パース
                            if let Ok(resp) = serde_json::from_str::<StdinoutResponse>(&line) {
                                // interaction_id 不一致を検出
                                if resp.interaction_id != id {
                                    let _ = tx.send(Ok(HumanOutcome::Unreachable(
                                        format!("interaction_id mismatch: expected {}, got {}", id, resp.interaction_id)
                                    )));
                                    return;
                                }
                                if let Some(outcome) = resp.outcome {
                                    let _ = tx.send(Ok(outcome));
                                    return;
                                }
                            }
                            // JSON パース失敗 → エラーとして伝播
                            let _ = tx.send(Err(DarviumError::HumanChannelIo(
                                format!("invalid JSON response: {}", line.trim())
                            )));
                            return;
                        }
                        Err(e) => {
                            // read_line の I/O エラー
                            let _ = tx.send(Err(DarviumError::HumanChannelIo(
                                format!("reader I/O error: {}", e)
                            )));
                            return;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(DarviumError::HumanChannelIo(
                        format!("reader mutex poisoned: {}", e)
                    )));
                    return;
                }
            }
        });
        Ok(InteractionHandle { interaction_id: id, rx })
    }

    fn reconnect(&self, interaction_id: uuid::Uuid, request: &HumanRequest)
        -> Result<InteractionHandle, DarviumError>
    {
        let (tx, rx) = std::sync::mpsc::channel();
        // セッションロック確保
        let _session = self.session.lock().map_err(|e| HumanChannelIo(e.to_string()))?;
        // 1. リクエスト再通知（同期的）
        {
            let mut writer = self.writer.lock().map_err(|e| HumanChannelIo(e.to_string()))?;
            write_json_line(&mut *writer, "reconnect", interaction_id, request)?;
            writer.flush().map_err(|e| HumanChannelIo(e.to_string()))?;
        }
        // 2. 応答読み取りスレッド（非同期的）
        let reader = self.reader.clone();
        std::thread::spawn(move || {
            let mut line = String::new();
            match reader.lock() {
                Ok(mut r) => {
                    match r.read_line(&mut line) {
                        Ok(0) => {
                            let _ = tx.send(Err(DarviumError::HumanChannelIo("reader EOF: response line expected".into())));
                            return;
                        }
                        Ok(_) => {
                            if let Ok(resp) = serde_json::from_str::<StdinoutResponse>(&line) {
                                if resp.interaction_id != interaction_id {
                                    let _ = tx.send(Ok(HumanOutcome::Unreachable(
                                        format!("interaction_id mismatch: expected {}, got {}", interaction_id, resp.interaction_id)
                                    )));
                                    return;
                                }
                                if let Some(outcome) = resp.outcome {
                                    let _ = tx.send(Ok(outcome));
                                    return;
                                }
                            }
                            let _ = tx.send(Err(DarviumError::HumanChannelIo(
                                format!("invalid JSON response: {}", line.trim())
                            )));
                            return;
                        }
                        Err(e) => {
                            let _ = tx.send(Err(DarviumError::HumanChannelIo(
                                format!("reader I/O error: {}", e)
                            )));
                            return;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(DarviumError::HumanChannelIo(
                        format!("reader mutex poisoned: {}", e)
                    )));
                    return;
                }
            }
        });
        Ok(InteractionHandle { interaction_id, rx })
    }
}

/// 内部ヘルパー
fn write_json_line<W: std::io::Write>(
    writer: &mut W, msg_type: &str, interaction_id: uuid::Uuid, request: &HumanRequest,
) -> Result<(), DarviumError> {
    let payload = serde_json::json!({
        "type": msg_type,
        "interaction_id": interaction_id,
        "request": request,
    });
    let line = serde_json::to_string(&payload)
        .map_err(|e| HumanChannelIo(e.to_string()))?;
    writeln!(writer, "{}", line).map_err(|e| HumanChannelIo(e.to_string()))?;
    Ok(())
}

/// StdinoutChannel 応答パース用の中間型
#[derive(Deserialize)]
struct StdinoutResponse {
    interaction_id: uuid::Uuid,
    outcome: Option<HumanOutcome>,
}
```

通信プロトコル（JSON Lines）:

```
# notify():
→ {"type":"notify","interaction_id":"xxx","request":{...}}
# （応答なし）

# communicate():
→ {"type":"communicate","interaction_id":"xxx","request":{...}}
← {"interaction_id":"xxx","outcome":{...}}

# reconnect():
→ {"type":"reconnect","interaction_id":"xxx","request":{...}}
← {"interaction_id":"xxx","outcome":{...}}
```

`reconnect()` も `communicate()` と同様に `request` 全体を書き出すため、
外部アプリが過去のリクエスト内容を失っていても再接続可能。

**`interaction_id` 不一致検出**: StdinoutChannel はレスポンス行の `interaction_id` が
リクエストの ID と一致しない場合、`HumanOutcome::Unreachable(...)` を返す。
これにより同一チャネル上でのプロトコル誤用を早期検出する。

### 7. DarviumError バリアント（`src/error.rs`）

```rust
// === HumanChannel ===
#[error("Human channel I/O error: {0}")]
HumanChannelIo(String),

#[error("Human channel disconnected")]
HumanChannelClosed,
```

### 8. 依存クレート

```bash
cargo add uuid@1 --features v4
```

`serde` と `serde_json` は既に依存済み。

### 9. モジュール構成

- `src/types.rs`: `HumanRequest`, `HumanOutcome`, `HumanResponse`, `HumanDecision`, `StoredInteraction`, `InteractionStatus` を追加
- `src/human_channel.rs`: `HumanChannel` トレイト + `InteractionHandle` + `FakeHumanChannel` + `StdinoutChannel` + テスト
- `src/store/metadata_store.rs`: 4 メソッド追加 + `InMemoryMetadataStore` に実装
- `src/error.rs`: 2 バリアント追加
- `src/lib.rs`: `pub mod human_channel;` 追加。`pub use` で `HumanChannel` / `InteractionHandle` / `FakeHumanChannel` / `StdinoutChannel` を公開。
  `InteractionRecord` は非公開（`pub use` 対象外）。

### 10. 依存関係グラフ

```
types.rs ←── human_channel.rs ←── lib.rs
  ↑                                    ↑
  └── store/metadata_store.rs ──────────┘
       (MetadataStore トレイト + InMemoryMetadataStore)
```

`StoredInteraction` が `types.rs` にあるため、`human_channel` と `metadata_store` は
互いに依存せず、両方とも `types.rs` にのみ依存する。循環依存が発生しない。

### 永続化と回復の責務分離

```
communicate() → HumanChannel 実装が interaction_id を発行
                     ↓
              Orchestrator（上位レイヤー）が MetadataStore に保存
              （M-0.5-4 では FakeHumanChannel の export_interactions()
                で保存内容を取得可能にしておく。実際の保存は M1 以降）
                     ↓
              プロセス再起動
                     ↓
              Orchestrator が MetadataStore.list_pending() を呼ぶ
                     ↓
              Orchestrator が channel.reconnect(id, request) を呼ぶ
                     ↓
              HumanChannel 実装が request を人間に再通知 → wait()
```

M-0.5-4 では MetadataStore へのメソッド追加と InMemory 実装までを行い、
実際の保存・回復ループは M1（Orchestrator）で実装する。ただしテストでは
`FakeHumanChannel` の `export_interactions()` と `reconnect()` を使って
保存→回復の全サイクルを擬似的に検証する。

### テスト計画（全網羅）

#### T1: FakeHumanChannel の基本動作（2 テスト）

| ID | 内容 | 検証 |
|----|------|------|
| T1-1 | 型境界充足 | `FakeHumanChannel` が `HumanChannel + Send + Sync` を実装 |
| T1-2 | notify fire-and-forget | `notify()` が常に `Ok(())`。`sent_count() == 1` |

#### T2: 単一 HITL 通信（3 テスト）

| ID | 内容 | 検証 |
|----|------|------|
| T2-1 | 基本送受信 | `communicate()` → `wait(None)` で `Responded` が返る |
| T2-2 | 全 decision × comment × revised_body 網羅（パラメタライズド） | 5×2×2=20 通りの全組み合わせで正しく伝播。<br>これにより T2-3（comment 付き）と T2-4（revised_body 付き）の<br>個別テストは兼ねる。 |
| T2-3 | 空文字 subject/body | 空文字列でもエラーにならず通信成立 |

#### T3: 複数 HITL の全件記録（6 テスト）

| ID | 内容 | 検証 |
|----|------|------|
| T3-1 | 3回 notify | `sent_count() == 3` かつ全内容一致 |
| T3-2 | 3回 communicate | `sent_count() == 3` かつ全 `wait()` が期待値一致 |
| T3-3 | FIFO 順序 | `requests_sent()` の順序と呼び出し順が一致 |
| T3-4 | 異種リクエスト | 異なる subject/body でも全件記録 |
| T3-5 | 大量 1,000 回 | 欠損なく全件記録 |
| T3-6 | インスタンス独立性 | 2 インスタンスのカウンタが混ざらない |

#### T4: InteractionHandle ブロッキング動作（5 テスト）

| ID | 内容 | 検証 |
|----|------|------|
| T4-1 | 即時解決 | `FakeHumanChannel` では `wait(None)` が即座に復帰 |
| T4-2 | タイムアウト（StdinoutChannel） | `wait(Some(1ms))` → 応答なし → `TimedOut`。<br>StdinoutChannel を使用。ライターにリクエストを送信した後、<br>reader からの応答が 1ms 以内に届かないことを確認。<br>FakeHumanChannel は `communicate()` 内で即時解決されるためこのテストには適さない。 |
| T4-3 | 無制限待機（StdinoutChannel） | `wait(None)` + 別スレッド 50ms 後ライターに応答書き込み → ブロック解除。<br>StdinoutChannel を使用。reader スレッドが別スレッドからの書き込みを<br>読み取って解決するまで main スレッドがブロックされることを確認。<br>FakeHumanChannel では `communicate()` 内で即時解決されるためこのテストには適さない。 |
| T4-4 | mpsc 切断 | mpsc クローズ → `HumanChannelClosed` |
| T4-5 | drop 安全 | `wait()` せず drop → リソースリークなし |

#### T5: トレイトオブジェクト安全性（3 テスト）

| ID | 内容 | 検証 |
|----|------|------|
| T5-1 | `Box<dyn HumanChannel>` | 動的ディスパッチで全 3 メソッド使用可能 |
| T5-2 | `&dyn HumanChannel` | 関数引数に参照として渡せる |
| T5-3 | `Arc<dyn HumanChannel>` | スレッド間共有可能 |

#### T6: StdinoutChannel 実装（12 テスト）

| ID | 内容 | 検証 |
|----|------|------|
| T6-1 | notify JSON | ライターに `type:"notify"` を含む正しい JSON |
| T6-2 | communicate 即時解決 | リーダーに応答プリセット → 即時解決 |
| T6-3 | communicate ブロッキング | 別スレッド 50ms 後ライターに応答を書き込む → `wait(None)` 解除。<br>**非同期書き込みの仕組み**: 事前に別スレッド（`std::thread::spawn`）を起動し、<br>50ms `sleep` 後に `writer` へ `{"interaction_id":"...","outcome":{"Responded":{"decision":"Approved",...}}}⏎` を<br>書き込む。メインスレッドの `read_line()` / `serde_json::from_str()` がその行を<br>読み取って解決する。 |
| T6-4 | タイムアウト | `wait(Some(10ms))` → `TimedOut` |
| T6-5 | 3往復セッション | 連続 HITL の完全性 |
| T6-6 | 複数インスタンス独立性 | 個別リーダー/ライター対で混ざらない |
| T6-7 | 不正 JSON 応答 | `}]invalid{:["` → `HumanChannelIo` |
| T6-8 | EOF | リーダー EOF → `HumanChannelIo` |
| T6-9 | reconnect JSON | ライターに `type:"reconnect"` + id + request が 1 行 |
| T6-10 | 巨大ペイロード | 1MB body → パニックしない |
| T6-11 | communicate interaction_id 不一致 | レスポンスの interaction_id がリクエストと異なる → `Unreachable(...)` |
| T6-12 | reconnect interaction_id 不一致 | reconnect 応答の interaction_id 不一致 → `Unreachable(...)` |

T6-11 と T6-12 は、プリロードされた応答行の `interaction_id` フィールドを
意図的に異なる値に書き換えることで実現する。

#### T7: エラーケースと境界値（5 テスト）

| ID | 内容 | 検証 |
|----|------|------|
| T7-1 | 任意の context | `serde_json::Value` を渡してもパニックしない |
| T7-2 | `wait(Some(0ns))` | 即 `TimedOut` |
| T7-3 | `wait(None)` 別スレッド | 解決後即座に復帰 |
| T7-4 | 空キュー communicate | panic（`#[should_panic]`） |
| T7-5 | 8 スレッド同時アクセス | データ競合なし |

#### T8: FakeHumanChannel リセット（3 テスト）

| ID | 内容 | 検証 |
|----|------|------|
| T8-1 | reset → sent_count == 0 | カウンタリセット |
| T8-2 | reset → requests_sent 空 | 記録リセット |
| T8-3 | reset → 再使用可能 | `notify()` → `sent_count() == 1` |

#### T9: reconnect 回復可能性（7 テスト）

| ID | 内容 | 検証 |
|----|------|------|
| T9-1 | 解決済み再接続 | `communicate()` → `wait(None)` → `reconnect(id, req)` → 同一 outcome |
| T9-2 | 未知 ID + 空キュー | `reconnect(unknown_id, req)` を空のプリロードキューで呼ぶ → `Err(HumanChannelIo)` |
| T9-3 | 未知 ID + プリロードあり | `reconnect(unknown_id, req)` をプリロードありで呼ぶ → キューから応答。`wait(None)` が解決 |
| T9-4 | Stdinout reconnect protocol | `reconnect(id, req)` → writer に `type:"reconnect"` + id + request |
| T9-5 | Stdinout reconnect 解決 | reconnect → 別スレッド応答 → `wait(None)` 解決 |
| T9-6 | Stdinout reconnect 不正応答 | 不正 JSON → `HumanChannelIo` |
| T9-7 | notify→communicate→reconnect 一貫性 | 全 3 メソッドの直列フロー |

**削除された T9-2（旧「未解決再接続」）**: `FakeHumanChannel` は `communicate()` 内で
即座にプリロードキューから消費し Resolved に遷移するため、同一インスタンスで
未解決状態のインタラクションは存在し得ない。代替として T10-7（Pending 生存）が
未解決からの回復パスをカバーする。

#### T10: MetadataStore HITL 永続化（8 テスト）

| ID | 内容 | 検証 |
|----|------|------|
| T10-1 | store → load 一致 | `store_human_interaction()` → `load_human_interaction()` で内容一致 |
| T10-2 | 存在しない ID | `load_human_interaction("nonexistent")` → `Err(NotFound)` |
| T10-3 | Pending のみ抽出 | 3 件保存（2 Pending + 1 Resolved）→ `list_pending()` が 2 件 |
| T10-4 | resolve で状態遷移 | `resolve_human_interaction()` → `load()` で status=Resolved + outcome=Some |
| T10-5 | 重複 store 上書き | 同一 ID で 2 回 store → 最新で上書きされる |
| T10-6 | FakeHumanChannel → MetadataStore 一貫性 | `export_interactions()` → MetadataStore に保存 → `list_pending()` で再現 |
| T10-7 | 再起動シミュレーション完全サイクル（Pending 生存） | クラッシュ時に未解決だったインタラクションの回復 |
| T10-8 | 再起動シミュレーション完全サイクル（Resolved 生存） | クラッシュ時に解決済みだったインタラクションの回復 |

**T10-7**（Pending 生存）: クラッシュ時に人間がまだ応答していなかったケースを模擬する。
手動構築した Pending レコードを MetadataStore に格納し、新インスタンスで reconnect する。

```
1. 手動で StoredInteraction (status=Pending) を構築
2. store.store_human_interaction(pending_record)
3. let pending_list = store.list_pending()? → 1 件（Pending）
4. 新しい FakeHumanChannel（プリロードキューに応答を設定）で reconnect(id, request)
   → FakeHumanChannel: HashMap 空 → プリロードキューから応答
5. handle.wait(None) → Responded（キューからの応答）
6. store.resolve_human_interaction(id, &outcome)
7. store.load_human_interaction(id) → status=Resolved, outcome 一致
```

**T10-8**（Resolved 生存）: クラッシュ時に人間の応答が既に MetadataStore に保存されていた
ケースを模擬する。FakeHumanChannel の export_interactions() を使って保存する。

```
1. FakeHumanChannel.communicate(req) → handle (FakeHumanChannel は即時解決)
2. handle.wait(None) → Responded
3. let interactions = channel.export_interactions() → status=Resolved
4. store.store_human_interaction(interactions[0])
5. drop(channel) // プロセス終了模擬
6. let stored = store.load_human_interaction(id)? → status=Resolved
7. 新しい FakeHumanChannel（プリロードキューに応答を設定）で reconnect(id, request)
8. handle2.wait(None) → Responded（キューからの応答）
9. store.resolve_human_interaction(id, &outcome) // 冪等
10. store.load_human_interaction(id) → status=Resolved, outcome 一致
```

**T10-7 と T10-8 の違い:**

| 観点 | T10-7（Pending） | T10-8（Resolved） |
|------|-----------------|------------------|
| データ源 | 手動構築 | FakeHumanChannel.export_interactions() |
| store 内 status | Pending | Resolved |
| 回復方法 | list_pending() → reconnect | load_human_interaction() → reconnect |
| クラッシュタイミング | 人間応答前 | 人間応答後・MetadataStore 更新後 |
| 検証範囲 | 起動時回復ループ | 既存データ整合性 |

### 観測テスト (OTS)

| ID | 内容 | n | 検証 |
|----|------|---|------|
| OTS-1 | 呼び出し回数 vs 記録件数完全一致 | 10,000 | σ² = 0 |
| OTS-2 | HumanRequest / HumanOutcome / StoredInteraction の Serde ラウンドトリップ | 8,192 | 全 field ランダム変化で原値一致。<br>`interaction_id` はランダム UUID、`created_at` / `updated_at` はランダム UNIX 時刻、<br>`status` は Pending / Resolved 交互、`outcome` は全バリアントをランダムに選択して検証する。 |

**OTS-3（削除）**: 従来の `5×2×2=20` 網羅は T2-2 のパラメタライズドテストとして統合した。
観測テスト枠は統計的性質の検証に特化する。

### 計装方法・観測対象

- `FakeHumanChannel`: `AtomicU64` で `sent_count()`。`Mutex<HashMap<Uuid, InteractionRecord>>` で全インタラクション管理
- OTS は `println!` + `--nocapture` で統計量を構造化テキスト出力
- `StdRng::seed_from_u64(12345)` を OTS で使用
- 本チケットに較正対象の連続値パラメータは存在しない

## Non-scope

- **WebSocketChannel**: `tokio-tungstenite` + async runtime。後段チケット
- **HttpChannel / gRPC Channel**: 同上
- **Slack / Teams / LINE / Email チャネル**: 各 SDK が必要。後段チケット
- **Orchestrator による起動時回復ループ**: M1 以降。本チケットでは MetadataStore の
  メソッド定義と `InMemoryMetadataStore` 実装 + T10-7/T10-8 で擬似サイクル検証まで
- **SqliteMetadataStore の実装**: 本チケットでは `InMemoryMetadataStore` への実装まで。
  SQLite への実装は後段チケット。ただし DDL 定義は本チケットで確定するため、
  SqliteMetadataStore は spec 内の DDL に従ってテーブルを作成すればよい
- **HumanReviewQueue**: M1-1
- **非同期 HumanChannel**: 本トレイトは同期的インターフェースとする

## Investigation

### コードベース調査結果

1. **既存トレイトパターン** (`src/llm/mod.rs:55`):
   全トレイトは `pub trait Foo: Send + Sync`。`HumanChannel` も同様。

2. **DarviumError 定義** (`src/error.rs`):
   `Internal` 直前に `// === HumanChannel ===` セクションとして 2 バリアント追加。

3. **既存データ型の配置パターン** (`src/types.rs`):
   `SearchTrace`, `TrustAuditLog`, `PatchHistory`, `TrainingMetadata`, `FusionMetadata` は
   全て `types.rs` に定義。`human_channel.rs` は自前の型を持たず `types.rs` からインポートする。

4. **MetadataStore トレイト** (`src/store/metadata_store.rs:17`):
   現在 10 メソッドを持つ。`store_*` / `load_*` のペアパターンが確立されている。
   新規追加する 4 メソッドも同一パターンに従う。
   `InMemoryMetadataStore` は `HashMap` + `RefCell` で実装されている。
   HITL 用に `HashMap<String, StoredInteraction>` を追加する。

5. **モジュール構成** (`src/lib.rs`):
   `pub mod mock;` の直後に `pub mod human_channel;` を挿入。
   全型は `crate::types` 経由で公開。

### 設計判断の根拠

**なぜ `reconnect(id, &request)` であって `reconnect(id)` だけではないのか:**

| 方式 | 問題 |
|------|------|
| `reconnect(id)` のみ | チャネル実装が request を知らない。StdinoutChannel が stdout に再通知できない |
| `reconnect(id, &request)` | MetadataStore から復元した request をチャネルが再通知できる。チャネルはストレージ不要 |

**なぜ MetadataStore への保存を HumanChannel 実装の責務にしないのか:**

`HumanChannel` は transport の抽象化。ストレージの知識を持つと単一責務を破る。
保存は上位レイヤー（Orchestrator）の責務とする。ただしテストでは
`FakeHumanChannel::export_interactions()` で保存内容を取り出せるようにし、
保存→回復のサイクルを検証可能にする。

**なぜ `StoredInteraction` を `types.rs` に定義するのか:**

`MetadataStore` トレイト（`store/metadata_store.rs`）と `HumanChannel` トレイト
（`human_channel.rs`）の両方から参照されるため。`types.rs` に置くことで
循環依存を回避し、既存パターン（SearchTrace 等）と一貫する。

## Boy Scout Rule — 翻訳可能性計画

### 新規作成コード（`src/human_channel.rs`）

- 関数名は動詞句: `notify`, `communicate`, `reconnect`, `wait`, `expect_response`
- 変数名はドメイン概念: `request`, `handle`, `outcome`, `decision`, `interaction_id`, `revised_body`
- 一関数一責務。ハードコード値の定数化。

### 編集する既存コード

- `src/error.rs`: `HumanChannelIo(String)` + `HumanChannelClosed` 追加
- `src/types.rs`: `HumanRequest`, `HumanOutcome`, `HumanResponse`, `HumanDecision`, `StoredInteraction`, `InteractionStatus` 追加（serde derive 付き）
- `src/store/metadata_store.rs`: 4 メソッド追加 + `InMemoryMetadataStore` に `HashMap<String, StoredInteraction>` + 実装 + テスト（T10）
- `src/lib.rs`: `pub mod human_channel;` + `pub use` で `HumanChannel` / `InteractionHandle` / `FakeHumanChannel` / `StdinoutChannel` を公開。`InteractionRecord` は非公開。
- `Cargo.toml`: `cargo add uuid@1 --features v4`

### RFC 交叉参照

- 事前: §13A（Training Orchestrator）, §13B（Communication Patterns）, §16A.1（HumanReviewQueue）
- 事後: 実装完了後、§13A 規範要件6項目とトレイトシグネチャの完全一致を確認

## Acceptance Criteria

### トレイト設計
- [ ] `HumanChannel` が `notify()` / `communicate()` / `reconnect(id, &request)` を定義
- [ ] `InteractionHandle` が `interaction_id()` / `wait(timeout)` を提供
- [ ] `InteractionHandle` は `pub struct`。`interaction_id` フィールドのみ `pub(crate)`。
- [ ] `wait()` の timeout 引数が `Option<Duration>`。`None` で無制限待機
- [ ] `TimedOut` が `HumanOutcome` の一値でありエラーではない
- [ ] `HumanResponse` に `revised_body: Option<String>` が含まれる
- [ ] 全チャネル実装が `reconnect()` を提供
- [ ] `StoredInteraction` が `types.rs` に定義され、`InteractionStatus` を持つ
- [ ] `MetadataStore` に 4 メソッド（store / load / list_pending / resolve）が追加されている
- [ ] `InMemoryMetadataStore` が上記 4 メソッドを実装している
- [ ] `FakeHumanChannel` が `export_interactions()` を提供する
- [ ] トレイトオブジェクト安全性（`Box<dyn HumanChannel>`）
- [ ] `DarviumError::HumanChannelIo(String)` + `HumanChannelClosed` が追加
- [ ] 空キュー `communicate()` が panic

### テスト充足（全 54 テスト）
- [ ] T1（2）/ T2（3）/ T3（6）/ T4（5）/ T5（3）/ T6（12）/ T7（5）/ T8（3）/ T9（7）/ T10（8）全て通過
- [ ] OTS-1（n=10,000, σ²=0）/ OTS-2（n=8,192 ラウンドトリップ, 3型網羅）通過

### 品質
- [ ] 既存テストが全て通過
- [ ] RFC §13A 規範要件6項目とトレイトシグネチャの完全一致を確認
- [ ] T10-7（Pending 生存: 手動構築 → store → list_pending → reconnect → wait → resolve 完全サイクル）成立
- [ ] T10-8（Resolved 生存: Fake → export → store → load → reconnect → wait → resolve 完全サイクル）成立
- [ ] 翻訳可能性検証通過
