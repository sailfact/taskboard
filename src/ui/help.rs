use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Clear, Padding, Paragraph};

use crate::ui::theme;

use super::layout::center;

const HELP_WIDTH: u16 = 48;
const HELP_HEIGHT: u16 = 8;

pub fn draw(frame: &mut Frame, area: Rect) {
    let area = center(
        area,
        Constraint::Length(HELP_WIDTH),
        Constraint::Length(HELP_HEIGHT),
    );
    let block = Block::bordered()
        .border_type(BorderType::Double)
        .border_style(theme::POPUP_BORDER)
        .style(theme::POPUP)
        .padding(Padding::horizontal(2))
        .title(Line::styled(" Help ", theme::TITLE).centered());

    let help = Paragraph::new(vec![
        Line::raw(""),
        Line::raw("  q / Esc    quit"),
        Line::raw("  ?          toggle this help"),
        Line::raw(""),
        Line::styled("  Resize the terminal to watch it react.", theme::MUTED),
    ])
    .block(block);

    frame.render_widget(Clear, area);
    frame.render_widget(help, area);
}
