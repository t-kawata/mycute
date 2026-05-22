# 実装サマリー: EmbeddingProvider 抽象トレイトの定義 (M-2-1.7)

## 変更ファイル

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/constants.rs` | 追加 (1行) | `FAKE_EMBEDDING_DEFAULT_DIMENSION: usize = 384` |
| `src/error.rs` | 追加＋再編 | `DarviumError::Embedding(String)` 追加。EmbeddingVersionMismatch, EmbeddingDimensionMismatch を Layer 3a から新設の Embedding セクションに移動 |
| `src/llm/mod.rs` | 追加 (全実装) | EmbeddingProvider トレイト、FakeEmbeddingProvider、ConstantEmbeddingProvider、内部ヘルパー (hash_text, generate_fake_embedding)、T1-T15 テスト群 |

## 公開API

- `pub trait EmbeddingProvider: Send + Sync` — `embed()`, `embed_dimension()`
- `pub struct FakeEmbeddingProvider` — `new(dimension)`, `Default` (384次元), 固定シードFNV-1aハッシュ + MMIX LCG による決定論的疑似埋め込み
- `pub struct ConstantEmbeddingProvider` — `new(dimension)`, `with_vector(vector)`, 常に同一ベクトルを返す

## 設計判断

- **PRNG方式**: rand クレートに依存しない。FNV-1a ハッシュ + MMIX LCG で疑似乱数生成
- **配置**: `src/llm/mod.rs` — LLMClient と同モジュール (モジュールを AI プロバイダ抽象化レイヤに昇華)

## テスト結果

- T1-T15: 全15テスト通過 (トレイト境界、オブジェクト安全性、決定論性、非衝突性、次元数、空文字/長大テキスト境界値、エラー型、分布観測)
- 既存テスト: 51テスト + 新規15テスト = 66テスト 全通過
- Clippy: 警告0
- Format: cargo fmt 準拠
