Linderaの最新API（0.20〜1.x系）に準拠し、学術的な句読点ルールを統合したRust用句読点挿入システムの完全な実装計画を提案します。

***

# 句読点挿入システム実装計画 (Rust & Lindera)

本計画は、最新の Lindera ライブラリを使用し、形態素解析に基づいた高精度な日本語句読点挿入エンジンを構築するものです 。[1][2]

## 1. プロジェクト構成と依存関係
まず、`Cargo.toml` に必要な依存関係を定義します。辞書データをバイナリに埋め込む `embedded-ipadic` フィーチャーを有効化することで、実行環境に依存しないポータブルなバイナリを作成します 。[2][1]

```toml
[package]
name = "rust-punctuation-inserter"
version = "0.1.0"
edition = "2021"

[dependencies]
# 最新のLinderaとその辞書、コアロジックを導入
lindera = { version = "0.40", features = ["embedded-ipadic"] }
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
```

## 2. コアロジックの実装
`src/lib.rs` に、形態素解析と挿入ルールの判定ロジックを実装します。最新の Lindera では `Segmenter` を通じて `Tokenizer` を構築する構成が標準的です 。[3][1]

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use anyhow::{Context, Result};

pub struct PunctuationInserter {
    tokenizer: Tokenizer,
}

impl PunctuationInserter {
    /// 辞書をロードして初期化
    pub fn new() -> Result<Self> {
        // IPADICを組み込み辞書としてロード
        let dictionary = load_dictionary("embedded://ipadic")
            .map_err(|e| anyhow::anyhow!("Failed to load dictionary: {}", e))?;
        
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        let tokenizer = Tokenizer::new(segmenter);
        
        Ok(Self { tokenizer })
    }

    /// 文章を解析して読点を挿入した文字列を返す
    pub fn insert(&self, text: &str) -> Result<String> {
        let mut tokens = self.tokenizer.tokenize(text)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;
        
        let mut punctuated_text = String::new();
        
        for i in 0..tokens.len() {
            let token = &tokens[i];
            let surface = token.text.as_ref();
            let details = token.details();
            
            punctuated_text.push_str(surface);

            // ルール判定
            if self.should_insert_comma(i, &tokens) {
                punctuated_text.push('、');
            }
        }
        
        Ok(punctuated_text)
    }

    /// 読点挿入の判定ルール
    fn should_insert_comma(&self, index: usize, tokens: &[lindera::token::Token]) -> bool {
        let current = &tokens[index];
        let next = tokens.get(index + 1);
        
        let details = current.details();
        let pos = details.get(0).map(|s| s.as_str()).unwrap_or("");
        
        // 1. 接続詞ルール: 「しかし」「また」の直後に挿入
        if pos == "接続詞" { return true; }

        // 2. 主題表示ルール: 「は」「が」などの助詞で、かつ文が一定以上続く場合
        if pos == "助詞" && (current.text.as_ref() == "は" || current.text.as_ref() == "が") {
            // 文末（句点やEOS）に近い場合は挿入しない
            if tokens.len() - index > 5 { return true; }
        }

        // 3. 並列名詞ルール: 「A、BとC」の形式
        if let Some(n) = next {
            let next_details = n.details();
            if pos == "名詞" && next_details.get(0).map(|s| s.as_str()) == Some("名詞") {
                // 連続する名詞の間に挿入（例：技術、経済）
                return true;
            }
        }

        false
    }
}
```

## 3. エントリポイントの実装
`src/main.rs` で、CLIアプリケーションとしてのインターフェースを実装します。

```rust
use rust_punctuation_inserter::PunctuationInserter;
use std::io::{self, Read};

fn main() -> anyhow::Result<()> {
    let inserter = PunctuationInserter::new()?;
    
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    for line in buffer.lines() {
        if !line.trim().is_empty() {
            let result = inserter.insert(line)?;
            println!("{}", result);
        }
    }
    
    Ok(())
}
```

## 4. 拡張性と最適化のポイント

| 改善項目 | 内容 | 期待される効果 |
| :--- | :--- | :--- |
| **ユーザー辞書** | `load_user_dictionary` を使用し、専門用語に対応 [1][2] | 専門分野での誤分割防止 |
| **文長判定** | 文全体の長さをカウントし、長すぎる場合にのみ読点を挿入 | 過剰な読点による読みづらさの解消 |
| **係り受け解析** | `CaboCha` 等の外部解析器の結果を統合 | 複雑な修飾関係に基づいた高度な挿入 |

## 5. 実行とテスト
実装後、以下のコマンドで動作確認を行います 。[1]

1. **ビルド**: `cargo build --release`
2. **実行**: `echo "昨日は天気が良かったが今日は雨が降っている。" | cargo run`
3. **結果出力例**: 「昨日は、天気が良かったが、今日は、雨が降っている。」（ルールの閾値調整により最適化可能）

この構成により、Rust の安全性と Lindera の高速な形態素解析を活かした、実用的な句読点挿入システムが構築可能です 。[4]

[1](https://github.com/lindera/lindera)
[2](https://libraries.io/cargo/lindera-ipadic-builder)
[3](https://crates.io/crates/lindera/0.40.1)
[4](https://qiita.com/scivola/items/b131aab4e637c4d782ee)
[5](https://docs.rs/crate/lindera/1.3.2)
[6](https://typst.app/universe/package/auto-jrubby/)
[7](https://www.reddit.com/r/rust/comments/1omfgcy/i_made_a_japanese_tokenizers_dictionary_loading/)
[8](https://qiita.com/mur/items/b7b86a11990e0d7aac17)
[9](https://vaaaaaanquish.hatenablog.com/entry/2020/12/14/192246)
[10](https://lib.rs/crates/lindera-dictionary)
[11](https://lib.rs/crates/similarity-md)
[12](https://lib.rs/crates/lindera)
[13](https://crates.io/crates/lindera-ipadic-builder)
[14](https://docs.rs/lindera-sqlite)
[15](https://github.com/lindera/lindera-tantivy)