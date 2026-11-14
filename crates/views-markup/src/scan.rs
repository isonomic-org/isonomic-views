//! The scanner. Iterative (no recursion), single pass, linear time.

use crate::{
    Attr, AttrValue, Diagnostic, Document, Element, ExprChild, MAX_DEPTH, Node, Severity, Span,
    Text,
};

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

pub fn parse(src: &str) -> Document {
    let mut p = Scanner {
        src,
        b: src.as_bytes(),
        i: 0,
        stack: Vec::new(),
        top: Vec::new(),
        diags: Vec::new(),
        consumed: 0,
        ignored_depth: 0,
    };
    p.run();
    p.finish()
}

struct Scanner<'a> {
    src: &'a str,
    b: &'a [u8],
    i: usize,
    stack: Vec<Element>,
    top: Vec<Node>,
    diags: Vec<Diagnostic>,
    consumed: usize,
    /// Open tags skipped because they were nested too deeply.
    ignored_depth: usize,
}

enum Step {
    Continue,
    /// The rest of the input is an incomplete construct: hold it back.
    Hold,
}

impl<'a> Scanner<'a> {
    fn diag(
        &mut self,
        code: &'static str,
        severity: Severity,
        msg: impl Into<String>,
        start: usize,
        end: usize,
    ) {
        self.diags.push(Diagnostic {
            code,
            severity,
            message: msg.into(),
            span: Span::new(start, end),
        });
    }

    fn push_node(&mut self, n: Node) {
        match self.stack.last_mut() {
            Some(e) => e.children.push(n),
            None => self.top.push(n),
        }
    }

    fn run(&mut self) {
        while self.i < self.b.len() {
            let step = match self.b[self.i] {
                b'<' => self.tag(),
                b'{' => self.expr_child(),
                _ => {
                    self.text();
                    Step::Continue
                }
            };
            if let Step::Hold = step {
                return;
            }
            self.consumed = self.i;
        }
        self.consumed = self.i;
    }

    fn finish(mut self) -> Document {
        let end = self.consumed;
        while let Some(mut e) = self.stack.pop() {
            e.span.end = end;
            self.diags.push(Diagnostic {
                code: "unclosed-tag",
                severity: Severity::Warning,
                message: format!("<{}> is not closed", e.tag),
                span: e.name_span,
            });
            self.push_node(Node::Element(e));
        }
        Document {
            nodes: self.top,
            diagnostics: self.diags,
            consumed: end,
        }
    }

    fn text(&mut self) {
        let start = self.i;
        while self.i < self.b.len() && self.b[self.i] != b'<' && self.b[self.i] != b'{' {
            self.i += 1;
        }
        let raw = &self.src[start..self.i];
        if raw.trim().is_empty() {
            return;
        }
        let keep = self
            .stack
            .last()
            .is_some_and(|e| crate::PRESERVE_WHITESPACE.contains(&e.tag.as_str()));
        let text = if keep {
            decode_entities(raw)
        } else {
            collapse(&decode_entities(raw))
        };
        self.push_node(Node::Text(Text {
            text,
            span: Span::new(start, self.i),
        }));
    }

    fn expr_child(&mut self) -> Step {
        let start = self.i;
        match match_brace(self.b, start) {
            Some(end) => {
                let src = self.src[start + 1..end].to_string();
                self.i = end + 1;
                if src.trim().is_empty() {
                    self.diag(
                        "empty-expression",
                        Severity::Warning,
                        "empty {}",
                        start,
                        self.i,
                    );
                } else {
                    self.push_node(Node::Expr(ExprChild {
                        src,
                        span: Span::new(start, self.i),
                    }));
                }
                Step::Continue
            }
            None => Step::Hold,
        }
    }

