use anyhow::{anyhow, Result};
use lindera::tokenizer::Tokenizer;

use super::lindera_util;
use crate::stt_config::LocaleCode;

#[derive(Debug, Clone)]
struct TokenInfo {
    surface: String,
    pos: String,
    pos_detail1: String,
    conjugation_form: String,
}

pub struct PunctuationMachine {
    tokenizer: Tokenizer,
}

impl PunctuationMachine {
    pub fn new() -> Result<Self> {
        let tokenizer = lindera_util::get_tokenizer()?;
        Ok(Self { tokenizer })
    }

    pub fn insert(&self, text: &str, locale: &LocaleCode) -> Result<String> {
        self.insert_with_context(text, "", locale, false)
    }

    pub fn insert_with_context(
        &self,
        text: &str,
        context: &str,
        locale: &LocaleCode,
        allow_terminal_punctuation: bool,
    ) -> Result<String> {
        if text.is_empty() {
            return Ok(String::new());
        }

        // Context cleaning (must match text cleaning to keep offsets consistent conceptually)
        // However, we only care about the cleaned length for offset calculation.
        let (text_clean, context_clean) = if locale == &LocaleCode::Ja {
            (
                text.replace("?", "？").replace("!", "！"),
                context.replace("?", "？").replace("!", "！"), // Clean context too to match tokenizer expectations
            )
        } else {
            (text.to_string(), context.to_string())
        };

        if locale != &LocaleCode::Ja {
            return Ok(text_clean);
        }

        // Combine for tokenization
        let full_text = format!("{}{}", context_clean, text_clean);
        let context_len = context_clean.len();

        let mut tokens_raw = self
            .tokenizer
            .tokenize(&full_text)
            .map_err(|e| anyhow!("Tokenization failed: {}", e))?;

        let mut tokens: Vec<TokenInfo> = tokens_raw
            .iter_mut()
            .map(|token| {
                let surface = token.surface.to_string();
                let details = token.details();
                TokenInfo {
                    surface,
                    pos: details.first().copied().unwrap_or("").to_string(),
                    pos_detail1: details.get(1).copied().unwrap_or("").to_string(),
                    conjugation_form: details.get(5).copied().unwrap_or("").to_string(),
                }
            })
            .collect();

        // 1.5. Voice Command Replacement (まる -> 。, てん -> 、)
        // Perform in-place replacement on tokens before processing
        for i in 0..tokens.len() {
            let token = &mut tokens[i];

            // Only targets Nouns, Interjections, or Adverbs (limited scope to avoid parts of words)
            // e.g. "丸" (noun), "まる" (noun/adverb), "てん" (noun), "点" (noun suffix?)
            // We check surface form primarily.
            if ["名詞", "感動詞", "副詞"].contains(&token.pos.as_str()) {
                if ["まる", "丸", "マル"].contains(&token.surface.as_str()) {
                    token.surface = "。".to_string();
                    token.pos = "補助記号".to_string(); // Treat as symbol
                    token.pos_detail1 = "句点".to_string();
                } else if ["てん", "点", "天", "テン"].contains(&token.surface.as_str()) {
                    token.surface = "、".to_string();
                    token.pos = "補助記号".to_string(); // Treat as symbol
                    token.pos_detail1 = "読点".to_string();
                }
            }
        }

        let mut result = String::new();
        let mut current_offset = 0;

        for i in 0..tokens.len() {
            let current = &tokens[i];
            let token_len = current.surface.len();

            // このトークンが 'text' (対象) の一部である場合、つまり context_len 以降から始まる場合のみ出力する
            // 境界をまたぐトークンの場合（稀）、大部分が新しい部分に属していれば含めるか？
            // 厳密なチェック: current_offset >= context_len
            if current_offset >= context_len {
                result.push_str(&current.surface);

                if self.should_insert_question_ja(i, &tokens, allow_terminal_punctuation) {
                    result.push('？');
                } else if self.should_insert_period_ja(i, &tokens, allow_terminal_punctuation) {
                    result.push('。');
                }
            } else if current_offset + token_len > context_len {
                // Straddling token (extremely rare with proper boundaries).
                // We output the partial surface corresponding to the new text.
                let overlap = context_len - current_offset;
                if overlap < token_len {
                    let partial = &current.surface[overlap..];
                    result.push_str(partial);

                    // Punctuation logic: if the token ends in the new part, we evaluate punctuation
                    if self.should_insert_question_ja(i, &tokens, allow_terminal_punctuation) {
                        result.push('？');
                    } else if self.should_insert_period_ja(i, &tokens, allow_terminal_punctuation) {
                        result.push('。');
                    }
                }
            }

            current_offset += token_len;
        }

        Ok(result)
    }

