// Darvium デュアルストア抽象化レイヤ
//
// 本モジュールは LadybugDB 責務 (GraphStore) と SQLite 責務 (MetadataStore) の
// 2系統トレイトを定義し、メモリ内実装 (InMemoryGraphStore / InMemoryMetadataStore) を提供する。
// 全13フェーズはこのトレイトに対するプログラミングで実装され、
// 実DB接続フェーズでは各トレイトの別実装を追加するだけで差し替えが完了する。

mod graph_store;
mod metadata_store;

pub use graph_store::{GraphStore, InMemoryGraphStore};
pub use metadata_store::{InMemoryMetadataStore, MetadataStore};
