//! Input, display and control components.

use super::layout_text::{DISABLED, HEIGHT, LABEL, ON_CHANGE, PLACEHOLDER, SIZE3, TONE, p};
use crate::{Category as C, Children, Component, Type::*};

pub const BUTTON: Component = Component {
    tag: "Button",
    category: C::Input,
    role: "button",
    children: Children::Text,
    parent: None,
    doc: "A button. Its text is the label.",
    props: &[
        p(
            "variant",
            Enum(&["primary", "secondary", "outline", "ghost", "danger"]),
            "Look (default secondary).",
        ),
        p("size", SIZE3, "Size."),
        p("icon", Icon, "Leading icon."),
        DISABLED,
        p("onClick", Action, "Actions to run."),
    ],
    example: "<Button variant=\"primary\" onClick={set(local.n, local.n + 1)}>Add</Button>",
};

pub const TEXT_FIELD: Component = Component {
    tag: "TextField",
    category: C::Input,
    role: "textbox",
    children: Children::None,
    parent: None,
    doc: "One line of text input.",
    props: &[
        p("bind", Bind("text"), "Where the text is stored."),
        LABEL,
        PLACEHOLDER,
        DISABLED,
        ON_CHANGE,
    ],
    example: "<TextField bind={local.name} placeholder=\"Name\" />",
};

pub const TEXT_AREA: Component = Component {
    tag: "TextArea",
    category: C::Input,
    role: "textbox",
    children: Children::None,
    parent: None,
    doc: "Multi-line text input.",
    props: &[
        p("bind", Bind("text"), "Where the text is stored."),
        LABEL,
        PLACEHOLDER,
        p("rows", Number, "Visible lines (default 3)."),
        DISABLED,
        ON_CHANGE,
    ],
    example: "<TextArea bind={shared.notes} rows={4} />",
};

pub const NUMBER_INPUT: Component = Component {
    tag: "NumberInput",
    category: C::Input,
    role: "spinbutton",
    children: Children::None,
    parent: None,
    doc: "A number field with step buttons.",
    props: &[
        p("bind", Bind("number"), "Where the number is stored."),
        LABEL,
        p("min", Number, "Smallest value."),
        p("max", Number, "Largest value."),
        p("step", Number, "Step for the buttons (default 1)."),
        p("prefix", Text, "Text before the number, like $."),
        p("suffix", Text, "Text after the number, like %."),
        DISABLED,
        ON_CHANGE,
    ],
    example: "<NumberInput bind={shared.price} min={0} prefix=\"$\" />",
};

pub const CHECKBOX: Component = Component {
    tag: "Checkbox",
    category: C::Input,
    role: "checkbox",
    children: Children::None,
    parent: None,
    doc: "A checkbox with a label.",
    props: &[
        p(
            "bind",
            Bind("boolean"),
            "Where the checked state is stored.",
        ),
        LABEL,
        DISABLED,
        ON_CHANGE,
    ],
    example: "<Checkbox bind={item.done} label={item.title} />",
};

pub const SWITCH: Component = Component {
    tag: "Switch",
    category: C::Input,
    role: "switch",
    children: Children::None,
    parent: None,
    doc: "An on/off switch with a label.",
    props: &[
        p("bind", Bind("boolean"), "Where the state is stored."),
        LABEL,
        DISABLED,
        ON_CHANGE,
    ],
    example: "<Switch bind={local.yearly} label=\"Bill yearly\" />",
};

pub const SLIDER: Component = Component {
    tag: "Slider",
    category: C::Input,
    role: "slider",
    children: Children::None,
    parent: None,
    doc: "Drag to pick a number in a range.",
    props: &[
        p("bind", Bind("number"), "Where the number is stored."),
        LABEL,
        p("min", Number, "Lowest value (default 0)."),
        p("max", Number, "Highest value (default 100)."),
        p("step", Number, "Step (default 1)."),
        DISABLED,
        ON_CHANGE,
    ],
    example: "<Slider bind={shared.seats} min={1} max={500} />",
};

