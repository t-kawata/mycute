use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Usr::Table)
                    .if_not_exists()
                    .col(pk_auto(Usr::Id))
                    .col(string_len(Usr::Name, 50).not_null().default("")) // Name: 50文字, not null
                    .col(tiny_unsigned(Usr::Type).not_null().default(0)) // 1: 法人, 2: 個人
                    .col(unsigned(Usr::Points).not_null().default(0)) // 現在の保有ポイント
                    .col(unsigned(Usr::SumP).not_null().default(0)) // 現金変換したポイントの累積値
                    .col(unsigned(Usr::SumC).not_null().default(0)) // 現金変換した現金の累積値
                    // --------- シンプル認証用 bgn
                    .col(string_len(Usr::Email, 100).not_null().default("")) // ログインID (ZITADEL連携時も使用)
                    .col(string_len(Usr::Password, 255).not_null().default("")) // パスワードハッシュ
                    // --------- シンプル認証用 end
                    // --------- ZITADEL連携用 bgn
                    .col(string_len(Usr::ZitadelID, 100).null()) // ZITADELのsub
                    .col(boolean(Usr::EmailVerified).not_null().default(false)) // メール検証済みフラグ
                    // --------- ZITADEL連携用 end
                    // --------- 法人だけの項目 bgn
                    .col(unsigned(Usr::FlushDays).not_null().default(0)) // 現金分配実行するためのサイクル
                    .col(unsigned(Usr::Badged).not_null().default(0)) // 授与した Badge の累積数
                    .col(ColumnDef::new(Usr::Rate).decimal_len(5, 5).not_null().default(0.0)) // 付与する割増ポイント率
                    // --------- 法人だけの項目 end
                    // --------- VDR だけの項目 bgn
                    .col(unsigned(Usr::TotalBadged).not_null().default(0)) // Vdr内のBadgedの合計
                    .col(unsigned(Usr::TotalBadges).not_null().default(0)) // Vdr内のバッジ保有総数
                    .col(unsigned(Usr::BasePoint).not_null().default(0)) // 付与される基本ポイント数
                    .col(ColumnDef::new(Usr::BelongRate).decimal_len(5, 5).not_null().default(0.0)) // 所属によるポイント割増率
                    .col(unsigned(Usr::MaxWorks).not_null().default(0)) // 個人が就労できる最大数
                    .col(ColumnDef::new(Usr::FlushFeeRate).decimal_len(5, 5).not_null().default(0.0)) // Pool から引かれる割合
                    // --------- VDR だけの項目 end
                    .col(boolean(Usr::IsStaff).not_null().default(false))
                    .col(ColumnDef::new(Usr::BgnAt).date_time().not_null())
                    .col(ColumnDef::new(Usr::EndAt).date_time().not_null())
                    .col(unsigned(Usr::ApxID).null())
                    .col(unsigned(Usr::VdrID).null())
                    .col(ColumnDef::new(Usr::CreatedAt).date_time().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(Usr::UpdatedAt).date_time().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // 複合ユニークインデックス (Email, ApxID, VdrID)
        manager
            .create_index(
                Index::create()
                    .name("usr_apxid_vdrid_email_unique")
                    .table(Usr::Table)
                    .col(Usr::ApxID)
                    .col(Usr::VdrID)
                    .col(Usr::Email)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Usr::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Usr {
    #[sea_orm(iden = "usrs")]
    Table,
    Id,
    Name,
    Type,
    Points,
    SumP,
    SumC,
    Email,
    Password,
    ZitadelID,
    EmailVerified,
    FlushDays,
    Badged,
    Rate,
    TotalBadged,
    TotalBadges,
    BasePoint,
    BelongRate,
    MaxWorks,
    FlushFeeRate,
    #[sea_orm(iden = "is_staff")]
    IsStaff,
    BgnAt,
    EndAt,
    ApxID,
    VdrID,
    CreatedAt,
    UpdatedAt,
}
