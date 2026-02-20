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
                    .col(
                        ColumnDef::new(Usr::Name)
                            .string_len(50)
                            .not_null()
                            .default(""),
                    ) // Name: 50文字, not null
                    .col(
                        ColumnDef::new(Usr::Type)
                            .tiny_unsigned()
                            .not_null()
                            .default(0),
                    ) // 1: 法人, 2: 個人
                    .col(ColumnDef::new(Usr::Points).unsigned().not_null().default(0)) // 現在の保有ポイント
                    .col(ColumnDef::new(Usr::SumP).unsigned().not_null().default(0)) // 現金変換したポイントの累積値
                    .col(ColumnDef::new(Usr::SumC).unsigned().not_null().default(0)) // 現金変換した現金の累積値
                    // --------- シンプル認証用 bgn
                    .col(
                        ColumnDef::new(Usr::Email)
                            .string_len(100)
                            .not_null()
                            .default(""),
                    ) // ログインID (ZITADEL連携時も使用)
                    .col(
                        ColumnDef::new(Usr::Password)
                            .string_len(255)
                            .not_null()
                            .default(""),
                    ) // パスワードハッシュ
                    // --------- シンプル認証用 end
                    // --------- ZITADEL連携用 bgn
                    .col(ColumnDef::new(Usr::ZitadelID).string_len(100).null()) // ZITADELのsub
                    .col(
                        ColumnDef::new(Usr::EmailVerified)
                            .boolean()
                            .not_null()
                            .default(false),
                    ) // メール検証済みフラグ
                    // --------- ZITADEL連携用 end
                    // --------- 法人だけの項目 bgn
                    .col(
                        ColumnDef::new(Usr::FlushDays)
                            .unsigned()
                            .not_null()
                            .default(0),
                    ) // 現金分配実行するためのサイクル
                    .col(ColumnDef::new(Usr::Badged).unsigned().not_null().default(0)) // 授与した Badge の累積数
                    .col(
                        ColumnDef::new(Usr::Rate)
                            .decimal_len(5, 5)
                            .not_null()
                            .default(0.0),
                    ) // 付与する割増ポイント率
                    // --------- 法人だけの項目 end
                    // --------- VDR だけの項目 bgn
                    .col(
                        ColumnDef::new(Usr::TotalBadged)
                            .unsigned()
                            .not_null()
                            .default(0),
                    ) // Vdr内のBadgedの合計
                    .col(
                        ColumnDef::new(Usr::TotalBadges)
                            .unsigned()
                            .not_null()
                            .default(0),
                    ) // Vdr内のバッジ保有総数
                    .col(
                        ColumnDef::new(Usr::BasePoint)
                            .unsigned()
                            .not_null()
                            .default(0),
                    ) // 付与される基本ポイント数
                    .col(
                        ColumnDef::new(Usr::BelongRate)
                            .decimal_len(5, 5)
                            .not_null()
                            .default(0.0),
                    ) // 所属によるポイント割増率
                    .col(
                        ColumnDef::new(Usr::MaxWorks)
                            .unsigned()
                            .not_null()
                            .default(0),
                    ) // 個人が就労できる最大数
                    .col(
                        ColumnDef::new(Usr::FlushFeeRate)
                            .decimal_len(5, 5)
                            .not_null()
                            .default(0.0),
                    ) // Pool から引かれる割合
                    // --------- VDR だけの項目 end
                    .col(
                        ColumnDef::new(Usr::IsStaff)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Usr::BgnAt).date_time().not_null())
                    .col(ColumnDef::new(Usr::EndAt).date_time().not_null())
                    .col(ColumnDef::new(Usr::ApxID).unsigned().null())
                    .col(ColumnDef::new(Usr::VdrID).unsigned().null())
                    .col(
                        ColumnDef::new(Usr::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Usr::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
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
