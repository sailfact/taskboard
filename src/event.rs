use std::io;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

/// Something the app knows how to react to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    ToggleHelp,
    None,
}

/// Block until the terminal gives us something, then translate it.
pub fn next_action() -> io::Result<Action> {
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => Ok(key_to_action(key)),
        _ => Ok(Action::None),
    }
}

fn key_to_action(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
        KeyCode::Char('?') => Action::ToggleHelp,
        _ => Action::None,
    }
}