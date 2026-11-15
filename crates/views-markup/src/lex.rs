//! Byte classes, brace matching, whitespace and entities.

/// 1-based line and column (in characters) of a byte offset.
pub fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let mut off = offset.min(src.len());
    while !src.is_char_boundary(off) {
        off -= 1;
    }
    let before = &src[..off];
    let line = before.matches('\n').count() + 1;
    let col = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
    (line, col)
}

pub(crate) fn is_name_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'.' || c == b'-'
}

pub(crate) fn is_attr_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c == b':'
}

/// Index of the `}` matching the `{` at `open`, skipping nested brackets and string literals.
pub(crate) fn match_brace(b: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = open;
    while i < b.len() {
        match b[i] {
            b'{' | b'(' | b'[' => depth += 1,
            b'}' | b')' | b']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return (b[i] == b'}').then_some(i);
                }
            }
            q @ (b'"' | b'\'') => {
                i += 1;
                while i < b.len() && b[i] != q {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                if i >= b.len() {
                    return None;
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

pub(crate) fn collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            space = true;
        } else {
            if space && !out.is_empty() {
                out.push(' ');
            }
            space = false;
            out.push(ch);
        }
    }
    // Keep one boundary space so "Total: {x}" keeps its gap.
    if s.starts_with(char::is_whitespace) && !s.contains('\n') {
        out.insert(0, ' ');
    }
    if s.ends_with(char::is_whitespace) && !s[s.trim_end().len()..].contains('\n') {
        out.push(' ');
    }
    out
}

pub(crate) fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lbrace;", "{")
        .replace("&rbrace;", "}")
        .replace("&nbsp;", "\u{a0}")
        .replace("&amp;", "&")
}
