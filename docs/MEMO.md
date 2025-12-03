# Phase-01 実装における変更点・欠損情報メモ

## 1. Search機能におけるグラフ探索のスキップ
`docs/DIRECTONS-PHASE-01.md` の当初設計では、`Search` 関数内で `GraphStorage.GetContext` を呼び出し、ベクトル検索で見つかったチャンクに関連するノードやエッジを取得してコンテキストに含める計画でした。
しかし、Phase-01の実装では、チャンクとノードの直接的なリンク（インデックス）が簡易実装のため不十分であり、複雑さを避けるためにこのステップをスキップしました。
Phase-02以降で、`Chunk` と `Node` の関連付けを強化し、`GetContext` を有効化する必要があります。

## 2. MetadataStorage の反復処理
`Cognify` プロセスにおいて、保存された全ての `Data` を反復処理する必要がありましたが、`MetadataStorage` インターフェースには `GetData(id)` しか定義されていませんでした。
Phase-01の実装では、`*inMemoryMetadataStorage` への型アサーションを行い、内部の `map` に直接アクセスすることでこれを解決しました。
本来はインターフェースに `GetAllData(ctx) ([]*Data, error)` などのメソッドを追加すべきです。
