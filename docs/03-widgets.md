# Ratatui from the Ground Up — 03: Widgets

> | # | Module | What you add |
> | --- | -------- | -------------- |
> | 01 | Layouts | Project structure and the region system |
> | 02 | Rendering UIs | The buffer, cells, `Text`/`Line`/`Span`, styling, the diff |
> | **03** | **Widgets** | **Lists, gauges, tables, scrollbars, and writing your own** |
> | 04 | Testing | `TestBackend`, buffer assertions, snapshot tests |
> | 05 | Applications | Event architecture, errors, config, logging, shipping |

Modules 01 and 02 built a board that draws but doesn't respond. This module makes it interactive by
handing the work to widgets that own state — and then shows you what a widget actually is by
replacing one of them with your own.

Same rules: **every code block is a complete file at a stated path**, shown in full when it changes.

Where you'll finish: the same tree, minus `ui/progress.rs`, plus `ui/card.rs`. The board gains a
selected card you can move between columns with the keyboard.

Five steps. No new dependencies.

---

## Step 1 — `List` and the state problem

The cards in `board.rs` are hand-laid-out `Paragraph`s. `List` does that work and tracks selection —
but selection is state, and state changes how the render pass is wired.

### `src/model.rs`

```rust
#[derive(Debug, Clone)]
pub struct Card {
    pub title: String,
    pub tag: Option<String>,
    pub notes: String,
}

impl Card {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            tag: None,
            notes: String::new(),
        }
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }
}

#[derive(Debug, Clone)]
pub struct Column {
    pub title: String,
    pub cards: Vec<Card>,
}

impl Column {
    pub fn new(title: impl Into<String>, cards: impl IntoIterator<Item = Card>) -> Self {
        Self {
            title: title.into(),
            cards: cards.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Board {
    pub columns: [Column; 3],
}

impl Board {
    pub fn open(&self) -> usize {
        self.columns[0].cards.len() + self.columns[1].cards.len()
    }

    pub fn done(&self) -> usize {
        self.columns[2].cards.len()
    }

    pub fn total(&self) -> usize {
        self.columns.iter().map(|c| c.cards.len()).sum()
    }

    /// Fraction of the board that is complete, in the range 0.0 to 1.0.
    pub fn ratio(&self) -> f64 {
        if self.total() == 0 {
            0.0
        } else {
            self.done() as f64 / self.total() as f64
        }
    }

    /// Take the card at `index` out of `from` and append it to `to`.
    ///
    /// Returns the index the card landed at, or `None` if the move was impossible.
    pub fn move_card(&mut self, from: usize, index: usize, to: usize) -> Option<usize> {
        if from == to || index >= self.columns[from].cards.len() {
            return None;
        }

        let card = self.columns[from].cards.remove(index);
        self.columns[to].cards.push(card);

        Some(self.columns[to].cards.len() - 1)
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            columns: [
                Column::new(
                    "Todo",
                    [
                        Card::new("Read the constraint docs")
                            .tag("layout")
                            .notes(
                                "Cassowary is a soft constraint solver. Conflicting constraints \
                                 resolve to a compromise rather than raising an error, which is \
                                 why a Length in a too-small terminal simply clips.",
                            ),
                        Card::new("Sketch the board layout").tag("design").notes(
                            "Three columns, a details pane past 100 columns, help floating on top.",
                        ),
                        Card::new("Pick a colour palette").tag("design").notes(
                            "Named colours respect the user's terminal theme. RGB does not.",
                        ),
                        Card::new("Wire up selection").tag("widgets").notes(
                            "ListState owns the selected index and the scroll offset. The widget \
                             is rebuilt every frame; the state is not.",
                        ),
                    ],
                ),
                Column::new(
                    "Doing",
                    [Card::new("Split the frame into regions")
                        .tag("layout")
                        .notes("AppLayout::compute is a pure function from one Rect to many.")],
                ),
                Column::new(
                    "Done",
                    [
                        Card::new("cargo new taskboard").tag("setup"),
                        Card::new("cargo add ratatui").tag("setup"),
                    ],
                ),
            ],
        }
    }
}
```

### `src/event.rs`

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
```

### `src/app.rs`

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
```

