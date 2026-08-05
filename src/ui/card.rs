use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Padding, Widget};

use crate::model::Card;
use crate::ui::theme;

pub struct CardView<'a> {
    card: &'a Card,
    accent: Color,
    selected: bool,
}

impl<'a> CardView<'a> {
    pub fn new(card: &'a Card) -> Self {
        Self {
            card,
            accent: Color::Reset,
            selected: false,
        }
    }
    pub fn accent(mut self, accent: Color) -> Self {
        self.accent = accent;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl Widget for CardView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let block = Block::new()
            .padding(Padding::horizontal(1))
            .style(if self.selected {
                theme::SELECTED
            } else {
                Style::new()
            });

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.is_empty() {
            return;
        }

        let [title_row, tag_row] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(inner);

        Line::styled(self.card.title.as_str(), Style::new().fg(self.accent)).render(title_row, buf);

        if let Some(tag) = &self.card.tag {
            Line::from(vec![Span::styled(format!("#{tag}"), theme::TAG)]).render(tag_row, buf);
        }
    }
}
