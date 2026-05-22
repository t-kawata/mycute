---
title: "/check-all コマンド 実装指示書"
description: "darvium プラグインに /check-all スラッシュコマンドを追加するための実装手順を定義する"
created_at: 2026-05-22
target_plugin: darvium@darvium-marketplace
status: draft
---

# /check-all コマンド 実装指示書

## 1. 概要

本指示書は、darvium プラグインに `/check-all` スラッシュコマンドを追加するための完全な実装手順を定義する。

### 1.1 目的

`/check-all` コマンドは、`Darvium-Tickets-v2.3.md` において ✅ マークが付いた**全完了チケット**の実装・実験・較正・観察が、`Darvium-RFC-0001-Unified-v2.3-final.md` および `Darvium-Tickets-v2.3.md` に対して矛盾・欠落・品質問題を持たないかを一括自動点検する。

### 1.2 背景

従来は手動で各チケットのレビューを個別に実行していたが、チケット数が増加するにつれて横断的な整合性チェックが大きな負担となっている。RFC 改訂時やマイルストーン完了時には全完了チケットの再検証が必要であり、この作業を自動化するために本コマンドを実装する。

### 1.3 `/review-ticket` との違い

| 観点 | `/review-ticket` | `/check-all` |
|------|-----------------|--------------|
| 対象 | 単一チケット | 全 ✅ チケット |
| 深さ | 詳細レビュー（コード品質・翻訳可能性・観測検証） | 横断チェック（成果物存在・RFC参照・定数/エラー型・テスト通過） |
| ステータス変更 | `done` → `reviewed` に遷移 | ステータス変更なし（読み取り専用） |
| 実行タイミング | 各チケット完了時 | マイルストーン完了時 / RFC改訂後 / リリース前 |

---

## 2. 必要な成果物

| # | ファイル | 格納先 | 説明 |
|---|----------|--------|------|
| 1 | `check-all.md` | `~/.claude/plugins/marketplaces/darvium-marketplace/commands/` | コマンド定義ファイル（YAML frontmatter + Markdown ワークフロー） |
| 2 | `run-check-all.js` | `~/.claude/plugins/marketplaces/darvium-marketplace/scripts/tickets/check-all/` | チェック実行本体の Node.js スクリプト |

---

## 3. ファイル仕様: `check-all.md`

### 3.1 設計方針

既存の `review-ticket.md` と同一のパターンに従う（YAML frontmatter、Markdown ワークフロー、bash step 内で `node "$_R/scripts/tickets/..."` を呼び出す）。ただし `/review-ticket` が単一チケットを対象とするのに対し、本コマンドは全 ✅ チケットを一括対象とする。

### 3.2 YAML frontmatter

```yaml
---
description: Darvium-Tickets-v2.3.md で「✅」が付いた全完了チケットの実装・実験・較正・観察を RFC およびチケット定義書と総交叉参照し、矛盾や品質問題を一括点検する。引数不要。
---
```

### 3.3 ワークフローステップ

**Step 0: 初期化**

既存コマンドと同一の `DARVIUM_PLUGIN_ROOT.md` 初期化ブロック。`$_R` 変数を設定する。

**Step 1: 全チェック実行**

```bash
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/check-all/run-check-all.js"
```

スクリプトは JSON レポートを stdout に出力する。末尾に以下のパース可能な行が stderr に含まれる：

```
CHECK-ALL-SUMMARY: passed=N warnings=N failed=N errors=N duration=Nms
```

**Step 2: AI によるレポート解釈表示**

AI は JSON レポートを読み、各チケットを等級別に表示する：

- **`PASS`**: 全チェック通過 → チケット名と PASS 表示のみ
- **`WARN`**: 軽微な問題あり → 問題内容を列挙し、AI が修正を提案
- **`FAIL`**: 重大な問題 → 問題内容を列挙し、ユーザーに修正方針を相談
- **`ERROR`**: チケット読み取り不可 → ファイル不備を報告

**Step 3: レポート保存（オプション）**

```bash
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/check-all/run-check-all.js" > check-all-report.json
```

### 3.4 エラー時の動作

- `cargo test` が失敗 → `global_checks.cargo_test.passed = false`、詳細は JSON 内に記録
- 特定チケットの spec が読めない → 該当チケットのみ ERROR、全体は継続
- Darvium-Tickets-v2.3.md が存在しない → エラー終了（process.exit(1)）

---

## 4. ファイル仕様: `run-check-all.js`

### 4.1 設計方針

