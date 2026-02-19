調査対象とした内容は以下の通りです。
```
「現在、Rust エコシステムにおける KuzuDB（LadybugDB の基礎）のバインディングは発展途上である。Go 版で実装された `MERGE` クエリや、FTS 拡張の動的ロード、ベクトルの Cosine Similarity 検索を安全かつ高性能に呼び出すための Rust ラッパー層を慎重に設計する必要がある。」

この情報の真偽を確かめてください。Rust版のバインディング（LadybugDB）を使用しなければならないが、今日時点での最新版Rustバインディングでもこのような発展途上な問題があるならばどのようにしてそれを解決してRustでLadybugDBを使用するかを真剣に考えなければならないのです。
```

調査結果は以下の通りです。


I'll now compile my research findings and generate the final report.

Based on my extensive research, I can now provide a comprehensive analysis of the current status of KuzuDB/LadybugDB Rust bindings and their extension support capabilities.

## KuzuDB/LadybugDB Rustバインディングの現状と拡張機能サポートに関する検証レポート

### エグゼクティブサマリー

ご質問の内容について真偽を検証した結果、**記載された情報は2025年3月時点では正確でしたが、2025年10月以降の状況変化により、部分的に解決されています**。KuzuDB v0.11.3（2025年10月リリース）およびLadybugDB（KuzuDBのフォーク、2025年11月リリース）において、Rustバインディングの拡張機能サポートは大幅に改善されました。[1][2][3]

### 背景：KuzuDBプロジェクトの状況変化

2025年10月10日、KuzuDB開発元のKùzu Inc.はGitHubリポジトリをアーカイブし、開発を停止しました。これを受けて、コミュニティ主導のフォークとしてLadybugDBが誕生し、MITライセンスの下でオープンソース開発が継続されています。[4][5][6][7][8][9]

### 検証結果：Rustバインディングにおける拡張機能サポートの真偽

#### 1. **MERGE クエリのサポート状況**

**結論：完全にサポートされています**

- KuzuDB/LadybugDBのCypherクエリ言語は`MERGE`句を完全サポートしています[10][11]
- Rustバインディング（`kuzu` crateおよび`lbug` crate）は、任意のCypherクエリを`Connection::query()`メソッド経由で実行可能です[12][13][14]
- `MERGE`クエリは小規模なグラフ更新に使用され、バルクインポートには`COPY FROM`が推奨されます[11]

**実装例（Rust）**：
```rust
use lbug::{Database, Connection, SystemConfig};

let db = Database::new("path/to/db", SystemConfig::default())?;
let conn = Connection::new(&db)?;

// MERGEクエリの実行
conn.query("
    MERGE (p:Person {name: 'Alice'})
    ON CREATE SET p.created = timestamp()
    ON MATCH SET p.updated = timestamp()
")?;
```

#### 2. **FTS（全文検索）拡張の動的ロード**

**結論：v0.11.3以降は事前バンドルされ、動的ロード不要**

**歴史的経緯と問題点**：
- **v0.8.x～v0.11.2まで**：Rustバインディングはデフォルトで静的ビルドを使用し、共有ライブラリベースの拡張機能と互換性がありませんでした[2]
- **Issue #5065**（2025年3月）で報告された問題：静的リンクされたRustバイナリが`libfts.kuzu_extension`のような共有拡張をロードしようとすると、`undefined symbol`エラーが発生[2]

**解決策（v0.11.3以降）**：
- **v0.11.3（2025年10月リリース）**で、4つの主要拡張（`algo`, `fts`, `json`, `vector`）が静的にバンドルされるようになりました[3][1]
- これらの拡張は`INSTALL`コマンド不要で即座に利用可能です
- PR #6043および#6044で「Rust static extensions」機能が実装され、静的リンク環境でも拡張機能が動作するようになりました[1]

**LadybugDB（v0.12.0以降）の状況**：
- 拡張機能サーバー（`extension.ladybugdb.com`）が稼働し、拡張機能テストが統合されています[9][15]
- FTSおよびVectorインデックスのバグフィックスが継続的に行われています[15]

#### 3. **Vector Cosine Similarity検索の呼び出し**

**結論：完全にサポートされ、高性能に最適化されています**

- **Vector拡張**はv0.11.3でプリインストールされています[3][1]
- HNSWベースのベクトルインデックスをサポートし、コサイン類似度検索が可能です[16][1]
- Rustバインディングから通常のCypherクエリとして実行できます：

