# Ratatui from the Ground Up — 04: Testing

> | # | Module | What you add |
> |---|--------|--------------|
> | 01 | Layouts | Project structure and the region system |
> | 02 | Rendering UIs | The buffer, cells, `Text`/`Line`/`Span`, styling, the diff |
> | 03 | Widgets | Lists, gauges, tables, scrollbars, and writing your own |
> | **04** | **Testing** | **`TestBackend`, buffer assertions, snapshot tests** |
> | 05 | Applications | Event architecture, errors, config, logging, shipping |

Terminal UIs have a reputation for being untestable. They aren't — Ratatui renders into a plain
`Buffer` you can construct, compare and print, with no terminal involved. The reason the reputation
persists is that most TUI code tangles state, layout and drawing into one function, and *that* is
untestable in any language.

`taskboard` has kept those apart since module 01. This module collects the payoff.

Same rules: **every code block is a complete file at a stated path**.

Five steps, each ending in a green `cargo test`.

---

## Step 1 — The parts that need no terminal

Start with the code that was designed to be testable, and notice that no Ratatui testing machinery is
required at all.

`src/ui/layout.rs`

```rust
use ratatui::layout::{Constraint, Flex, Layout, Rect};

/// Terminals narrower than this don't get a details pane.
const DETAILS_BREAKPOINT: u16 = 100;
/// Width of the details pane when it is shown.
const DETAILS_WIDTH: u16 = 32;

/// Every region the UI needs, computed once per frame from the terminal size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppLayout {
    pub header: Rect,
    pub columns: [Rect; 3],
    pub details: Option<Rect>,
    pub footer: Rect,
}

impl AppLayout {
    pub fn compute(area: Rect) -> Self {
        let [header, body, footer] = Layout::vertical([
            Constraint::Length(4),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let (board, details) = if area.width >= DETAILS_BREAKPOINT {
            let [board, details] =
                Layout::horizontal([Constraint::Fill(1), Constraint::Length(DETAILS_WIDTH)])
                    .areas(body);
            (board, Some(details))
        } else {
            (body, None)
        };

        let columns = Layout::horizontal([Constraint::Fill(1); 3])
            .spacing(1)
            .areas(board);

        Self { header, columns, details, footer }
    }
}

/// Centre a rect of the given size inside `area`.
pub fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal]).flex(Flex::Center).areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_footer_are_fixed_height() {
        let layout = AppLayout::compute(Rect::new(0, 0, 120, 40));

        assert_eq!(layout.header.height, 4);
        assert_eq!(layout.footer.height, 1);
    }

    #[test]
    fn body_absorbs_the_remaining_height() {
        let layout = AppLayout::compute(Rect::new(0, 0, 120, 40));
        let body_height = layout.columns[0].height;

        assert_eq!(layout.header.height + body_height + layout.footer.height, 40);
    }

    #[test]
    fn details_pane_appears_at_the_breakpoint() {
        assert!(AppLayout::compute(Rect::new(0, 0, 99, 40)).details.is_none());
        assert!(AppLayout::compute(Rect::new(0, 0, 100, 40)).details.is_some());
    }

    #[test]
    fn details_pane_has_a_fixed_width() {
        let layout = AppLayout::compute(Rect::new(0, 0, 200, 40));

        assert_eq!(layout.details.map(|d| d.width), Some(DETAILS_WIDTH));
    }

    #[test]
    fn columns_do_not_overlap() {
        let layout = AppLayout::compute(Rect::new(0, 0, 120, 40));

        for pair in layout.columns.windows(2) {
            assert!(
                pair[0].right() <= pair[1].left(),
                "{:?} overlaps {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn columns_stay_inside_the_board() {
        let layout = AppLayout::compute(Rect::new(0, 0, 120, 40));
        let last = layout.columns[2];

        match layout.details {
            Some(details) => assert!(last.right() <= details.left()),
            None => assert!(last.right() <= 120),
        }
    }

    #[test]
    fn no_panic_at_any_plausible_size() {
        for width in 0..=200 {
            for height in [0, 1, 2, 3, 4, 5, 40] {
                let _ = AppLayout::compute(Rect::new(0, 0, width, height));
            }
        }
    }

    #[test]
    fn centering_is_symmetric() {
        let area = Rect::new(0, 0, 100, 50);
        let centred = center(area, Constraint::Length(40), Constraint::Length(10));

        assert_eq!(centred.left(), area.width - centred.right());
        assert_eq!(centred.top(), area.height - centred.bottom());
    }
}
```

```bash
cargo test
```

