use std::io;

use ratatui::DefaultTerminal;
use ratatui::widgets::ListState;

use crate::event::{self, Action};
use crate::model::{Board, Card};
use crate::ui;

#[derive(Debug, Default)]
pub struct App {
    pub board: Board,
    pub show_help: bool,
    pub frames: u64,
    /// Which column has the keyboard.
    pub focus: usize,
    /// One Selection + scroll offset perm column. Owned by the app, not the widget.
    pub lists: [ListState; 3],
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self::default();
        app.lists[0].select(Some(0));
        app
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.should_quit {
            self.frames += 1;
            terminal.draw(|frame| ui::draw(&mut self, frame))?;
            self.handle(event::next_action()?);
        }
        Ok(())
    }

    /// The card the user is currently looking at, if any.
    pub fn selected_card(&self) -> Option<&Card> {
        let index = self.lists[self.focus].selected()?;
        self.board.columns[self.focus].cards.get(index)
    }

    pub fn handle(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::ToggleHelp => self.show_help = !self.show_help,
            Action::SelectNext => self.select_next(),
            Action::SelectPrev => self.select_prev(),
            Action::FocusNext => self.focus_by(1),
            Action::FocusPrev => self.focus_by(-1),
            Action::Advance => self.move_selected(1),
            Action::Retreat => self.move_selected(-1),
            Action::None => {}
        }
    }

    fn select_next(&mut self) {
        let len = self.board.columns[self.focus].cards.len();

        if len == 0 {
            return;
        }

        // ListState::select_next has no idea how many items exist, so clamp here.
        let next = match self.lists[self.focus].selected() {
            Some(index) => (index + 1).min(len - 1),
            None => 0,
        };

        self.lists[self.focus].select(Some(next));
    }

    fn select_prev(&mut self) {
        let len = self.board.columns[self.focus].cards.len();

        if len == 0 {
            return;
        }

        let prev = match self.lists[self.focus].selected() {
            Some(index) => index.saturating_sub(1),
            None => 0,
        };

        self.lists[self.focus].select(Some(prev));
    }

    fn focus_by(&mut self, delta: isize) {
        let count = self.board.columns.len() as isize;
        self.focus = (self.focus as isize + delta).rem_euclid(count) as usize;
        self.clamp_selection(self.focus);
    }

    fn move_selected(&mut self, delta: isize) {
        let Some(index) = self.lists[self.focus].selected() else {
            return;
        };

        let count = self.board.columns.len() as isize;
        let target = (self.focus as isize + delta).rem_euclid(count) as usize;

        if let Some(landed) = self.board.move_card(self.focus, index, target) {
            self.lists[target].select(Some(landed));
            self.clamp_selection(self.focus);
        }
    }

    /// Keep a column's selection inside its card list after the list changes size.
    fn clamp_selection(&mut self, column: usize) {
        let len = self.board.columns[column].cards.len();

        if len == 0 {
            self.lists[column].select(None);
        } else {
            let index = self.lists[column].selected().unwrap_or(0).min(len - 1);
            self.lists[column].select(Some(index));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Column;

    /// A board with predictable, minimal contents.
    fn fixture() -> App {
        let mut app = App {
            board: Board {
                columns: [
                    Column::new("Todo", [Card::new("a"), Card::new("b")]),
                    Column::new("Doing", [Card::new("c")]),
                    Column::new("Done", []),
                ],
            },
            ..App::default()
        };
        app.lists[0].select(Some(0));
        app
    }

    #[test]
    fn quit_sets_the_flag() {
        let mut app = fixture();
        assert!(!app.should_quit);

        app.handle(Action::Quit);

        assert!(app.should_quit);
    }

    #[test]
    fn help_toggles() {
        let mut app = fixture();

        app.handle(Action::ToggleHelp);
        assert!(app.show_help);

        app.handle(Action::ToggleHelp);
        assert!(!app.show_help);
    }

    #[test]
    fn focus_wraps_in_both_directions() {
        let mut app = fixture();

        app.handle(Action::FocusPrev);
        assert_eq!(app.focus, 2);

        app.handle(Action::FocusNext);
        assert_eq!(app.focus, 0);
    }

    #[test]
    fn selection_does_not_run_past_the_end() {
        let mut app = fixture();

        for _ in 0..10 {
            app.handle(Action::SelectNext);
        }

        assert_eq!(app.lists[0].selected(), Some(1));
    }

    #[test]
    fn advancing_moves_the_card_and_follows_it() {
        let mut app = fixture();

        app.handle(Action::Advance);

        assert_eq!(app.board.columns[0].cards.len(), 1);
        assert_eq!(app.board.columns[1].cards.len(), 2);
        assert_eq!(app.lists[1].selected(), Some(1));
    }

    #[test]
    fn emptying_a_column_clears_its_selection() {
        let mut app = fixture();
        app.focus = 1;
        app.lists[1].select(Some(0));

        app.handle(Action::Advance);

        assert!(app.board.columns[1].cards.is_empty());
        assert_eq!(app.lists[1].selected(), None);
    }

    #[test]
    fn focusing_an_empty_column_selects_nothing() {
        let mut app = fixture();
        app.focus = 1;

        app.handle(Action::FocusNext);

        assert_eq!(app.focus, 2);
        assert_eq!(app.lists[2].selected(), None);
        assert!(app.selected_card().is_none());
    }

    #[test]
    fn advancing_with_no_selection_is_a_no_op() {
        let mut app = fixture();
        app.focus = 2;
        let before = app.board.total();

        app.handle(Action::Advance);

        assert_eq!(app.board.total(), before);
    }
}
