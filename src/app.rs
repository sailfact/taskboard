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
    /// One selection + scroll offset per column. Owned by the app, not the widget.
    pub lists: [ListState; 3],
    should_quit: bool,
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
        if !self.board.columns[self.focus].cards.is_empty() {
            self.lists[self.focus].select_next();
        }
    }

    fn select_prev(&mut self) {
        if !self.board.columns[self.focus].cards.is_empty() {
            self.lists[self.focus].select_previous();
        }
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