Eight tests, no terminal, no `TestBackend`, and they run in microseconds.

## What just happened

**`AppLayout::compute` is a pure function, so testing it is unremarkable.** This is the whole
argument for the module-01 structure. Layout is where geometry bugs live, and geometry is exactly the
part that needs no rendering to verify.

**`#[cfg(test)] mod tests` inside the file is the Rust convention for unit tests.** It compiles only
under `cargo test`, and `use super::*` gives it access to private items — which is how the tests can
assert on `DETAILS_BREAKPOINT` and `DETAILS_WIDTH` without making them `pub`. Tests that need to see
inside a module belong inside it; tests that use only the public API can go in `tests/`, which is
step 3.

**`no_panic_at_any_plausible_size` is the highest-value test in the file.** It asserts nothing about
correctness — it just runs `compute` 1,400 times looking for a panic. Layout code is full of `u16`
subtraction, and `u16` subtraction underflows and panics in debug builds. A zero-height terminal is
not hypothetical; it happens during window drags. This test catches the class of bug that gets
reported as "it crashed when I resized".

**Structural assertions beat coordinate assertions.** `columns_do_not_overlap` compares regions to
each other rather than pinning `Rect::new(41, 4, 39, 35)`. Pin exact coordinates and every test in
the file breaks the moment you change the spacing by one cell — for a change that was intentional.
Assert the invariant, not the arithmetic.

**`PartialEq` on `AppLayout`** costs nothing and lets you compare whole layouts when you do want an
exact check.

---

## Step 2 — Testing state transitions

`App::handle` maps an `Action` to a state change. No terminal, no rendering, no I/O.

`src/app.rs`

```rust
use std::io;

use ratatui::widgets::ListState;
use ratatui::DefaultTerminal;

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

    /// Whether the run loop is finished. Exposed for tests.
    pub fn should_quit(&self) -> bool {
        self.should_quit
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
        assert!(!app.should_quit());

        app.handle(Action::Quit);

        assert!(app.should_quit());
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
```

`src/event.rs`

```rust
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

pub fn key_to_action(key: KeyEvent) -> Action {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> Action {
        key_to_action(KeyEvent::from(code))
    }

    #[test]
    fn vim_and_arrow_keys_agree() {
        assert_eq!(press(KeyCode::Char('j')), press(KeyCode::Down));
        assert_eq!(press(KeyCode::Char('k')), press(KeyCode::Up));
        assert_eq!(press(KeyCode::Char('l')), press(KeyCode::Right));
        assert_eq!(press(KeyCode::Char('h')), press(KeyCode::Left));
    }

    #[test]
    fn both_quit_keys_quit() {
        assert_eq!(press(KeyCode::Char('q')), Action::Quit);
        assert_eq!(press(KeyCode::Esc), Action::Quit);
    }

    #[test]
    fn unbound_keys_do_nothing() {
        assert_eq!(press(KeyCode::Char('z')), Action::None);
        assert_eq!(press(KeyCode::F(7)), Action::None);
    }
}
```

`cargo test`.

## What just happened

**`Action` is what makes any of this possible.** Because `key_to_action` and `App::handle` are
separate pure functions with a plain enum between them, you can test the keymap without a terminal
and the state machine without a keyboard. Had `handle` matched on `KeyEvent` directly, every state
test would need to fabricate crossterm events; had `run` mutated state inline, there'd be nothing to
call.

**The `fixture()` helper is doing real work.** `Board::default()` is demo data that will change as the
tutorial's copy changes, and tests pinned to it would break for cosmetic reasons. A minimal fixture
with two cards, one card and none covers the interesting shapes — including the empty column, which
is where selection bugs live.

**Test the edges, not the happy path.** `emptying_a_column_clears_its_selection` and
`advancing_with_no_selection_is_a_no_op` are the two tests here that would actually have caught bugs
during module 03. Selection state that outlives its data is *the* recurring stateful-widget failure,
and it is cheap to pin down.

**`should_quit()` was added as a method** because the field is private and tests live in the same
module — `super::*` would reach it anyway, but exposing the accessor keeps the test honest about
using the same surface the rest of the app would.

**`KeyEvent::from(code)`** builds an event with no modifiers, which is all `key_to_action` looks at.
When you add modifier-sensitive bindings, use `KeyEvent::new(code, KeyModifiers::SHIFT)`.

---

# Step 3 — `TestBackend` and buffer assertions

Now the rendering. `TestBackend` is a `Backend` implementation whose "terminal" is a `Buffer` in
memory.

`tests/render.rs`

