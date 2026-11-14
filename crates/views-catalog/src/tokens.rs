//! Design tokens a view may name. Arbitrary values are rejected.

/// Semantic tones. The renderer maps each to the design system's colors.
pub const TONES: &[&str] = &["neutral", "info", "success", "warning", "danger", "accent"];

/// Named palette colors (charts, icons, badges).
pub const PALETTE: &[&str] = &[
    "blue", "green", "red", "yellow", "magenta", "cyan", "gray", "chart-1", "chart-2", "chart-3",
    "chart-4", "chart-5",
];

/// Spacing steps for gap and padding utilities. Step n is n × the base unit (4 px).
pub const SPACING: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 8, 10, 12];

/// Size steps for width and height utilities, in base units.
pub const SIZES: &[u16] = &[8, 12, 16, 20, 24, 32, 40, 48, 56, 64, 72, 80, 96, 128];

/// Height of a size word used by ScrollArea, Chart and Skeleton.
pub fn size_px(word: &str) -> f32 {
    match word {
        "xs" => 96.0,
        "sm" => 160.0,
        "lg" => 320.0,
        "xl" => 440.0,
        _ => 240.0,
    }
}

/// Icon names a view may use: the part of the component library's icon set that every host
/// is guaranteed to embed. A name outside this list renders as a neutral square.
pub const ICONS: &[&str] = &[
    "arrow-down",
    "arrow-left",
    "arrow-right",
    "arrow-up",
    "asterisk",
    "ban",
    "battery",
    "bell",
    "book-open",
    "bot",
    "building-2",
    "calendar",
    "chart-pie",
    "check",
    "chevron-down",
    "chevron-left",
    "chevron-right",
    "chevron-up",
    "circle-alert",
    "circle-check",
    "circle-user",
    "circle-x",
    "copy",
    "cpu",
    "ellipsis",
    "external-link",
    "eye",
    "eye-off",
    "file",
    "file-text",
    "folder",
    "folder-open",
    "globe",
    "hard-drive",
    "heart",
    "inbox",
    "info",
    "layout-dashboard",
    "loader",
    "map",
    "maximize",
    "mic",
    "minimize",
    "minus",
    "moon",
    "network",
    "palette",
    "pause",
    "play",
    "plus",
    "refresh-cw",
    "rotate-cw",
    "search",
    "settings",
    "square",
    "square-terminal",
    "star",
    "star-fill",
    "sun",
    "thumbs-down",
    "thumbs-up",
    "triangle-alert",
    "undo",
    "user",
];