    fn tag(&mut self) -> Step {
        let start = self.i;
        let rest = &self.b[start..];
        if rest.starts_with(b"<!--") {
            return match self.src[start..].find("-->") {
                Some(e) => {
                    self.i = start + e + 3;
                    Step::Continue
                }
                None => Step::Hold,
            };
        }
        if rest.len() < 2 {
            return Step::Hold;
        }
        if rest[1] == b'/' {
            return self.close_tag();
        }
        if !rest[1].is_ascii_alphabetic() {
            // A lone '<' in text.
            self.i += 1;
            self.diag(
                "stray-lt",
                Severity::Warning,
                "'<' in text; write {'<'} instead",
                start,
                self.i,
            );
            return Step::Continue;
        }
        self.i += 1;
        let name_start = self.i;
        while self.i < self.b.len() && is_name_byte(self.b[self.i]) {
            self.i += 1;
        }
        if self.i >= self.b.len() {
            return Step::Hold;
        }
        let tag = self.src[name_start..self.i].to_string();
        let mut el = Element {
            tag,
            attrs: Vec::new(),
            children: Vec::new(),
            span: Span::new(start, self.i),
            name_span: Span::new(name_start, self.i),
            closed: false,
        };
        // Attributes.
        loop {
            self.skip_ws();
            if self.i >= self.b.len() {
                return self.open_partial(el);
            }
            match self.b[self.i] {
                b'>' => {
                    self.i += 1;
                    el.span.end = self.i;
                    let literal = crate::LITERAL.contains(&el.tag.as_str());
                    let tag = el.tag.clone();
                    self.open(el);
                    if literal && self.ignored_depth == 0 {
                        self.literal_body(&tag);
                    }
                    return Step::Continue;
                }
                b'/' if self.b.get(self.i + 1) == Some(&b'>') => {
                    self.i += 2;
                    el.span.end = self.i;
                    el.closed = true;
                    if self.ignored_depth == 0 {
                        self.push_node(Node::Element(el));
                    }
                    return Step::Continue;
                }
                b'/' if self.i + 1 >= self.b.len() => return self.open_partial(el),
                c if c.is_ascii_alphabetic() || c == b'_' => match self.attr() {
                    Ok(Some(a)) => {
                        if el.attrs.iter().any(|x| x.name == a.name) {
                            self.diag(
                                "duplicate-attribute",
                                Severity::Warning,
                                format!("'{}' is set twice; the last one wins", a.name),
                                a.span.start,
                                a.span.end,
                            );
                            el.attrs.retain(|x| x.name != a.name);
                        }
                        el.attrs.push(a);
                    }
                    Ok(None) => {}
                    Err(()) => return self.open_partial(el),
                },
                _ => {
                    let s = self.i;
                    let ch = self.src[s..].chars().next().map_or(1, char::len_utf8);
                    self.i += ch;
                    self.diag(
                        "bad-attribute",
                        Severity::Error,
                        "unexpected character in tag",
                        s,
                        self.i,
                    );
                }
            }
        }
    }

    /// The input ended inside an opening tag: show the element with its complete attributes.
    fn open_partial(&mut self, mut el: Element) -> Step {
        el.span.end = self.consumed_tag_end(&el);
        self.i = self.b.len();
        self.open(el);
        Step::Hold
    }

    fn consumed_tag_end(&self, el: &Element) -> usize {
        el.attrs.last().map_or(el.name_span.end, |a| a.span.end)
    }

    fn open(&mut self, el: Element) {
        if self.ignored_depth > 0 || self.stack.len() >= MAX_DEPTH {
            self.ignored_depth += 1;
            self.diag(
                "too-deep",
                Severity::Error,
                format!("nesting deeper than {MAX_DEPTH} is ignored"),
                el.name_span.start,
                el.name_span.end,
            );
            return;
        }
        self.stack.push(el);
    }

