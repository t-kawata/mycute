use tower_http::cors::{AllowOrigin, CorsLayer};
use axum::http::{self, Method, header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, ORIGIN}};
use std::time::Duration;

pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        // AllowOrigins: []string{"*"} かつ AllowCredentials: true を実現するため
        // リクエストの Origin ヘッダーをそのままミラーして許可する設定にします
        .allow_origin(AllowOrigin::mirror_request()) 
        
        // AllowMethods: []string{"GET", "POST", "PUT", "PATCH", "DELETE"}
        .allow_methods([
            Method::GET, 
            Method::POST, 
            Method::PUT, 
            Method::PATCH, 
            Method::DELETE
        ])
        
        // AllowHeaders: []string{"Origin", "Content-Type", "Authorization", "X-Key"}
        .allow_headers([
            ORIGIN, 
            CONTENT_TYPE, 
            AUTHORIZATION, 
            "X-Key".parse::<http::HeaderName>().unwrap()
        ])
        
        // ExposeHeaders: []string{"Content-Length"}
        .expose_headers([CONTENT_LENGTH])
        
        // AllowCredentials: true
        .allow_credentials(true)
        
        // MaxAge: 12 * time.Hour
        .max_age(Duration::from_secs(12 * 3600))
}
