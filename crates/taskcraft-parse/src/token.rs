//! Tokenizer: splits input into words, numbers and punctuation with byte offsets.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokKind {
    Word,
    Number,
    Punct,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub text: String,
    pub lower: String,
    pub start: usize,
    pub end: usize,
    pub kind: TokKind,
}

const LEADING: &[char] = &['(', '[', '"', '\'', '\u{201c}', '\u{2018}'];
const TRAILING: &[char] = &[
    ',', '.', '!', '?', ';', ':', ')', ']', '"', '\'', '\u{201d}', '\u{2019}',
];

fn make(text: &str, start: usize) -> Token {
    let kind = match text.chars().next() {
        Some(c) if c.is_ascii_digit() => TokKind::Number,
        Some(c) if text.chars().count() == 1 && c.is_ascii_punctuation() => TokKind::Punct,
        _ => TokKind::Word,
    };
    Token { text: text.to_string(), lower: text.to_lowercase(), start, end: start + text.len(), kind }
}

fn is_url(chunk: &str) -> bool {
    let l = chunk.to_ascii_lowercase();
    l.starts_with("http://") || l.starts_with("https://") || l.starts_with("www.")
}

/// Splits `s` on whitespace, then peels leading/trailing punctuation off each
/// chunk as separate tokens. Internal punctuation stays ("3pm", "10:30",
/// "7/11", "Sant'Eustachio", "to-do"). Offsets are `base + index into s`.
pub(crate) fn tokenize(s: &str, base: usize) -> Vec<Token> {
    let mut out = Vec::new();
    let mut i = 0;
    let bytes = s.len();
    while i < bytes {
        let rest = &s[i..];
        let ws = rest.len() - rest.trim_start().len();
        i += ws;
        if i >= bytes {
            break;
        }
        let rest = &s[i..];
        let len = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let chunk = &rest[..len];
        push_chunk(chunk, base + i, &mut out);
        i += len;
    }
    out
}

fn push_chunk(chunk: &str, start: usize, out: &mut Vec<Token>) {
    let mut lead = Vec::new();
    let mut body = chunk;
    let mut body_start = start;
    while let Some(c) = body.chars().next() {
        if LEADING.contains(&c) && body.len() > c.len_utf8() {
            lead.push(make(&body[..c.len_utf8()], body_start));
            body_start += c.len_utf8();
            body = &body[c.len_utf8()..];
        } else {
            break;
        }
    }
    let mut trail = Vec::new();
    while let Some(c) = body.chars().last() {
        if TRAILING.contains(&c) && body.len() > c.len_utf8() {
            let at = body.len() - c.len_utf8();
            trail.push(make(&body[at..], body_start + at));
            body = &body[..at];
        } else {
            break;
        }
    }
    out.extend(lead);
    if !body.is_empty() {
        if is_url(body) {
            out.push(Token { kind: TokKind::Word, ..make(body, body_start) });
        } else {
            out.push(make(body, body_start));
        }
    }
    out.extend(trail.into_iter().rev());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_keeps_compound_tokens() {
        let s = "Call mom at 3pm, 7/11!";
        let toks = tokenize(s, 0);
        let texts: Vec<&str> = toks.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["Call", "mom", "at", "3pm", ",", "7/11", "!"]);
        for t in &toks {
            assert_eq!(&s[t.start..t.end], t.text);
        }
        assert_eq!(toks[3].kind, TokKind::Number);
        assert_eq!(toks[4].kind, TokKind::Punct);
    }

    #[test]
    fn tokenize_keeps_urls_and_apostrophes() {
        let s = "at Caffè Sant'Eustachio https://meet.example.com/abc.";
        let texts: Vec<String> = tokenize(s, 0).into_iter().map(|t| t.text).collect();
        assert_eq!(texts, ["at", "Caffè", "Sant'Eustachio", "https://meet.example.com/abc", "."]);
    }

    #[test]
    fn tokenize_offsets_respect_base() {
        let toks = tokenize("buy milk", 6);
        assert_eq!((toks[0].start, toks[0].end), (6, 9));
    }
}