```rust
// ベクトルインデックスの作成
conn.query("
    CALL CREATE_VECTOR_INDEX('MyTable', 'embedding', 
         {metric: 'cosine', dim: 384})
")?;

// コサイン類似度検索
let result = conn.query("
    CALL QUERY_VECTOR_INDEX('MyTable', 'embedding', 
         [0.1, 0.2, ...], 10)
    RETURN node.id, node.name, score
")?;
```

### Rustバインディングにおける拡張機能の使用方法（2026年1月現在）

#### 方法1：プリバンドル拡張の使用（推奨）

**LadybugDB v0.12.x / KuzuDB v0.11.3を使用する場合**：

```toml
[dependencies]
lbug = "0.12"  # LadybugDBの場合
# または
kuzu = "0.11.3"  # KuzuDB最終版の場合
```

プリバンドルされた拡張（`fts`, `vector`, `json`, `algo`）は追加設定不要で使用可能です。[1][3]

#### 方法2：動的拡張ロード（その他の拡張用）

動的拡張をロードする場合、バイナリが拡張機能と動的リンクできるよう`build.rs`に設定が必要です：[13][12]

```rust
// build.rs
fn main() {
    // Linuxの場合：拡張機能がバイナリのシンボルにアクセスできるようにする
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg=-rdynamic");
    
    // macOSの場合
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-arg=-Wl,-export_dynamic");
}
```

その後、拡張をロード：

```rust
conn.query("INSTALL postgres;")?;
conn.query("LOAD EXTENSION postgres;")?;
```

#### 方法3：共有ライブラリビルド（レガシーアプローチ）

環境変数を設定して共有ライブラリとしてビルド：

```bash
export LBUG_SHARED=1
export LBUG_LIBRARY_DIR=/path/to/prebuilt/lbug
export LBUG_INCLUDE_DIR=/path/to/prebuilt/lbug
cargo build
```

この方法は、v0.11.3以前のバージョンやプリバンドルされていない拡張を使用する場合に有効です。[12][2]

### 技術的詳細：なぜ問題が発生していたのか

1. **静的リンクと動的ロードの非互換性**：
   - Rustバインディングはデフォルトで静的ビルド（リンク時にすべてのシンボルを解決）
   - 動的拡張（`.so`/`.dylib`）は実行時にシンボルを解決する必要がある
   - 静的バイナリは通常、未使用シンボルをエクスポートしないため、拡張が必要なシンボル（例：`_ZTIN4kuzu8function17TableFuncBindDataE`）を見つけられない[2]

2. **解決策の実装**：
   - `-rdynamic`フラグにより、バイナリがすべてのシンボルをエクスポート
   - v0.11.3では主要拡張を静的リンクすることで、動的ロードの必要性を排除[1]

### 現在のRust APIの制約事項

#### サポートされている機能
- ✅ すべてのCypherクエリ（CREATE, MATCH, MERGE, DELETE, UPDATE等）
- ✅ プリペアドステートメント
- ✅ トランザクション管理
- ✅ Arrow形式でのクエリ結果取得
- ✅ プリバンドル拡張（FTS, Vector, JSON, Algo）
- ✅ CSV/Parquetからのデータインポート

#### 制約事項
- ⚠️ プリバンドルされていない拡張の動的ロードには`build.rs`設定が必要[13][12]
- ⚠️ Windows/MSVC以外の環境では、バイナリが拡張と互換性を持つよう明示的設定が必要
- ⚠️ クロスコンパイル時の拡張機能サポートは複雑[17]

### 推奨される実装戦略

#### 戦略1：LadybugDBへの移行（推奨）

**理由**：
- MITライセンスでオープンソース継続が保証されている[7][9]
- アクティブなコミュニティ開発とバグフィックス[9][15]
- KuzuDBとの高い互換性（パッケージ名変更のみで移行可能）[7]

**実装手順**：
```toml
[dependencies]
lbug = "0.12"
```

```rust
// import lbug; // Pythonの場合
use lbug::{Database, Connection, SystemConfig};

let db = Database::new("./my_graph.db", SystemConfig::default())?;
let conn = Connection::new(&db)?;

// FTS拡張の使用（プリバンドル済み）
conn.query("
    CALL CREATE_FTS_INDEX('Document', 'doc_index', ['title', 'content'])
")?;

let results = conn.query("
    CALL QUERY_FTS_INDEX('Document', 'doc_index', 'rust database')
    RETURN node.title, score
")?;

// Vector拡張の使用
conn.query("
    CALL CREATE_VECTOR_INDEX('Embedding', 'vec_idx', 
         {metric: 'cosine', dim: 768})
")?;

let vec_results = conn.query("
    CALL QUERY_VECTOR_INDEX('Embedding', 'vec_idx', $embedding, 5)
    RETURN node.id, score
")?;
```

