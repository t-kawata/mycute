use axum::http::{
    self,
    header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, ORIGIN},
    Method,
};
use std::time::Duration;
use tower_http::cors::{AllowOrigin, CorsLayer};

pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        // AllowOrigins: []string{"*"} かつ AllowCredentials: true を実現するため
        // リクエストの Origin ヘッダーをそのままミラーして許可する設定にします
        .allow_origin(AllowOrigin::mirror_request())
        // 許可するメソッド: GET, POST, PUT, PATCH, DELETE
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        // 許可するヘッダー: Origin, Content-Type, Authorization, X-BD
        .allow_headers([
            ORIGIN,
            CONTENT_TYPE,
            AUTHORIZATION,
            http::HeaderName::from_static("x-bd"),
            http::HeaderName::from_static("x-bifrost-routing"),
        ])
        // 公開するヘッダー: Content-Length
        .expose_headers([CONTENT_LENGTH])
        // クレデンシャル（Cookie等）の送信を許可
        // ------------------------------------
        .allow_credentials(true)
        // プリフライトリクエストのキャッシュ有効期間: 12時間
        .max_age(Duration::from_secs(12 * 3600))
}