### `src/ui/theme.rs`

```rust
use ratatui::style::{Color, Modifier, Style};

/// Accent colour for each board column, in order.
pub const COLUMN_ACCENTS: [Color; 3] = [
    Color::LightBlue,
    Color::LightYellow,
    Color::LightGreen,
];

/// Chrome: borders and separators.
pub const BORDER: Style = Style::new().fg(Color::DarkGray);

/// The border of the column that currently has the keyboard.
pub const BORDER_FOCUS: Style = Style::new().fg(Color::White);

/// De-emphasised text: counts, hints, placeholders.
pub const MUTED: Style = Style::new().fg(Color::DarkGray);

/// The app name and other primary emphasis.
pub const TITLE: Style = Style::new()
    .fg(Color::Cyan)
    .add_modifier(Modifier::BOLD);

/// A card's tag chip.
pub const TAG: Style = Style::new().fg(Color::Magenta);

/// The selected row in the focused column.
pub const SELECTED: Style = Style::new()
    .bg(Color::DarkGray)
    .add_modifier(Modifier::BOLD);

/// The selected row in a column that doesn't have the keyboard.
pub const SELECTED_BLUR: Style = Style::new().add_modifier(Modifier::DIM);

/// The help popup's surface and frame.
pub const POPUP: Style = Style::new().bg(Color::Black);
pub const POPUP_BORDER: Style = Style::new().fg(Color::Cyan);

/// The accent colour belonging to the column at `index`.
pub fn column(index: usize) -> Color {
    COLUMN_ACCENTS[index]
}
```

### `src/ui/board.rs`

```rust
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, List, ListItem, ListState, Padding};
use ratatui::Frame;

use crate::app::App;
use crate::model::Column;
use crate::ui::theme;

pub fn draw(app: &mut App, frame: &mut Frame, areas: [Rect; 3]) {
    for index in 0..areas.len() {
        let focused = index == app.focus;
        let accent = theme::column(index);

        // Split the borrow: the column and its list state come from different fields.
        let column = &app.board.columns[index];
        let state = &mut app.lists[index];

        draw_column(column, state, accent, focused, frame, areas[index]);
    }
}

fn draw_column(
    column: &Column,
    state: &mut ListState,
    accent: Color,
    focused: bool,
    frame: &mut Frame,
    area: Rect,
) {
    let title = Line::from(vec![
        Span::styled(format!(" {} ", column.title), Style::new().fg(accent)),
        Span::styled(format!("{} ", column.cards.len()), theme::MUTED),
    ]);

    let block = Block::bordered()
        .border_type(if focused {
            BorderType::Thick
        } else {
            BorderType::Rounded
        })
        .border_style(if focused {
            theme::BORDER_FOCUS
        } else {
            theme::BORDER
        })
        .padding(Padding::horizontal(1))
        .title(title);

    let items: Vec<ListItem> = column
        .cards
        .iter()
        .map(|card| {
            let mut lines = vec![Line::styled(card.title.as_str(), Style::new().fg(accent))];

            match &card.tag {
                Some(tag) => lines.push(Line::styled(format!("#{tag}"), theme::TAG)),
                None => lines.push(Line::raw("")),
            }

            ListItem::new(Text::from(lines))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(if focused {
            theme::SELECTED
        } else {
            theme::SELECTED_BLUR
        })
        .highlight_symbol(Line::from("▌"));

    frame.render_stateful_widget(list, area, state);
}
```

### 1 `src/ui/mod.rs`

```rust
mod board;
mod details;
mod footer;
mod header;
mod help;
mod layout;
mod progress;
mod theme;

use ratatui::Frame;

use crate::app::App;
use crate::ui::layout::AppLayout;

pub fn draw(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let layout = AppLayout::compute(area);

    header::draw(app, frame, layout.header);
    board::draw(app, frame, layout.columns);

    if let Some(details) = layout.details {
        details::draw(app, frame, details);
    }

    footer::draw(app, frame, layout.footer);

    if app.show_help {
        help::draw(frame, area);
    }
}
```

`cargo run`. Arrow keys or `hjkl` move around; space pushes a card to the next column, backspace
pulls it back.

### What just happened

