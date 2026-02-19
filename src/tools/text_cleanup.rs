use regex::Regex;
use once_cell::sync::Lazy;

static RE_HIRAGANA_FRAGMENT: Lazy<Regex> = Lazy::new(|| {
    // 「。<2文字以内のひらがな>。」にマッチする正規表現
    Regex::new(r"。[\u3041-\u3096]{1,2}。").unwrap()
});

static RE_QUESTION_MARK: Lazy<Regex> = Lazy::new(|| {
    // 「ですか」「ますか」「でしょうか」の直後に「。」がある、または文末（または直後に句読点がない）場合にマッチ
    // すでに「？」がある場合は除外するため、否定先読みを使用するか、置換ロジックで制御する
    // 今回は単純化のため、対象の語句をキャプチャし、その直後の「。」を置換、または文末に「？」を付加する方針
    Regex::new(r"(ですか|ますか|でしょうか|ませんか)([。]?)").unwrap()
});

/// 最終補正後のテキストをクリーンアップする
/// 1. 連続する句点「。。」を「。」に統合
/// 2. 「。<1-2文字のひらがな>。」を除去
/// 3. 「ですか」「ますか」「でしょうか」の後に「？」を付与/補正
pub fn cleanup_final_text(text: &str) -> String {
    let mut result = text.to_string();

    // 1. 2回以上の連続する句点を1つに統合
    while result.contains("。。") {
        result = result.replace("。。", "。");
    }

    // 2. 「。<2文字以内のひらがな>。」のパターンを除去
    // 置換結果は最初の句点「。」に置き換える
    result = RE_HIRAGANA_FRAGMENT.replace_all(&result, "。").to_string();

    // 3. 疑問文の「？」補完
    // 「ですか」「ますか」「でしょうか」の後に「。」がある場合は「？」に置換
    // 末尾で何もなければ「？」を付加
    // 除外リストにある語句（ですから、ですかね等）の場合は「？」を付加せずそのままにする
    let mut cleaned = String::new();
    let mut last_end = 0;

    // 除外リスト（網羅版）
    // 前方一致判定を行うため、最短の接頭辞を定義することで、
    // 「ですかねえ」「ですかねー」などのバリエーションもカバーされる。
    let exclusion_list = [
        "ですから", "ですかね", "ですかな",
        "ますから", "ますかね", "ますかな",
        "でしょうから", "でしょうかね",
        "ませんから", "ませんかね"
    ];
    
    for cap in RE_QUESTION_MARK.captures_iter(&result) {
        let match_range = cap.get(0).unwrap().range();
        
        // マッチ箇所の開始位置から、除外リストのいずれかで始まっているかチェック
        let text_from_match = &result[match_range.start..];
        let is_excluded = exclusion_list.iter().any(|ex| text_from_match.starts_with(ex));

        cleaned.push_str(&result[last_end..match_range.start]);
        
        let question_word = cap.get(1).unwrap().as_str();
        let suffix = cap.get(2).unwrap().as_str();
        
        if is_excluded {
            // 除外対象ならそのまま追加
            cleaned.push_str(question_word);
            cleaned.push_str(suffix);
        } else {
            // 安全なスライシングとチェック
            // 次の文字が「？」（全角・半角問わず）ならそのまま（二重付与防止）
            if result[match_range.end..].starts_with('？') || result[match_range.end..].starts_with('?') {
                cleaned.push_str(question_word);
                cleaned.push_str(suffix);
            } else {
                cleaned.push_str(question_word);
                cleaned.push_str("？");
            }
        }
        
        last_end = match_range.end;
    }
    cleaned.push_str(&result[last_end..]);
    result = cleaned;

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_multiple_periods() {
        assert_eq!(cleanup_final_text("こんにちは。。"), "こんにちは。");
        assert_eq!(cleanup_final_text("こんにちは。。。"), "こんにちは。");
        assert_eq!(cleanup_final_text("こんにちは。。元気ですか。。"), "こんにちは。元気ですか？");
    }

    #[test]
    fn test_cleanup_hiragana_fragment() {
        assert_eq!(cleanup_final_text("今日はいい天気ですね。い。"), "今日はいい天気ですね。");
        assert_eq!(cleanup_final_text("今日はいい天気ですね。です。"), "今日はいい天気ですね。");
        assert_eq!(cleanup_final_text("テスト。あいう。残る"), "テスト。あいう。残る");
    }

    #[test]
    fn test_cleanup_question_mark() {
        assert_eq!(cleanup_final_text("お元気ですか。"), "お元気ですか？");
        assert_eq!(cleanup_final_text("お元気ですか"), "お元気ですか？");
        assert_eq!(cleanup_final_text("お元気ですか？"), "お元気ですか？");
        assert_eq!(cleanup_final_text("お元気ですか。いい天気ですね。"), "お元気ですか？いい天気ですね。");
        assert_eq!(cleanup_final_text("どうでしょうか。ますか。"), "どうでしょうか？ますか？");
    }

    #[test]
    fn test_cleanup_question_mark_exclusions() {
        // 除外要件のテスト
        assert_eq!(cleanup_final_text("ですから。"), "ですから。");
        assert_eq!(cleanup_final_text("ですかね。"), "ですかね。");
        assert_eq!(cleanup_final_text("ですかねえ。"), "ですかねえ。");
        assert_eq!(cleanup_final_text("ですかねー。"), "ですかねー。");
        assert_eq!(cleanup_final_text("ですかな。"), "ですかな。");
        
        assert_eq!(cleanup_final_text("ますから。"), "ますから。");
        assert_eq!(cleanup_final_text("ますかね。"), "ますかね。");
        
        assert_eq!(cleanup_final_text("でしょうから。"), "でしょうから。");
        assert_eq!(cleanup_final_text("でしょうかね。"), "でしょうかね。");

        // 複合ケース
        assert_eq!(cleanup_final_text("元気ですか。ですから。"), "元気ですか？ですから。");
    }

    #[test]
    fn test_cleanup_combined() {
        assert_eq!(cleanup_final_text("今日はいい。い。元気ですか。。"), "今日はいい。元気ですか？");
    }
}
