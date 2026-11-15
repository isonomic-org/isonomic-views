//! Parsing a source that grows at the end.

use crate::scan::{self, Pending, State, Tail};
use crate::{Diagnostic, Document, Element, Node, Severity, Text};

/// A parser for a source that only grows at the end, as a model writes it.
///
/// Everything before the construct the end of the input cut off is kept between calls; each
/// [`Stream::advance`] scans again only from the start of that construct. The tree it shows
/// is always the one [`crate::parse`] returns for the same text.
#[derive(Clone, Debug, Default)]
pub struct Stream {
    st: State,
    pending: Pending,
    len: usize,
}

/// The construct the end of the input cut off, as the tree shows it.
#[derive(Clone, Copy, Debug)]
pub enum Cut<'a> {
    /// Nothing is shown for it.
    None,
    /// Text, or the body of a literal element, still being written: the last child of the
    /// innermost open element, or a top-level node when no element is open.
    Text(&'a Text),
    /// An opening tag still being written, shown with its complete attributes as the
    /// innermost open element.
    Element(&'a Element),
}

impl Stream {
    pub fn new() -> Stream {
        Stream::default()
    }

    /// Continue with `src`, which must begin with the text passed before. Text that does
    /// not grow the source is ignored.
    pub fn advance(&mut self, src: &str) {
        if src.len() <= self.len {
            return;
        }
        self.len = src.len();
        self.pending = scan::run(src, &mut self.st);
    }

    /// Bytes of source seen so far.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Bytes that produced the tree; anything after was held back.
    pub fn consumed(&self) -> usize {
        self.pending.consumed
    }

    /// Final top-level nodes. The outermost open element follows them.
    pub fn top(&self) -> &[Node] {
        &self.st.top
    }

    /// Open elements, outermost first, each with its final children only.
    pub fn open(&self) -> &[Element] {
        &self.st.stack
    }

    pub fn cut(&self) -> Cut<'_> {
        match &self.pending.tail {
            Tail::Text(Some(t)) => Cut::Text(t),
            Tail::Open { el, ignored: false } => Cut::Element(el),
            _ => Cut::None,
        }
    }

    /// Final diagnostics, in source order of the scan.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.st.diags
    }

    /// Diagnostics inside the cut-off construct; they follow the final ones.
    pub fn cut_diagnostics(&self) -> &[Diagnostic] {
        &self.pending.diags
    }

    /// The tree for the source so far, equal to `parse` of the same text.
    pub fn document(&self) -> Document {
        self.clone().into_document()
    }

    pub fn into_document(self) -> Document {
        let Stream { st, pending, .. } = self;
        let State {
            mut stack,
            mut top,
            mut diags,
            ..
        } = st;
        diags.extend(pending.diags);
        match pending.tail {
            Tail::Text(Some(t)) => match stack.last_mut() {
                Some(e) => e.children.push(Node::Text(t)),
                None => top.push(Node::Text(t)),
            },
            Tail::Open { el, ignored: false } => stack.push(el),
            _ => {}
        }
        let end = pending.consumed;
        while let Some(mut e) = stack.pop() {
            e.span.end = end;
            diags.push(unclosed(&e));
            let n = Node::Element(e);
            match stack.last_mut() {
                Some(p) => p.children.push(n),
                None => top.push(n),
            }
        }
        Document {
            nodes: top,
            diagnostics: diags,
            consumed: end,
        }
    }
}

/// The diagnostic for an element the end of the input leaves open.
pub fn unclosed(e: &Element) -> Diagnostic {
    Diagnostic {
        code: "unclosed-tag",
        severity: Severity::Warning,
        message: format!("<{}> is not closed", e.tag),
        span: e.name_span,
    }
}

/// Parse a complete source, or any prefix of one.
pub fn parse(src: &str) -> Document {
    let mut s = Stream::new();
    s.advance(src);
    s.into_document()
}
