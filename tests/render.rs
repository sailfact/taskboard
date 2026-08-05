use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

use taskboard::app::App;
use taskboard::ui;

/// Render the whole app at a given size and hand back the resulting buffer.
fn render(app: &mut App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| ui::draw(app, frame)).unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn header_shows_the_app_name() {
    let mut app = App::new();
    let buffer = render(&mut app, 60, 20);

    let first_row: String = (0..60)
        .map(|x| buffer[(x, 1)].symbol().to_string())
        .collect();

    assert!(first_row.contains("taskboard"), "got: {first_row:?}");
}

#[test]
fn narrow_terminal_has_no_details_pane() {
    let mut app = App::new();
    let buffer = render(&mut app, 80, 20);

    let text = buffer_text(&buffer);

    assert!(!text.contains("Details"));
}

#[test]
fn wide_terminal_has_a_details_pane() {
    let mut app = App::new();
    let buffer = render(&mut app, 120, 20);

    assert!(buffer_text(&buffer).contains("Details"));
}

#[test]
fn help_overlay_covers_the_board() {
    let mut app = App::new();

    let without = buffer_text(&render(&mut app, 120, 30));
    app.show_help = true;
    let with = buffer_text(&render(&mut app, 120, 30));

    assert!(!without.contains("toggle this help"));
    assert!(with.contains("toggle this help"));
}

#[test]
fn tiny_terminals_do_not_panic() {
    let mut app = App::new();

    for width in [1, 2, 5, 20, 41, 99, 100, 101] {
        for height in [1, 2, 4, 5, 30] {
            let _ = render(&mut app, width, height);
        }
    }
}

/// Flatten a buffer into one string per row, joined by newlines.
fn buffer_text(buffer: &Buffer) -> String {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
