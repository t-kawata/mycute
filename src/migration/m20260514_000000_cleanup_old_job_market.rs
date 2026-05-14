use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // --------------------------------------------------
        // 1. 旧ジョブマーケットモデルのテーブルを削除
        // --------------------------------------------------
        let drop_tables = [
            "badges", "belongs", "burned_keys", "flushes", "jobs",
            "match_statuses", "matches", "payments", "payouts",
            "points", "pools", "usr_badges", "works",
        ];
        for table in &drop_tables {
            manager
                .drop_table(Table::drop().table(Alias::new(*table)).if_exists().to_owned())
                .await?;
        }

        // --------------------------------------------------
        // 2. usrs テーブルから旧カラムを削除
        // --------------------------------------------------
        let drop_columns = [
            "points", "sum_p", "sum_c", "flush_days", "badged",
            "rate", "total_badged", "total_badges", "base_point",
            "belong_rate", "max_works", "flush_fee_rate",
        ];
        for col in &drop_columns {
            manager
                .alter_table(
                    Table::alter()
                        .table(Alias::new("usrs"))
                        .drop_column(Alias::new(*col))
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ロールバック: usrs に旧カラムを復元
        let restore_columns: [(&str, &str, &str); 12] = [
            ("points", "integer", "not null default 0"),
            ("sum_p", "integer", "not null default 0"),
            ("sum_c", "integer", "not null default 0"),
            ("flush_days", "integer", "not null default 0"),
            ("badged", "integer", "not null default 0"),
            ("rate", "decimal(5,5)", "not null default 0"),
            ("total_badged", "integer", "not null default 0"),
            ("total_badges", "integer", "not null default 0"),
            ("base_point", "integer", "not null default 0"),
            ("belong_rate", "decimal(5,5)", "not null default 0"),
            ("max_works", "integer", "not null default 0"),
            ("flush_fee_rate", "decimal(5,5)", "not null default 0"),
        ];
        for (name, col_type, extra) in &restore_columns {
            let stmt = format!(
                "ALTER TABLE usrs ADD COLUMN {} {} {}",
                name, col_type, extra
            );
            manager.get_connection().execute_unprepared(&stmt).await?;
        }
        Ok(())
    }
}
