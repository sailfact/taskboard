use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, List, ListItem, ListState, Padding, Scrollbar, ScrollbarOrientation,
    ScrollbarState,
};

use crate::app::App;
use crate::model::Column;
use crate::ui::theme;

const CARD_LINES: usize = 2;

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

    let inner_height = block.inner(area).height as usize;

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

    draw_scrollbar(column, state, inner_height, frame, area)
}

fn draw_scrollbar(
    column: &Column,
    state: &ListState,
    inner_height: usize,
    frame: &mut Frame,
    area: Rect,
) {
    let total = column.cards.len() * CARD_LINES;

    if total <= inner_height {
        return;
    }

    let mut scroll = ScrollbarState::new(total.saturating_sub(inner_height))
        .position(state.offset() * CARD_LINES);

    let track = area.inner(Margin {
        horizontal: 0,
        vertical: 1,
    });

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_style(theme::BORDER_FOCUS)
            .track_style(theme::BORDER),
        track,
        &mut scroll,
    );
}
