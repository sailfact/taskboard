use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use taskboard::app::App;
use taskboard::model::{Board, Card, Column};
use taskboard::ui;

fn fixture() -> App {
    let mut app = App {
        board: Board {
            columns: [
                Column::new("Todo", [Card::new("write tests").tag("qa")]),
                Column::new("Doing", []),
                Column::new("Done", []),
            ],
        },
        ..App::default()
    };
    app.lists[0].select(Some(0));
    app
}

fn render(app: &mut App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| ui::draw(app, frame)).unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn footer_pins_the_size_readout_to_the_right_edge() {
    let mut app = fixture();
    let buffer = render(&mut app, 40, 12);
    let footer_row = 11;

    let text: String = (0..40)
        .map(|x| buffer[(x, footer_row)].symbol().to_string())
        .collect();

    assert!(text.trim_end().ends_with("40×12"), "got: {text:?}");
}

#[test]
fn focused_column_border_is_brighter() {
    let mut app = fixture();
    let buffer = render(&mut app, 60, 14);

    // Top-left corner of the first column, which has focus by default.
    let focused = buffer[(0, 4)].style();

    app.focus = 1;
    let buffer = render(&mut app, 60, 14);
    let blurred = buffer[(0, 4)].style();

    assert_eq!(focused.fg, Some(Color::White));
    assert_eq!(blurred.fg, Some(Color::DarkGray));
}

#[test]
fn selected_card_is_emphasised() {
    let mut app = fixture();
    let buffer = render(&mut app, 60, 14);

    let selected = find_cell(&buffer, "write tests").expect("card title should render");

    assert!(
        buffer[selected]
            .style()
            .add_modifier
            .contains(Modifier::BOLD)
    );
}

#[test]
fn help_popup_matches_exactly() {
    let mut app = fixture();
    app.show_help = true;

    let buffer = render(&mut app, 60, 14);
    let popup = crop(&buffer, Rect::new(6, 3, 48, 8));

    let expected = Buffer::with_lines([
        "╔══════════════════ Help ══════════════════════╗",
        "║                                              ║",
        "║  q / Esc    quit                             ║",
        "║  ?          toggle this help                 ║",
        "║                                              ║",
        "║  Resize the terminal to watch it react.      ║",
        "║                                              ║",
        "╚══════════════════════════════════════════════╝",
    ]);

    assert_eq!(strip_styles(&popup), expected);
}

/// Find the position of the first cell where `needle` starts on a row.
fn find_cell(buffer: &Buffer, needle: &str) -> Option<(u16, u16)> {
    for y in 0..buffer.area.height {
        let row: String = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol().to_string())
            .collect();

        if let Some(byte) = row.find(needle) {
            let column = row[..byte].chars().count() as u16;
            return Some((column, y));
        }
    }
    None
}

/// Copy a sub-rectangle of a buffer into a new buffer with its own origin at 0,0.
fn crop(buffer: &Buffer, area: Rect) -> Buffer {
    let mut out = Buffer::empty(Rect::new(0, 0, area.width, area.height));

    for y in 0..area.height {
        for x in 0..area.width {
            out[(x, y)] = buffer[(area.x + x, area.y + y)].clone();
        }
    }

    out
}

/// Reset every cell's style so only symbols are compared.
fn strip_styles(buffer: &Buffer) -> Buffer {
    let mut out = buffer.clone();

    for y in 0..out.area.height {
        for x in 0..out.area.width {
            out[(x, y)].set_style(Style::reset());
        }
    }

    out
}
