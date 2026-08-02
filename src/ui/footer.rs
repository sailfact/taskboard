use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};

use crate::app::App;
use crate::ui::theme;

pub fn draw(app: &App, frame: &mut Frame, area: Rect) {
    let size = frame.area();
    let status = Line::from(vec![
        Span::styled(format!("{} frames", app.frames), theme::MUTED),
        Span::styled(format!("  {}x{} ", size.width, size.height), theme::MUTED),
    ]);

    let width = status.width() as u16;

    let [left, right] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(width)]).areas(area);

    let keys = Line::from(vec![
        Span::styled(" q", theme::TITLE),
        Span::styled(" quit   ", theme::MUTED),
        Span::styled("?", theme::TITLE),
        Span::styled(" help", theme::MUTED),
    ]);

    frame.render_widget(keys, left);
    frame.render_widget(status.right_aligned(), right);
}