    /// Ok(None): attribute skipped with a diagnostic. Err: input ended mid-attribute.
    fn attr(&mut self) -> Result<Option<Attr>, ()> {
        let start = self.i;
        while self.i < self.b.len() && is_attr_byte(self.b[self.i]) {
            self.i += 1;
        }
        let name = self.src[start..self.i].to_string();
        self.skip_ws();
        if self.i >= self.b.len() {
            return Err(());
        }
        if self.b[self.i] != b'=' {
            let end = start + name.len();
            return Ok(Some(Attr {
                name,
                value: AttrValue::Flag,
                span: Span::new(start, end),
            }));
        }
        self.i += 1;
        self.skip_ws();
        if self.i >= self.b.len() {
            return Err(());
        }
        match self.b[self.i] {
            q @ (b'"' | b'\'') => {
                let vs = self.i + 1;
                match self.b[vs..].iter().position(|&c| c == q) {
                    Some(len) => {
                        self.i = vs + len + 1;
                        let v = decode_entities(&self.src[vs..vs + len]);
                        Ok(Some(Attr {
                            name,
                            value: AttrValue::Str(v),
                            span: Span::new(start, self.i),
                        }))
                    }
                    None => Err(()),
                }
            }
            b'{' => match match_brace(self.b, self.i) {
                Some(end) => {
                    let v = self.src[self.i + 1..end].to_string();
                    self.i = end + 1;
                    Ok(Some(Attr {
                        name,
                        value: AttrValue::Expr(v),
                        span: Span::new(start, self.i),
                    }))
                }
                None => Err(()),
            },
            _ => {
                let s = self.i;
                while self.i < self.b.len()
                    && !self.b[self.i].is_ascii_whitespace()
                    && self.b[self.i] != b'>'
                    && self.b[self.i] != b'/'
                {
                    self.i += 1;
                }
                if self.i >= self.b.len() {
                    return Err(());
                }
                self.diag(
                    "unquoted-value",
                    Severity::Error,
                    format!("value of '{name}' needs quotes or braces"),
                    s,
                    self.i,
                );
                Ok(None)
            }
        }
    }

    /// Text up to `</tag>`, taken literally: no tags, no expressions, whitespace kept.
    fn literal_body(&mut self, tag: &str) {
        let close = format!("</{tag}>");
        let start = self.i;
        let end = match self.src[start..].find(&close) {
            Some(off) => start + off,
            None => {
                // Still streaming: hold back a partial closing tag at the very end.
                let rest = &self.src[start..];
                let hold = (1..close.len())
                    .rev()
                    .find(|k| rest.ends_with(&close[..*k]))
                    .unwrap_or(0);
                self.src.len() - hold
            }
        };
        if end > start {
            let text = decode_entities(&self.src[start..end]);
            self.push_node(Node::Text(Text {
                text,
                span: Span::new(start, end),
            }));
        }
        self.i = end;
    }

    fn close_tag(&mut self) -> Step {
        let start = self.i;
        let Some(len) = self.b[start..].iter().position(|&c| c == b'>') else {
            return Step::Hold;
        };
        self.i = start + len + 1;
        let name = self.src[start + 2..start + len].trim().to_string();
        if self.ignored_depth > 0 {
            self.ignored_depth -= 1;
            return Step::Continue;
        }
        match self.stack.iter().rposition(|e| e.tag == name) {
            Some(pos) => {
                while self.stack.len() > pos + 1 {
                    let mut e = self.stack.pop().expect("stack has an element above pos");
                    e.span.end = start;
                    self.diag(
                        "unclosed-tag",
                        Severity::Warning,
                        format!("<{}> closed by </{name}>", e.tag),
                        e.name_span.start,
                        e.name_span.end,
                    );
                    self.push_node(Node::Element(e));
                }
                let mut e = self.stack.pop().expect("stack has the element at pos");
                e.span.end = self.i;
                e.closed = true;
                self.push_node(Node::Element(e));
            }
            None => self.diag(
                "stray-close",
                Severity::Warning,
                format!("</{name}> has no matching open tag"),
                start,
                self.i,
            ),
        }
        Step::Continue
    }

    fn skip_ws(&mut self) {
        while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
}

fn is_name_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'.' || c == b'-'
}

fn is_attr_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c == b':'
}

/// Index of the `}` matching the `{` at `open`, skipping nested brackets and string literals.
fn match_brace(b: &[u8], open: usize) -> Option<usize> {
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

fn collapse(s: &str) -> String {
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

fn decode_entities(s: &str) -> String {
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
