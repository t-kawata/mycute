# CA 証明書（CA Cert）への改名および権限変更の計画

ユーザーより、CA トークンの登録処理におけるロール制限の変更と、システム外部向けの呼称を「CA Token」から「CA Cert」へ統一するよう指示をいただきました。これに基づき、関連するコードおよびインターフェースを修正します。

## ユーザーレビューが必要な項目

> [!IMPORTANT]
> **API 破壊的変更の確認**
> - **URLパスの変更**: `/v1/ca/token/register` が `/v1/ca/cert/register` に、`/v1/mycute/catoken/verify` が `/v1/mycute/cacert/verify` に変更されます。
> - **JSONフィールド名の変更**: リクエストボディの `ca_token` フィールドが `ca_cert` に変更されます。
> フロントエンドや外部クライアントの修正が必要になりますが、よろしいでしょうか。

## 提案される変更

### 1. 権限（ロール制限）の変更
- **対象**: `src/mode/rt/rthandler/ca_handler.rs` の `register_ca_token_ca` 関数。
- **変更内容**: `ju.allow_roles(&[JwtRole::APX])?` を `ju.allow_roles(&[JwtRole::USR])?` に変更します。

### 2. 呼称の統一 (CA Token -> CA Cert)

#### [MODIFY] [ca_handler.rs](file:///Users/kawata/shyme/mycute/src/mode/rt/rthandler/ca_handler.rs)
- 関数名を `register_ca_token_ca` から `register_ca_cert_ca` へ変更。
- `utoipa` の `path`, `summary`, `description` (特に `REGISTER_CA_TOKEN_DESC`) を更新し、名称や権限の説明を最新化。
- URLパスを `/ca/token/register` から `/ca/cert/register` へ変更。

#### [MODIFY] [mycute_handler.rs](file:///Users/kawata/shyme/mycute/src/mode/rt/rthandler/mycute_handler.rs)
- 関数名を `verify_ca_token` から `verify_ca_cert` へ変更。
- `utoipa` の `path`, `summary`, `description` を更新。
- URLパスを `/mycute/catoken/verify` から `/mycute/cacert/verify` へ変更。

#### [MODIFY] [ca_req.rs](file:///Users/kawata/shyme/mycute/src/mode/rt/rtreq/ca_req.rs) / [mycute_req.rs](file:///Users/kawata/shyme/mycute/src/mode/rt/rtreq/mycute_req.rs)
- 構造体名を `RegisterCaTokenReq` -> `RegisterCaCertReq` 等に変更。
- フィールド名を `ca_token` -> `ca_cert` へ変更。

#### [MODIFY] [ca_res.rs](file:///Users/kawata/shyme/mycute/src/mode/rt/rtres/ca_res.rs) / [mycute_res.rs](file:///Users/kawata/shyme/mycute/src/mode/rt/rtres/mycute_res.rs)
- 構造体名を `RegisterCaTokenRes` -> `RegisterCaCertRes` 等に変更。
- Swagger 用のドキュメント、サンプル値を更新。

#### [MODIFY] [ca_bl.rs](file:///Users/kawata/shyme/mycute/src/mode/rt/rtbl/ca_bl.rs) / [mycute_bl.rs](file:///Users/kawata/shyme/mycute/src/mode/rt/rtbl/mycute_bl.rs)
- エラーメッセージや成功メッセージ内の "CA Token" を "CA Cert" に置換。
- 成功時のメッセージを "CA Cert registered successfully. You are now authorized as a Central Authority (Trust Anchor)." 等、README.md の定義（中央認証局 / トラスト・アンカー）に即した内容に修正。

#### [MODIFY] [req_map.rs](file:///Users/kawata/shyme/mycute/src/mode/rt/req_map.rs)
- `routes!` 指定を新しい関数名（`register_ca_cert_ca`, `verify_ca_cert` 等）に更新。

## オープンな質問
- [ ] 成功メッセージの文言について、"You are now authorized as a Central Authority (Trust Anchor)." でよろしいでしょうか？（README.md の 2.2節、8.1節に基づいています）。

## 検証計画

### 自動テスト
- `make check-be` によるコンパイル確認。
- 改称後の新しいエンドポイントに対して `curl` を実行し、適切なレスポンス（および正しいロールでのアクセス可否）を確認します。

### 手動確認
- Swagger UI (`/swagger-ui`) を表示し、ドキュメントの記述とパラメータ名が正しく「CA Cert」に変わっていることを目視で確認します。
