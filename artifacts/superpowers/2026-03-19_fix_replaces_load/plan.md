# 実装計画: ConfigManager (Live) 移行時の辞書データ（replaces）のインスタンス引き継ぎ

## ゴール
ユーザーからのご指摘の通り、DBへの不要な二重アクセス（リロード）を避け、`ConfigManager::new_bootstrap()` 時代に `init_client_db()` 内で既にロード済みとなっている `replaces` と `replaces_active_ids` のデータを、新たに生成される `ConfigManager::new_live()` インスタンスへ直接・安全に引き継ぎます。
これにより、v2.3.1で発生している「STT置換辞書が効かない」不具合を、無駄なDBクエリを発行することなく根本解決します。

## プロセス概要
`main_of_cl.rs` において、`ConfigManager::new_live()` が呼ばれる二箇所（バックエンドサーバーモード用、およびメインGUIプロセス用）で、新インスタンスの `replaces` および `replaces_active_ids` に旧インスタンスの `Arc<RwLock<_>>` 参照をクローンして引き継ぎます。

---

## 変更箇所と詳細 (Surgical Diff)

### 1. `src/mode/cl/main_of_cl.rs` の修正 (バックエンドサーバーモード側)
#### [MODIFY] src/mode/cl/main_of_cl.rs(file:///Users/kawata/shyme/mycute/src/mode/cl/main_of_cl.rs)
- 現状 (L143 付近):
  ```rust
  let config_mgr_live = Arc::new(ConfigManager::new_live(db_pools));
  ```
- 変更後:
  ```rust
  let mut live = ConfigManager::new_live(db_pools);
  // Boot用(config_mgr)で既にロード済みの辞書データを引き継ぐ (DB二重アクセス防止)
  live.replaces = config_mgr.replaces.clone();
  live.replaces_active_ids = config_mgr.replaces_active_ids.clone();
  let config_mgr_live = Arc::new(live);
  ```

### 2. `src/mode/cl/main_of_cl.rs` の修正 (メインGUIプロセス側)
#### [MODIFY] src/mode/cl/main_of_cl.rs(file:///Users/kawata/shyme/mycute/src/mode/cl/main_of_cl.rs)
- 現状 (L294 付近):
  ```rust
  let config_mgr = Arc::new(ConfigManager::new_live(db_pools.clone()));
  ```
- 変更後:
  ```rust
  let mut live = ConfigManager::new_live(db_pools.clone());
  // Boot用(config_mgr)で既にロード済みの辞書データを引き継ぐ (DB二重アクセス防止)
  live.replaces = config_mgr.replaces.clone();
  live.replaces_active_ids = config_mgr.replaces_active_ids.clone();
  let config_mgr = Arc::new(live);
  ```

## 期待される効果と検証方法
- **効果**: `ConfigManager::new_live()` により破棄されていた辞書データが保持され、`SpeechRecognizer::new` に渡されるため、STT置換（replaces）が再び正常に動作するようになります。また、無駄なローカルDBへの再クエリが発生しません。
- **検証**:
  1. `make check-be` でコンパイルエラーが無いことを確認します。
  2. 実際にアプリを起動し、音声入力時に辞書が正しく適用されるかの確認をお願いします。

## User Review Required
上記の方針でよろしければ、`/superpowers-execute-plan` または承認の旨をお知らせください。