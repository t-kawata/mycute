# 計画: M1-4 HITL 起動時回復ループ

## 要件
プロセス再起動後、MetadataStore 上に Pending 状態で残存する全 HITL インタラクションを
確実に回復する起動時回復ループを実装する。

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---|---|---|
| src/constants.rs | MODIFY | HITL_DEFAULT_TIMEOUT_SECS, HITL_RECONNECT_BACKOFF_SECS 追加 |
| src/store/json_metadata_store.rs | CREATE | JsonMetadataStore（ファイル永続化、原子書き込み） |
| src/store/mod.rs | MODIFY | JsonMetadataStore を pub use でエクスポート |
| src/recovery.rs | CREATE | recover_pending_interactions() 回復ループ関数 |
| src/lib.rs | MODIFY | recovery モジュール登録 + JsonMetadataStore 公開API |

## テスト計画
### 不変条件テスト（全 11 テスト）
1-2: JsonMetadataStore 基本動作・原子書き込み
3-8: 単一回復・N≥10・混合・Stdinout クロス・TimedOut・競合状態
9: 初回起動時ファイル不在 → 空状態で正常動作
10: ファイル破損（不正 JSON）→ Err 検出、クラッシュしない
11: 異種チャネル差し替え（Fake → Stdinout）

### 観測テスト（2 テスト）
OTS-1: バッチ回復成功率 N ∈ {1, 10, 100}
OTS-2: 回復レイテンシ分布（中央値・P90・P99）

## 実装手順
1. constants.rs → 2 定数追加
2. json_metadata_store.rs → 新規作成 + テスト
3. store/mod.rs → pub use 追加
4. recovery.rs → 回復ループ関数 + 全テスト
5. lib.rs → モジュール登録
6. cargo test → --nocapture → cargo clippy → cargo fmt

## 変更不可定数
HITL_RECONNECT_BACKOFF_SECS = 5.0 (Calibration Candidate)
HITL_DEFAULT_TIMEOUT_SECS = 3600 (Environment Policy Knob)
