現在の「厳密一致（Exact Match）」ベースのロジックを、ASRの揺らぎに強い「曖昧一致（Fuzzy Match）」ベースに変換するための実装指示書を作成しました。

この改善の核となる考え方は、**「1文字の不一致で破綻する `find_overlap_len` を捨て、編集距離（Levenshtein Distance）に基づいて『もっともらしい結合点』を探す」**ことです。

***

# ASRテキスト結合ロジック改善：実装指示書

## 1. 補助関数の追加：Fuzzy Overlap 探索
厳密な文字列一致の代わりに、編集距離を用いて「重なりのコスト」が最小になる位置を探す関数を実装します。

### 指針
*   文字単位（`char`）で比較を行います。
*   `edit_distance` クレートなどの外部ライブラリ、もしくは動的計画法による簡易実装を利用します。
*   「前回の末尾」と「今回の先頭」をスライドさせながら、一致率が最も高い（コストが低い）オフセットを返します。

```rust
fn find_fuzzy_overlap(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    if a_chars.is_empty() || b_chars.is_empty() { return 0; }

    let max_overlap = a_chars.len().min(b_chars.len()).min(50); // 探索範囲を限定
    let mut best_overlap = 0;
    let mut min_cost = f32::MAX;

    for len in 1..=max_overlap {
        let suffix = &a_chars[a_chars.len() - len..];
        let prefix = &b_chars[..len.min(b_chars.len())];
        
        // 編集距離を計算（文字単位）
        let dist = edit_distance::edit_distance(
            &suffix.iter().collect::<String>(),
            &prefix.iter().collect::<String>()
        );
        
        // 正規化コスト（距離 / 長さ）
        let cost = dist as f32 / len as f32;
        
        // コストが低く、かつ一定の閾値（例: 0.3以下）ならベスト候補
        if cost < min_cost && cost < 0.3 {
            min_cost = cost;
            best_overlap = len;
        }
    }
    best_overlap
}
```

## 2. Phase 6 の修正：安定マージ（Stability Margin）
窓の「一番新しい部分」は書き換わる可能性が高いため、**窓の末尾数文字を常に「未確定（Volatile）」として残し、確定（Commit）を遅らせます**。

### 変更内容
*   `overlap_len` を使って「重複している範囲」を特定します。
*   「前回のテキスト」のうち、重複範囲より前の部分を `committed_text` に追加します。
*   **追加修正点**: `window_text` が前回と大幅に異なる（`overlap_len` が極端に小さい）場合は、ASRが大幅な修正を行ったと判断し、安易に `committed_text` を増やさず、現在の窓を優先します。

## 3. Phase 7 の修正：句読点と履歴の分離
`punctuation_inserter` が文字列の長さを変えてしまう問題を回避するため、**「論理的なテキスト結合」と「表示用の装飾（句読点）」を明確に分離**します。

### 実装手順
1.  **Raw結合**: `committed_text` + `window_text` で「生の連結テキスト」を一旦作ります。
2.  **句読点適用**: そのコピーに対して句読点を入れます。
3.  **文の切り出し**: 句読点が見つかったら、**「句読点に対応する生のテキストの位置」**を逆算して `session_history` に送ります。
    *   *ヒント*: 句読点が入る前の文字数を記録しておくか、句読点を除去した文字数でカウントします。

## 4. チャンク管理の改善
句読点検知時の `self.chunk_queue.clear()` は、現在「一言目の欠落」を引き起こしている可能性が高いです。

### 修正
*   `clear()` するのではなく、**「句読点までの時間（サンプル数）に相当する古いチャンクだけを pop_front する」**ように変更してください。
*   これにより、次の文の開始部分（句読点の直後の発話）がキューに残るようになります。

***

## 期待される効果
| 課題 | 解決策 |
| :--- | :--- |
| **重複が消えない** | 編集距離により「私は」と「渡しは」が同じ位置だと認識され、二重書きが抑制されます。 |
| **文字が欠落する** | チャンクの部分削除により、文のつなぎ目での音声ロストを防ぎます。 |
| **ガクつき** | 確定を数文字分遅らせる（Stability Margin）ことで、ASRの迷い（Flicker）が表示に反映されるのを防ぎます。 |

まずは `find_fuzzy_overlap` を実装し、Phase 6 の `overlap_len` 算出箇所を差し替えるところから着手することをお勧めします。この「曖昧さの許容」だけで、不整合の 8 割は解消されるはずです。