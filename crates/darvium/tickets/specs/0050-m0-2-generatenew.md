---
ticket_id: 50
title: M0-2: GenerateNew 選択時のレビュー強制・安全ガードロジックの検証
slug: m0-2-generatenew
status: reviewed
created_at: 2026-05-23
updated_at: 2026-05-23
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0050-m0-2-generatenew/observation-20260523-134545.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0050-m0-2-generatenew/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0050-m0-2-generatenew/review.md
---

# M0-2: `GenerateNew` 選択時のレビュー強制・安全ガードロジックの検証

## Summary

`GenerateNew` が選択された際、対象ミッションの副作用プロファイル（`SideEffectSet`）と plane 属性（Production / Training / SafeSandbox）に基づいて、production plane では即座に実環境へ投入せず人間承認待ち状態へバイパスするガードロジックを実装する。training plane / safe sandbox に限り、明示的に安全と分類された scope に対してのみ自動承認を許容する分岐を設計に含める。

## Background

### 対象不変条件 / 規範

- **RFC §13.6**: ガード条件 — side-effect safety invariant に反する SearchStep 遷移（review-gated でない実プロバイダ呼び出しを `GenerateNew` で即採択する経路）は `UnsafeSearchTransition` として拒否すること (MUST)。
- **RFC §13.6**: `GenerateNew` および `ComposeExisting` の実 execution は review-gated とし、少なくとも M3 までは proposal validity のみを評価対象としてよい (SHOULD)。
- **RFC §16A (v2.3 補足)**: Training Plane は safe sandbox scope に限定して optional Auto-Approval Exception Policy を導入してもよい (MAY)。この例外 policy は namespace、artifact kind、side-effect envelope、resource budget、external write 禁止、production promotion 不可の条件で bounded に定義されなければならない。
- **RFC §16A**: Auto-approved training artifact は、auto-approved である事実、適用された policy ID、理由、scope boundary、実行 trace を audit log に残さなければならない (MUST)。この optional policy は training trust / production trust separation、promotion gate、human override 権限を弱めてはならない (MUST NOT)。
- **RFC §6.1**: `SideEffectSet` 構造体（`writes_external_api`, `sends_notification`, `has_hitl_communicate`, `modifies_persistent_state`, `irreversible`, `risk_score`）が副作用の定量的評価単位。

### 現状のコード

- `SearchOutcome::NeedsHumanReview { reason: String }` は既に `src/types.rs:4218` で定義済み。
- `SearchOutcome::GenerateNew { proposal: WorkflowGraph }` も同ファイルで定義済み。
- `DarviumError::SearchValidation(String)` がエラー型として存在（`src/error.rs:33`）。`UnsafeSearchTransition` はこのバリアントで表現可能。
- `SideEffectSet` 構造体は**未実装**（RFC §6.1 には定義があるがコードには存在しない）。
- `PlaneKind` 列挙型は**未定義**。
- M0-1 で実装した `composition.rs` のバリデーションパターン（`validate_*` → `Err(DarviumError::...)`）が直接の参考になる。

### 過去の観察レポートからの知見

- `tickets/context/0049-m0-1-compositionplan/observation-20260523-132423.md` — 静的バリデータのパターン。`Err(DarviumError::VariableScopeViolation)` 返却による不変条件強制の実装・観測手法。
- `tickets/context/0046-ag-06-ag-07/observation-20260523-103051.md` — バリデーションハードゲート（AG-06/AG-07）の全弾ブロック観測テスト。副作用ゲートと同様の 100% ブロック検証パターンの参考になる。

## Scope

### 実装スコープ

1. **`SideEffectSet` 構造体の定義** (`src/types.rs`)
   - RFC §6.1 準拠: `writes_external_api: bool`, `sends_notification: bool`, `has_hitl_communicate: bool`, `modifies_persistent_state: bool`, `irreversible: bool`, `risk_score: f32`
   - `#[derive(Debug, Clone, PartialEq, Default)]`
   - `contains(&self, required: &SideEffectSet) -> bool` メソッド（Stage 0 フィルタ用）
   - `is_safe_for_auto_approval(&self) -> bool` メソッド（外部書き込み・不可逆副作用がないことを確認）