**`ui::draw` now takes `&mut App`, and that is the headline change.** Ratatui is immediate mode, so
the *widget* is rebuilt from scratch every frame — but selection and scroll position must survive
between frames, so something outside the widget has to own them. `render_stateful_widget` takes
`&mut State`, which means mutability has to reach all the way down from the draw closure.

This is the moment where "rendering is a pure function of state" stops being literally true. It's
still nearly true: the only mutation is bookkeeping the widget does on its own state.

**`ListState` holds two things: `selected: Option<usize>` and `offset: usize`.** The offset is
scrolling, and `List` updates it during render — which is exactly why it needs `&mut`. The widget
knows how tall its area is; your app doesn't. Ask `List` to render a selection at index 40 in a
10-row area and it scrolls to bring it into view, writing the new offset back into your state. You
never compute a scroll position by hand.

**`select_next` and `select_previous` are on `ListState`, not on `List`.** They saturate rather than
wrap, and they treat `None` as "before the start", so `select_next` on an empty selection lands on
index 0. There's also `select_first`, `select_last`, and `scroll_down_by`/`scroll_up_by`. None of
them know how many items exist — `ListState` has no idea what it's selecting into — which is why
`App::clamp_selection` exists. **A stateful widget's state can outlive the data it indexes**, and
after any mutation of the underlying list, clamping is your job. Deleting a card without clamping
leaves a selection pointing past the end.

**The split borrow in `board::draw` is deliberate.** `&app.board.columns[index]` and
`&mut app.lists[index]` are disjoint fields, so the borrow checker allows both at once. Try hoisting
them into a helper taking `&mut App` plus an index and you'll be fighting it — passing the two pieces
separately is the idiomatic fix, and it also makes `draw_column` testable without an `App`.

**`highlight_symbol` takes `Into<Line>` in 0.30**, not `&str` as in earlier versions, so it can be
styled independently. It's no longer callable in const context. `highlight_spacing` controls whether
the symbol's column is reserved when nothing is selected — `HighlightSpacing::Always` prevents the
list jumping sideways when a selection appears.

---

## Step 2 — `LineGauge` replaces the hand-rolled bar

`ui/progress.rs` was written in module 02 to show what cell-level drawing looks like. Its job is done.

Delete the file:

```bash
rm src/ui/progress.rs
```

### `src/ui/header.rs`

```rust
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, LineGauge, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme;

pub fn draw(app: &App, frame: &mut Frame, area: Rect) {
    let board = &app.board;
    let percent = (board.ratio() * 100.0).round() as u16;

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::BORDER);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [title_area, bar_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(inner);

    let title = Line::from(vec![
        Span::styled("taskboard", theme::TITLE),
        Span::raw("   "),
        Span::styled(format!("{} open", board.open()), theme::MUTED),
        Span::styled("  ·  ", theme::MUTED),
        Span::styled(format!("{} done", board.done()), theme::MUTED),
        Span::raw("   "),
        Span::styled(format!("{percent}%"), theme::TITLE),
    ]);

    frame.render_widget(Paragraph::new(title), title_area);

    let gauge = LineGauge::default()
        .ratio(board.ratio())
        .filled_style(Style::new().fg(theme::COLUMN_ACCENTS[2]))
        .unfilled_style(theme::MUTED)
        .filled_symbol("━")
        .unfilled_symbol("─")
        .label("");

    frame.render_widget(gauge, bar_area);
}
```

### 2 `src/ui/mod.rs`

```rust
mod board;
mod details;
mod footer;
mod header;
mod help;
mod layout;
mod theme;

use ratatui::Frame;

use crate::app::App;
use crate::ui::layout::AppLayout;

pub fn draw(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let layout = AppLayout::compute(area);

    header::draw(app, frame, layout.header);
    board::draw(app, frame, layout.columns);

    if let Some(details) = layout.details {
        details::draw(app, frame, details);
    }

    footer::draw(app, frame, layout.footer);

    if app.show_help {
        help::draw(frame, area);
    }
}
```

`cargo run`, then move a card with space. The bar tracks it.

### Step 2 What just happened

**Twenty-two lines became eight, and the eight are more correct.** `LineGauge` clamps, handles a
zero-width area, and does the same eighth-block arithmetic `progress.rs` did by hand.

