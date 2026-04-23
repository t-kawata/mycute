use std::sync::Arc;
use reqwest::Client;
use serde_json::Value;
use crate::constants::IP_LOCALHOST;
use crate::{
    mode::rt::{
        rtreq::lmgws_req::{
            CreateLmgwProviderReq,
            UpdateLmgwProviderReq,
            UpdateLmgwConfigReq,
            UpdateLmgwProxyConfigReq,
            SearchLmgwModelsReq,
            SearchLmgwBaseModelsReq,
        },
        rtres::{
            lmgws_res::{
                GetLmgwConfigRes,
                UpdateLmgwConfigRes,
                LmgwProxyConfigRes,
                UpdateLmgwProxyConfigRes,
                SearchLmgwProvidersRes,
                GetLmgwProviderRes,
                LmgwProviderRes,
                CreateLmgwProviderRes,
                UpdateLmgwProviderRes,
                DeleteLmgwProviderRes,
                SearchLmgwKeysRes,
                LmgwKeyRes,
                SearchLmgwModelsRes,
                LmgwModelRes,
                SearchLmgwModelParametersRes,
                SearchLmgwBaseModelsRes,
            },
            errs_res::ApiError,
        },
        rterr::rterr,
    },
    mycute_settings::ConfigManager,
};

/// Bifrost HTTP API への透過プロキシクライアント。
/// ConfigManager から生成した `lmgw_secret` を Bearer トークンとして全リクエストに付与することで、
/// Bifrost の認証機能（BIFROST_AUTH_SECRET）との透過的な認証を実現する。
pub struct BifrostClient {
    hc: Arc<Client>,
    config_manager: Arc<ConfigManager>,
}

impl BifrostClient {
    pub fn new(hc: Arc<Client>, config_manager: Arc<ConfigManager>) -> Self {
        Self { hc, config_manager }
    }

    /// Bifrost のベース URL を組み立てる。
    /// ポートは ConfigManager の設定値から動的に取得するため、設定変更に追従できる。
    fn get_base_url(&self) -> String {
        let port = self.config_manager.settings.read().server.bifrost_port;
        format!("http://{}:{}", IP_LOCALHOST, port)
    }

    /// LMGW シークレットを取得する。
    /// 起動時に生成・設定されているはずだが、未設定の場合は空文字列で代替する（エラー回避）。
    fn get_secret(&self) -> String {
        self.config_manager.get_lmgw_secret().unwrap_or_default()
    }

    /// Bifrost からのエラーレスポンスを統一的な ApiError に変換する。
    /// ステータスコードとボディテキストをそのまま保持し、デバッグ時の可視性を確保する。
    async fn handle_error(&self, res: reqwest::Response) -> ApiError {
        let status = axum::http::StatusCode::from_u16(res.status().as_u16())
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        let body = res.text().await.unwrap_or_default();
        log::error!("<LMGW> Bifrost API error ({}): {}", status, body);
        ApiError::new_system(status, rterr::ERR_UNEXPECTED, format!("Bifrost API error: {}", body))
    }

    /// reqwest の送信エラーを ApiError に変換するヘルパー。
    fn map_send_err(e: reqwest::Error) -> ApiError {
        ApiError::new_system(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            rterr::ERR_UNEXPECTED,
            e.to_string(),
        )
    }

    /// reqwest の JSON デコードエラーを ApiError に変換するヘルパー。
    fn map_json_err(e: reqwest::Error) -> ApiError {
        ApiError::new_system(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            rterr::ERR_UNEXPECTED,
            format!("Failed to parse Bifrost response: {}", e),
        )
    }

    // ============================================================
    // Config (構成設定)
    // ============================================================

    /// Bifrost の現在の構成設定を取得する。
    /// 対応 Bifrost エンドポイント: GET /api/config
    pub async fn get_config(&self) -> Result<GetLmgwConfigRes, ApiError> {
        let url = format!("{}/api/config", self.get_base_url());
        let res = self.hc.get(&url)
            .header("Authorization", format!("Bearer {}", self.get_secret()))
            .send().await
            .map_err(Self::map_send_err)?;

        if !res.status().is_success() {
            return Err(self.handle_error(res).await);
        }
        log::debug!("<LMGW> get_config: success");
        res.json::<GetLmgwConfigRes>().await.map_err(Self::map_json_err)
    }

    /// Bifrost の構成設定を更新する。
    /// 対応 Bifrost エンドポイント: PUT /api/config
    pub async fn update_config(&self, req: UpdateLmgwConfigReq) -> Result<UpdateLmgwConfigRes, ApiError> {
        let url = format!("{}/api/config", self.get_base_url());
        let res = self.hc.put(&url)
            .header("Authorization", format!("Bearer {}", self.get_secret()))
            .json(&req)
            .send().await
            .map_err(Self::map_send_err)?;

        if !res.status().is_success() {
            return Err(self.handle_error(res).await);
        }
        log::debug!("<LMGW> update_config: success");
        // Bifrost は SuccessResponse を返すが、MYCUTEは独自のメッセージ構造体でラップする
        Ok(UpdateLmgwConfigRes { message: "Config updated successfully".to_string() })
    }

