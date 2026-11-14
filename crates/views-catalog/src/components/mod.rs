//! The catalog table. Version 0.1.

mod input_display;
mod layout_text;

use crate::Component;
use input_display::*;
use layout_text::*;

/// Every component, in guide order.
pub static COMPONENTS: &[Component] = &[
    VIEW,
    ROW,
    COL,
    GRID,
    CARD,
    TABS,
    TAB,
    ACCORDION,
    SECTION,
    DIVIDER,
    SPACER,
    SCROLL_AREA,
    HEADING,
    TEXT,
    MARKDOWN,
    CODE,
    BADGE,
    KBD,
    LINK,
    BUTTON,
    TEXT_FIELD,
    TEXT_AREA,
    NUMBER_INPUT,
    CHECKBOX,
    SWITCH,
    SLIDER,
    SELECT,
    SEGMENTED_CONTROL,
    DATE_PICKER,
    STAT,
    TABLE,
    LIST,
    PROGRESS,
    CHART,
    ICON,
    AVATAR,
    CALLOUT,
    TOOLTIP,
    SKELETON,
    SPINNER,
    IF,
    FOR,
];