**`ratio` panics outside 0.0–1.0.** Not a soft clamp — a panic, in render code. `Board::ratio` can't
produce an out-of-range value, so this is safe, but any gauge fed by division or user input wants a
`.clamp(0.0, 1.0)` first. `percent(u16)` is the alternative constructor.

**`LineGauge` is one row; `Gauge` is a block.** `Gauge` fills its whole area, centres a label, and
renders at eighth-cell resolution horizontally — the right choice for a progress screen. `LineGauge`
is the right choice here because the header reserves exactly one row.

**The 0.30 style methods are `filled_style` and `unfilled_style`**, which set the two halves
independently. `gauge_style` still exists but is deprecated and confusingly maps its `bg` onto the
filled part's foreground. Older tutorials use it; don't. `filled_symbol` and `unfilled_symbol` are
also new in 0.30, replacing the deprecated `line_set`.

**The empty `label("")` is deliberate.** `LineGauge` renders a percentage label by default, and the
header already shows one; without this you get it twice.

---

## Step 3 — `Table` in the details pane

Lists are one column of items. Tables are several, aligned.

### `src/ui/details.rs`

```rust
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Cell, Padding, Paragraph, Row, Table, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme;

pub fn draw(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::BORDER)
        .padding(Padding::horizontal(1))
        .title(Line::styled(" Details ", theme::TITLE).centered());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [summary_area, focus_area] =
        Layout::vertical([Constraint::Length(5), Constraint::Fill(1)]).areas(inner);

    frame.render_widget(summary(app), summary_area);
    frame.render_widget(focus(app), focus_area);
}

fn summary(app: &App) -> Table<'static> {
    let rows = app.board.columns.iter().enumerate().map(|(index, column)| {
        let marker = Span::styled("■", Style::new().fg(theme::column(index)));
        let name = Span::raw(column.title.clone());
        let count = Span::styled(column.cards.len().to_string(), theme::MUTED);

        Row::new([Cell::from(marker), Cell::from(name), Cell::from(count)])
    });

    let widths = [
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(3),
    ];

    Table::new(rows, widths)
        .column_spacing(1)
        .header(Row::new(["", "Column", "  #"]).style(theme::MUTED))
}

fn focus(app: &App) -> Paragraph<'static> {
    let Some(card) = app.selected_card() else {
        return Paragraph::new(Line::styled("Nothing selected.", theme::MUTED));
    };

    let mut lines = vec![Line::styled(
        card.title.clone(),
        Style::new().add_modifier(Modifier::BOLD),
    )];

    if let Some(tag) = &card.tag {
        lines.push(Line::styled(format!("#{tag}"), theme::TAG));
    }

    if !card.notes.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled(card.notes.clone(), theme::MUTED));
    }

    Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true })
}
```

`cargo run` past 100 columns. Move between cards and the lower half follows the selection.

## Step 3 - What just happened

**`Table::new(rows, widths)` takes constraints, and they're the same `Constraint` type as `Layout`.**
Column widths are solved by the same Cassowary solver from module 01 — `Fill(1)` on the name column
absorbs the leftover, `Length` pins the marker and count. Everything you learned about constraints
applies unchanged.

**`Row` and `Cell` both accept anything convertible to `Text`.** A cell can be multi-line, in which
case you'll want `Row::height`. Rows are one line tall by default.

**Note the return types.** `summary` returns `Table<'static>` and clones each column title, because
the rows are built inside the function and the borrow of `app` would end at the return. That's the
lifetime trade-off from module 02 showing up in practice: clone the small string, don't fight it.
`focus` does the same. If you want to avoid the allocations, build the widget in `draw` where the
borrow is still live.

**`row_highlight_style` is the 0.29+ name.** `Table::highlight_style` is deprecated, because tables
grew `column_highlight_style` and `cell_highlight_style` and the unqualified name became ambiguous.
This table isn't interactive, so none are used — but `TableState` works exactly like `ListState` if
you want it to be.

**`column_spacing` is the gap between columns**, and it eats into the space the width constraints are
competing for, just like `Layout::spacing`.

---

## Step 4 — `Scrollbar`

A list that scrolls without saying so is a list that looks broken.

