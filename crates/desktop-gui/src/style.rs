use iced::Color;

pub const BG_DARK: Color = Color::from_rgb(0.08, 0.08, 0.1);
pub const BG_PANEL: Color = Color::from_rgb(0.12, 0.12, 0.15);
pub const BG_INPUT: Color = Color::from_rgb(0.18, 0.18, 0.22);
pub const TEXT_PRIMARY: Color = Color::from_rgb(0.9, 0.9, 0.95);
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.6, 0.6, 0.65);
pub const BORDER_COLOR: Color = Color::from_rgb(0.25, 0.25, 0.3);
pub const BID_COLOR: Color = Color::from_rgb(0.2, 0.7, 0.3);
pub const ASK_COLOR: Color = Color::from_rgb(0.8, 0.2, 0.2);
pub const ACCENT_COLOR: Color = Color::from_rgb(0.2, 0.5, 0.9);
pub const WARNING_COLOR: Color = Color::from_rgb(0.9, 0.6, 0.1);

pub type Theme = iced::Theme;

pub fn panel_background() -> iced::Background {
    iced::Background::Color(BG_PANEL)
}