pub const SELECT: Component = Component {
    tag: "Select",
    category: C::Input,
    role: "combobox",
    children: Children::None,
    parent: None,
    doc: "Pick one option from a dropdown.",
    props: &[
        p("bind", Bind("text"), "Where the chosen value is stored."),
        p("options", List, "Texts, or records {label, value}.").required(),
        LABEL,
        PLACEHOLDER,
        DISABLED,
        ON_CHANGE,
    ],
    example: "<Select bind={local.plan} options={[\"Free\", \"Pro\"]} />",
};

pub const SEGMENTED_CONTROL: Component = Component {
    tag: "SegmentedControl",
    category: C::Input,
    role: "radiogroup",
    children: Children::None,
    parent: None,
    doc: "A row of mutually exclusive buttons.",
    props: &[
        p("bind", Bind("text"), "Where the chosen value is stored."),
        p("options", List, "Texts, or records {label, value}.").required(),
        DISABLED,
        ON_CHANGE,
    ],
    example: "<SegmentedControl bind={local.period} options={[\"Month\", \"Year\"]} />",
};

pub const DATE_PICKER: Component = Component {
    tag: "DatePicker",
    category: C::Input,
    role: "combobox",
    children: Children::None,
    parent: None,
    doc: "Pick a calendar date. Stores a date value.",
    props: &[
        p("bind", Bind("date"), "Where the date is stored."),
        LABEL,
        PLACEHOLDER,
        DISABLED,
        ON_CHANGE,
    ],
    example: "<DatePicker bind={shared.due} />",
};

pub const STAT: Component = Component {
    tag: "Stat",
    category: C::Display,
    role: "group",
    children: Children::None,
    parent: None,
    doc: "A headline number with a label.",
    props: &[
        p("label", Text, "What the number is.").required(),
        p("value", Any, "The number or text shown large.").required(),
        p("delta", Text, "Change, like +4%."),
        p("caption", Text, "Small note under the value."),
        TONE,
    ],
    example: "<Stat label=\"Revenue\" value={currency(data.sales.total)} delta=\"+4%\" tone=\"success\" />",
};

pub const TABLE: Component = Component {
    tag: "Table",
    category: C::Display,
    role: "table",
    children: Children::None,
    parent: None,
    doc: "Rows of records in columns.",
    props: &[
        p("rows", List, "List of records.").required(),
        p(
            "columns",
            List,
            "Field names, or records {key, label, format: number|currency|percent|date, align: left|right}. Default: the first row's fields.",
        ),
        p("empty", Text, "Shown when there are no rows."),
    ],
    example: "<Table rows={shared.orders} columns={[{key: \"name\", label: \"Item\"}, {key: \"price\", format: \"currency\", align: \"right\"}]} />",
};

pub const LIST: Component = Component {
    tag: "List",
    category: C::Display,
    role: "list",
    children: Children::None,
    parent: None,
    doc: "A simple list of items.",
    props: &[
        p(
            "items",
            List,
            "Texts, or records {title, description, badge, icon}.",
        )
        .required(),
        p("ordered", Bool, "Number the items."),
        p("empty", Text, "Shown when there are no items."),
    ],
    example: "<List items={[\"Bread\", \"Milk\"]} />",
};

pub const PROGRESS: Component = Component {
    tag: "Progress",
    category: C::Display,
    role: "progressbar",
    children: Children::None,
    parent: None,
    doc: "A bar filled to a percentage.",
    props: &[p("value", Number, "0 to 100.").required(), LABEL, TONE],
    example: "<Progress value={local.done / local.total * 100} label=\"Done\" />",
};