2. **`PlaneKind` 列挙型の定義** (`src/types.rs`)
   - バリアント: `Production`, `Training`, `SafeSandbox`
   - `SafeSandbox` は Training Plane 下の safe-scoped sandbox を表す
   - `#[derive(Debug, Clone, Copy, PartialEq)]`

3. **`SafeSandboxScope` 構造体の定義** (`src/types.rs`)
   - `namespace: String`, `artifact_kind: String`, `allowed_side_effects: SideEffectSet`
   - Auto-Approval Exception Policy の scope boundary を表現
   - `#[derive(Debug, Clone, PartialEq)]`

4. **`check_generate_new_safety` ガード関数の実装**（`src/guard.rs` 新規ファイル）
   - シグネチャ: `pub fn check_generate_new_safety(side_effects: &SideEffectSet, plane: PlaneKind, scope: Option<&SafeSandboxScope>) -> Result<(), DarviumError>`
   - ロジック:
     - `Production` plane: 副作用の有無にかかわらず `Err(DarviumError::SearchValidation("UnsafeSearchTransition: GenerateNew in production requires human review"))` を返す
     - `Training` plane: `side_effects.is_safe_for_auto_approval()` が true かつオプションの scope 条件を満たせば `Ok(())`、そうでなければ `Err`
     - `SafeSandbox` plane: 渡された `SafeSandboxScope` の `allowed_side_effects` で包含チェック（`side_effects.contains(&scope.allowed_side_effects)`）を通れば `Ok(())`

5. **`guard_new_proposal_or_review` 公開関数の実装** (`src/guard.rs`)
   - `GenerateNew` 選択後の高レベル決定関数
   - 安全なら `SearchOutcome::GenerateNew { proposal }` を返す
   - 不安全なら `SearchOutcome::NeedsHumanReview { reason: "GenerateNew in production requires human review: side-effect profile ...".into() }` を返す
   - エラーは `DarviumError::SearchValidation` で返す（プログラム的誤用の検出用）
   - シグネチャ: `pub fn guard_new_proposal_or_review(proposal: WorkflowGraph, side_effects: &SideEffectSet, plane: PlaneKind, scope: Option<&SafeSandboxScope>) -> Result<SearchOutcome, DarviumError>`

6. **`src/lib.rs` への登録**
   - `pub mod guard;` の追加
   - `pub use guard::guard_new_proposal_or_review;` の追加（`check_generate_new_safety` は内部使用のため非公開）

### Non-scope

- `ComposeExisting` のレビューガードは本チケットの対象外（RFC §13.6 で同様のガード要件があるが、M0-3 以降で対応）。
- 実際の HumanChannel へのキューイング・通知ロジックは M1 系チケットの対象。
- `TrustAuditLog` への自動承認記録は M1-2 以降で対応。
- `AdminFastTrack` による強制信頼値更新は M1-2 の対象。

## Investigation

### コードベース調査結果

| 発見事項 | ファイル | 行 |
|---------|----------|-----|
| `SearchOutcome::GenerateNew` 定義済み | `src/types.rs` | 4214 |
| `SearchOutcome::NeedsHumanReview` 定義済み | `src/types.rs` | 4218 |
| `DarviumError::SearchValidation` 定義済み（UnsafeSearchTransition 表現可） | `src/error.rs` | 33 |
| `SideEffectSet` 未実装（RFC §6.1 にのみ存在） | RFC v2.3-final.md | 405-427 |
| `PlaneKind` 未定義 | — | — |
| M0-1 静的バリデーションパターン（validate → Err） | `src/composition.rs` | 24-37 |
| 定数 `DEFAULT_MAX_ITERATIONS` 等（Environment Policy Knob） | `src/constants.rs` | 54-60 |

### アーキテクチャ上の決定

- ガードロジックは新規ファイル `src/guard.rs` に実装する（`src/composition.rs` は組成プラン検証専用、汎用ガードロジックは分離）。
- `UnsafeSearchTransition` は専用エラー型を追加せず、既存の `DarviumError::SearchValidation(String)` で表現する（RFC の命名はエラーメッセージ文字列として保持）。
- `guard_new_proposal_or_review` はエラーケースでも `NeedsHumanReview` を返せる二重出力を持つ（プログラム的誤用はエラー、設計通りのルーティングは正常系として扱う）。

