Rustのビルド時間を短縮するには、リンカーの切り替え、コンパイラバックエンドの最適化、およびプロジェクト構造の見直しという3つの側面からのアプローチが極めて効果的です 。特に、2025年後半に実用段階に達したCraneliftバックエンドやパラレルコンパイラの活用により、開発サイクルを劇的に高速化できます 。[1][2][3][4]

### リンカーとキャッシュの最適化
リンク工程はRustのビルドにおける大きなボトルネックですが、デフォルトのリンカーを`mold`（Linux）や`lld`に変更することで、リンク時間を最大10分の1程度まで短縮可能です 。また、`sccache`を導入してコンパイル成果物をグローバルにキャッシュすることで、ブランチ切り替え時や依存関係の再ビルド時間を大幅に削減できます 。[5][1]

| ツール | 対象 | 主な効果 |
| :--- | :--- | :--- |
| **mold** [1] | Linux (x86_64/ARM) | リンク時間を劇的に短縮。LLDよりも高速。 |
| **lld** [1] | 汎用 | 標準リンカーより高速。Windows/macOSでも有効。 |
| **sccache** [1][2] | 汎用 | コンパイル済み成果物の再利用による高速化。 |

### コンパイラ設定による高速化
2026年現在、開発用ビルドにおいて`Cranelift`をコード生成バックエンドとして使用することで、LLVMを使用する場合に比べ生成時間を約20%削減できます 。また、開発版（Nightly）や最新の安定版で利用可能な「パラレルフロントエンド」を有効化（`-Z threads=8`など）することで、マルチコアCPUの性能をフルに活用した解析が可能になります 。これらは`.cargo/config.toml`で設定可能です 。[3][6][7][4]

### 開発ワークフローの改善
コードの変更を即座に検証するために、完全なビルドの代わりに`cargo check`を常用する習慣は依然として重要です 。また、テストの実行には`cargo nextest`を利用することで、標準のテストランナーよりも効率的な並列実行が可能になり、CIやローカルでの待ち時間を30%以上削減できる場合があります 。[2][8]

### プロジェクト構造とジェネリクス
巨大な単一クレートは並列ビルドの恩恵を受けにくいため、プロジェクトを複数のワークスペース・クレートに分割し、依存関係のグラフを疎に保つことが推奨されます 。また、過度なジェネリクスの使用はモノモーフィゼーション（具体型への展開）によるコード肥大化を招きコンパイルを遅くするため、必要に応じてトレイトオブジェクト（`dyn`）への切り替えや、共通処理の外出し（`inner`関数パターン）を検討してください 。[9][10][1]

