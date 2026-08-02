use std::io;

use ratatui::DefaultTerminal;

use crate::event::{self, Action};
use crate::model::Board;
use crate::ui;

#[derive(Debug, Default)]
pub struct App {
    pub board: Board,
    pub show_help: bool,
    pub frames: u64,
    should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.should_quit {
            self.frames += 1;
            terminal.draw(|frame| ui::draw(&self, frame))?;
            self.handle(event::next_action()?);
        }
        Ok(())
    }

    fn handle(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::ToggleHelp => self.show_help = !self.show_help,
            Action::None => {}
        }
    }
}
