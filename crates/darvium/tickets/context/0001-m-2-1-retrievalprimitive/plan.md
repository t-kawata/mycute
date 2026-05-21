# 実装計画: M-2-1 RetrievalPrimitive 抽象インターフェース及びコアデータ型の定義

## 要件
RetrievalPrimitive トレイトと5つのコアデータ型（QueryRepresentation, RetrievalPolicy, RankedCandidate, CandidateSet, 4つの列挙型）を定義する。

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/types.rs | 修正 | 空構造体→具体化、新規型追加、トレイト追加、Default実装、テストモジュール追加 |

## 実装手順
1. src/types.rs に列挙型4種、QueryRepresentation（+Default）、RetrievalPolicy（+Default）、RankedCandidate、CandidateSet、RetrievalPrimitive トレイトを追加
2. テストモジュールでコンパイル時検証（ダミー実装、型境界チェック、デフォルト値検証）
3. cargo check + cargo test で確認

## レビュー方法
- cargo check が通ること
- 翻訳可能性 grep（名詞始まり関数、1文字変数）
- cargo test 全パス
