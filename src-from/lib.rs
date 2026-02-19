// 共通の更新処理マクロ（sea-orm-cli による上書きを避けるため lib.rs に配置）

// Trait to handle different Timestamp types (NaiveDateTime vs DateTime<Utc>)
// and different nullability (Option vs value).
pub trait CurrentTimestamp {
    fn current() -> Self;
}

impl CurrentTimestamp for chrono::NaiveDateTime {
    fn current() -> Self {
        chrono::Local::now().naive_local()
    }
}

// DateTime<Utc> is NOT used for DB models in JST mode.
// We implement it just to satisfy the trait bounds if accidentally used,
// but for DB entities we expect NaiveDateTime.
impl CurrentTimestamp for chrono::DateTime<chrono::Utc> {
    fn current() -> Self {
        chrono::Utc::now()
    }
}

impl CurrentTimestamp for Option<chrono::NaiveDateTime> {
    fn current() -> Self {
        Some(chrono::Local::now().naive_local())
    }
}

impl CurrentTimestamp for Option<chrono::DateTime<chrono::Utc>> {
    fn current() -> Self {
        Some(chrono::Utc::now())
    }
}

#[macro_export]
macro_rules! impl_utc_timestamp_behavior {
    ($model:ident) => {
        #[async_trait::async_trait]
        impl sea_orm::ActiveModelBehavior for $model {
            async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, sea_orm::DbErr>
            where
                C: sea_orm::ConnectionTrait,
            {
                use sea_orm::ActiveValue::Set;
                
                self.updated_at = Set($crate::CurrentTimestamp::current());
                Ok(self)
            }
        }
    };
}

pub mod config;
pub mod enums;
pub mod mode;
pub mod utils;
pub mod migration;
pub mod entities;
pub mod vo;
pub mod cuber;

