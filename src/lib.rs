// 共通の更新処理マクロ（sea-orm-cli による上書きを避けるため lib.rs に配置）

// Trait to handle different Timestamp types (NaiveDateTime vs DateTime<Utc>)
// and different nullability (Option vs value).
pub trait CurrentTimestamp {
    fn current() -> Self;
}

// NaiveDateTime assumes UTC for DB models.
impl CurrentTimestamp for chrono::NaiveDateTime {
    fn current() -> Self {
        time::now()
    }
}

// DateTime<Utc> can also be used.
impl CurrentTimestamp for chrono::DateTime<chrono::Utc> {
    fn current() -> Self {
        time::now_utc()
    }
}

impl CurrentTimestamp for Option<chrono::NaiveDateTime> {
    fn current() -> Self {
        Some(time::now())
    }
}

impl CurrentTimestamp for Option<chrono::DateTime<chrono::Utc>> {
    fn current() -> Self {
        Some(crate::utils::time::now_utc())
    }
}

#[macro_export]
macro_rules! impl_utc_timestamp_behavior {
    ($model:ident) => {
        #[async_trait::async_trait]
        impl sea_orm::ActiveModelBehavior for $model {
            async fn before_save<C>(
                mut self,
                _db: &C,
                _insert: bool,
            ) -> Result<Self, sea_orm::DbErr>
            where
                C: sea_orm::ConnectionTrait,
            {
                use sea_orm::ActiveValue::Set;

                // Polymorphically get the current timestamp (UTC).
                self.updated_at = Set($crate::CurrentTimestamp::current());
                Ok(self)
            }
        }
    };
}

pub mod config;
pub mod enums;

#[cfg(target_os = "macos")]
pub mod hotkey_mac;
#[cfg(target_os = "macos")]
pub use hotkey_mac as hotkey;

#[cfg(target_os = "windows")]
pub mod hotkey_win;
#[cfg(target_os = "windows")]
pub use hotkey_win as hotkey;

pub mod constants;
pub mod cuber;
pub mod entities;
pub mod input;
pub mod llm;
pub mod migration;
pub mod mode;
pub mod mycute_manager;
pub mod myproxy;
pub mod stt;
pub mod stt_config;
pub mod tauri_cmd;
pub mod tools;
pub mod types;
pub mod utils;
pub mod vo;
use crate::utils::time;