## Test Plan

### ユニットテスト計画（`src/guard.rs` 内 `mod tests`）

#### T1: Production plane — 全副作用パターンでブロック

- **条件:** `PlaneKind::Production`, 8 パターンの `SideEffectSet`（3 主要 bool の全組合せ）
- **期待:** 全パターンで `guard_new_proposal_or_review` が `SearchOutcome::NeedsHumanReview` を返す
- **検証:** `assert!(matches!(result, Ok(SearchOutcome::NeedsHumanReview { .. })))`

#### T2: Training plane — 安全な副作用で auto-approval

- **条件:** `PlaneKind::Training`, `SideEffectSet { writes_external_api: false, sends_notification: false, modifies_persistent_state: false, irreversible: false, .. }`
- **期待:** `Ok(SearchOutcome::GenerateNew { .. })` が返る
- **検証:** `assert!(matches!(result, Ok(SearchOutcome::GenerateNew { .. })))`

#### T3: Training plane — 不安全な副作用でブロック

- **条件:** `PlaneKind::Training`, `SideEffectSet { writes_external_api: true, .. }`（外部 API 書き込みあり）
- **期待:** `Err(DarviumError::SearchValidation( .. ))` が返る
- **検証:** `assert!(result.is_err())` + エラーメッセージに "UnsafeSearchTransition" を含む

#### T4: Training plane — `irreversible: true` でブロック

- **条件:** `PlaneKind::Training`, `SideEffectSet { irreversible: true, .. }`
- **期待:** `Err(DarviumError::SearchValidation( .. ))`
- **検証:** 同上

#### T5: SafeSandbox — 許可範囲内で auto-approval

- **条件:** `PlaneKind::SafeSandbox`, `scope = Some(&SafeSandboxScope { allowed_side_effects: SideEffectSet { writes_external_api: false, .. }, .. })`, 対象の副作用が許可範囲内
- **期待:** `Ok(SearchOutcome::GenerateNew { .. })`

#### T6: SafeSandbox — 許可範囲外でブロック

- **条件:** `PlaneKind::SafeSandbox`, 対象の副作用が scope の許可範囲を超過
- **期待:** `Err(DarviumError::SearchValidation( .. ))`

#### T7: `SideEffectSet::contains` 包含関係

- **条件:** `self = { writes_external_api: true }`, `required = { writes_external_api: true }` → true
- **条件:** `self = { writes_external_api: false }`, `required = { writes_external_api: true }` → false
- **条件:** `self = { writes_external_api: true, sends_notification: false }`, `required = { writes_external_api: true, sends_notification: true }` → false
- **条件:** 空の required は常に true
- **検証:** 全条件の網羅的チェック

#### T8: `SideEffectSet::is_safe_for_auto_approval` 判定

- **条件:** `writes_external_api: false, irreversible: false` → true
- **条件:** `writes_external_api: true` → false
- **条件:** `irreversible: true` → false
- **条件:** `modifies_persistent_state: true` → true（sandbox 内では許容）
- **条件:** `sends_notification: true` → true
- **検証:** 全条件の網羅的チェック

#### T9: `PlaneKind` のデバッグ表現

- **条件:** 全 3 バリアントの `Debug` 出力が readable
- **検証:** `assert!(!format!("{:?}", ...).is_empty())`

#### T10: 空の `SideEffectSet` デフォルト

- **条件:** `SideEffectSet::default()` が全 bool false, risk_score 0.0 であること
- **検証:** `assert_eq!(default.risk_score, 0.0)`, 全 bool が false

### 観測テスト（OTS）

#### OTS-1: 副作用ベクトル空間の全軌道閉包性

- **計装:** 5 bool の全 2^5 = 32 パターン × `risk_score ∈ [0.0, 1.0]` を 0.1 刻み（11 段階）= 352 の組合せを `PlaneKind::Production` で投入
- **観測:** `guard_new_proposal_or_review` の戻り値が `NeedsHumanReview` である割合が 1.0（100%）であることを確認
- **出力:** 全パターンの結果テーブル（CSV 形式、`--nocapture`）

#### OTS-2: Training plane 通過率とリスクスコアの関係