```rust
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

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
```

`src/lib.rs`

```rust
pub mod app;
pub mod event;
pub mod model;
pub mod ui;
```

### `src/main.rs`

```rust
use std::io;

use taskboard::app::App;

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::new().run(terminal))
}
```

`Cargo.toml`

```toml
[package]
name = "taskboard"
version = "0.1.0"
edition = "2024"

[dependencies]
ratatui = "0.30"
```

`cargo test`.

## What just happened

**The crate became a library plus a binary, and that was unavoidable.** Integration tests in
`tests/` link against your crate the way an external user would, so they can only see what a library
target exports. Adding `src/lib.rs` gives them something to import; `main.rs` shrinks to a launcher
that uses the same public API. Cargo picks both up automatically with no `[lib]` or `[[bin]]` section
— the file names are the configuration.

Note this forced `ui` to become `pub mod`. That's the cost of integration-testing the render pass,
and it's worth being deliberate about: if you'd rather keep `ui` private, move these tests into
`src/ui/mod.rs` under `#[cfg(test)]` instead. Same assertions, different visibility trade-off.

**`TestBackend::new(width, height)` fixes the terminal size**, which is the real superpower here. You
can assert behaviour at 41 columns without owning a terminal that can be 41 columns. Resize
behaviour, breakpoints, and squashed layouts all become ordinary tests.

**`terminal.backend().buffer()` is the rendered result.** It's a normal `Buffer` — the same type
module 02 wrote cells into — so `buffer[(x, y)]` gives you a `Cell`, and `cell.symbol()` its
contents. `TestBackend` also has `assert_buffer`, `assert_buffer_lines` and `assert_cursor_position`
for common cases.

**Substring assertions are the pragmatic default.** `buffer_text` flattens the buffer to a string and
tests ask whether something appears. It's coarse — it can't tell you *where* — but it survives
cosmetic changes. Reach for exact-position assertions only when position is the thing under test.

**`tiny_terminals_do_not_panic` earns its keep the same way its layout counterpart did**, but it
covers far more: every widget's internal arithmetic, `block.inner` on a 1×1 rect, the scrollbar's
`saturating_sub`, `CardView`'s `is_empty` guards. One loop, the entire render path, every awkward
size. If you write one rendering test, write this one.

**Styles are not compared here.** `Buffer`'s `PartialEq` *does* compare styles, but `symbol()`
extraction throws them away. Colour assertions come next.

---

## Step 4 — Exact buffers and styles

Substring matching won't catch a border drawn in the wrong colour, or content one cell off. For that,
compare whole buffers.

`tests/exact.rs`

```rust
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::Terminal;

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

    assert!(buffer[selected].style().add_modifier.contains(Modifier::BOLD));
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
```

`cargo test`.

If `help_popup_matches_exactly` fails, read the panic output before changing anything — `Buffer`'s
`Debug` impl prints the rows as readable strings, so the diff shows you the two rectangles side by
side.

## What just happened

**`Buffer::with_lines` builds an expected buffer from string literals**, sizing itself from the
lines. It's the single most useful function in Ratatui's testing surface: your assertion *looks like*
the UI, so a failure is legible instead of arithmetic.

**`assert_buffer_eq!` is gone.** It was deprecated in 0.26.3 and removed since; plain `assert_eq!`
does the job, because `Buffer` implements `PartialEq` and has a `Debug` impl designed for exactly
this. Older tutorials still use the macro.

**Styles are compared by default, which is usually not what you want in an exact test.** Two buffers
with identical text and different colours are *not* equal. `strip_styles` resets them so the
assertion is about layout and content only, and separate tests check colours at specific cells. Mix
the two and every palette tweak breaks your layout tests.

**`crop` exists because the popup is centred**, and asserting on a 60×14 buffer to test an 8-row
overlay is unreadable. Extracting the region under test is the difference between a maintainable
exact assertion and one that gets deleted in six months. Note it copies with an origin at `(0, 0)`,
because `Buffer::with_lines` builds at the origin and `PartialEq` compares `area` too.

**`find_cell` returns a position so a style assertion can be anchored to content**, not to
coordinates. `selected_card_is_emphasised` doesn't care where the title landed — only that wherever
it is, it's bold. That's the robust way to test styling.

**One caution on the `×` in the footer test.** The symbol is one cell but two bytes, so
`row[..byte].chars().count()` is needed to convert a byte offset to a column. Indexing a string by
byte offset and calling it a column is a bug that only appears once someone types a non-ASCII
character.

---

## Step 5 — Snapshots with `insta`