- **フェーズ構成**: 3 フェーズ（チケット抽出 → 個別チェック → グローバルチェック）で構成。各フェーズは独立しており、エラーが発生しても後続フェーズは継続する
- **パフォーマンス最適化**: `cargo test` / `clippy` / `fmt` は全体で 1 回のみ実行。`constants.rs` / `error.rs` / RFC の内容は 1 回だけファイル読み込みし、メモリにキャッシュして全チケットで共有する
- **モジュール化**: 各関数は `module.exports` で公開し、ユニットテスト可能にする

### 4.2 依存モジュール

```javascript
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");
const { parseFrontmatter, CFG } = require("../../lib/tickets");
```

### 4.3 M-label → ticket_id の動的解決

ハードコードされたマップは使用しない。代わりに全 spec ファイルの frontmatter `title` フィールドから M-label を抽出し、動的にマッピングを構築する：

```javascript
// 全 spec ファイルをスキャンし、title から M-label を抽出してマッピングを構築する
function buildLabelToIdMap() {
  const map = {};
  if (!fs.existsSync(SPECS_DIR)) return map;
  const files = fs.readdirSync(SPECS_DIR).filter(f => f.endsWith(".md")).sort();
  for (const file of files) {
    const content = fs.readFileSync(path.join(SPECS_DIR, file), "utf8");
    const { attrs } = parseFrontmatter(content);
    if (!attrs || !attrs.ticket_id || !attrs.title) continue;
    // title から M-label を抽出: "M-2-1: ...", "M-2-1.5: ...", "M-1.5-1: ..."
    const match = attrs.title.match(/^M-[\d.]+/);
    if (match) {
      map[match[0]] = attrs.ticket_id;
    }
  }
  return map;
}
```

この関数は `tickets/specs/` 内の全 spec ファイルを読み、各ファイルの frontmatter `title` から先頭の M-label（例: `M-2-1`, `M-1.5-3`, `M-1-1`）を正規表現 `/^M-[\d.]+/` で抽出し、`{ "M-2-1": 1, "M-1.5-3": 10, ... }` のマップを返す。

これにより、新チケット追加時にスクリプト側の変更は一切不要となる。

### 4.4 設定: 期待される定数一覧

各チケットで定義されるべき定数。チケットID をキーとする：

```javascript
const EXPECTED_CONSTANTS = {
  1: [],  2: [],  3: ["FAKE_LLM_DEFAULT_MALFORMED_PROB"],
  4: ["FAKE_EMBEDDING_DEFAULT_DIMENSION"],
  5: ["CLOCK_DEFAULT_START_MS"],
  6: ["DEFAULT_MAX_ITERATIONS", "DEFAULT_MAX_RETRIEVAL_CALLS", "DEFAULT_MAX_WALL_CLOCK_MS", "DEFAULT_RECURSION_MAX_DEPTH"],
  7: [],  8: [],  9: [],
  10: ["OSCILLATION_MAX_COUNT"],
  11: ["EVALUATION_THRESHOLD", "SELF_CONF_DISCOUNT"],
};
```

検証方法: `src/constants.rs` を読み込み、各定数名に対して `content.includes("pub const ${name}")` で存在確認する。

### 4.5 設定: 期待されるエラーバリアント一覧

各チケットで追加されるべきエラーバリアント：

```javascript
const EXPECTED_ERRORS = {
  1: ["SearchValidation"],
  2: ["Storage", "NotFound"],
  3: ["Llm", "LlmMalformedJson"],
  4: ["Embedding", "EmbeddingDimensionMismatch"],
  5: [],
  6: ["SearchBudgetExceeded", "SearchRecursionExceeded"],
  7: ["Retrieval", "RetrievalTimeout"],
  8: ["SearchValidation"],
  9: ["TerminalStateViolation"],
  10: ["SearchPolicyOscillation"],
  11: ["InvalidScore"],
};
```

検証方法: `src/error.rs` を読み込み、各バリアント名に対して `content.includes(name)` または `content.includes(name + "(")` で存在確認する。

### 4.6 設定: 翻訳可能性チェックパターン

4 つのソースファイル（`src/types.rs`, `src/lib.rs`, `src/constants.rs`, `src/error.rs`）を対象に以下の grep パターンチェックを実行する：

```javascript
const TRANSLATABILITY_PATTERNS = [
  { name: "unwrap_calls", pattern: /\.unwrap\(\)/g, severity: "major" },
  { name: "magic_numbers", pattern: /\b\d{4,}\b/g, severity: "warning" },
  { name: "single_letter_vars", pattern: /\b(mut|let)\s+([a-hj-z])\b(?!\s*=)/g, severity: "minor" },
  { name: "todo_comments", pattern: /\/\/\s*(TODO|FIXME|HACK|XXX)/g, severity: "warning" },
];
```

