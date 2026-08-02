use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Padding, Paragraph};

use crate::app::App;
use crate::model::{Card, Column};
use crate::ui::theme;

/// Two rows of content plus a top and bottom border.
const CARD_HEIGHT: u16 = 4;

pub fn draw(app: &App, frame: &mut Frame, areas: [Rect; 3]) {
    for ((index, column), area) in app.board.columns.iter().enumerate().zip(areas) {
        draw_column(column, theme::column(index), frame, area);
    }
}

fn draw_column(column: &Column, accent: Color, frame: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        Span::styled(
            format!(" {} ", column.title),
            Style::new().fg(accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("{} ", column.cards.len()), theme::MUTED),
    ]);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::BORDER)
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if column.cards.is_empty() {
        let empty = Paragraph::new(Line::styled("nothing here", theme::MUTED)).centered();
        frame.render_widget(empty, inner);
        return;
    }

    let card_areas = Layout::vertical(vec![Constraint::Length(CARD_HEIGHT); column.cards.len()])
        .spacing(1)
        .flex(Flex::Start)
        .split(inner);

    for (card, &card_area) in column.cards.iter().zip(card_areas.iter()) {
        draw_card(card, accent, frame, card_area);
    }
}

fn draw_card(card: &Card, accent: Color, frame: &mut Frame, area: Rect) {
    let mut lines = vec![Line::styled(card.title.as_str(), Style::new().fg(accent))];

    if let Some(tag) = &card.tag {
        lines.push(Line::styled(format!("#{tag}"), theme::TAG));
    }

    let block = Block::bordered()
        .border_style(theme::BORDER)
        .padding(Padding::horizontal(1));

    frame.render_widget(Paragraph::new(Text::from(lines)).block(block), area);
}
