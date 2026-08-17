//! Colors lifted directly from the design's `Bluetooth TUI.dc.html` /
//! `colors_and_type.css` — an amber-on-near-black "industrial" palette,
//! truecolor throughout.

use ratatui::style::Color;

/// The screen's base background.
pub const BG_BASE: Color = Color::Rgb(0x0A, 0x0A, 0x0A);
/// A panel/card's background, a touch lighter than the base.
pub const BG_PANEL: Color = Color::Rgb(0x0E, 0x0E, 0x0E);
/// Header/footer bar background.
pub const BG_BAR: Color = Color::Rgb(0x1A, 0x1A, 0x1A);
/// Modal/overlay background.
pub const BG_MODAL: Color = Color::Rgb(0x14, 0x14, 0x14);
/// Selected row in a column that doesn't currently have focus.
pub const BG_SELECTED_DIM: Color = Color::Rgb(0x24, 0x24, 0x24);

/// Default panel/box border color.
pub const BORDER: Color = Color::Rgb(0x2E, 0x2E, 0x2E);
/// A dimmer border for less prominent separators.
pub const BORDER_FAINT: Color = Color::Rgb(0x1F, 0x1F, 0x1F);

/// The palette's signature accent color - headings, focus, primary actions.
pub const AMBER: Color = Color::Rgb(0xF5, 0xB8, 0x00);
/// A slightly muted amber for warning states.
pub const AMBER_WARN: Color = Color::Rgb(0xE5, 0xA8, 0x00);

/// Primary body text.
pub const TEXT_PRIMARY: Color = Color::Rgb(0xF0, 0xEB, 0xE1);
/// Data values (addresses, numbers) - a touch dimmer than primary text.
pub const TEXT_VALUE: Color = Color::Rgb(0xC8, 0xC2, 0xB6);
/// Secondary/supporting text.
pub const TEXT_SECONDARY: Color = Color::Rgb(0x8A, 0x8A, 0x8A);
/// Field labels (the left side of a `label   value` row).
pub const TEXT_LABEL: Color = Color::Rgb(0x7A, 0x7A, 0x7A);
/// De-emphasized text - placeholders, "unresolved", disabled affordances.
pub const TEXT_MUTED: Color = Color::Rgb(0x6A, 0x6A, 0x6A);
/// Fainter still than `TEXT_MUTED`.
pub const TEXT_FAINT: Color = Color::Rgb(0x5E, 0x5E, 0x5E);
/// Fainter still than `TEXT_FAINT`.
pub const TEXT_DIM: Color = Color::Rgb(0x4A, 0x4A, 0x4A);
/// The dimmest text tone in the palette - decorative separators, e.g. breadcrumb slashes.
pub const TEXT_VERY_DIM: Color = Color::Rgb(0x3E, 0x3E, 0x3E);

/// Error/failure foreground text.
pub const ERROR_FG: Color = Color::Rgb(0xE5, 0x48, 0x4D);
/// Error banner/box border.
pub const ERROR_BORDER: Color = Color::Rgb(0x5A, 0x2C, 0x2E);
/// Error banner/box background.
pub const ERROR_BG: Color = Color::Rgb(0x1C, 0x12, 0x13);

/// Text/icon color rendered on top of an amber-filled background (row
/// selection, active tab, primary action button).
pub const ON_AMBER: Color = BG_PANEL;
