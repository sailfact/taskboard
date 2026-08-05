use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, List, ListItem, ListState, Padding, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Widget,
};

use crate::app::App;
use crate::model::Column;
use crate::ui::card::CardView;
use crate::ui::theme;

/// Each card renders as a title line plus a tag line.
const CARD_LINES: usize = 2;

pub fn draw(app: &mut App, frame: &mut Frame, areas: [Rect; 3]) {
    for (index, area) in areas.into_iter().enumerate() {
        let focused = index == app.focus;
        let accent = theme::column(index);

        let column = &app.board.columns[index];
        let state = &mut app.lists[index];

        draw_column(column, state, accent, focused, frame, area);
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
        .enumerate()
        .map(|(index, card)| {
            let selected = focused && state.selected() == Some(index);
            let view = CardView::new(card).accent(accent).selected(selected);

            // Render the widget into a two-row buffer, then hand those rows to the list.
            let mut scratch = ratatui::buffer::Buffer::empty(Rect::new(0, 0, 40, 2));
            view.render(scratch.area, &mut scratch);

            ListItem::new(buffer_to_lines(&scratch))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_symbol(Line::from("▌"));

    frame.render_stateful_widget(list, area, state);

    draw_scrollbar(column, state, inner_height, frame, area);
}

/// Read a buffer's rows back out as styled lines.
fn buffer_to_lines(buffer: &ratatui::buffer::Buffer) -> Vec<Line<'static>> {
    (0..buffer.area.height)
        .map(|y| {
            let spans = (0..buffer.area.width)
                .map(|x| {
                    let cell = &buffer[(x, y)];
                    Span::styled(cell.symbol().to_string(), cell.style())
                })
                .collect::<Vec<_>>();

            Line::from(spans)
        })
        .collect()
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
