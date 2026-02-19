pub use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260104_092035_create_bds_tbl::Migration),
            Box::new(m20260104_092136_create_usrs_tbl::Migration),
            Box::new(m20260107_050440_create_cryptos_tbl::Migration),
            Box::new(m20260107_050440_create_jobs_tbl::Migration),
            Box::new(m20260107_050440_create_matches_tbl::Migration),
            Box::new(m20260107_050440_create_match_statuses_tbl::Migration),
            Box::new(m20260107_050440_create_works_tbl::Migration),
            Box::new(m20260107_050440_create_belongs_tbl::Migration),
            Box::new(m20260107_050440_create_badges_tbl::Migration),
            Box::new(m20260107_050440_create_usr_badges_tbl::Migration),
            Box::new(m20260107_050440_create_points_tbl::Migration),
            Box::new(m20260107_050440_create_payments_tbl::Migration),
            Box::new(m20260107_050440_create_pools_tbl::Migration),
            Box::new(m20260107_050440_create_flushes_tbl::Migration),
            Box::new(m20260107_050440_create_payouts_tbl::Migration),
            Box::new(m20260108_063735_create_chat_models_tbl::Migration),
            Box::new(m20260108_063739_create_cubes_tbl::Migration),
            Box::new(m20260108_064424_create_cube_model_stats_tbl::Migration),
            Box::new(m20260108_064424_create_cube_contributors_tbl::Migration),
            Box::new(m20260108_064424_create_cube_lineages_tbl::Migration),
            Box::new(m20260108_064424_create_exports_tbl::Migration),
            Box::new(m20260108_064424_create_burned_keys_tbl::Migration),
            Box::new(m20260203_061306_create_identities_tbl::Migration),
            Box::new(m20260203_061358_create_apps_tbl::Migration),
            Box::new(m20260207_125623_create_tickets_table::Migration),
            Box::new(m20260209_110000_create_verifications_tbl::Migration),
            Box::new(m20260211_000000_create_blacklists_tbl::Migration),
            Box::new(m20260213_095700_create_forums_table::Migration),
            Box::new(m20260213_170000_create_ca_vote_allocated_summaries::Migration),
            Box::new(m20260216_160000_create_ca_vote_item_summaries::Migration),
        ]
    }
}

mod m20260104_092035_create_bds_tbl;
mod m20260104_092136_create_usrs_tbl;
mod m20260107_050440_create_cryptos_tbl;
mod m20260107_050440_create_jobs_tbl;
mod m20260107_050440_create_matches_tbl;
mod m20260107_050440_create_match_statuses_tbl;
mod m20260107_050440_create_works_tbl;
mod m20260107_050440_create_belongs_tbl;
mod m20260107_050440_create_badges_tbl;
mod m20260107_050440_create_usr_badges_tbl;
mod m20260107_050440_create_points_tbl;
mod m20260107_050440_create_payments_tbl;
mod m20260107_050440_create_pools_tbl;
mod m20260107_050440_create_flushes_tbl;
mod m20260107_050440_create_payouts_tbl;
mod m20260108_063735_create_chat_models_tbl;
mod m20260108_063739_create_cubes_tbl;
mod m20260108_064424_create_cube_model_stats_tbl;
mod m20260108_064424_create_cube_contributors_tbl;
mod m20260108_064424_create_cube_lineages_tbl;
mod m20260108_064424_create_exports_tbl;
mod m20260108_064424_create_burned_keys_tbl;
mod m20260203_061306_create_identities_tbl;
mod m20260203_061358_create_apps_tbl;
mod m20260207_125623_create_tickets_table;
mod m20260209_110000_create_verifications_tbl;
mod m20260211_000000_create_blacklists_tbl;
mod m20260213_095700_create_forums_table;
mod m20260213_170000_create_ca_vote_allocated_summaries;
mod m20260216_160000_create_ca_vote_item_summaries;
