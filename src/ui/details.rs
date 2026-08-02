use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Padding, Paragraph, Wrap};

use crate::app::App;
use crate::model::Card;
use crate::ui::theme;

pub fn draw(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::BORDER)
        .padding(Padding::horizontal(1))
        .title(Line::styled(" Details ", theme::TITLE).centered());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [summary_area, focus_area] =
        Layout::vertical([Constraint::Length(5), Constraint::Fill(1)]).areas(inner);

    frame.render_widget(Paragraph::new(summary(app)), summary_area);

    match app
        .board
        .columns
        .iter()
        .find_map(|column| column.cards.first())
    {
        Some(card) => frame.render_widget(focus(card).wrap(Wrap { trim: true }), focus_area),
        None => frame.render_widget(
            Paragraph::new(Line::styled("Nothing to show.", theme::MUTED)),
            focus_area,
        ),
    }
}

fn summary(app: &App) -> Text<'static> {
    let mut lines: Vec<Line> = app
        .board
        .columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            Line::from(vec![
                Span::styled("■ ", Style::new().fg(theme::column(index))),
                Span::raw(column.title.clone()),
                Span::styled(format!("   {}", column.cards.len()), theme::MUTED),
            ])
        })
        .collect();

    lines.push(Line::raw(""));
    lines.push(Line::styled(
        format!("{} cards total", app.board.total()),
        theme::MUTED,
    ));

    Text::from(lines)
}

fn focus(card: &Card) -> Paragraph<'_> {
    let mut lines = vec![Line::styled(
        card.title.as_str(),
        Style::new().add_modifier(Modifier::BOLD),
    )];

    if let Some(tag) = &card.tag {
        lines.push(Line::styled(format!("#{tag}"), theme::TAG));
    }

    if !card.notes.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled(card.notes.as_str(), theme::MUTED));
    }

    Paragraph::new(Text::from(lines))
}