### Step 4 —`src/ui/board.rs`

```rust
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, List, ListItem, ListState, Padding, Scrollbar, ScrollbarOrientation,
    ScrollbarState,
};
use ratatui::Frame;

use crate::app::App;
use crate::model::Column;
use crate::ui::theme;

/// Each card renders as a title line plus a tag line.
const CARD_LINES: usize = 2;

pub fn draw(app: &mut App, frame: &mut Frame, areas: [Rect; 3]) {
    for index in 0..areas.len() {
        let focused = index == app.focus;
        let accent = theme::column(index);

        let column = &app.board.columns[index];
        let state = &mut app.lists[index];

        draw_column(column, state, accent, focused, frame, areas[index]);
    }
}

fn draw_column(
    column: &Column,
    state: &mut ListState,
    accent: Color,
    focused: bool,
    frame: &mut Frame,
    area: Rect,
) {
    let title = Line::from(vec![
        Span::styled(format!(" {} ", column.title), Style::new().fg(accent)),
        Span::styled(format!("{} ", column.cards.len()), theme::MUTED),
    ]);

    let block = Block::bordered()
        .border_type(if focused {
            BorderType::Thick
        } else {
            BorderType::Rounded
        })
        .border_style(if focused {
            theme::BORDER_FOCUS
        } else {
            theme::BORDER
        })
        .padding(Padding::horizontal(1))
        .title(title);

    let inner_height = block.inner(area).height as usize;

    let items: Vec<ListItem> = column
        .cards
        .iter()
        .map(|card| {
            let mut lines = vec![Line::styled(card.title.as_str(), Style::new().fg(accent))];

            match &card.tag {
                Some(tag) => lines.push(Line::styled(format!("#{tag}"), theme::TAG)),
                None => lines.push(Line::raw("")),
            }

            ListItem::new(Text::from(lines))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(if focused {
            theme::SELECTED
        } else {
            theme::SELECTED_BLUR
        })
        .highlight_symbol(Line::from("▌"));

    frame.render_stateful_widget(list, area, state);

    draw_scrollbar(column, state, inner_height, frame, area);
}

fn draw_scrollbar(
    column: &Column,
    state: &ListState,
    inner_height: usize,
    frame: &mut Frame,
    area: Rect,
) {
    let total = column.cards.len() * CARD_LINES;

    if total <= inner_height {
        return;
    }

    let mut scroll = ScrollbarState::new(total.saturating_sub(inner_height))
        .position(state.offset() * CARD_LINES);

    let track = area.inner(Margin {
        horizontal: 0,
        vertical: 1,
    });

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_style(theme::BORDER_FOCUS)
            .track_style(theme::BORDER),
        track,
        &mut scroll,
    );
}
```

`cargo run`, shrink the window vertically, then hold `j` in the Todo column.

### Step 4 — What just happened

**`Scrollbar` renders *over* the block's right border**, which is why it's given the full `area` (not
`block.inner`) with one row trimmed off each end so it doesn't paint the corners. That's the intended
usage, and it's why the scrollbar is drawn after the list rather than laid out beside it — there's no
layout region for it.

**`ScrollbarState` counts scrollable positions, not items.** `ScrollbarState::new(n)` means "there
are `n` positions past the top", so the correct argument is `total - visible`, not `total`. Pass the
item count and the thumb never reaches the bottom. This is the single most common mistake with this
widget.

**Note the unit mismatch this exposes.** `ListState::offset()` counts *items*; the scrollbar wants
*rows*. Each card is two lines, hence `CARD_LINES`. Any time an item isn't one row tall, something
has to reconcile the two, and nothing in the library does it for you.

**The state here is local, not stored in `App`.** It's derived entirely from the list state and the
area, so there's nothing to remember between frames — construct it, render, drop it. Not every
stateful widget needs a home in your app struct; the test is whether the state carries information
that can't be recomputed.

**`ScrollbarOrientation`** also offers `VerticalLeft`, `HorizontalBottom` and `HorizontalTop`.
`begin_symbol` and `end_symbol` default to `◄`/`►`-style arrows; `None` on both gives the cleaner
look used here.

---

## Step 5 — Writing a widget

