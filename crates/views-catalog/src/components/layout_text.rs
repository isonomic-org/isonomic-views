//! Layout and text components.

use crate::{Category as C, Children, Component, Prop, Type::*};

pub(crate) const ALIGN: crate::Type = Enum(&["start", "center", "end", "stretch"]);
pub(crate) const JUSTIFY: crate::Type = Enum(&["start", "center", "end", "between"]);
pub(crate) const SIZE3: crate::Type = Enum(&["sm", "md", "lg"]);
pub(crate) const HEIGHT: crate::Type = Enum(&["xs", "sm", "md", "lg", "xl"]);

pub(crate) const fn p(name: &'static str, ty: crate::Type, doc: &'static str) -> Prop {
    Prop::new(name, ty, doc)
}

pub(crate) const LABEL: Prop = p("label", Text, "Visible label.");
pub(crate) const DISABLED: Prop = p("disabled", Bool, "Not interactive.");
pub(crate) const ON_CHANGE: Prop = p(
    "onChange",
    Action,
    "Runs after the value changes; the new value is `event`.",
);
pub(crate) const TONE: Prop = p("tone", Tone, "Semantic color.");
pub(crate) const PLACEHOLDER: Prop = p("placeholder", Text, "Shown when empty.");

pub const VIEW: Component = Component {
    tag: "View",
    category: C::Layout,
    role: "group",
    children: Children::Any,
    parent: None,
    doc: "The root of every view. Declares the catalog version and starting state.",
    props: &[
        p(
            "version",
            Text,
            "Catalog version this view targets: \"0.1\".",
        )
        .literal(),
        p(
            "title",
            Text,
            "Name of the view for the host and assistive technology.",
        ),
        p(
            "local",
            Record,
            "Starting per-viewer state, not saved: local={{ tab: \"a\" }}.",
        ),
        p(
            "shared",
            Record,
            "Starting shared state, saved with the view and the same for every viewer.",
        ),
    ],
    example: "<View version=\"0.1\" local={{ n: 0 }}>...</View>",
};

pub const ROW: Component = Component {
    tag: "Row",
    category: C::Layout,
    role: "group",
    children: Children::Any,
    parent: None,
    doc: "Lays children out left to right.",
    props: &[
        p("align", ALIGN, "Cross-axis alignment (default center)."),
        p("justify", JUSTIFY, "Main-axis distribution."),
        p("wrap", Bool, "Wrap onto new lines."),
    ],
    example: "<Row class=\"gap-2\"><Badge>a</Badge><Badge>b</Badge></Row>",
};

pub const COL: Component = Component {
    tag: "Col",
    category: C::Layout,
    role: "group",
    children: Children::Any,
    parent: None,
    doc: "Lays children out top to bottom.",
    props: &[
        p("align", ALIGN, "Cross-axis alignment (default stretch)."),
        p("justify", JUSTIFY, "Main-axis distribution."),
    ],
    example: "<Col class=\"gap-3 p-4\">...</Col>",
};

pub const GRID: Component = Component {
    tag: "Grid",
    category: C::Layout,
    role: "group",
    children: Children::Any,
    parent: None,
    doc: "Equal-width columns; children fill rows in order.",
    props: &[p("cols", Number, "Number of columns, 1 to 6 (default 2).")],
    example: "<Grid cols={3} class=\"gap-3\">...</Grid>",
};

pub const CARD: Component = Component {
    tag: "Card",
    category: C::Layout,
    role: "group",
    children: Children::Any,
    parent: None,
    doc: "A bordered surface grouping related content.",
    props: &[
        p("title", Text, "Heading shown at the top."),
        p("description", Text, "Muted line under the title."),
    ],
    example: "<Card title=\"Plan\">...</Card>",
};

pub const TABS: Component = Component {
    tag: "Tabs",
    category: C::Layout,
    role: "tablist",
    children: Children::Only(&["Tab"]),
    parent: None,
    doc: "Switches between Tab children. Bind it to remember the selected tab's value.",
    props: &[
        p("bind", Bind("text"), "The selected tab's value."),
        ON_CHANGE,
    ],
    example: "<Tabs bind={local.tab}><Tab label=\"One\">...</Tab><Tab label=\"Two\">...</Tab></Tabs>",
};

pub const TAB: Component = Component {
    tag: "Tab",
    category: C::Layout,
    role: "tabpanel",
    children: Children::Any,
    parent: Some("Tabs"),
    doc: "One page of a Tabs.",
    props: &[
        p("label", Text, "Tab title.").required(),
        p(
            "value",
            Text,
            "Value stored in the Tabs bind (default: the label).",
        ),
    ],
    example: "<Tab label=\"Summary\">...</Tab>",
};