    fn is_sentence_starter(&self, token: &TokenInfo) -> bool {
        let starters = [
            "はい",
            "ええ",
            "うん",
            "いや",
            "まあ",
            "さて",
            "そう",
            "でも",
            "しかし",
            "ただ",
            "じゃあ",
            "では",
            "じゃ",
            "それ",
            "あと",
            "もう",
            "また",
            "そして",
            "だから",
        ];
        if ["感動詞", "接続詞", "副詞"].contains(&token.pos.as_str()) {
            return true;
        }
        starters.contains(&token.surface.as_str())
    }

    fn should_insert_period_ja(
        &self,
        index: usize,
        tokens: &[TokenInfo],
        allow_terminal_punctuation: bool,
    ) -> bool {
        // 【最重要】ライブエッジ（最後尾）には絶対に打たない
        // ただし、allow_terminal_punctuation が true の場合（タイムアウト時）は許可する
        if index >= tokens.len() - 1 {
            return allow_terminal_punctuation;
        }

        let current = &tokens[index];
        // Safely get next token (will be None if index is last element)
        let next_opt = tokens.get(index + 1);

        // 継続表現の絶対禁止
        if current.pos == "助詞" && ["接続助詞", "格助詞"].contains(&current.pos_detail1.as_str())
        {
            if [
                "が",
                "けど",
                "けれど",
                "けれども",
                "し",
                "から",
                "ので",
                "のに",
                "て",
                "で",
            ]
            .contains(&current.surface.as_str())
            {
                return false;
            }
        }

        // 0. 引用の「と」が続く場合は絶対に打たない (〜していこう。と思います 等を防ぐ)
        if let Some(next) = next_opt {
            if next.surface == "と" && next.pos == "助詞" {
                return false;
            }
        }

        // 1. 丁寧語(です・ます) や依頼(ください)の終止
        if current.pos == "助動詞" || current.pos == "動詞" {
            let polite = [
                "です", "ます", "でした", "ました", "ございます", "でしょう",
                "ください", "くださいませ", "ません", "ありません",
            ];
            if polite.contains(&current.surface.as_str()) {
                // 次が終助詞や接続助詞でなければ終わりとみなす
                if let Some(next) = next_opt {
                    return next.pos != "助詞";
                } else {
                    // 末尾（nextなし）の場合
                    // allow_terminal_punctuation が true ならここで打つ
                    return allow_terminal_punctuation;
                }
            }
        }

        // 2. 終助詞 (ね・よ・わ・な)
        if current.pos == "助詞" && current.pos_detail1 == "終助詞" {
            if ["ね", "よ", "わ", "な", "よね", "わね"].contains(&current.surface.as_str())
            {
                // 次が助詞でないなら終わりとみなす
                if let Some(next) = next_opt {
                    return next.pos != "助詞";
                } else {
                    return allow_terminal_punctuation;
                }
            }
        }

        // 3. 自立語による遡及判定 (強い開始語が来たら、前を閉じる)
        if (current.pos == "動詞" || current.pos == "形容詞" || current.pos == "助動詞")
            && (current.conjugation_form.contains("基本形")
                || current.conjugation_form.contains("タ形"))
        {
            if let Some(next) = next_opt {
                // 次が「接続詞」「感動詞」「副詞」などの文頭要素なら切る
                if self.is_sentence_starter(next) {
                    return true;
                }
            } else {
                // 末尾に自立語が来た場合 (TimeOut時のみ)
                return allow_terminal_punctuation;
            }

            // 次が名詞や動詞などで、明らかに別の文節が始まっている場合
            // (ただし連体修飾の可能性があるので慎重に。接続助詞がないなら切れる可能性が高いが、
            //  安全側に倒して「強い開始語」以外はスルーするのが無難か？計画では「自立語による遡及」とある)
            //  -> 計画に従い、強い開始語のみに限る
        }

        false
    }

    fn should_insert_question_ja(
        &self,
        index: usize,
        tokens: &[TokenInfo],
        allow_terminal_punctuation: bool,
    ) -> bool {
        // 【最重要】ライブエッジ（最後尾）には絶対に打たない
        // ただし、allow_terminal_punctuation が true の場合（タイムアウト時）は許可する
        // → 許可するということは、「index check で return false しない」だけであり、
        //   「無条件に true を返す」わけではない。下の判定ロジックを通す必要がある。
        if index >= tokens.len() - 1 {
            if !allow_terminal_punctuation {
                return false;
            }
            // タイムアウト時はここを通過して下の判定へ進む
        }

        let current = &tokens[index];
        // Safely get next token
        let next_opt = tokens.get(index + 1);

        // 明確な疑問終助詞のみ
        let interrogatives = ["か", "かい", "だい", "かな", "かしら"];
        if current.pos == "助詞" && interrogatives.contains(&current.surface.as_str()) {
            if let Some(next) = next_opt {
                return next.pos != "助詞";
            } else {
                // 末尾かつフラグ有効なら（質問で終わる場合）
                return allow_terminal_punctuation;
            }
        }

        false
    }
}
