//! The scanner. Iterative (no recursion), single pass, linear time.
//!
//! The scanner works on a [`State`] it does not own, so a [`crate::Stream`] can keep the state
//! between chunks. A construct that the end of the input cuts off (text, a literal body, an
//! opening tag, or anything held back) is never added to the state: it is returned as
//! [`Pending`], and the next run starts again at its first byte.

use crate::lex::{collapse, decode_entities, is_attr_byte, is_name_byte, match_brace};
use crate::{
    Attr, AttrValue, Diagnostic, Element, ExprChild, MAX_DEPTH, Node, Severity, Span, Text,
};

/// What the scanner keeps between runs. Everything in it is final.
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    /// Where the next run starts.
    pub i: usize,
    /// Open elements, outermost first, each with its final children.
    pub stack: Vec<Element>,
    /// Final top-level nodes.
    pub top: Vec<Node>,
    pub diags: Vec<Diagnostic>,
    /// Open tags skipped because they were nested too deeply.
    pub ignored_depth: usize,
    /// The input ended inside the body of a literal element (the innermost open one).
    pub in_literal: bool,
}

/// The construct the end of the input cut off.
#[derive(Clone, Debug, Default)]
pub(crate) enum Tail {
    #[default]
    None,
    /// Text, or a literal body, running to the end; `None` when nothing would be shown.
    Text(Option<Text>),
    /// An opening tag. `ignored`: it is nested too deeply and would not be shown.
    Open { el: Element, ignored: bool },
    /// Something held back with nothing to show (an expression, a comment, a closing tag).
    Hold,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Pending {
    pub tail: Tail,
    /// Bytes a parse of this input reports as consumed.
    pub consumed: usize,
    /// Diagnostics found inside the cut-off construct.
    pub diags: Vec<Diagnostic>,
}

/// Scan `src` from `st.i` to the end. `src` must extend the input of earlier runs on `st`.
pub(crate) fn run(src: &str, st: &mut State) -> Pending {
    let mut p = Scanner {
        src,
        b: src.as_bytes(),
        st,
    };
    p.run()
}

struct Scanner<'a> {
    src: &'a str,
    b: &'a [u8],
    st: &'a mut State,
}