    // ============================================================
    // ProxyConfig (プロキシ設定)
    // ============================================================

    /// Bifrost のプロキシ設定を取得する。
    /// 対応 Bifrost エンドポイント: GET /api/proxy-config
    pub async fn get_proxy_config(&self) -> Result<LmgwProxyConfigRes, ApiError> {
        let url = format!("{}/api/proxy-config", self.get_base_url());
        let res = self.hc.get(&url)
            .header("Authorization", format!("Bearer {}", self.get_secret()))
            .send().await
            .map_err(Self::map_send_err)?;

        if !res.status().is_success() {
            return Err(self.handle_error(res).await);
        }
        log::debug!("<LMGW> get_proxy_config: success");
        res.json::<LmgwProxyConfigRes>().await.map_err(Self::map_json_err)
    }

    /// Bifrost のプロキシ設定を更新する。
    /// 対応 Bifrost エンドポイント: PUT /api/proxy-config
    pub async fn update_proxy_config(&self, req: UpdateLmgwProxyConfigReq) -> Result<UpdateLmgwProxyConfigRes, ApiError> {
        let url = format!("{}/api/proxy-config", self.get_base_url());
        let res = self.hc.put(&url)
            .header("Authorization", format!("Bearer {}", self.get_secret()))
            .json(&req)
            .send().await
            .map_err(Self::map_send_err)?;

        if !res.status().is_success() {
            return Err(self.handle_error(res).await);
        }
        log::debug!("<LMGW> update_proxy_config: success");
        Ok(UpdateLmgwProxyConfigRes { message: "Proxy config updated successfully".to_string() })
    }

    // ============================================================
    // Providers (プロバイダー管理)
    // ============================================================

    /// 全プロバイダーの一覧を取得する。
    /// 対応 Bifrost エンドポイント: GET /api/providers
    pub async fn search_providers(&self) -> Result<SearchLmgwProvidersRes, ApiError> {
        let url = format!("{}/api/providers", self.get_base_url());
        let res = self.hc.get(&url)
            .header("Authorization", format!("Bearer {}", self.get_secret()))
            .send().await
            .map_err(Self::map_send_err)?;

        if !res.status().is_success() {
            return Err(self.handle_error(res).await);
        }
        let providers: Vec<LmgwProviderRes> = res.json().await.map_err(Self::map_json_err)?;
        let total = providers.len();
        log::debug!("<LMGW> search_providers: {} providers found", total);
        Ok(SearchLmgwProvidersRes { providers, total })
    }

    /// 特定プロバイダーの設定を取得する。
    /// 対応 Bifrost エンドポイント: GET /api/providers/{provider}
    pub async fn get_provider(&self, name: &str) -> Result<GetLmgwProviderRes, ApiError> {
        let url = format!("{}/api/providers/{}", self.get_base_url(), name);
        let res = self.hc.get(&url)
            .header("Authorization", format!("Bearer {}", self.get_secret()))
            .send().await
            .map_err(Self::map_send_err)?;

        if !res.status().is_success() {
            return Err(self.handle_error(res).await);
        }
        let provider: LmgwProviderRes = res.json().await.map_err(Self::map_json_err)?;
        log::debug!("<LMGW> get_provider '{}': success", name);
        Ok(GetLmgwProviderRes { provider })
    }

    /// 新しいプロバイダーを追加する。
    /// 対応 Bifrost エンドポイント: POST /api/providers
    pub async fn create_provider(&self, req: CreateLmgwProviderReq) -> Result<CreateLmgwProviderRes, ApiError> {
        let url = format!("{}/api/providers", self.get_base_url());
        let res = self.hc.post(&url)
            .header("Authorization", format!("Bearer {}", self.get_secret()))
            .json(&req)
            .send().await
            .map_err(Self::map_send_err)?;

        if !res.status().is_success() {
            return Err(self.handle_error(res).await);
        }
        let provider: LmgwProviderRes = res.json().await.map_err(Self::map_json_err)?;
        log::debug!("<LMGW> create_provider '{}': success", provider.name);
        Ok(CreateLmgwProviderRes { provider })
    }

    /// プロバイダー設定を更新する（全フィールド上書き）。
    /// 対応 Bifrost エンドポイント: PUT /api/providers/{provider}
    /// 注意: Bifrost は部分更新をサポートしないため、必ず全フィールドを送ること。
    pub async fn update_provider(&self, name: &str, req: UpdateLmgwProviderReq) -> Result<UpdateLmgwProviderRes, ApiError> {
        let url = format!("{}/api/providers/{}", self.get_base_url(), name);
        let res = self.hc.put(&url)
            .header("Authorization", format!("Bearer {}", self.get_secret()))
            .json(&req)
            .send().await
            .map_err(Self::map_send_err)?;

        if !res.status().is_success() {
            return Err(self.handle_error(res).await);
        }
        let provider: LmgwProviderRes = res.json().await.map_err(Self::map_json_err)?;
        log::debug!("<LMGW> update_provider '{}': success", name);
        Ok(UpdateLmgwProviderRes { provider })
    }

