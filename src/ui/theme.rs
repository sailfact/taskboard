use ratatui::style::{Color, Modifier, Style};

pub const COLUMN_ACCENTS: [Color; 3] = [Color::LightBlue, Color::LightYellow, Color::LightGreen];

pub const BORDER: Style = Style::new().fg(Color::DarkGray);

pub const MUTED: Style = Style::new().fg(Color::DarkGray);

pub const TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

pub const TAG: Style = Style::new().fg(Color::Magenta);

pub const POPUP: Style = Style::new().bg(Color::Black);

pub const POPUP_BORDER: Style = Style::new().bg(Color::Cyan);

pub fn column(index: usize) -> Color {
    COLUMN_ACCENTS[index]
}
