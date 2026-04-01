# ライセンス管理の完全同期メカニズムの実装計画

ライセンス管理（登録・削除）において、言語設定等で使用されている「Event-driven Sync」メカニズムを導入し、複数ウィンドウ間でのリアルタイム同期と、アプリ起動時の整合性を確保します。

## ユーザーレビューが必要な項目

> [!IMPORTANT]
> - ライセンス一覧はデータサイズが大きくなる可能性があるため、イベントペイロードに含めるのではなく「ライセンスが変更されたこと」のみを通知し、フロントエンド側で再取得を行う設計も検討可能ですが、今回は整合性を重視し、既存の LLM 設定等と同様に「最新のリストをイベントで送信する」方式を採用します。

## 提案される変更

### [Component] バックエンド (Rust) - イベント定義と発行

#### [MODIFY] [src/constants.rs](file:///Users/kawata/shyme/mycute/src/constants.rs)
- `EVENT_APP_LICENSES_CHANGED` 定数を追加し、フロントエンドと共有します。

#### [MODIFY] [src/types.rs](file:///Users/kawata/shyme/mycute/src/types.rs)
- `TauriEvent` 列挙型に `AppLicensesChanged` を追加。
- `EventKind` 列挙型に `LicensesChanged(Vec<LicenseSummary>)` を追加。

#### [MODIFY] [src/mode/rt/rthandler/mycute_handler.rs](file:///Users/kawata/shyme/mycute/src/mode/rt/rthandler/mycute_handler.rs)
- `register_license` および `unregister_license` ハンドラー内において、処理成功後に最新のライセンス一覧を `license_bl::list_licenses` で取得し、`InternalEvent` をブロードキャストする処理を追加します。

---

### [Component] フロントエンド (Vue/Pinia) - 同期と初期化

#### [MODIFY] [web/src/App.vue](file:///Users/kawata/shyme/mycute/web/src/App.vue)
- `initApp` 関数内で `mainStore.fetchLicenses()` を呼び出し、アプリ起動時に最新の状態を確保します。
- `EVENT_APP_LICENSES_CHANGED` イベントのリスナーを追加し、受信時に `mainStore.setLicenses` を実行して状態を同期します。

#### [MODIFY] [web/src/stores/main-store.ts](file:///Users/kawata/shyme/mycute/web/src/stores/main-store.ts)
- 必要に応じて、イベント経由での更新を許容するようにアクションを微調整します（現状の `setLicenses` で対応可能です）。

---

### [Component] 自動生成資産の更新

#### [RUN] `make gen-entities` (または定数生成コマンド)
- Rust 側で追加した定数を `web/src/consts/generated_constants.ts` へ反映させます。

## オープンな質問

- **パフォーマンスについての懸念**: ライセンス数が多い場合、毎回全リストをブロードキャストするのは非効率ですが、一般的な利用シーン（数件〜数十件程度）であれば問題ないと判断して良いでしょうか？

## 検証プラン

### 自動テスト
- `make check-fe` および `make check-be` を実行し、コンパイルエラーがないことを確認します。

### 手動確認
1. アプリを起動し、設定画面のライセンス一覧が正しく表示されることを確認。
2. ライセンスを登録/削除し、その結果が即座に UI に反映されることを確認。
3. (マルチウィンドウ環境が再現可能な場合) 別のウィンドウでも変更が即座に反映されることを確認。