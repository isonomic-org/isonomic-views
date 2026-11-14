//! The Isonomic Views component catalog.
//!
//! A versioned registry mapping each tag to a component with a typed prop schema, its events
//! and its accessibility role. From it come a JSON Schema, a plain-text authoring guide for
//! models, and a validator whose diagnostics are machine-readable so a model can repair its
//! own output.

mod classes;
mod components;
mod guide;
mod schema;
mod tokens;
mod validate;

use serde::Serialize;

pub use classes::{Utility, parse_class};
pub use components::COMPONENTS;
pub use guide::authoring_guide;
pub use schema::json_schema;
pub use tokens::{ICONS, PALETTE, SIZES, SPACING, TONES, size_px};
pub use validate::{Report, check, validate};

/// Corpus views (file stems) quoted as examples at the end of the authoring guide.
pub const GUIDE_EXAMPLES: &[&str] = &[
    "seat-cost-calculator",
    "todo-list",
    "sales-dashboard",
    "capitals-quiz",
    "event-signup",
    "order-search",
];

/// The catalog version a view declares with `<View version="0.1">`.
pub const VERSION: &str = "0.1";

/// Props every element accepts.
pub const UNIVERSAL: &[Prop] = &[
    Prop::new(
        "class",
        Type::Class,
        "Layout and spacing utilities (see Class utilities).",
    )
    .literal(),
    Prop::new(
        "key",
        Type::Any,
        "Identity among siblings; keeps state with the right item when lists change.",
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Layout,
    Text,
    Input,
    Display,
    Control,
}

impl Category {
    pub fn name(self) -> &'static str {
        match self {
            Category::Layout => "layout",
            Category::Text => "text",
            Category::Input => "input",
            Category::Display => "display",
            Category::Control => "control",
        }
    }
}

/// What a prop accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Text,
    Number,
    Bool,
    Enum(&'static [&'static str]),
    /// A semantic tone: neutral, info, success, warning, danger, accent.
    Tone,
    /// A palette color name.
    Color,
    /// An icon name from the icon set.
    Icon,
    /// Utility classes.
    Class,
    /// A list value.
    List,
    /// A record value.
    Record,
    /// Any value.
    Any,
    /// A calendar date (`"2026-03-01"` or a date value).
    Date,
    /// A writable state path holding a value of this kind.
    Bind(&'static str),
    /// An event handler: a `;`-separated list of actions.
    Action,
    /// A name the host resolves (an image). Never a URL.
    Asset,
}

impl Type {
    pub fn describe(self) -> String {
        match self {
            Type::Text => "text".into(),
            Type::Number => "number".into(),
            Type::Bool => "boolean".into(),
            Type::Enum(v) => v.join(" | "),
            Type::Tone => TONES.join(" | "),
            Type::Color => "palette color".into(),
            Type::Icon => "icon name".into(),
            Type::Class => "utility classes".into(),
            Type::List => "list".into(),
            Type::Record => "record".into(),
            Type::Any => "any value".into(),
            Type::Date => "date".into(),
            Type::Bind(k) => format!("state path ({k})"),
            Type::Action => "actions".into(),
            Type::Asset => "host asset name".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prop {
    pub name: &'static str,
    pub ty: Type,
    pub required: bool,
    /// Must be written as a literal (`name="..."`), never as `{expression}`.
    pub literal_only: bool,
    pub doc: &'static str,
}

impl Prop {
    pub const fn new(name: &'static str, ty: Type, doc: &'static str) -> Prop {
        Prop {
            name,
            ty,
            required: false,
            literal_only: false,
            doc,
        }
    }

    pub const fn required(mut self) -> Prop {
        self.required = true;
        self
    }

    pub const fn literal(mut self) -> Prop {
        self.literal_only = true;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Children {
    None,
    /// Text and `{expressions}` only.
    Text,
    Any,
    /// Only these tags.
    Only(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug)]
pub struct Component {
    pub tag: &'static str,
    pub category: Category,
    pub doc: &'static str,
    pub props: &'static [Prop],
    /// Accessibility role.
    pub role: &'static str,
    pub children: Children,
    /// The only tag this may appear directly inside, if restricted.
    pub parent: Option<&'static str>,
    /// A short example.
    pub example: &'static str,
}

impl Component {
    pub fn prop(&self, name: &str) -> Option<&'static Prop> {
        self.props
            .iter()
            .chain(UNIVERSAL.iter())
            .find(|p| p.name == name)
    }

    /// Event props (`onClick`, `onChange`).
    pub fn events(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.props
            .iter()
            .filter(|p| p.ty == Type::Action)
            .map(|p| p.name)
    }
}

/// Look up a component by tag.
pub fn component(tag: &str) -> Option<&'static Component> {
    COMPONENTS.iter().find(|c| c.tag == tag)
}

/// The closest known tag, for "did you mean" hints.
pub fn closest<'a>(word: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    candidates
        .map(|c| (distance(&word.to_lowercase(), &c.to_lowercase()), c))
        .filter(|(d, c)| *d <= 2.max(c.len() / 3))
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != *cb))
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}
