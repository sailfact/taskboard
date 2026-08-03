use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, List, ListItem, ListState, Padding};

use crate::app::App;
use crate::model::Column;
use crate::ui::theme;

pub fn draw(app: &mut App, frame: &mut Frame, areas: [Rect; 3]) {
    for index in 0..areas.len() {
        let focused = index == app.focus;
        let accent = theme::column(index);

        let column = &app.board.columns[index];
        let state = &mut app.lists[index];

        draw_column(column, state, accent, focused, frame, areas[index]);
    }
}

fn draw_column(
    column: &Column,
    state: &mut ListState,
    accent: Color,
    focused: bool,
    frame: &mut Frame,
    area: Rect,
) {
    let title = Line::from(vec![
        Span::styled(format!(" {} ", column.title), Style::new().fg(accent)),
        Span::styled(format!("{} ", column.cards.len()), theme::MUTED),
    ]);

    let block = Block::bordered()
        .border_type(if focused {
            BorderType::Thick
        } else {
            BorderType::Rounded
        })
        .border_style(if focused {
            theme::BORDER_FOCUS
        } else {
            theme::BORDER
        })
        .padding(Padding::horizontal(1))
        .title(title);

    let items: Vec<ListItem> = column
        .cards
        .iter()
        .map(|card| {
            let mut lines = vec![Line::styled(card.title.as_str(), Style::new().fg(accent))];

            match &card.tag {
                Some(tag) => lines.push(Line::styled(format!("#{tag}"), theme::TAG)),
                None => lines.push(Line::raw("")),
            }

            ListItem::new(Text::from(lines))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(if focused {
            theme::SELECTED
        } else {
            theme::SELECTED_BLUR
        })
        .highlight_symbol(Line::from("▌"));

    frame.render_stateful_widget(list, area, state);
}
