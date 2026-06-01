# レビュー報告書: #158 SubWorkflow 親子関係に基づく GC 生存ガード

## 静的品質チェック
- 新規コード: unwrap/expect なし、println! なし、単一文字変数なし
- ガード条件の bounds check (`parent_id < population.len()`) あり

## 観測検証
- ✅ validate-observation.js 通過（valid: true, issues: 0）

## 構造整合性チェック
- ✅ validate-structure.js 通過（valid: true, issues: 0）

## 翻訳可能性チェック
- ✅ 新規フィールド `parent_id` — ドメイン概念「親のID」を直接表現
- ✅ ガード条件の変数名 `parent_id`, `population` — 翻訳可能
- ✅ コメント: なぜ親生存中に子を殺さないか（dangling reference防止）を説明

## RFC 交叉参照
- RFC の数式・型定義に変更なし。ガード条件の追加のみ
- RFC との矛盾なし

## Acceptance Criteria 達成状況
- [x] MemoizedGraph.parent_id 追加、0 初期化
- [x] 親生存中の子が絶対に死なない
- [x] 親死亡後は通常 GC 判定
- [x] 既存テスト全通過（1394 passed, 0 failed）

## 所見
- TC2/TC3 で PersonId=0 の特殊ケースに起因するテストバグがあったが修正済み
- 推移的な保護（親の親）は未実装だが、scope 外として定義済み
