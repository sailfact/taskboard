use ratatui::Terminal;
use ratatui::backend::TestBackend;

use taskboard::app::App;
use taskboard::event::Action;
use taskboard::model::{Board, Card, Column};
use taskboard::ui;

fn fixture() -> App {
    let mut app = App {
        board: Board {
            columns: [
                Column::new(
                    "Todo",
                    [
                        Card::new("write tests").tag("qa"),
                        Card::new("read the buffer docs").tag("docs"),
                    ],
                ),
                Column::new("Doing", [Card::new("snapshot the UI").tag("qa")]),
                Column::new("Done", [Card::new("cargo add insta").tag("setup")]),
            ],
        },
        ..App::default()
    };
    app.lists[0].select(Some(0));
    app
}

/// Render at a given size and return the buffer's `Debug` output.
fn snapshot(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| ui::draw(app, frame)).unwrap();
    format!("{:#?}", terminal.backend().buffer())
}

#[test]
fn wide() {
    let mut app = fixture();
    insta::assert_snapshot!(snapshot(&mut app, 120, 24));
}

#[test]
fn narrow() {
    let mut app = fixture();
    insta::assert_snapshot!(snapshot(&mut app, 72, 24));
}

#[test]
fn short() {
    let mut app = fixture();
    insta::assert_snapshot!(snapshot(&mut app, 100, 10));
}

#[test]
fn with_help_open() {
    let mut app = fixture();
    app.show_help = true;
    insta::assert_snapshot!(snapshot(&mut app, 100, 20));
}

#[test]
fn after_advancing_a_card() {
    let mut app = fixture();
    app.handle(Action::Advance);
    insta::assert_snapshot!(snapshot(&mut app, 120, 24));
}

#[test]
fn focus_on_the_last_column() {
    let mut app = fixture();
    app.handle(Action::FocusPrev);
    insta::assert_snapshot!(snapshot(&mut app, 120, 24));
}
