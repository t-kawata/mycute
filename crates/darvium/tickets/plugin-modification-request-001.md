---
title: プラグイン改修依頼 001 — plan/review への RFC 既存実装状態検証ステップ追加
requestor: darvium 利用者
target_version: darvium プラグイン v0.1.0
priority: high (品質担保)
---

# プラグイン改修依頼 001: RFC 既存実装状態検証ステップの追加

## 1. 問題

### 1.1 背景

M-2-2（SearchBudget / RecursionGuard）の make-ticket において、現行実装が RFC §13.3 の型定義と大きく乖離していることが判明した：

| 問題 | 現行コード | RFC §13.3 正規定義 |
|------|-----------|-------------------|
| フィールドの混入 | SearchBudget に max_depth/current_depth | RecursionGuard の責務 |
| フィールド欠落 | SearchBudget に max_iterations なし | u32 必須 |
| フィールド欠落 | RecursionGuard に allow_reentrant なし | bool 必須 |
| 型不一致 | RecursionGuard が usize | RFC は u32 |
| 型未定義 | SearchBudgetSnapshot なし | RFC で定義 |

これらの問題は、CLAUDE.md に「RFC 交叉参照の義務化」という抽象的な指示は存在するものの、**系統立ったチェック手順がプラグインコマンドに組み込まれていない**ために発生した。

### 1.2 根本原因

`plan-ticket` のワークフローに **「現行コードと RFC の型レベル一致検証」** が存在しない。現状の Step 5「Investigation の再検証」は spec の Investigation セクションの記述が古くなっていないかを確認するだけであり、**RFC の該当セクションをソースコードと系統的に比較する**プロセスがない。

同様に `review-ticket` も「plan のレビュー方法を再実行する」とあるが、そもそもの plan が型比較を含んでいなければレビューでも検出できない。

## 2. 改修対象ファイル

| ファイル | 改修内容 |
|----------|---------|
| `commands/plan-ticket.md` | Step 5 を「RFC 既存実装状態検証」に拡張（新規サブステップ追加） |
| `commands/review-ticket.md` | Step 5 翻訳可能性チェックと並列で「RFC 既存実装状態検証」の再実行を追加 |
| `contexts/review.md` | Review Checklist に「RFC フィールドレベル比較の実施有無」を追記 |

## 3. 改修詳細

### 3.1 `commands/plan-ticket.md` の変更

現在の Step 5（Investigation の再検証）を以下のように拡張する。

**変更前（現在の Step 5）:**

```markdown
### Step 5: Investigation の再検証

spec 作成時から時間が経過している場合、当時記録された Investigation セクションの物理的証拠が現在のコードベースと一致しているとは限らない。以下の観点で再検証する：

- Investigation に記載されたファイルの該当行が現在も同じ内容か確認する
- 既に修正・改善されていたり、逆に新たな問題が発生していないか grep やテスト実行で確認する
- 検証結果に基づき、Investigation の情報を最新の状態に更新する

**計画は常に現在のコードベースの状態に基づいて策定しなければならない。**
```

**変更後:**

```markdown
### Step 5: RFC 既存実装状態検証（新規: 必須ステップ）

**このステップは plan 策定の前提条件である。** 以下の手順を必ず実行し、結果を plan.md の先頭に記載する。

検証を省略した計画は不完全とみなす。

#### 5a: 該当 RFC セクションの特定

spec に記載された「対象不変条件 / 規範」を手がかりに、RFC の該当セクション番号（例: §13.3、§13.6）を特定する。

該当セクションが spec に明記されていない場合は、チケットタイトルと実装スコープから自力で特定し、欠落を spec に追記する。

#### 5b: RFC 型定義の抽出

該当 RFC セクションに定義された全 `struct` / `enum` / `trait` について、以下を抽出する：
- 型名
- フィールド名
- フィールドの型
- オプション性（必須 or 省略可）

#### 5c: 現行コードとの比較

抽出した型定義を現行ソースコードと 1 フィールド単位で比較し、以下の観点で評価する：

| 観点 | 判定基準 |
|------|---------|
| 完全一致 | フィールド名・型が RFC と同一 |
| 型不一致 | フィールド名は同じだが型が異なる（例: RFC u32 vs 実装 usize） |
| フィールド欠落 | RFC に定義されているフィールドが実装に存在しない |
| 余剰フィールド | 実装に存在するが RFC に定義がないフィールド |
| 型未定義 | 構造体そのものが未実装 |

比較結果は以下のテーブル形式で plan.md に記載する：

```markdown
## RFC 既存実装状態検証

### RFC §X.Y `SomeStruct`
| フィールド | RFC の型 | 現行コードの型 | 状態 |
|---|---|---|---|
| field_a | u32 | u32 | ✅ 一致 |
| field_b | u64 | u32 | ❌ 型不一致 |
| field_c | String | (欠落) | ❌ フィールド欠落 |
| field_d | (未定義) | bool | ⚠️ 余剰フィールド |

