//! 關鍵字搜尋：中文分詞簡易版＋不分大小寫＋標點空白容錯。
//!
//! 作法（刻意簡單、跨 DB 通用，資料小所以內存過濾）：
//! - 正規化：轉小寫，去掉空白與常見標點。
//! - 切 token：ASCII 按非字母數字切詞；CJK（中日韓字符）逐字成 token。
//! - 匹配：query 所有 token 都出現在（標題＋描述）裡才算中（AND）。
//!   例：「高麗菜水餃」→ 高/麗/菜/水/餃；「poke bowl」→ poke/bowl；「水餃 20顆」→ 水/餃/20/顆。

/// 分類白名單（前後端＋種子共用同一組字串）
pub const CATEGORIES: &[&str] = &["fresh", "food", "daily", "service", "other"];

pub fn valid_category(c: &str) -> bool {
    CATEGORIES.contains(&c)
}

fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4e00}'..='\u{9fff}'   // CJK 統一表意文字
        | '\u{3400}'..='\u{4dbf}'   // 擴展 A
        | '\u{3040}'..='\u{309f}'   // 平假名
        | '\u{30a0}'..='\u{30ff}'   // 片假名
        | '\u{ac00}'..='\u{d7af}'   // 諺文
        | '\u{ff00}'..='\u{ffef}'   // 全形 ASCII/標點
    )
}

/// 全形轉半形（只處理 ASCII 範圍，如 Ａ→A、，→,），其餘原樣
fn narrow(c: char) -> Option<char> {
    let n = c as u32;
    if (0xff01..=0xff5e).contains(&n) {
        char::from_u32(n - 0xfee0)
    } else if c == '\u{3000}' {
        Some(' ')
    } else {
        Some(c)
    }
}

fn is_punct(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            ',' | '.'
                | '，'
                | '。'
                | '、'
                | '；'
                | ';'
                | '：'
                | ':'
                | '！'
                | '!'
                | '？'
                | '?'
                | '('
                | ')'
                | '（'
                | '）'
                | '['
                | ']'
                | '【'
                | '】'
                | '"'
                | '\''
                | '「'
                | '」'
                | '『'
                | '』'
                | '·'
                | '・'
                | '-'
                | '_'
                | '/'
                | '~'
                | '～'
        )
}

/// 查詢字串 → tokens（已小寫、去重、保序）
pub fn tokenize(q: &str) -> Vec<String> {
    let mut tokens: Vec<String> = vec![];
    let mut buf = String::new();
    let push_buf = |buf: &mut String, tokens: &mut Vec<String>| {
        if !buf.is_empty() {
            let t = buf.to_lowercase();
            if !tokens.contains(&t) {
                tokens.push(t);
            }
            buf.clear();
        }
    };
    for c in q.chars().filter_map(narrow) {
        if is_punct(c) {
            push_buf(&mut buf, &mut tokens);
        } else if is_cjk(c) {
            push_buf(&mut buf, &mut tokens);
            let t = c.to_lowercase().to_string();
            if !tokens.contains(&t) {
                tokens.push(t);
            }
        } else {
            buf.push(c);
        }
    }
    push_buf(&mut buf, &mut tokens);
    tokens
}

/// haystack（標題＋描述拼起來）是否包含全部 token
pub fn matches(haystack: &str, tokens: &[String]) -> bool {
    if tokens.is_empty() {
        return true;
    }
    let h: String = haystack.chars().filter_map(narrow).collect();
    let h = h.to_lowercase();
    tokens.iter().all(|t| h.contains(t.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_cjk_chars() {
        assert_eq!(tokenize("高麗菜水餃"), vec!["高", "麗", "菜", "水", "餃"]);
    }

    #[test]
    fn tokenize_mixed() {
        // 空白標點容錯＋英文小寫＋去重
        assert_eq!(tokenize("水餃 20顆"), vec!["水", "餃", "20", "顆"]);
        assert_eq!(tokenize("Poke Bowl"), vec!["poke", "bowl"]);
        assert_eq!(tokenize("柴犬、柴犬"), vec!["柴", "犬"]);
    }

    #[test]
    fn tokenize_fullwidth() {
        assert_eq!(tokenize("９：００"), vec!["9", "00"]);
    }

    #[test]
    fn match_and_semantics() {
        let hay = "高麗菜水餃 20 顆 冷凍面交";
        assert!(matches(hay, &tokenize("高麗菜")));
        assert!(matches(hay, &tokenize("水餃 20顆")));
        assert!(!matches(hay, &tokenize("韭菜水餃"))); // 韭不在
        assert!(matches(hay, &tokenize("")));
    }

    #[test]
    fn match_english_and_desc() {
        let hay = "Beginner Surf Lesson 1-hour lesson, board included";
        assert!(matches(hay, &tokenize("surf lesson")));
        assert!(matches(hay, &tokenize("BOARD"))); // 不分大小寫
        assert!(!matches(hay, &tokenize("surf yoga")));
    }
}
