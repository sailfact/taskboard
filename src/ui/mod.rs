mod board;
mod card;
mod details;
mod footer;
mod header;
mod help;
mod layout;
mod theme;

use ratatui::Frame;

use crate::app::App;
use crate::ui::layout::AppLayout;

pub fn draw(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let layout = AppLayout::compute(area);

    header::draw(app, frame, layout.header);
    board::draw(app, frame, layout.columns);

    if let Some(details) = layout.details {
        details::draw(app, frame, details);
    }

    footer::draw(app, frame, layout.footer);

    if app.show_help {
        help::draw(frame, area);
    }
}
