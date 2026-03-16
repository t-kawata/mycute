use crate::mycute_settings::Settings;

pub enum CryptoTarget {
    /// DBカラム (SeaORMのColumnTraitを利用して動的にクエリ構築)
    Db {
        table_name: &'static str,
        col_name: &'static str,
        pk_col: &'static str, // 更新用PK
    },
    /// 設定ファイルのフィールド (アクセッサ経由で更新)
    Config {
        path: &'static str, // ログ用
        // 設定構造体への可変参照を返すクロージャ
        // Send + Sync はスレッド間共有のために必須
        getter: Box<dyn Fn(&mut Settings) -> &mut Option<String> + Send + Sync>,
    },
}

// システム上の全暗号化データをここで定義。漏れがあれば即事故に繋がる。
pub fn get_registry() -> Vec<CryptoTarget> {
    vec![
        // 1. DB: 汎用暗号化ストレージ (cryptos table)
        // cryptos table: id(PK), key, value(Encrypted), ...
        CryptoTarget::Db {
            table_name: "cryptos",
            col_name: "value",
            pk_col: "id",
        },
        // 2. Config: 自身のアイデンティティ (Node Identity)
        // settings.json: my_pub (Encrypted)
        CryptoTarget::Config {
            path: "settings.my_pub",
            getter: Box::new(|s| &mut s.my_pub),
        },
        // settings.json: my_sec (Encrypted)
        CryptoTarget::Config {
            path: "settings.my_sec",
            getter: Box::new(|s| &mut s.my_sec),
        },
    ]
}