Exact buffer tests are precise and tedious to write by hand. Snapshot testing writes them for you and
turns review into the workflow.

`Cargo.toml`

```toml
[package]
name = "taskboard"
version = "0.1.0"
edition = "2024"

[dependencies]
ratatui = "0.30"

[dev-dependencies]
insta = "1"
```

`tests/snapshots.rs`

```rust
use ratatui::backend::TestBackend;
use ratatui::Terminal;

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
```

First run:

```bash
cargo test
```

Every snapshot test fails, because no snapshot exists yet. The recorded output is written to
`tests/snapshots/*.snap.new`. Review and accept them:

```bash
cargo install cargo-insta
cargo insta review
```

`cargo insta review` shows each snapshot as a diff and asks accept / reject / skip. Accepting renames
`.snap.new` to `.snap`. Commit those files — they're the test.

From then on, `cargo test` compares against them and any change to the rendered output fails loudly.

## What just happened

**A snapshot test is an exact assertion you didn't have to type.** You write the setup, run it once,
eyeball the result, and accept it. The cost moves from writing assertions to reviewing diffs, which
is a much better trade for UI code — where "is this right?" is a question your eyes answer faster
than your fingers.

**`{:#?}` on a `Buffer` is the right thing to snapshot.** The alternate-form `Debug` impl prints the
area, the content as one string per row, and a style summary listing only the cells where the style
*changes*. So a snapshot captures layout, text and colour, but stays readable and doesn't explode
into one line per cell.

**Snapshot names come from the test function**, so `fn wide()` in `tests/snapshots.rs` produces
`tests/snapshots/snapshots__wide.snap`. Rename a test and you orphan its snapshot; `cargo insta test
--unreferenced=delete` cleans those up.

**The danger is accepting diffs without reading them.** A snapshot suite that gets bulk-accepted is
worse than no tests, because it looks like coverage. Two habits keep it honest: keep each snapshot
small enough to actually read, and never run `cargo insta accept` on a diff you haven't looked at.
That's why the fixtures here are minimal rather than using `Board::default()`.

**Choose your sizes deliberately.** These six cover both sides of the details breakpoint, a short
terminal that forces scrollbars, the overlay path, and two state transitions. That's a real
regression suite for a UI in about forty lines.

**On flakiness:** these snapshots are deterministic because nothing in the render pass reads the
clock, the filesystem or the environment. The moment you render a timestamp — and module 05 adds a
tick — that stops being true, and you'll need to inject the value rather than read it in `draw`.
Worth keeping in mind before you put `SystemTime::now()` in a widget.

---

## Where you've landed

```
taskboard/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs        ← new
│   ├── app.rs
│   ├── event.rs
│   ├── model.rs
│   └── ui/
│       ├── mod.rs
│       ├── layout.rs
│       ├── theme.rs
│       ├── card.rs
│       ├── header.rs
│       ├── board.rs
│       ├── details.rs
│       ├── footer.rs
│       └── help.rs
└── tests/
    ├── render.rs     ← new
    ├── exact.rs      ← new
    ├── snapshots.rs  ← new
    └── snapshots/    ← generated, and committed
```

`cargo test` should report tests from five files: the two `#[cfg(test)]` modules inside `src/`, and
the three integration files.

Worth confirming:

- `cargo test` is fast — well under a second. Nothing here touches a real terminal.
- Changing `DETAILS_BREAKPOINT` breaks a layout test, a render test and two snapshots. That's the
  suite working, at three levels of granularity.
- Changing an accent colour in `theme.rs` breaks the style test and the snapshots, but *not* the
  layout or substring tests. That separation is deliberate.

### You should now be able to explain

- Why layout and state logic are testable without any Ratatui testing machinery at all.
- Why a crate needs `lib.rs` before anything in `tests/` can see it.
- What `TestBackend` replaces, and why fixing the terminal size is the point.
- When to assert on substrings, on exact buffers, and on individual cell styles.
- Why `Buffer::with_lines` plus `assert_eq!` beats the removed `assert_buffer_eq!`.
- What snapshot testing moves the cost to, and the failure mode that comes with it.

---

## Next: 05 — Applications

The last module turns this into something you'd ship. A non-blocking event loop with ticks, so the UI
can update without a keypress. Real error handling with `color_eyre`, and a panic hook that restores
the terminal instead of wrecking it. Configuration from a TOML file, so `theme.rs` stops being
hard-coded. Logging to a file, because stdout belongs to the UI. Then the release checklist:
argument parsing, build profile, and what to do about the terminal state when things go wrong.
