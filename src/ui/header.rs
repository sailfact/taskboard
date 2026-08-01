use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};

use crate::app::App;
use crate::ui::theme;

pub fn draw(app: &App, frame: &mut Frame, area: Rect) {
    let board = &app.board;
    let percent = if board.total() == 0 {
        0
    } else {
        board.done() * 100 / board.total()
    };

    let title = Line::from(vec![
        Span::styled("taskboard", theme::TITLE),
        Span::raw("   "),
        Span::styled(format!("{} open", board.open()), theme::MUTED),
        Span::styled("  ·  ", theme::MUTED),
        Span::styled(format!("{} done", board.done()), theme::MUTED),
        Span::raw("   "),
        Span::styled(format!("{percent}%"), theme::TITLE),
    ]);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::BORDER);

    frame.render_widget(Paragraph::new(title).block(block), area);
}