### 4.7 フェーズ1: `parseTicketsDoc()`

`Darvium-Tickets-v2.3.md` をパースし、✅ 完了チケットの一覧を抽出する。

**処理内容**:
1. ファイルを読み込む（存在しなければエラー終了）
2. `buildLabelToIdMap()` を呼び出し、全 spec ファイルから M-label → ticket_id マップを構築する
3. 正規表現 `/#### ✅ チケット\s+([\w.-]+):\s*(.+?)\n/g` で全マッチを抽出
4. 各マッチについて、マッチ位置以降のテキストから以下を抽出：
   - `**対象不変条件 / 規範:**` 行 → RFC セクション番号の配列（カンマ/セミコロン/読点で分割）
   - `**実装スコープ:**` 行 → スコープ説明文字列
5. `buildLabelToIdMap()` の返したマップから ticketId を解決（見つからなければ null）

**返り値**: `{ completed: [{ label, title, rfcSections: string[], scope: string, ticketId: number|null }] }`

### 4.8 フェーズ2: `checkTicket(ticket)`

個別チケットの spec 読み取りと成果物存在確認を行う。

**処理内容**:
1. `ticketId` が null の場合は ERROR を返す
2. `ticketId` を 4 桁ゼロ埋めして prefix を生成（例: `1` → `0001`）
3. `tickets/specs/` から `${prefix}-*.md` にマッチするファイルを検索
4. 見つからなければ ERROR を返す
5. YAML frontmatter をパース
6. `plan_path`, `implementation_path`, `review_report_path` の各ファイル存在確認
7. `context/${prefix}-${slug}/` 内の `observation-*.md` ファイルを glob
8. `## Acceptance Criteria` セクションをパース（`- [ ]` / `- [x]` 行を抽出）

**返り値**: 以下の構造を持つオブジェクト（エラー時は `{ error: string }`）：

```javascript
{
  label: string,
  ticketId: number,
  title: string,
  status: string,
  artifacts: { plan: boolean, implementation: boolean, review: boolean, observation: boolean, observationCount: number },
  acceptance: { defined: number },
  rfcSections: string[],
}
```

### 4.9 フェーズ3: `runGlobalChecks()`

プロジェクト全体のチェックを 1 回だけ実行する。

**処理内容**:

1. **`cargo test`**: `execSync("cargo test 2>&1", { timeout: 180000 })` を実行
   - 終了コード 0 かつ stdout に "FAILED" が含まれないことを確認
   - タイムアウト時も `passed: false` で記録し継続

2. **`cargo clippy`**: `execSync("cargo clippy -- -D warnings 2>&1", { timeout: 120000 })`
   - 終了コード 0 であることを確認

3. **`cargo fmt --check`**: `execSync("cargo fmt --check 2>&1", { timeout: 60000 })`
   - 終了コード 0 であることを確認

4. **`validate-structure.js`**: 既存の構造検証スクリプトを実行
   - `node "${scriptDir}/../validate-structure.js"` の JSON 出力をパース
   - `valid` フィールドが true であることを確認

### 4.10 レポート組み立て: `assembleReport()`

フェーズ1〜3 の結果を統合し、チケットごとの verdict を計算して最終 JSON レポートを生成する。

**verdict 判定ロジック**:
```
if (ticket に error がある) → "ERROR"
else if (failures 配列が空でない) → "FAIL"
else if (warnings 配列が空でない) → "WARN"
else → "PASS"
```

**failures 条件**（1つでも該当で FAIL）:
- `artifacts.plan === false`
- `artifacts.implementation === false`
- `artifacts.review === false`
- `rfc_crossref.passed === false`
- `errors.passed === false`

**warnings 条件**（該当で WARN、failures がない場合のみ）:
- `artifacts.observation === false`
- `acceptance.passed === false`
- `constants.passed === false`

### 4.11 キャッシュ戦略（パフォーマンス最適化）

以下のファイル内容はモジュールレベルの変数に 1 回だけ読み込み、全チケットで共有する：

```javascript
const _constantsContent = { value: null };
const _errorsContent = { value: null };
const _rfcContent = { value: null };
```

各チェック関数はこれらのキャッシュを参照する。これによりファイル I/O を最小化する。

### 4.12 出力形式

stdout に整形済み JSON、stderr にサマリ行を出力する。