    /// プロバイダーを削除する。
    /// 対応 Bifrost エンドポイント: DELETE /api/providers/{provider}
    pub async fn delete_provider(&self, name: &str) -> Result<DeleteLmgwProviderRes, ApiError> {
        let url = format!("{}/api/providers/{}", self.get_base_url(), name);
        let res = self.hc.delete(&url)
            .header("Authorization", format!("Bearer {}", self.get_secret()))
            .send().await
            .map_err(Self::map_send_err)?;

        if !res.status().is_success() {
            return Err(self.handle_error(res).await);
        }
        log::debug!("<LMGW> delete_provider '{}': success", name);
        Ok(DeleteLmgwProviderRes { message: format!("Provider '{}' deleted successfully", name) })
    }

    // ============================================================
    // Keys (API キー管理)
    // ============================================================

    /// 全プロバイダーのAPIキー一覧を取得する。
    /// 対応 Bifrost エンドポイント: GET /api/keys
    pub async fn search_keys(&self) -> Result<SearchLmgwKeysRes, ApiError> {
        let url = format!("{}/api/keys", self.get_base_url());
        let res = self.hc.get(&url)
            .header("Authorization", format!("Bearer {}", self.get_secret()))
            .send().await
            .map_err(Self::map_send_err)?;

        if !res.status().is_success() {
            return Err(self.handle_error(res).await);
        }
        let keys: Vec<LmgwKeyRes> = res.json().await.map_err(Self::map_json_err)?;
        let total = keys.len();
        log::debug!("<LMGW> search_keys: {} keys found", total);
        Ok(SearchLmgwKeysRes { keys, total })
    }

    // ============================================================
    // Models (モデル情報)
    // ============================================================

    /// 利用可能なモデルの一覧を取得する。
    /// クライアントから受け取ったフィルターパラメータを Bifrost のクエリパラメータに変換して転送する。
    /// 対応 Bifrost エンドポイント: GET /api/models
    pub async fn search_models(&self, req: SearchLmgwModelsReq) -> Result<SearchLmgwModelsRes, ApiError> {
        let url = format!("{}/api/models", self.get_base_url());
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(q) = &req.query {
            query.push(("query", q.clone()));
        }
        if let Some(p) = &req.provider {
            query.push(("provider", p.clone()));
        }
        if let Some(l) = req.limit {
            query.push(("limit", l.to_string()));
        }

        let res = self.hc.get(&url)
            .header("Authorization", format!("Bearer {}", self.get_secret()))
            .query(&query)
            .send().await
            .map_err(Self::map_send_err)?;

        if !res.status().is_success() {
            return Err(self.handle_error(res).await);
        }
        // Bifrost は ListModelsResponse を返すが、内部の data 配列を取り出す
        let raw: Value = res.json().await.map_err(Self::map_json_err)?;
        let models_val = raw.get("data").cloned().unwrap_or(raw);
        let models: Vec<LmgwModelRes> = serde_json::from_value(models_val)
            .map_err(|e| ApiError::new_system(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_UNEXPECTED,
                format!("Failed to parse models list: {}", e),
            ))?;
        let total = models.len();
        log::debug!("<LMGW> search_models: {} models found", total);
        Ok(SearchLmgwModelsRes { models, total })
    }

    /// モデルパラメーター定義一覧を取得する。
    /// Bifrost は additionalProperties 形式で返すため、serde_json::Value のまま保持する。
    /// 対応 Bifrost エンドポイント: GET /api/models/parameters
    pub async fn search_model_parameters(&self) -> Result<SearchLmgwModelParametersRes, ApiError> {
        let url = format!("{}/api/models/parameters", self.get_base_url());
        let res = self.hc.get(&url)
            .header("Authorization", format!("Bearer {}", self.get_secret()))
            .send().await
            .map_err(Self::map_send_err)?;

        if !res.status().is_success() {
            return Err(self.handle_error(res).await);
        }
        let parameters: Value = res.json().await.map_err(Self::map_json_err)?;
        log::debug!("<LMGW> search_model_parameters: success");
        Ok(SearchLmgwModelParametersRes { parameters })
    }

    /// ベースモデルカタログ一覧を取得する。
    /// 対応 Bifrost エンドポイント: GET /api/models/base
    pub async fn search_base_models(&self, req: SearchLmgwBaseModelsReq) -> Result<SearchLmgwBaseModelsRes, ApiError> {
        let url = format!("{}/api/models/base", self.get_base_url());
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(q) = &req.query {
            query.push(("query", q.clone()));
        }
        if let Some(p) = &req.provider {
            query.push(("provider", p.clone()));
        }
        if let Some(l) = req.limit {
            query.push(("limit", l.to_string()));
        }

        let res = self.hc.get(&url)
            .header("Authorization", format!("Bearer {}", self.get_secret()))
            .query(&query)
            .send().await
            .map_err(Self::map_send_err)?;

        if !res.status().is_success() {
            return Err(self.handle_error(res).await);
        }
        let models: Value = res.json().await.map_err(Self::map_json_err)?;
        log::debug!("<LMGW> search_base_models: success");
        Ok(SearchLmgwBaseModelsRes { models })
    }
}
