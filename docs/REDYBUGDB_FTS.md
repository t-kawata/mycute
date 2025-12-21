**LadybugDBは全文検索（Full Text Search）機能を提供しています**。これはコア機能の一部として提供される「FTS拡張（Full Text Search Extension）」を通じて実装されています。

以下に、LadybugDBの全文検索機能の正確な情報をまとめました。

### 1. 全文検索機能の実装形態
LadybugDBの全文検索は、標準機能ではなく**拡張機能（Extension）**として提供されていますが、バージョン**v0.11.3以降では標準でプリインストール・プリロード**されています。

*   **拡張機能名**: `fts`
*   **アルゴリズム**: BM25（Okapi BM25）ランキング関数を使用
*   **特徴**: キーワードベースの検索、ステミング（語幹処理）、ストップワード除去をサポート

### 2. 使用方法（構文）

LadybugDBはKuzuDBのアーキテクチャ（DuckDB for Graphsを目指す設計）をベースにしているため、Cypherの`CALL`句を使用してインデックスの作成と検索を行います。

#### インデックスの作成
特定のノードテーブルのプロパティに対して全文検索インデックスを作成します。

```cypher
// 構文: CALL create_fts_index('インデックス名', 'テーブル名', ['プロパティ名'])
CALL create_fts_index('desc_idx', 'Movie', ['description']);
```

#### 全文検索の実行
作成したインデックスを使用して検索を行います。検索結果として、ノード、検索スコア（類似度）などが返されます。

```cypher
// 構文: CALL query_fts_index('インデックス名', 'テーブル名', '検索クエリ')
CALL query_fts_index('desc_idx', 'Movie', 'dystopian future')
RETURN node.title, score;
```
