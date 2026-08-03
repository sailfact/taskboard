use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Cell, Padding, Paragraph, Row, Table, Wrap};

use crate::app::App;
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

    frame.render_widget(summary(app), summary_area);
    frame.render_widget(focus(app), focus_area);
}

fn summary(app: &App) -> Table<'static> {
    let rows = app.board.columns.iter().enumerate().map(|(index, column)| {
        let marker = Span::styled("■", Style::new().fg(theme::column(index)));
        let name = Span::raw(column.title.clone());
        let count = Span::styled(column.cards.len().to_string(), theme::MUTED);

        Row::new([Cell::from(marker), Cell::from(name), Cell::from(count)])
    });

    let widths = [
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(3),
    ];

    Table::new(rows, widths)
        .column_spacing(1)
        .header(Row::new(["", "Column", "  #"]).style(theme::MUTED))
}

fn focus(app: &App) -> Paragraph<'static> {
    let Some(card) = app.selected_card() else {
        return Paragraph::new(Line::styled("Nothing selected.", theme::MUTED));
    };

    let mut lines = vec![Line::styled(
        card.title.clone(),
        Style::new().add_modifier(Modifier::BOLD),
    )];

    if let Some(tag) = &card.tag {
        lines.push(Line::styled(format!("#{tag}"), theme::TAG));
    }

    if !card.notes.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled(card.notes.clone(), theme::MUTED));
    }

    Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true })
}