#### 戦略2：拡張機能が不要な場合

プリバンドル拡張で要件が満たされる場合、追加設定不要でそのまま使用可能です。

#### 戦略3：カスタム拡張が必要な場合

1. `build.rs`で動的シンボルエクスポートを有効化
2. ローカル拡張サーバーをセットアップ
3. `INSTALL <extension> FROM 'http://localhost:8080/';`でインストール

```rust
// build.rs
fn main() {
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg=-rdynamic");
}
```

### パフォーマンス考慮事項

| 機能 | パフォーマンス特性 | 推奨使用ケース |
|------|-------------------|---------------|
| MERGE | 行単位処理、比較的遅い | 小規模更新（数千ノード以下）[11] |
| COPY FROM | バルク処理、非常に高速 | 大規模初期ロード（百万ノード以上）[11] |
| FTS検索 | BM25アルゴリズム、高速 | テキスト検索、ドキュメント検索[18] |
| Vector検索 | HNSW、サブ秒レスポンス | セマンティック検索、RAG[16] |

### 今後の展望

LadybugDBプロジェクトは以下を計画しています：[19][7][9]

1. **グラフデータレイク**：「グラフ版Snowflake」の実現
2. **No-Ingest アーキテクチャ**：ETLプロセスの排除
3. **標準化されたインターチェンジフォーマット**：Apache Graph Archiveとの連携
4. **オンライン、ゼロコピーグラフ処理**：スケーラビリティの大幅向上

### 結論と推奨事項

**ご質問への回答**：

> 「Rust版のバインディング（LadybugDB）を使用しなければならないが、今日時点での最新版Rustバインディングでもこのような発展途上な問題があるならば...」

**→ いいえ、問題は既に解決されています。**

1. **MERGE、FTS、Vector Cosine Similarityはすべて完全サポート**されており、安全かつ高性能に実行可能です[11][12][13][1]

2. **2026年1月現在の推奨アプローチ**：
   - LadybugDB v0.12.x（または KuzuDB v0.11.3）を使用
   - プリバンドル拡張（fts, vector, json, algo）をそのまま利用
   - カスタム拡張が必要な場合のみ`build.rs`設定を追加

3. **移行の容易性**：
   - KuzuDBからLadybugDBへの移行は、クレート名を`kuzu`から`lbug`に変更するだけ[7][9]
   - APIは完全互換性を維持

4. **長期的な安定性**：
   - オープンソースガバナンスにより継続的開発が保証[9][7]
   - GitHub Actions CI/CDにより、透明性の高い開発プロセス[9]

**実装を開始しても問題ありません。** 記載されていた「発展途上な問題」はv0.11.3のリリースで解決されており、現在のRustバインディングは本番環境での使用に十分成熟しています。