pub const ACCORDION: Component = Component {
    tag: "Accordion",
    category: C::Layout,
    role: "group",
    children: Children::Only(&["Section"]),
    parent: None,
    doc: "Collapsible Section children.",
    props: &[p("multiple", Bool, "Allow several sections open at once.")],
    example: "<Accordion><Section title=\"What is it?\">...</Section></Accordion>",
};

pub const SECTION: Component = Component {
    tag: "Section",
    category: C::Layout,
    role: "region",
    children: Children::Any,
    parent: Some("Accordion"),
    doc: "One collapsible part of an Accordion.",
    props: &[
        p("title", Text, "Header text.").required(),
        p("open", Bool, "Open at first."),
    ],
    example: "<Section title=\"Details\" open>...</Section>",
};

pub const DIVIDER: Component = Component {
    tag: "Divider",
    category: C::Layout,
    role: "separator",
    children: Children::None,
    parent: None,
    doc: "A thin line between content.",
    props: &[p("vertical", Bool, "Vertical line inside a Row.")],
    example: "<Divider />",
};

pub const SPACER: Component = Component {
    tag: "Spacer",
    category: C::Layout,
    role: "none",
    children: Children::None,
    parent: None,
    doc: "Takes up the remaining space in a Row or Col.",
    props: &[],
    example: "<Row><Text>Left</Text><Spacer /><Text>Right</Text></Row>",
};

pub const SCROLL_AREA: Component = Component {
    tag: "ScrollArea",
    category: C::Layout,
    role: "region",
    children: Children::Any,
    parent: None,
    doc: "Scrolls its content vertically within a fixed height.",
    props: &[p("height", HEIGHT, "Height token (default md).")],
    example: "<ScrollArea height=\"sm\">...</ScrollArea>",
};

pub const HEADING: Component = Component {
    tag: "Heading",
    category: C::Text,
    role: "heading",
    children: Children::Text,
    parent: None,
    doc: "A title.",
    props: &[p("level", Number, "1 (largest) to 4. Default 2.")],
    example: "<Heading level={1}>Budget</Heading>",
};

pub const TEXT: Component = Component {
    tag: "Text",
    category: C::Text,
    role: "paragraph",
    children: Children::Text,
    parent: None,
    doc: "Body text. Mix literal text with {expressions}.",
    props: &[
        p(
            "tone",
            Enum(&["default", "muted", "accent", "success", "warning", "danger"]),
            "Text color.",
        ),
        p(
            "size",
            Enum(&["xs", "sm", "base", "lg", "xl"]),
            "Text size.",
        ),
        p(
            "weight",
            Enum(&["normal", "medium", "semibold", "bold"]),
            "Text weight.",
        ),
    ],
    example: "<Text tone=\"muted\">Total: {currency(local.total)}</Text>",
};

pub const MARKDOWN: Component = Component {
    tag: "Markdown",
    category: C::Text,
    role: "document",
    children: Children::Text,
    parent: None,
    doc: "Markdown text: headings, lists, emphasis, code, tables. No images or raw HTML.",
    props: &[],
    example: "<Markdown>**Bold** and a list:\n- one\n- two</Markdown>",
};

pub const CODE: Component = Component {
    tag: "Code",
    category: C::Text,
    role: "code",
    children: Children::Text,
    parent: None,
    doc: "A block of monospaced code. Its content is literal: braces and tags inside are text.",
    props: &[p("lang", Text, "Language name, for display.")],
    example: "<Code lang=\"rust\">fn main() {}</Code>",
};

pub const BADGE: Component = Component {
    tag: "Badge",
    category: C::Text,
    role: "status",
    children: Children::Text,
    parent: None,
    doc: "A small label.",
    props: &[TONE],
    example: "<Badge tone=\"success\">Paid</Badge>",
};

pub const KBD: Component = Component {
    tag: "Kbd",
    category: C::Text,
    role: "none",
    children: Children::Text,
    parent: None,
    doc: "A keyboard shortcut.",
    props: &[],
    example: "<Kbd>cmd-k</Kbd>",
};

pub const LINK: Component = Component {
    tag: "Link",
    category: C::Text,
    role: "link",
    children: Children::Text,
    parent: None,
    doc: "Inline link. `to` asks the host to open a target; it opens nothing by itself.",
    props: &[
        p(
            "to",
            Text,
            "Target the host opens (a document, record or address).",
        ),
        p("onClick", Action, "Actions to run instead of opening `to`."),
    ],
    example: "<Link to=\"doc:pricing\">Pricing details</Link>",
};
