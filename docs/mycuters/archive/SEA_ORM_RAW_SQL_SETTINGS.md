SeaORMでプレースホルダー（`?`）ではなく、実際の値が埋め込まれた実行SQLを出力するには、SeaORMの**`debug-print`**機能を使用します 。現在表示されているログはSeaORMの基盤である`sqlx`によるもので、`sqlx`は仕様上プレースホルダー形式でしか出力しません 。[1][2][3]

以下の3ステップで、値が埋め込まれたデバッグログを出力できます。

### 1. `Cargo.toml`の設定
`sea-orm`の依存関係に`debug-print`機能を追加します 。[4][1]

```toml
[dependencies]
sea-orm = { version = "1.1", features = ["debug-print", "runtime-tokio-rustls", "sqlx-mysql"] } # 使用するDBに合わせて変更
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### 2. 接続オプションの設定
SQLx自体のログ（`?`付きのログ）を無効化し、SeaORM側のログのみが表示されるように設定します 。[5][1]

```rust
use sea_orm::{ConnectOptions, Database};
use std::time::Duration;

let mut opt = ConnectOptions::new("protocol://username:password@host/database".to_owned());
opt.sqlx_logging(false); // SQLx側のプレースホルダーログを無効化 [web:28]

let db = Database::connect(opt).await?;
```

### 3. `tracing-subscriber`の初期化
`main`関数の冒頭などで、ログ出力を有効にします 。レベルを`DEBUG`に設定することでSQLが表示されます。[1][5]

```rust
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    // ... DB操作
}
```

### 補足：手動でSQLを確認する方法
特定のクエリのみを確認したい場合は、実行前に`.build()`と`.to_string()`を呼び出すことで、実際のSQL文字列を取得できます 。[6][7]

| 方法 | 用途 | 特徴 |
| :--- | :--- | :--- |
| **`debug-print`機能** | 開発中の全体的なデバッグ | パラメータが自動的に挿入された状態でログ出力される [1] |
| **`to_string()`** | 特定のクエリの調査 | プログラム内で文字列として取得し、`println!`などで確認可能 [6] |

実行時のログには `sea_orm::driver::...` というプレフィックスが付き、`SELECT ... WHERE id = 101` のような形式で出力されるようになります 。[3][1]

[1](https://www.sea-ql.org/SeaORM/docs/install-and-config/debug-log/)
[2](https://users.rust-lang.org/t/how-to-log-parameters-instead-of-placeholders-using-sqlx-with-postgres/86741)
[3](https://www.sea-ql.org/SeaORM/docs/0.7.x/install-and-config/debug-log/)
[4](https://www.sea-ql.org/SeaORM-X/docs/install-and-config/database-and-async-runtime/)
[5](https://www.sea-ql.org/SeaORM/docs/0.11.x/install-and-config/debug-log/)
[6](https://www.sea-ql.org/SeaORM/docs/0.12.x/basic-crud/raw-sql/)
[7](https://www.sea-ql.org/SeaORM/docs/advanced-query/conditional-expression/)
[8](https://zenn.dev/masa0317/articles/a4441b2241e5ee)
[9](https://docs.rs/sea-orm/latest/sea_orm/enum.Value.html)
[10](https://learn.microsoft.com/en-us/answers/questions/666780/placeholder-showing-parameter-values-without-showi)
[11](https://docs.rs/sea-orm-tracing/latest/sea_orm_tracing/)
[12](https://github.com/SeaQL/sea-orm)
[13](https://forums.percona.com/t/pg-stat-monitor-see-placeholders-instead-of-actual-query/8152)
[14](https://www.sea-ql.org/SeaORM/docs/0.8.x/install-and-config/debug-log/)
[15](https://www.sea-ql.org/SeaORM/docs/0.7.x/generate-entity/sea-orm-cli/)
[16](https://www.sea-ql.org/SeaORM/docs/0.4.x/basic-crud/raw-sql/)
[17](https://www.reddit.com/r/rust/comments/13d9ayi/what_orm_do_you_use/)
[18](https://syu-m-5151.hatenablog.com/entry/2025/12/24/110101)
[19](https://docs.rs/sea-orm/latest/sea_orm/struct.Statement.html)
[20](https://stackoverflow.com/questions/76494597/how-to-break-sea-orm-query-using-multiline-statement)
[21](https://rightcode.co.jp/blogs/50883)
[22](https://www.reddit.com/r/rust/comments/1act34t/sql_in_rust_raw_prepared_statements_or/)
[23](https://qiita.com/isaka1022/items/4b37481ec216e2fbf507)
[24](https://www.sea-ql.org/SeaORM/docs/0.4.x/install-and-config/debug-log/)
[25](https://docs.rs/sea-orm)
[26](https://github.com/SeaQL/sea-orm/discussions/291)
[27](https://www.muehlencord.de/wordpress/2023/08/30/log-queries-with-parameters-in-hibernate-5-and-6/)
[28](https://www.sea-ql.org/SeaORM/docs/basic-crud/raw-sql/)
[29](https://docs.rs/sea-orm/latest/sea_orm/struct.ConnectOptions.html)
[30](https://github.com/SeaQL/sea-orm/blob/master/CHANGELOG.md)
[31](https://github.com/SeaQL/sea-orm/issues/888)
[32](https://www.sea-ql.org/SeaORM/docs/install-and-config/connection/)
[33](https://cprimozic.net/notes/posts/trying-out-sea-orm/)
[34](https://zenn.dev/collabostyle/articles/0641d73f776d80)
[35](https://dev.to/seaql/seaorm-20-new-entity-format-5e3g)
[36](https://www.sea-ql.org/SeaORM-X/docs/install-and-config/connection/)
[37](https://stackoverflow.com/questions/55899/how-to-see-the-actual-oracle-sql-statement-that-is-being-executed)