[1](https://github.com/kuzudb/kuzu/releases)
[2](https://github.com/kuzudb/kuzu/issues/5065)
[3](https://docs.rs/crate/kuzu/latest)
[4](https://www.linkedin.com/posts/year-of-the-graph_opensource-announcement-knowledgegraph-activity-7384825332786270208-G5hq)
[5](https://thedataquarry.com/blog/i-joined-lancedb)
[6](https://www.reddit.com/r/Database/comments/1o3w73z/status_of_kuzudb_from_kuzu_inc/)
[7](https://ladybugdb.com/faq.html)
[8](https://news.ycombinator.com/item?id=45560036)
[9](https://blog.ladybugdb.com/post/ladybug-release/)
[10](https://cran.r-project.org/web/packages/kuzuR/refman/kuzuR.html)
[11](https://docs.ladybugdb.com/import/)
[12](https://docs.rs/lbug/latest/lbug/)
[13](https://docs.rs/kuzu/latest/kuzu/)
[14](https://github.com/kuzudb/kuzu/issues/560)
[15](https://github.com/LadybugDB/ladybug/releases)
[16](https://arxiv.org/html/2506.23397v1)
[17](https://users.rust-lang.org/t/need-help-cross-compiling-to-aarch64-unknown-linux-gnu/123318)
[18](https://github.com/kuzudb/kuzu/issues/5324)
[19](https://www.youtube.com/watch?v=bJwT5ecbFRQ)
[20](https://github.com/kuzudb/kuzu)
[21](https://github.com/kuzudb/kuzu/pulls)
[22](https://data.safetycli.com/packages/pypi/kuzu/changelog?page=5)
[23](https://github.com/i64/kuzu-rs)
[24](https://docs.rs/ladybug)
[25](https://lobehub.com/ja/mcp/sasajib-unified-mcp/)
[26](https://skywork.ai/skypage/en/kuzudb-mcp-server-ai-engineers-guide/1980102965186568192)
[27](https://github.com/kuzudb/kuzu/issues)
[28](https://www.youtube.com/watch?v=B8IjggrJKwk)
[29](https://docs.falkordb.com/migration/kuzu-to-falkordb.html)
[30](https://discourse.julialang.org/c/offtopic/29)
[31](https://github.com/kuzudb/kuzu/discussions/4300)
[32](https://github.com/zengzzzzz/golang-trending-archive)
[33](https://randomgeekery.org/note/2025/10/2025-10-20/)
[34](https://datalabtechtv.com/posts/graphrag-with-kuzudb/)
[35](https://github.com/LadybugDB/ladybug)
[36](https://docs.ladybugdb.com/developer-guide/)
[37](https://docs.ladybugdb.com/installation/)
[38](https://docs.ladybugdb.com/client-apis/rust/)
[39](https://elixirforum.com/t/attempt-at-adapting-kuzudbs-rust-crate-into-elixir-via-nif-any-tips-thoughts/66100)
[40](https://docs.ladybugdb.com)
[41](https://kuzudb.github.io/docs/extensions/)
[42](https://lib.rs/~sadikkuzu)
[43](https://www.mexc.co/en-NG/news/296911)
[44](https://doc.rust-jp.rs/book-ja/ch14-00-more-about-cargo.html)
[45](https://docs.ladybugdb.com/tutorials/)
[46](https://data.safetycli.com/packages/pypi/kuzu/changelog)
[47](https://crates.io/crates/kudzu)
[48](https://github.com/kuzudb/kuzu/issues/5062)
[49](https://github.com/kuzudb/kuzu/discussions/categories/q-a)
[50](https://thedataquarry.com/blog/embedded-db-2)
[51](https://dataengineeringpodcast.com/episodepage/high-performance-and-low-overhead-graphs-with-kuzudb)
[52](https://docs.rs/merge)
[53](https://www.graphgeeks.org/blog/what-every-developer-needs-to-know-about-in-process-dbmss)
[54](https://lib.rs/crates/doc-merge)
[55](https://github.com/kuzudb/kuzu/issues/1959)
[56](https://internals.rust-lang.org/t/mini-rfc-merge-cargo-edit-into-cargo-and-allow-both-and-in-cargo-toml-dependency-names/9025)
[57](https://community.neo4j.com/t/help-with-cypher-query-merge-if-not-exists/39818)
[58](https://kobzol.github.io/rust/2025/12/30/investigating-and-fixing-a-nasty-clone-bug.html)
[59](https://github.com/orgs/kuzudb/packages/container/package/extension-repo)
[60](https://www.reddit.com/r/LocalLLaMA/comments/1nqgio2/inbrowser_codebase_to_knowledge_graph_generator/)
[61](https://crates.io/crates/merge)
[62](https://arxiv.org/html/2407.04823v2)
[63](https://kuzudb.github.io/docs/extensions/json/)
[64](https://github.com/HKUDS/LightRAG/pull/1763)
[65](https://www.vldb.org/pvldb/vol18/p5516-angela.pdf)
[66](https://stackoverflow.com/questions/75830551/does-merge-clause-in-cypher-allows-creating-relationships-without-specifying-the)
[67](https://db-engines.com/en/system/Kuzu)
[68](https://docs.ladybugdb.com/migrate/)
[69](https://pypi.org/project/kuzu/)
[70](https://dev.to/cocoindex/build-real-time-knowledge-graphs-from-documents-using-cocoindex-kuzu-with-llms-live-updates-n1b)
[71](https://www.emerald.com/ftdbs/article/14/2/72/1320831/Modern-Techniques-For-Querying-Graph-structured)
[72](https://github.com/kuzudb/kuzu-mcp-server)
[73](https://github.com/LadybugDB/ladybug/activity)
[74](https://www.cs.helsinki.fi/u/jilu/paper/dasfaa23.pdf)
[75](https://www.linkedin.com/pulse/ladybug-next-chapter-embedded-graph-databases-arun-sharma-29xuc)
[76](https://www.bauplanlabs.com/post/ephemeral-graphs-for-data-dags)
[77](https://lib.rs/crates/kuzu)
[78](https://stackoverflow.com/questions/49077147/how-can-i-force-build-rs-to-run-again-without-cleaning-my-whole-project)
[79](https://gauravgahlot.in/rust-dynamic-libraries/)
[80](https://github.com/kuzudb/kuzu/issues/4150)
[81](https://doc.rust-lang.org/beta/releases.html)
[82](https://www.linkedin.com/posts/mrdenjosip_github-kuzudbkuzu-embedded-property-graph-activity-7383500186360221696-4y1B)
[83](https://creators.spotify.com/pod/profile/kaivalya-apte/episodes/A-Graph-Database-That-You-Can-Embed---KuzuDB-e2hkf9n)
[84](https://raw.githubusercontent.com/modelcontextprotocol/servers/refs/heads/main/README.md)
[85](https://www.freshports.org/databases/Makefile)
[86](https://crates.io/crates/kuzu/0.11.3)
[87](https://stackoverflow.com/questions/866921/static-extension-methods)
[88](https://dev.to/rasheedmozaffar/exploring-extension-blocks-in-net-10-ijo)
[89](https://github.com/dotnet/csharplang/discussions/2505)
[90](https://github.com/kuzudb/kuzu/issues/4841)
[91](https://adamstorr.co.uk/blog/dont-do-this-with-extension-methods/)
[92](https://github.com/louthy/language-ext/discussions/1487)
[93](https://www.reddit.com/r/rust/comments/1i4ekc7/rustdata_a_declarative_data_persistence_framework/)
[94](https://daedtech.com/why-i-dont-like-c-extension-methods/)
[95](https://forums.foundationdb.org/t/record-layer-design-questions/3468?page=2)
[96](https://blog.stackademic.com/net-10s-most-powerful-feature-isn-t-what-you-think-7d507dd254dc)
[97](https://skywork.ai/skypage/en/ultimate-ai-engineer-guide-cognee-mcp-server/1977912822261551104)
[98](https://www.linkedin.com/posts/connecteddataworld_knowledgegraph-datamodeling-tutorial-activity-7379761899082686464-zU3Z)
[99](https://www.pulsemcp.com/servers/memory-graph)
[100](https://forums.theregister.com/forum/all/2024/09/09/opinion_column_rust_linux/)
[101](https://github.com/pgcentralfoundation/pgrx)
[102](https://packagist.org/packages/league/commonmark)
[103](https://forums.developer.nvidia.com/t/isaac-sim-people-simulation-broken-in-4-1-0/301378)
[104](https://cran.r-project.org/web/packages/RSQLite/news/news.html)
[105](https://napari-hub.org/plugins/partseg.html)
[106](https://github.com/thomasklemm/awesome-stars)
[107](https://discourse.holoviz.org/t/serving-panel-with-plotly-extension-offline/124)
[108](https://news.ycombinator.com/item?id=46264704)
[109](https://www.bioinformatics.babraham.ac.uk/projects/fastqc/)
[110](https://github.com/kuzudb/kuzu/packages/2258307)
[111](https://news.ycombinator.com/item?id=46264491)
[112](https://pear.php.net/package/PHP_Archive/download/All)
[113](https://www.sktime.net/en/v0.30.0/changelog.html)
[114](https://support.illumina.com/content/dam/illumina-support/documents/documentation/chemistry_documentation/trusight/oncology-500-ctdna/dragen-trusight-oncology-500-ctDNA-analysis-software-v1.1-Readme-1000000135569_00.pdf)
[115](https://github.com/getzep/graphiti/issues/1132)
[116](https://code.martini.nu/mahlon/nim-ladybug/src/commit/ee0e8a72c0d216d8f0cf8547c718d4f707e0eb64)
[117](https://github.com/run-llama/llama_index/issues/20232/linked_closing_reference)
[118](https://gdotv.com/blog/gdotv-release-apache-age-rdf-ux-update/)
[119](https://www.cs.cmu.edu/~pavlo/blog/2026/01/2025-databases-retrospective.html)
[120](https://docs.ladybugdb.com/extensions/attach/iceberg)
[121](https://news.ycombinator.com/item?id=45751969)
[122](https://github.com/stefanak-michal/php-bolt-driver)
[123](https://ai.plainenglish.io/the-disappearance-of-kuzu-a-cautionary-tale-for-ai-and-knowledge-graph-development-5daffcaebcd8)
[124](https://github.com/stevereiner/flexible-graphrag)
[125](https://github.com/RoboFinSystems/robosystems/blob/main/robosystems/graph_api/core/README.md)