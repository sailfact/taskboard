use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Paragraph};

use crate::app::App;
use crate::model::Column;

/// One row of text plus a top and bottom border.
const CARD_HEIGHT: u16 = 3;

pub fn draw(app: &App, frame: &mut Frame, areas: [Rect; 3]) {
    for (column, area) in app.board.columns.iter().zip(areas) {
        draw_column(column, frame, area);
    }
}

fn draw_column(column: &Column, frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(Line::from(format!(" {} ({}) ", column.title, column.cards.len())).centered());

    // Work out the content area *before* the block is moved into the frame.
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let card_areas = Layout::vertical(vec![Constraint::Length(CARD_HEIGHT); column.cards.len()])
        .spacing(1)
        .flex(Flex::Start)
        .split(inner);

    for (card, &card_area) in column.cards.iter().zip(card_areas.iter()) {
        frame.render_widget(
            Paragraph::new(card.title.as_str()).block(Block::bordered()),
            card_area,
        );
    }
}
