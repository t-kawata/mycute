# KuzuDB Integration - Comprehensive Test Plan 01

## 1. 目的
KuzuDBをバックエンドとしたCogneeシステムの検索精度を定量的に検証する。
特に、以下の各処理段階における精度の変化を測定し、Memifyおよび再帰的処理の効果を科学的に立証することを目的とする。

## 2. テスト環境

### 2.1 データセット
- **ソーステキスト**: `src/test_data/test_data/sample.txt` (プログラミング言語の歴史と特徴に関する記述)
- **評価用QAデータ**: `src/test_data/test_data/QA.json`
    - データ構造: `{ "question": "...", "answer": "...", "is_correct": bool, "is_answerable": bool }`
    - **クレンジング**: テスト実行時は、`is_correct: true` かつ `is_answerable: true` のエントリ（正解データ）のみを対象とするフィルタリングを行うことで、純粋な正答率を測定する。

### 2.2 システム設定
- **Database Mode**: `kuzudb`
- **Model**: `gpt-4o-mini` (コスト効率と安定性のバランスを考慮)
- **S3 Strategy**: Local Filesystem (for consistency)

### 2.3 実行条件 (Updated)
- **サンプリング**: `QA.json` (正解データフィルタ後) から指定件数 `N` を**ランダムに抽出**して実行する。
- **試行回数**: 各フェーズにつき **M回 (例: 5回)** のベンチマークを実行し、その**平均値**を最終的な測定値とする。これにより、ランダム性によるばらつきを排除し、信頼性の高いデータを取得する。

## 3. 評価指標 (Metrics)

本テストでは、以下の数値を指標として採用する。

1.  **平均コサイン類似度 (ACS) の平均**
    - M回の試行における ACS の相加平均。
    
2.  **正解率 (Accuracy) の平均**
    - M回の試行における Accuracy の相加平均。

3.  **デルタ (Δ)**
    - フェーズ間のスコア増減。統計的有意性の確認（簡易的な標準偏差の比較など）も推奨される。

## 4. テストフェーズ

テストは以下の順序で実施し、各段階でベンチマークを実行する。
各フェーズの実行前にデータベースはリセット（Clean State）されるが、フェーズ2以降は前フェーズの状態を引き継ぐ（累積効果の検証）。

### Phase 1: Baseline (Add + Cognify)
- **操作**:
    1. DB初期化
    2. `sample.txt` を `Add`
    3. `Cognify` を実行 (Chunking + Graph Extraction + Storage)
- **検証**:
    - ベンチマーク実行 (Benchmark-1)
- **期待値**: 
    - テキストチャンク検索と基本的なグラフ検索によるベースライン精度の確立。

### Phase 2: Memify Effect (Depth=0)
- **操作**:
    1. Phase 1の状態から継続
    2. `Memify` (RecursiveDepth=0) を実行
        - Bulk/Batch処理による知識グラフの強化（ルール抽出）
- **検証**:
    - ベンチマーク実行 (Benchmark-2)
- **比較**: Benchmark-2 vs Benchmark-1
- **期待値**: 
    - 明示的なルールや関係性の抽出により、複雑な質問への回答能力が向上する（ACSの増加）。

### Phase 3: Recursive Effect (Depth=1)
- **操作**:
    1. Phase 2の状態から継続
    2. `Memify` (RecursiveDepth=1) を実行
        - 既存グラフに基づいた更なる深掘りと拡張
- **検証**:
    - ベンチマーク実行 (Benchmark-3)
- **比較**: Benchmark-3 vs Benchmark-2
- **期待値**: 
    - 高次の概念や推論が必要な質問への精度向上。

### Phase 4: Recursive Saturation (Depth=2) [Optional]
- **操作**:
    1. Phase 3の状態から継続
    2. `Memify` (RecursiveDepth=2) を実行
- **検証**:
    - ベンチマーク実行 (Benchmark-4)
- **比較**: Benchmark-4 vs Benchmark-3
- **期待値**: 
    - 精度の収束（Saturation）確認。Depth=1との差分がわずかであれば、コスト対効果の観点からDepth=1が最適と結論付ける。

## 5. 実装計画

### 5.1 ベンチマークツールの拡張
既存の `src/pkg/cognee/tools/benchmark/benchmark.go` を以下の要件に合わせて拡張しました：

- ✅ **フィルタリング機能**: `QA.json` の `is_correct: true` かつ `is_answerable: true` のみを対象とするロジック追加済み。
- ✅ **ランダムサンプリング**: `math/rand` を使用したシャッフル機能を実装済み。
- ✅ **結果集約**: `BenchmarkResult` 構造体を返すように変更し、複数回実行の平均計算をサポート。

### 5.2 実行コマンド

#### 1. セットアップ
```bash
./mycute test-kuzudb-comprehensive-setup
```
- データベースとS3ディレクトリのクリーンアップ
- テストデータ（`sample.txt`）のコピー

#### 2. ベースライン測定
```bash
./mycute test-kuzudb-comprehensive-run --phase baseline --runs 5 --n 20
```

#### 3. Memify効果測定（Depth=0）
```bash
./mycute test-kuzudb-comprehensive-run --phase memify-depth0 --runs 5 --n 20
```

#### 4. 再帰効果測定（Depth=1）
```bash
./mycute test-kuzudb-comprehensive-run --phase memify-depth1 --runs 5 --n 20
```

**パラメータ説明:**
- `--phase`: テストフェーズ（`baseline`, `memify-depth0`, `memify-depth1`）
- `--runs`: 試行回数（デフォルト: 5）
- `--n`: 1回あたりの質問数（デフォルト: 20、0で全件）

## 6. 合理性・科学的妥当性

- **対照実験デザイン**: 
    - 同一データセット・同一質問セットに対し、処理パイプラインのみを変数として変更するため、介入（Memify等）の効果を直接的に測定可能。
- **定量的評価**: 
    - 主観的な「良くなった」ではなく、Embedding類似度という客観的数値を用いることで再現性を担保。
- **統計的有意性 (将来的な拡張)**:
    - サンプルサイズが十分であれば、フェーズ間のスコア差に対してt検定を行うことで、改善が誤差ではないことを証明可能（今回は記述統計比較にとどめる）。

---
**作成日**: 2025-12-07
**作成者**: Antigravity
