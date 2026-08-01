use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(Line::from(" Details ").centered());

    let body = Paragraph::new(vec![
        Line::from(format!("{} cards total", app.board.total())),
        Line::from(""),
        Line::from("Nothing selected.".dim()),
    ])
    .block(block);

    frame.render_widget(body, area);
}