[1](https://zenn.dev/fairydevices/articles/59cd718341da58)
[2](https://depot.dev/blog/guide-to-faster-rust-builds-in-ci)
[3](https://rust-lang.github.io/rust-project-goals/2024h2/parallel-front-end.html)
[4](https://rust-lang.github.io/rust-project-goals/2025h2/production-ready-cranelift.html)
[5](https://qiita.com/tatsuya6502/items/76b28a6786a1ddc9d479)
[6](https://github.com/rust-lang/rustc_codegen_cranelift)
[7](https://github.com/rust-lang/rust/issues/113349)
[8](https://dev.to/sgchris/reducing-compilation-time-practical-tips-4k1)
[9](https://qiita.com/kawaemon/items/709b8cd2f0462e5967ca)
[10](https://note.com/leapcell/n/n5d5d7f51e780)
[11](https://www.reddit.com/r/rust/comments/uds7v5/rust_build_time_is_so_slow/)
[12](https://postd.cc/fast-rust-builds/)
[13](https://leapcell.io/blog/ja/anata-no-rust-wa-ososugiru-20-no-jissen-teki-na-houhou-de-kodo-wo-saitekika-suru)
[14](https://www.reddit.com/r/rust/comments/1ehkz1y/rust_compile_so_slow/)
[15](https://atmarkit.itmedia.co.jp/ait/articles/2509/22/news014.html)


Craneliftは、高速なコード生成（コンパイル）を重視して設計されたコンパイラバックエンドおよびコード生成器です 。従来のLLVMが高度な最適化と高性能な実行バイナリの生成を得意とするのに対し、Craneliftは「コンパイル自体の速さ」を最優先事項として開発されています 。[1][3][5]

### 主な特徴と目的
Craneliftは、もともとWebAssembly（Wasm）のJIT（実行時）コンパイルを高速化するために、Bytecode Alliance（FastlyやMozillaなどが参加）によって開発されました 。Rustプロジェクトにおいては、主に開発中のビルド（デバッグビルド）の待ち時間を短縮するための代替バックエンドとして導入が進んでいます 。[3][4][1]

- **ビルド速度の追求**: LLVMのような重厚な最適化をスキップし、より単純なパスで機械語を生成するため、ビルド時間が大幅に短縮されます 。[5][1]
- **WebAssemblyとの親和性**: 多くのWasmランタイム（Wasmtimeなど）で標準のバックエンドとして採用されています 。[1]
- **ピュアRust実装**: LLVMがC++で記述されているのに対し、CraneliftはRustで実装されているため、Rustエコシステムとの統合が容易です 。[8]

### LLVMとの比較
LLVMとCraneliftは、それぞれ異なるトレードオフを持っています。2026年現在の一般的な使い分けは以下の通りです。

| 項目 | LLVM (デフォルト) | Cranelift (cg_clif) |
| :--- | :--- | :--- |
| **主な用途** [1] | リリースビルド（本番用） | デバッグビルド（開発用） |
| **ビルド速度** [1][5] | 低速（高度な最適化を実行） | 高速（最小限の最適化） |
| **実行速度** [1] | 最高速 | 高速（LLVMよりは数〜十数%劣る） |
| **対応アーキテクチャ** [1] | 非常に広範 | x86_64, AArch64, RISC-V等に限定 |

### 導入のメリットと注意点
開発者は`.cargo/config.toml`などの設定を通じて、デバッグビルド時のみCraneliftを使用するように設定できます 。これにより、コードを変更してからテストを実行するまでのフィードバックループを20%以上短縮できるケースが多いです 。[4][10][11]

ただし、注意点として、**proc-macro（手続き型マクロ）**を多用するプロジェクトでは、マクロ自体の実行が遅くなることでビルド全体の短縮効果が相殺される場合があります 。また、浮動小数点の厳密な挙動や一部のインラインアセンブリなど、LLVMに依存した一部の機能が完全にはサポートされていない可能性があるため、開発環境での利用が推奨されます 。[7][11][4][1]

[1](https://blog.rust-jp.rs/tatsuya6502/posts/2020-12-rustc-cranelift/)
[2](https://www.reddit.com/r/rust/comments/1h1tnms/is_cranelift_better_than_llvm/)
[3](https://www.fastly.com/jp/blog/how-we-vetted-cranelift-for-secure-sandboxing-in-compute-edge)
[4](https://zenn.dev/fairydevices/articles/59cd718341da58)
[5](https://zenn.dev/0yoyoyo/articles/d5f8d46078552e2fcc0d)
[6](https://scrapbox.io/pogi-log/Cranelift)
[7](https://www.reddit.com/r/rust/comments/17p7sqv/aiming_for_10x_faster_by_taking_charge_of_the/)
[8](https://blog.pickledchair.com/post/develop-my-lang-with-cranelift/)
[9](https://www.reddit.com/r/rust/comments/10h4gd5/cranelifts_instruction_selector_dsl_isle/)
[10](https://github.com/rust-lang/rustc_codegen_cranelift)
[11](https://rust-lang.github.io/rust-project-goals/2025h2/production-ready-cranelift.html)


# 追加情報

M2 Mac（Apple Silicon / AArch64）でも、**Craneliftは使用可能**です 。かつてはmacOSのAArch64特有の呼び出し規則（ABI）や可変長引数の扱いに制限がありましたが、2024年から2025年にかけての開発により、現在は実用的なレベルでサポートされています 。[1][2][3]

### M2 Macでの利用状況
2026年現在、M2 Macを含むApple Silicon環境は、Cranelift（`rustc_codegen_cranelift`）の主要なターゲットの一つとして扱われています 。[4][2]

- **ネイティブ対応**: M2プロセッサ上でRosetta 2を介さず、ネイティブなAArch64コードを高速に生成できます 。[2]
- **導入方法**: `rustup`を通じてコンポーネントとして簡単にインストールできるようになっており、Nightly版だけでなく、安定版（Stable）でも実験的機能として利用が進んでいます 。[2]
- **パフォーマンス**: LLVMに比べて、コンパイル時間は約20%〜30%短縮される傾向にあり、開発中の「書いては実行する」サイクルを大幅に高速化できます 。[5][4]

### 注意点と制限事項
M2 Macで利用する際、以下の点に留意してください。

- **デバッグビルド推奨**: 前述の通り、Craneliftは最適化よりも速度を優先するため、本番環境向けのリリースビルドでは引き続きLLVM（デフォルト）を使用するのが一般的です 。[5]
- **SIMDと特定のイントリンジック**: Apple Silicon特有のNEON命令（SIMD）などを多用するコードでは、LLVMに比べて最適化が及ばない、あるいは一部の高度な命令が未実装である場合があります 。[4]
- **設定方法**: 以下のように`.cargo/config.toml`に設定を追加することで、M2 Macでのビルドに適用できます 。[6]

```toml
[unstable]
codegen-backend = true

[profile.dev]
codegen-backend = "cranelift"
```

これにより、開発時のビルドのみCraneliftが適用され、M2 Macのマルチコア性能を活かした高速なコンパイルを享受できます 。[6][4]

[1](https://bjorn3.github.io/2023/10/31/progress-report-oct-2023.html)
[2](https://www.reddit.com/r/rust/comments/1gv9m3j/progress_report_on_rustc_codegen_cranelift/)
[3](https://sdr-podcast.com/episodes/cranelift/)
[4](https://rust-lang.github.io/rust-project-goals/2025h2/production-ready-cranelift.html)
[5](https://blog.rust-jp.rs/tatsuya6502/posts/2020-12-rustc-cranelift/)
[6](https://zenn.dev/fairydevices/articles/59cd718341da58)
[7](https://github.com/rust-lang/rustc_codegen_cranelift/issues/1248)
[8](https://doc.rust-lang.org/beta/rustc/platform-support.html)
[9](https://github.com/rust-lang/rust/issues/73908)
[10](https://rust.googlesource.com/rust/+/29d4cbafa418c9546546d8867bdd07afa8fbcda3)
[11](https://forum.opencraft.com/t/apple-silicon-compatibility/951)
[12](https://github.com/rust-lang/rustc_codegen_cranelift/issues/1402)
[13](https://forums.negativelabpro.com/t/native-apple-silicon-support/6017)
[14](https://doc.rust-lang.org/nightly/rustc/platform-support/apple-darwin.html)
[15](https://www.youtube.com/watch?v=VUlX_-hEQ44)
[16](https://www.reddit.com/r/Python/comments/15pfvxt/is_apple_silicon_mostly_supported_now/)
[17](https://www.reddit.com/r/rust/comments/jcbh2h/aarch64appledarwin_is_now_a_tier2_target/)