**stdout**:
```json
{
  "timestamp": "2026-05-22T20:30:00.000Z",
  "darviumRoot": "/Users/kawata/shyme/mycute/crates/darvium",
  "durationMs": 45782,
  "summary": { "total": 11, "passed": 9, "warnings": 2, "failed": 0, "errors": 0 },
  "tickets": [
    {
      "label": "M-2-1",
      "ticketId": 1,
      "title": "RetrievalPrimitive ...",
      "status": "reviewed",
      "verdict": "PASS",
      "failures": [],
      "warnings": ["missing_observation"],
      "checks": {
        "artifacts": { "plan": true, "implementation": true, "review": true, "observation": false },
        "acceptance": { "passed": true, "defined": 5 },
        "rfc_crossref": { "passed": true, "checked": [{ "section": "§13.4", "found": true }] },
        "constants": { "passed": true, "checked": [] },
        "errors": { "passed": true, "checked": [{ "name": "SearchValidation", "found": true }] }
      }
    }
  ],
  "global_checks": {
    "cargo_test": { "passed": true, "exitCode": 0, "testRuns": 12, "summary": "test result: ok. 45 passed; 0 failed" },
    "cargo_clippy": { "passed": true, "exitCode": 0 },
    "cargo_fmt": { "passed": true, "exitCode": 0 },
    "validate_structure": { "passed": true, "issues": 0 }
  },
  "translatability": {
    "passed": true,
    "total": 3,
    "bySeverity": { "major": 0, "warning": 1, "minor": 2 },
    "issues": [{ "file": "src/types.rs", "line": 42, "type": "todo_comments", "severity": "warning", "match": "// TODO: ..." }]
  }
}
```

**stderr**:
```
CHECK-ALL-SUMMARY: passed=9 warnings=2 failed=0 errors=0 duration=45782ms
```

---

## 5. 実装手順

### Step 1: スクリプトディレクトリ作成

```bash
mkdir -p ~/.claude/plugins/marketplaces/darvium-marketplace/scripts/tickets/check-all
```

### Step 2: `run-check-all.js` の実装

セクション4の仕様に従い実装する。実装順序：
1. 設定定数を定義
2. ユーティリティ関数を実装
3. parseTicketsDoc() を実装
4. checkTicket() を実装
5. キャッシュ付きサブチェック関数を実装
6. runGlobalChecks() を実装
7. checkTranslatability() を実装
8. assembleReport() を実装
9. main() で全フェーズを直列実行

### Step 3: `check-all.md` の実装

セクション3の仕様に従い作成する。

### Step 4: 動作確認

darvium プロジェクトルートで以下のコマンドを実行：

```bash
node ~/.claude/plugins/marketplaces/darvium-marketplace/scripts/tickets/check-all/run-check-all.js
```

期待される結果：
- JSON が stdout に出力される
- 全チケットが status: "reviewed" で認識される
- 各チケットの成果物が正しく検出される
- cargo test / clippy / fmt がパスする
- 末尾に CHECK-ALL-SUMMARY 行が stderr に出力される

### Step 5: 結合テスト

darvium プラグインが読み込まれた Claude Code セッション内で `/check-all` を実行し、AI がレポートを正しく解釈表示することを確認する。

---

## 6. 保守ガイド

### 6.1 新チケット追加時

新しい ✅ チケットが追加された場合、スクリプト側の変更は不要。`buildLabelToIdMap()` が spec ファイルの title から自動的に M-label を抽出するため、spec ファイルの title が `"M-X-Y: ..."` 形式に従っていれば自動追従する。

ただし `EXPECTED_CONSTANTS` と `EXPECTED_ERRORS` に新しい ticket_id のエントリを追加する必要がある（定数やエラー型が存在する場合のみ）。

### 6.2 RFC 改訂時

RFC セクションは Darvium-Tickets-v2.3.md から動的に抽出する設計のため、チケット文書が更新されていれば自動追従する。

### 6.3 M-label 命名規則変更時

`buildLabelToIdMap()` 内の正規表現 `/^M-[\d.]+/` を対応するパターンに更新する必要がある。

---

## 7. 付録: テストケース一覧

| # | テスト内容 | 確認方法 |
|---|----------|---------|
| T1 | Darvium-Tickets-v2.3.md が存在しない場合、エラー終了する | 一時的にリネームして実行 |
| T2 | ✅ チケットが 0 件の場合、空のレポートを返す | 一時ファイルでテスト |
| T3 | 全チケットが全てのチェックを PASS する | 実際のプロジェクトで実行 |
| T4 | 期待定数が欠落している場合、WARN が出る | 定数を一時的にリネーム |
| T5 | cargo test が失敗している場合、FAIL と判定される | テストコードに意図的な失敗を仕込む |
| T6 | JSON 出力が `JSON.parse()` 可能である | pipe で検証 |
