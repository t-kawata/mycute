# レビュー報告書: M-2-2 SearchBudget / RecursionGuard

## 静的品質チェック
- run-quality-checks.js: 13 issues detected
  - unwrap()/expect() 使用: 8件 (全てテストコード内で意図的)
  - println! 出力: 5件 (OTS-1 観測テスト用で設計上の意図)
- **判定: 通過** (全 issue はテスト/観測の特性上許容)

## 構造整合性チェック
- validate-structure.js: valid, 0 issues
- **判定: 通過**

## 翻訳可能性チェック
- 名詞始まりの関数: 該当なし
- 1文字変数の新規追加: なし (既存クロージャ `|v|` はイディオム範囲内)
- マジックナンバー: 該当なし (LCG 乗数は標準 MMIX 定数、テスト用に限定)
- エラー握りつぶし: なし (全エラーは Result で伝播)
- **判定: 通過**

## テスト検証
- cargo test: 100 passed, 0 failed (M-2-2 新規16テスト含む)
- cargo clippy -- -D warnings: 通過
- cargo fmt --check: 通過
- OTS-1: 10,000 アンサンブル全軌道飽和確認 (τ_relax = 0)

## Acceptance Criteria 充足状況
| AC | 結果 |
|----|------|
| RFC §13.3 フィールド完全一致 | ✅ |
| デフォルトコンストラクタ適切な初期値 | ✅ |
| サチュレーション演算の正しいエラー返却 | ✅ |
| 境界値テスト全パターン通過 | ✅ |
| 既存テスト全通過 | ✅ |
| cargo clippy -- -D warnings 通過 | ✅ |
| cargo fmt 通過 | ✅ |

## 特記事項
### 実装と計画の乖離
Spec Scope item 5 のメソッドシグネチャは以下のように変更された：
```
// Spec 記載 (RFC 非適合)
try_consume_iteration(&self) -> Result<SearchBudgetSnapshot, DarviumError>

// 実装 (RFC 責務分離に適合)
try_consume_iteration(&self, snapshot: &mut SearchBudgetSnapshot) -> Result<(), DarviumError>
```
変更理由: 累積使用量を追跡するには同一スナップショットへの可変参照が必要。Spec のシグネチャでは毎回新しいスナップショットが返るため累積が不可能。この変更は RFC §13.3 の SearchBudget（上限）と SearchBudgetSnapshot（使用量）の責務分離に忠実である。

### Boy Scout 改善
- lib.rs に SearchBudget/RecursionGuard/SearchBudgetSnapshot の pub use 追加
- SearchBudget の &mut self を &self に修正 (Boy Scout レビュー時発見)

## 総評
**PASS**. 全ての品質チェックを通過。Spec からのメソッドシグネチャ変更は RFC 準拠のための正当な設計判断。レビュー中に発見した &mut self の改善も実施済み。
