use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, LineGauge, Paragraph};

use crate::app::App;
use crate::ui::theme;

pub fn draw(app: &App, frame: &mut Frame, area: Rect) {
    let board = &app.board;
    let percent = (board.ratio() * 100.0).round() as u16;

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::BORDER);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [title_area, bar_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(inner);

    let title = Line::from(vec![
        Span::styled("taskboard", theme::TITLE),
        Span::raw("   "),
        Span::styled(format!("{} open", board.open()), theme::MUTED),
        Span::styled("  ·  ", theme::MUTED),
        Span::styled(format!("{} done", board.done()), theme::MUTED),
        Span::raw("   "),
        Span::styled(format!("{percent}%"), theme::TITLE),
    ]);

    frame.render_widget(Paragraph::new(title), title_area);

    let gauge = LineGauge::default()
        .ratio(board.ratio())
        .filled_style(Style::new().fg(theme::COLUMN_ACCENTS[2]))
        .unfilled_style(theme::MUTED)
        .filled_symbol("━")
        .unfilled_symbol("─")
        .label("");

    frame.render_widget(gauge, bar_area);
}
