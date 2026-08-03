use std::io;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

/// Something the app knows how to react to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    ToggleHelp,
    SelectNext,
    SelectPrev,
    FocusNext,
    FocusPrev,
    Advance,
    Retreat,
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
        KeyCode::Down | KeyCode::Char('j') => Action::SelectNext,
        KeyCode::Up | KeyCode::Char('k') => Action::SelectPrev,
        KeyCode::Right | KeyCode::Char('l') => Action::FocusNext,
        KeyCode::Left | KeyCode::Char('h') => Action::FocusPrev,
        KeyCode::Char(' ') | KeyCode::Enter => Action::Advance,
        KeyCode::Backspace => Action::Retreat,
        _ => Action::None,
    }
}