enum Step {
    Continue,
    /// The construct starting at `at` runs past the end of the input. `literal`: it is the
    /// body of the innermost open element, which is a literal one.
    Cut {
        at: usize,
        consumed: usize,
        tail: Tail,
        literal: bool,
    },
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
        self.st.diags.push(Diagnostic {
            code,
            severity,
            message: msg.into(),
            span: Span::new(start, end),
        });
    }

    fn push_node(&mut self, n: Node) {
        match self.st.stack.last_mut() {
            Some(e) => e.children.push(n),
            None => self.st.top.push(n),
        }
    }

    fn hold(&self, at: usize) -> Step {
        Step::Cut {
            at,
            consumed: at,
            tail: Tail::Hold,
            literal: false,
        }
    }

    fn run(&mut self) -> Pending {
        if std::mem::take(&mut self.st.in_literal) {
            let tag = self
                .st
                .stack
                .last()
                .map(|e| e.tag.clone())
                .unwrap_or_default();
            let mark = self.st.diags.len();
            let step = self.literal_body(&tag);
            if let Some(p) = self.cut(step, mark) {
                return p;
            }
        }
        while self.st.i < self.b.len() {
            let mark = self.st.diags.len();
            let step = match self.b[self.st.i] {
                b'<' => self.tag(),
                b'{' => self.expr_child(),
                _ => self.text(),
            };
            if let Some(p) = self.cut(step, mark) {
                return p;
            }
        }
        Pending {
            tail: Tail::None,
            consumed: self.st.i,
            diags: Vec::new(),
        }
    }

    /// Turn a cut-off step into the pending construct, rewinding the state to its start.
    fn cut(&mut self, step: Step, mark: usize) -> Option<Pending> {
        let Step::Cut {
            at,
            consumed,
            tail,
            literal,
        } = step
        else {
            return None;
        };
        self.st.i = at;
        self.st.in_literal = literal;
        let diags = if matches!(tail, Tail::Open { .. }) {
            self.st.diags.split_off(mark)
        } else {
            Vec::new()
        };
        Some(Pending {
            tail,
            consumed,
            diags,
        })
    }

    /// Text up to the next tag or expression. Text running to the end of the input is cut
    /// off: more of it may follow.
    fn text(&mut self) -> Step {
        let start = self.st.i;
        while self.st.i < self.b.len() && self.b[self.st.i] != b'<' && self.b[self.st.i] != b'{' {
            self.st.i += 1;
        }
        let complete = self.st.i < self.b.len();
        let raw = &self.src[start..self.st.i];
        let node = (!raw.trim().is_empty()).then(|| {
            let keep = self
                .st
                .stack
                .last()
                .is_some_and(|e| crate::PRESERVE_WHITESPACE.contains(&e.tag.as_str()));
            let text = if keep {
                decode_entities(raw)
            } else {
                collapse(&decode_entities(raw))
            };
            Text {
                text,
                span: Span::new(start, self.st.i),
                complete,
            }
        });
        if !complete {
            return Step::Cut {
                at: start,
                consumed: self.b.len(),
                tail: Tail::Text(node),
                literal: false,
            };
        }
        if let Some(t) = node {
            self.push_node(Node::Text(t));
        }
        Step::Continue
    }

    fn expr_child(&mut self) -> Step {
        let start = self.st.i;
        match match_brace(self.b, start) {
            Some(end) => {
                let src = self.src[start + 1..end].to_string();
                self.st.i = end + 1;
                if src.trim().is_empty() {
                    self.diag(
                        "empty-expression",
                        Severity::Warning,
                        "empty {}",
                        start,
                        self.st.i,
                    );
                } else {
                    self.push_node(Node::Expr(ExprChild {
                        src,
                        span: Span::new(start, self.st.i),
                    }));
                }
                Step::Continue
            }
            None => self.hold(start),
        }
    }

    fn tag(&mut self) -> Step {
        let start = self.st.i;
        let rest = &self.b[start..];
        if rest.len() < 4 && b"<!--".starts_with(rest) && rest.len() > 1 {
            // Could still become a comment.
            return self.hold(start);
        }
        if rest.starts_with(b"<!--") {
            return match self.src[start..].find("-->") {
                Some(e) => {
                    self.st.i = start + e + 3;
                    Step::Continue
                }
                None => self.hold(start),
            };
        }
        if rest.len() < 2 {
            return self.hold(start);
        }
        if rest[1] == b'/' {
            return self.close_tag();
        }
        if !rest[1].is_ascii_alphabetic() {
            // A lone '<' in text.
            self.st.i += 1;
            self.diag(
                "stray-lt",
                Severity::Warning,
                "'<' in text; write {'<'} instead",
                start,
                self.st.i,
            );
            return Step::Continue;
        }
        self.st.i += 1;
        let name_start = self.st.i;
        while self.st.i < self.b.len() && is_name_byte(self.b[self.st.i]) {
            self.st.i += 1;
        }
        if self.st.i >= self.b.len() {
            return self.hold(start);
        }
        let tag = self.src[name_start..self.st.i].to_string();
        let mut el = Element {
            tag,
            attrs: Vec::new(),
            children: Vec::new(),
            span: Span::new(start, self.st.i),
            name_span: Span::new(name_start, self.st.i),
            closed: false,
        };
        // Attributes.
        loop {
            self.skip_ws();
            if self.st.i >= self.b.len() {
                return self.open_partial(el);
            }
            match self.b[self.st.i] {
                b'>' => {
                    self.st.i += 1;
                    el.span.end = self.st.i;
                    let literal = crate::LITERAL.contains(&el.tag.as_str());
                    let tag = el.tag.clone();
                    self.open(el);
                    if literal && self.st.ignored_depth == 0 {
                        return self.literal_body(&tag);
                    }
                    return Step::Continue;
                }
                b'/' if self.b.get(self.st.i + 1) == Some(&b'>') => {
                    self.st.i += 2;
                    el.span.end = self.st.i;
                    el.closed = true;
                    if self.st.ignored_depth == 0 {
                        self.push_node(Node::Element(el));
                    }
                    return Step::Continue;
                }
                b'/' if self.st.i + 1 >= self.b.len() => return self.open_partial(el),
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
                    let s = self.st.i;
                    let ch = self.src[s..].chars().next().map_or(1, char::len_utf8);
                    self.st.i += ch;
                    self.diag(
                        "bad-attribute",
                        Severity::Error,
                        "unexpected character in tag",
                        s,
                        self.st.i,
                    );
                }
            }
        }
    }

    /// The input ended inside an opening tag: show the element with its complete attributes,
    /// but keep it out of the state, since the tag is scanned again when more input arrives.
    fn open_partial(&mut self, mut el: Element) -> Step {
        let at = el.span.start;
        el.span.end = el.attrs.last().map_or(el.name_span.end, |a| a.span.end);
        let ignored = self.too_deep();
        if ignored {
            self.too_deep_diag(&el);
        }
        Step::Cut {
            at,
            consumed: at,
            tail: Tail::Open { el, ignored },
            literal: false,
        }
    }

    fn too_deep(&self) -> bool {
        self.st.ignored_depth > 0 || self.st.stack.len() >= MAX_DEPTH
    }

    fn too_deep_diag(&mut self, el: &Element) {
        self.diag(
            "too-deep",
            Severity::Error,
            format!("nesting deeper than {MAX_DEPTH} is ignored"),
            el.name_span.start,
            el.name_span.end,
        );
    }

    fn open(&mut self, el: Element) {
        if self.too_deep() {
            self.st.ignored_depth += 1;
            self.too_deep_diag(&el);
            return;
        }
        self.st.stack.push(el);
    }

    /// Ok(None): attribute skipped with a diagnostic. Err: input ended mid-attribute.
    fn attr(&mut self) -> Result<Option<Attr>, ()> {
        let start = self.st.i;
        while self.st.i < self.b.len() && is_attr_byte(self.b[self.st.i]) {
            self.st.i += 1;
        }
        let name = self.src[start..self.st.i].to_string();
        self.skip_ws();
        if self.st.i >= self.b.len() {
            return Err(());
        }
        if self.b[self.st.i] != b'=' {
            let end = start + name.len();
            return Ok(Some(Attr {
                name,
                value: AttrValue::Flag,
                span: Span::new(start, end),
            }));
        }
        self.st.i += 1;
        self.skip_ws();
        if self.st.i >= self.b.len() {
            return Err(());
        }
        match self.b[self.st.i] {
            q @ (b'"' | b'\'') => {
                let vs = self.st.i + 1;
                match self.b[vs..].iter().position(|&c| c == q) {
                    Some(len) => {
                        self.st.i = vs + len + 1;
                        let v = decode_entities(&self.src[vs..vs + len]);
                        Ok(Some(Attr {
                            name,
                            value: AttrValue::Str(v),
                            span: Span::new(start, self.st.i),
                        }))
                    }
                    None => Err(()),
                }
            }
            b'{' => match match_brace(self.b, self.st.i) {
                Some(end) => {
                    let v = self.src[self.st.i + 1..end].to_string();
                    self.st.i = end + 1;
                    Ok(Some(Attr {
                        name,
                        value: AttrValue::Expr(v),
                        span: Span::new(start, self.st.i),
                    }))
                }
                None => Err(()),
            },
            _ => {
                let s = self.st.i;
                while self.st.i < self.b.len()
                    && !self.b[self.st.i].is_ascii_whitespace()
                    && self.b[self.st.i] != b'>'
                    && self.b[self.st.i] != b'/'
                {
                    self.st.i += 1;
                }
                if self.st.i >= self.b.len() {
                    return Err(());
                }
                self.diag(
                    "unquoted-value",
                    Severity::Error,
                    format!("value of '{name}' needs quotes or braces"),
                    s,
                    self.st.i,
                );
                Ok(None)
            }
        }
    }

    /// Text up to `</tag>`, taken literally: no tags, no expressions, whitespace kept. A body
    /// without its closing tag yet is cut off, holding back a partial closing tag at the end.
    fn literal_body(&mut self, tag: &str) -> Step {
        let close = format!("</{tag}>");
        let start = self.st.i;
        let (end, complete) = match self.src[start..].find(&close) {
            Some(off) => (start + off, true),
            None => {
                let rest = &self.src[start..];
                let hold = (1..close.len())
                    .rev()
                    .find(|k| rest.ends_with(&close[..*k]))
                    .unwrap_or(0);
                (self.src.len() - hold, false)
            }
        };
        let node = (end > start).then(|| Text {
            text: decode_entities(&self.src[start..end]),
            span: Span::new(start, end),
            complete,
        });
        if !complete {
            return Step::Cut {
                at: start,
                consumed: end,
                tail: Tail::Text(node),
                literal: true,
            };
        }
        if let Some(t) = node {
            self.push_node(Node::Text(t));
        }
        self.st.i = end;
        Step::Continue
    }

    fn close_tag(&mut self) -> Step {
        let start = self.st.i;
        let Some(len) = self.b[start..].iter().position(|&c| c == b'>') else {
            return self.hold(start);
        };
        self.st.i = start + len + 1;
        let name = self.src[start + 2..start + len].trim().to_string();
        if self.st.ignored_depth > 0 {
            self.st.ignored_depth -= 1;
            return Step::Continue;
        }
        match self.st.stack.iter().rposition(|e| e.tag == name) {
            Some(pos) => {
                while self.st.stack.len() > pos + 1 {
                    let mut e = self.st.stack.pop().expect("stack has an element above pos");
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
                let mut e = self.st.stack.pop().expect("stack has the element at pos");
                e.span.end = self.st.i;
                e.closed = true;
                self.push_node(Node::Element(e));
            }
            None => self.diag(
                "stray-close",
                Severity::Warning,
                format!("</{name}> has no matching open tag"),
                start,
                self.st.i,
            ),
        }
        Step::Continue
    }

    fn skip_ws(&mut self) {
        while self.st.i < self.b.len() && self.b[self.st.i].is_ascii_whitespace() {
            self.st.i += 1;
        }
    }
}