- **計装:** 32 パターン × risk_score 11 段階を `PlaneKind::Training` + デフォルト scope で投入
- **観測:** `is_safe_for_auto_approval` 通過条件と実際の auto-approval 率の一致
- **出力:** risk_score 分布別の auto-approval / review-routed 比率ヒストグラム

#### OTS-3: SafeSandbox scope 境界感度

- **計装:** `SafeSandboxScope` の `allowed_side_effects` を 5 次元 bool 空間で sweep（各次元を 1 つずつ反転）
- **観測:** scope の包含境界での通過/拒否の一致率が 1.0 であること + 境界付近で曖昧な判定がないこと
- **試行数:** 5 次元 × 各 2 値 = 10 通りの scope, 各 32 パターンの副作用 = 320 試行

## 計装方法・観測対象

### 計装方法

- `src/guard.rs` の `mod tests` 内に全テストを実装
- 計装プローブ: `println!` + `--nocapture` で構造化出力（CSV/テーブル形式）
- 固定シードは不要（決定論的ロジックのため）

### 観測対象

- OTS-1: 352 試行 × 全 Production ブロック（閉包性 100%）
- OTS-2: 352 試行 × Training 通過率（safe 条件との一致率 100%）
- OTS-3: 320 試行 × SafeSandbox 境界（境界一致率 100%）

### 較正計画

本チケットはパラメータ較正を伴わない（純粋な決定論的ガードロジックの実装）。ただし、将来 `SAFE_SCOPE_AUTO_APPROVAL_ENABLED` 等の Policy Knob を `constants.rs` に追加する可能性に備え、実装は Boolean フラグで auto-approval 有効/無効を切り替え可能にしておく。

## Boy Scout Rule — 翻訳可能性計画

本チケットで新規作成するコードにおいて、以下を徹底する：

- **関数名は動詞句**: `check_generate_new_safety`（「GenerateNew の安全性を検査する」）、`guard_new_proposal_or_review`（「新規提案をガードするかレビューに回す」）
- **変数名はドメイン概念**: `plane_kind`（プレーン種別）、`side_effects`（副作用セット）、`scope_boundary`（スコープ境界）
- **一関数一責務**: ガード判定関数は判定のみ、高レベル決定関数はルーティングのみ
- **ハードコード値の禁止**: エラーメッセージは定数 or フォーマット文字列で、ガード条件の判定ロジックにマジックナンバーを含めない
- **エラー握りつぶし禁止**: 全 `Result` を `?` または明示的な `match` で伝播

既存コードへの影響は最小限に留める（新しい `src/guard.rs` の追加のみ、既存ファイルの修正は `src/types.rs` への型追加と `src/lib.rs` へのモジュール登録・公開 API 追加に限定）。

## Acceptance Criteria

- [ ] `SideEffectSet` 構造体が RFC §6.1 に準拠して定義されている（5 bool + risk_score + contains メソッド）
- [ ] `PlaneKind` 列挙型（Production / Training / SafeSandbox）が定義されている
- [ ] `SafeSandboxScope` 構造体（namespace, artifact_kind, allowed_side_effects）が定義されている
- [ ] `check_generate_new_safety` ガード関数が実装されている
- [ ] `guard_new_proposal_or_review` 公開関数が実装されている
- [ ] T1-T10 の全ユニットテストが PASS
- [ ] OTS-1/OTS-2/OTS-3 の全観測テストが PASS（閉包性 100%）
- [ ] RFC §13.6 および §16A との無矛盾確認完了
- [ ] 既存の全テストが通過している（後退なし）
- [ ] 翻訳可能性（関数名は動詞句、変数名はドメイン概念、一関数一責務）を満たしている

## Notes

- `plan_path`: /plan-ticket が plan.md 作成後に frontmatter に更新する
- `implementation_path`: /start-ticket が implementation.md 作成後に frontmatter に更新する
- `review_report_path`: /review-ticket が review.md 作成後に frontmatter に更新する
- `observation_report_path`: /start-ticket が observation-YYYYMMDD-HHmmss.md 作成後に frontmatter に最新パスを更新する

### 成果物

- 計画: context/0050-m0-2-generatenew/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0050-m0-2-generatenew/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0050-m0-2-generatenew/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0050-m0-2-generatenew/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