Everything so far has been a `draw` function taking a `Frame`. That's fine, but it isn't what the
library does. Here's the real thing.

### `src/ui/card.rs`

```rust
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Padding, Widget};

use crate::model::Card;
use crate::ui::theme;

/// A single card, rendered as a two-row chip.
///
/// Borrows the card rather than owning it: the widget lives for exactly one render.
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

        let block = Block::new().padding(Padding::horizontal(1)).style(
            if self.selected {
                theme::SELECTED
            } else {
                Style::new()
            },
        );

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
```

### Step 5 —`src/ui/board.rs`

```rust
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, List, ListItem, ListState, Padding, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Widget,
};
use ratatui::Frame;

use crate::app::App;
use crate::model::Column;
use crate::ui::card::CardView;
use crate::ui::theme;

/// Each card renders as a title line plus a tag line.
const CARD_LINES: usize = 2;

pub fn draw(app: &mut App, frame: &mut Frame, areas: [Rect; 3]) {
    for index in 0..areas.len() {
        let focused = index == app.focus;
        let accent = theme::column(index);

        let column = &app.board.columns[index];
        let state = &mut app.lists[index];

        draw_column(column, state, accent, focused, frame, areas[index]);
    }
}

fn draw_column(
    column: &Column,
    state: &mut ListState,
    accent: Color,
    focused: bool,
    frame: &mut Frame,
    area: Rect,
) {
    let title = Line::from(vec![
        Span::styled(format!(" {} ", column.title), Style::new().fg(accent)),
        Span::styled(format!("{} ", column.cards.len()), theme::MUTED),
    ]);

    let block = Block::bordered()
        .border_type(if focused {
            BorderType::Thick
        } else {
            BorderType::Rounded
        })
        .border_style(if focused {
            theme::BORDER_FOCUS
        } else {
            theme::BORDER
        })
        .padding(Padding::horizontal(1))
        .title(title);

    let inner_height = block.inner(area).height as usize;

    let items: Vec<ListItem> = column
        .cards
        .iter()
        .enumerate()
        .map(|(index, card)| {
            let selected = focused && state.selected() == Some(index);
            let view = CardView::new(card).accent(accent).selected(selected);

            // Render the widget into a two-row buffer, then hand those rows to the list.
            let mut scratch = ratatui::buffer::Buffer::empty(Rect::new(0, 0, 40, 2));
            view.render(scratch.area, &mut scratch);

            ListItem::new(buffer_to_lines(&scratch))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_symbol(Line::from("▌"));

    frame.render_stateful_widget(list, area, state);

    draw_scrollbar(column, state, inner_height, frame, area);
}

/// Read a buffer's rows back out as styled lines.
fn buffer_to_lines(buffer: &ratatui::buffer::Buffer) -> Vec<Line<'static>> {
    (0..buffer.area.height)
        .map(|y| {
            let spans = (0..buffer.area.width)
                .map(|x| {
                    let cell = &buffer[(x, y)];
                    Span::styled(cell.symbol().to_string(), cell.style())
                })
                .collect::<Vec<_>>();

            Line::from(spans)
        })
        .collect()
}

fn draw_scrollbar(
    column: &Column,
    state: &ListState,
    inner_height: usize,
    frame: &mut Frame,
    area: Rect,
) {
    let total = column.cards.len() * CARD_LINES;

    if total <= inner_height {
        return;
    }

    let mut scroll = ScrollbarState::new(total.saturating_sub(inner_height))
        .position(state.offset() * CARD_LINES);

    let track = area.inner(Margin {
        horizontal: 0,
        vertical: 1,
    });

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_style(theme::BORDER_FOCUS)
            .track_style(theme::BORDER),
        track,
        &mut scroll,
    );
}
```

### 3 `src/ui/mod.rs`

```rust
mod board;
mod card;
mod details;
mod footer;
mod header;
mod help;
mod layout;
mod theme;

use ratatui::Frame;

use crate::app::App;
use crate::ui::layout::AppLayout;

pub fn draw(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let layout = AppLayout::compute(area);

    header::draw(app, frame, layout.header);
    board::draw(app, frame, layout.columns);

    if let Some(details) = layout.details {
        details::draw(app, frame, details);
    }

    footer::draw(app, frame, layout.footer);

    if app.show_help {
        help::draw(frame, area);
    }
}
```