**評価サマリ**: 3/5 フィールドに乖離あり。実装前に修正が必要。
```

#### 5d: Investigation の更新

5c の発見を spec の Investigation セクションに追記する（古い情報はそのまま残し、「updated at plan time: YYYY-MM-DD」として追記）。

### 3.2 `commands/review-ticket.md` の変更

現在の Step 5（翻訳可能性チェック）の後に、以下の Step 5b として追加する。

**追加するステップ（Step 5 を翻訳可能性チェックから「静的品質チェック」に拡張）:**

```markdown
### Step 5: 静的品質チェック

#### 5a: 翻訳可能性チェック
（既存の翻訳可能性チェック内容 — 変更なし）

#### 5b: RFC 既存実装状態検証の再実行（新規）

plan.md の「RFC 既存実装状態検証」セクションを読み、plan 策定時に記録された全ての乖離が実装によって解消されたことを確認する：

1. plan.md の RFC 比較テーブルを読み込む
2. 各「❌ 乖離あり」フィールドに対して、現在のソースコードが修正されていることを grep で確認する
3. 1 つでも未修正の乖離があればレビュー不通過（ステータスを implementing に差し戻し）

**追加で、実装者が新たに導入した型（plan に記載のなかった構造体等）についても、RFC 無矛盾性をスポットチェックする。**
```

### 3.3 `contexts/review.md` の変更

Review Checklist に以下を追加：

```markdown
- [ ] **RFC フィールドレベル比較**: plan の RFC 既存実装状態検証テーブルが全フィールドで ✅ になっているか
```

## 4. 改修の期待効果

| 効果 | 説明 |
|------|------|
| 未実装フィールドの計画的検出 | 実装着手前に RFC とコードの乖離が全て可視化される |
| レビューの具体性向上 | 「なんとなく RFC と合わない気がする」ではなく、テーブルで一覧比較できる |
| 作業効率の向上 | 後戻り（実装してから作り直し）が激減する |
| 知識の非対称性解消 | 実装者とレビュアーが同一の比較テーブルを参照できる |

## 5. 実装例（plan.md に記載する比較テーブルのサンプル）

SearchBudget の例：

```markdown
## RFC 既存実装状態検証

### RFC §13.3 `SearchBudget`
| フィールド | RFC の型 | 現行コードの型 | 状態 |
|---|---|---|---|
| max_iterations | u32 | (欠落) | ❌ フィールド欠落 |
| max_retrieval_calls | u32 | (欠落) | ❌ フィールド欠落 |
| max_prompt_tokens | u64 | u64 | ✅ 一致 |
| max_wall_clock_ms | u64 | (欠落) | ❌ フィールド欠落 |
| prompt_tokens_used | (未定義) | u64 | ⚠️ 余剰フィールド（SearchBudgetSnapshot へ移動） |
| max_depth | (未定義) | usize | ⚠️ 余剰フィールド（RecursionGuard へ移動・型変更） |
| current_depth | (未定義) | usize | ⚠️ 余剰フィールド（RecursionGuard へ移動・型変更） |

### RFC §13.3 `RecursionGuard`
| フィールド | RFC の型 | 現行コードの型 | 状態 |
|---|---|---|---|
| max_depth | u32 | usize | ❌ 型不一致 + フィールド欠落(allow_reentrant) |
| current_depth | u32 | usize | ⚠️ 型不一致（移動元が SearchBudget だった） |
| allow_reentrant | bool | (欠落) | ❌ フィールド欠落 |

### RFC §13.3 `SearchBudgetSnapshot`
| フィールド | RFC の型 | 現行コードの型 | 状態 |
|---|---|---|---|
| — | — | (未定義) | ❌ 型未定義（新規作成必須） |
```

## 6. 補足: なぜ make-ticket ではなく plan-ticket か

`make-ticket` の Investigation でも今回の問題は発見できているが、そこで発見した乖離が**確実に実装計画に反映され、確実にレビューされる**ことを保証する仕組みが必要である。

- `make-ticket` は「問題の発見」を役割とする
- `plan-ticket` は「発見された問題を確実に計画に含める」ことをルール化する
- `review-ticket` は「計画通りに直されたか」を確認する

この責務分離により、誰が実行しても同じ品質が担保される。

## 7. 参考: 現行ファイルの該当行

| ファイル | 行 | 内容 |
|----------|-----|------|
| `commands/plan-ticket.md` | L106-L114 | 現在の Step 5（Investigation の再検証）— これに 5a-5d を追加 |
| `commands/review-ticket.md` | L108-L109 | 現在の Step 5（翻訳可能性チェック）— 5b を追加 |
| `contexts/review.md` | L13 | 「RFC 無矛盾」チェック項目 — 現状は抽象的な一文のみ |