pub const CHART: Component = Component {
    tag: "Chart",
    category: C::Display,
    role: "img",
    children: Children::None,
    parent: None,
    doc: "A bar, line, area or pie chart of a list of records.",
    props: &[
        p("type", Enum(&["bar", "line", "area", "pie"]), "Chart type.").required(),
        p("data", List, "List of records.").required(),
        p("x", Text, "Field for categories or labels.").required(),
        p("y", Text, "Field for values.").required(),
        p("color", Color, "Series color (default chart-1)."),
        p("height", HEIGHT, "Height token (default md)."),
        LABEL,
    ],
    example: "<Chart type=\"bar\" data={data.sales.months} x=\"month\" y=\"revenue\" />",
};

pub const ICON: Component = Component {
    tag: "Icon",
    category: C::Display,
    role: "img",
    children: Children::None,
    parent: None,
    doc: "An icon from the icon set.",
    props: &[
        p("name", Icon, "Icon name.").required(),
        p("size", SIZE3, "Size."),
        p("color", Color, "Palette color."),
        TONE,
        LABEL,
    ],
    example: "<Icon name=\"star\" tone=\"warning\" />",
};

pub const AVATAR: Component = Component {
    tag: "Avatar",
    category: C::Display,
    role: "img",
    children: Children::None,
    parent: None,
    doc: "A person's initials, or an image the host provides.",
    props: &[
        p("name", Text, "Full name; initials are shown.").required(),
        p(
            "asset",
            Asset,
            "Image the host knows by this name. Never a URL.",
        )
        .literal(),
        p("size", SIZE3, "Size."),
    ],
    example: "<Avatar name=\"Ada Lovelace\" />",
};

pub const CALLOUT: Component = Component {
    tag: "Callout",
    category: C::Display,
    role: "note",
    children: Children::Text,
    parent: None,
    doc: "A highlighted message.",
    props: &[
        TONE,
        p("title", Text, "Bold first line."),
        p("icon", Icon, "Overrides the tone's icon."),
    ],
    example: "<Callout tone=\"warning\" title=\"Heads up\">Prices change in May.</Callout>",
};

pub const TOOLTIP: Component = Component {
    tag: "Tooltip",
    category: C::Display,
    role: "tooltip",
    children: Children::Any,
    parent: None,
    doc: "Shows text when the pointer rests on its child.",
    props: &[p("text", Text, "Tooltip text.").required()],
    example: "<Tooltip text=\"Billed yearly\"><Badge>Annual</Badge></Tooltip>",
};

pub const SKELETON: Component = Component {
    tag: "Skeleton",
    category: C::Display,
    role: "status",
    children: Children::None,
    parent: None,
    doc: "A placeholder shimmer while something loads.",
    props: &[
        p("lines", Number, "Number of text lines (default 3)."),
        p(
            "height",
            HEIGHT,
            "Draw one block of this height instead of lines.",
        ),
    ],
    example: "<Skeleton lines={2} />",
};

pub const SPINNER: Component = Component {
    tag: "Spinner",
    category: C::Display,
    role: "status",
    children: Children::None,
    parent: None,
    doc: "An activity indicator.",
    props: &[p("size", SIZE3, "Size."), LABEL],
    example: "<Spinner label=\"Loading\" />",
};

pub const IF: Component = Component {
    tag: "If",
    category: C::Control,
    role: "none",
    children: Children::Any,
    parent: None,
    doc: "Shows its children only when `when` is true.",
    props: &[p("when", Any, "Condition.").required()],
    example: "<If when={shared.seats > 100}><Callout>Volume pricing applies.</Callout></If>",
};

pub const FOR: Component = Component {
    tag: "For",
    category: C::Control,
    role: "none",
    children: Children::Any,
    parent: None,
    doc: "Repeats its children for each item of a list. Inside, the item is `item` (or the `as` name) and its position is `index`.",
    props: &[
        p("each", List, "The list.").required(),
        p("as", Text, "Name for the item (default item).").literal(),
        p("index", Text, "Name for the position (default index).").literal(),
    ],
    example: "<For each={shared.todos} as=\"t\"><Checkbox bind={t.done} label={t.title} /></For>",
};