`cargo run`. It looks near-identical — that's the point. The rendering moved without the output
changing.

### Step 5 — What just happened

**`Widget` has exactly one method.**

```rust
pub trait Widget {
    fn render(self, area: Rect, buf: &mut Buffer);
}
```

`self` by value, a rectangle, a buffer. Everything in the library — `Paragraph`, `List`, `Block`,
`Clear` — is that signature and nothing more. `frame.render_widget(w, area)` is a thin wrapper that
calls it with the frame's buffer. **`Frame` is a convenience, not a requirement**, which is why
`CardView` can compose `Block` and `Line` directly by calling `.render(rect, buf)` on them.

**Consuming `self` is what makes the builder pattern work.** `.accent(x)` takes `mut self` and
returns `Self`, so the chain is one moved value with no clones. The cost is that a widget can't be
rendered twice — which is fine, because you rebuild it every frame anyway. When you *do* need to
render one twice, most library widgets also implement `Widget for &T`, so `frame.render_widget(&w,
area)` works.

**`StatefulWidget` is the same shape with one more argument:**

```rust
pub trait StatefulWidget {
    type State;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}
```

`List` implements it with `type State = ListState`. If `CardView` needed to remember something across
frames — a scroll offset within a long card — this is the trait it would implement instead.

**Bounds checking is now your problem.** `CardView::render` starts with `area.is_empty()` and checks
`inner` too. Widgets get handed rectangles they didn't choose, including zero-sized ones in a squashed
terminal, and the library will not stop you writing outside your area — `buf[(x, y)]` will happily
panic, or worse, scribble on a neighbour. Every widget in the library begins with a guard like this.

**About the scratch buffer.** Rendering `CardView` into a temporary `Buffer` and reading it back as
`Line`s is a genuine technique — `Buffer::empty` plus reading `cell.symbol()` and `cell.style()` — and
it's how you compose a `Widget` into something that only accepts `Text`. But it costs an allocation
per card per frame, and the fixed width of 40 is a wart. The honest lesson: `List` wants `ListItem`s,
not widgets. If your item rendering is complex enough to want a widget, you've outgrown `List` and
should implement `StatefulWidget` for the whole column, laying out cards yourself with the `Layout`
code from module 01 and tracking your own offset.

That's the decision this file is really teaching. Reach for `List` when items are text. Write a
`StatefulWidget` when they aren't — and then you own the scrolling.

---

## Where you've landed

```bash
taskboard/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── app.rs
    ├── event.rs
    ├── model.rs
    └── ui/
        ├── mod.rs
        ├── layout.rs
        ├── theme.rs
        ├── card.rs       ← new
        ├── header.rs
        ├── board.rs
        ├── details.rs
        ├── footer.rs
        └── help.rs
```

`ui/progress.rs` is gone.

Run it and confirm:

- `hjkl` and the arrow keys move selection and focus; the focused column has a thicker, brighter border.
- Space advances a card, backspace retreats it, and the header gauge follows.
- Emptying a column leaves no stale selection, and the details pane says so.
- A short window makes a scrollbar appear, and its thumb reaches the bottom when the list does.

That last one is the assertion worth checking carefully — it's the `total - visible` detail, and it's
wrong in a lot of published Ratatui code.

## You should now be able to explain

- Why `ui::draw` had to take `&mut App`, and what that says about immediate mode.
- What `ListState` holds, who updates the offset, and why clamping after a mutation is your job.
- Why `Table` column widths use the same `Constraint` type as `Layout`.
- Why `ScrollbarState::new` takes `total - visible` and not `total`.
- The `Widget` trait's one method, and why it takes `self` by value.
- When `List` is the right tool and when you should write a `StatefulWidget` instead.

---

## Next: 04 — Testing

`AppLayout::compute` has been a pure `Rect -> Rects` function since module 01, and `App::handle` is a
pure state transition. Both are testable without a terminal. Then `TestBackend` renders into a
`Buffer` you can compare against string literals, and `insta` turns those comparisons into snapshots
you review rather than write. The tree gains a `tests/` directory.
