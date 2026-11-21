use std::path::Path;

use super::LOG_TAIL_LINES;

const HOME_MARK: &str = "~";
const REDACTED: &str = "<redacted>";
const RIOT_KEYS: [&str; 7] = [
    "riotidgamename",
    "riotidtagline",
    "summonername",
    "riotid",
    "summoner",
    "gamename",
    "tagline",
];

pub fn tail_lines(text: &str, count: usize) -> &str {
    let mut start = 0;
    let mut seen = 0;
    let body = text.strip_suffix('\n').unwrap_or(text);
    for (at, byte) in body.bytes().enumerate().rev() {
        if byte == b'\n' {
            seen += 1;
            if seen == count {
                start = at + 1;
                break;
            }
        }
    }
    if seen < count { text } else { &text[start..] }
}

pub fn home_variants(home: &Path) -> Vec<String> {
    let plain = home
        .to_string_lossy()
        .trim_end_matches(['/', '\\'])
        .to_owned();
    if plain.is_empty() {
        return Vec::new();
    }
    let mut variants = vec![
        plain.replace('\\', "\\\\"),
        plain.replace('\\', "/"),
        plain.replace('/', "\\"),
        plain,
    ];
    variants.sort_by_key(|variant| std::cmp::Reverse(variant.len()));
    variants.dedup();
    variants
}

pub fn scrub_logs(raw: &str, home_dirs: &[String]) -> String {
    let mut text = tail_lines(raw, LOG_TAIL_LINES).to_owned();
    for home in home_dirs.iter().filter(|home| !home.is_empty()) {
        text = replace_ignoring_case(&text, home, HOME_MARK);
    }
    redact_riot_values(&text)
}

fn replace_ignoring_case(text: &str, needle: &str, with: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while let Some(found) = lower[at..].find(&needle) {
        let start = at + found;
        out.push_str(&text[at..start]);
        out.push_str(with);
        at = start + needle.len();
    }
    out.push_str(&text[at..]);
    out
}

fn redact_riot_values(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while let Some((start, end)) = next_value(text, &lower, at) {
        out.push_str(&text[at..start]);
        out.push_str(REDACTED);
        at = end;
    }
    out.push_str(&text[at..]);
    out
}

fn next_value(text: &str, lower: &str, from: usize) -> Option<(usize, usize)> {
    (from..text.len())
        .filter(|at| text.is_char_boundary(*at))
        .find_map(|at| {
            RIOT_KEYS
                .iter()
                .filter(|key| lower[at..].starts_with(**key))
                .find_map(|key| value_after_key(text, at, key.len()))
        })
}

fn value_after_key(text: &str, key_at: usize, key_len: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    if key_at > 0 && bytes[key_at - 1].is_ascii_alphanumeric() {
        return None;
    }
    let mut at = key_at + key_len;
    if bytes.get(at) == Some(&b'"') {
        at += 1;
    }
    at = skip_spaces(bytes, at);
    if !matches!(bytes.get(at), Some(b'=' | b':')) {
        return None;
    }
    at = skip_spaces(bytes, at + 1);
    if bytes.get(at) == Some(&b'"') {
        return Some(quoted_value(text, at + 1));
    }
    let end = text[at..]
        .find(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '}' | ']' | ')' | '"'))
        .map_or(text.len(), |offset| at + offset);
    (end > at).then_some((at, end))
}

fn quoted_value(text: &str, start: usize) -> (usize, usize) {
    let mut escaped = false;
    for (offset, c) in text[start..].char_indices() {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return (start, start + offset);
        }
    }
    (start, text.len())
}

fn skip_spaces(bytes: &[u8], mut at: usize) -> usize {
    while bytes.get(at) == Some(&b' ') {
        at += 1;
    }
    at
}
