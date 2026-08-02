# Ratatui from the Ground Up — 01: Layouts

> **The series.** You build one app, `taskboard` — a terminal kanban board — across five modules.
> Each module extends the same codebase. Nothing gets thrown away.
>
> | # | Module | What you add |
> |---|--------|--------------|
> | **01** | **Layouts** | **Project structure and the region system that everything else renders into** |
> | 02 | Rendering UIs | The buffer, cells, `Text`/`Line`/`Span`, styling, the diff |
> | 03 | Widgets | Lists, tables, gauges, stateful widgets, custom widgets |
> | 04 | Testing | `TestBackend`, buffer assertions, snapshot tests |
> | 05 | Applications | Event architecture, errors, config, logging, shipping |

## How to read this

Every code block below is a **complete file at a stated path**. There are no illustrative snippets
to mentally discard — if it's in a code block, it belongs in your project. When a file needs to
change, the whole file is shown again so you can replace it outright rather than patching by eye.

Five steps. Each one ends with a binary you can run.

---

## What you're building in this module

```
╭──────────────────────────────────────────────────────────────────────────────╮
│ taskboard · 4 open · 2 done                                                  │
╰──────────────────────────────────────────────────────────────────────────────╯
╭───── Todo (3) ─────╮ ╭──── Doing (1) ─────╮ ╭──── Done (2) ──────╮╭ Details ─╮
│ ┌────────────────┐ │ │ ┌────────────────┐ │ │ ┌────────────────┐ ││6 cards   │
│ │Read constraints│ │ │ │Split the frame │ │ │ │cargo new       │ ││          │
│ └────────────────┘ │ │ └────────────────┘ │ │ └────────────────┘ ││Nothing   │
│                    │ │                    │ │                    ││selected. │
│ ┌────────────────┐ │ │                    │ │ ┌────────────────┐ ││          │
│ │Sketch the board│ │ │                    │ │ │cargo add       │ ││          │
│ └────────────────┘ │ │                    │ │ └────────────────┘ ││          │
╰────────────────────╯ ╰────────────────────╯ ╰────────────────────╯╰──────────╯
 q quit · ? help                                                   resize me →
```

Resizable, with a details pane that appears past 100 columns and a help modal that floats over the
top. Interaction is limited to quit and help on purpose — layout is a pure function from one
rectangle to many, and it's worth learning while nothing else is moving.

The finished tree:

```
taskboard/
├── Cargo.toml
└── src/
    ├── main.rs        entry point, terminal lifecycle
    ├── app.rs         application state and the run loop
    ├── event.rs       terminal input → actions
    ├── model.rs       the board, columns and cards
    └── ui/
        ├── mod.rs     the render pass
        ├── layout.rs  ← the subject of this module
        ├── header.rs
        ├── board.rs
        ├── details.rs
        ├── footer.rs
        └── help.rs
```

That split is not ceremony for its own sake. `ui/layout.rs` computes regions and draws nothing;
every other `ui/` file draws into a region it is handed and computes nothing. Keeping that boundary
clean is what makes module 04's tests possible — you can assert on the layout without rendering a
single cell.

---

# Step 1 — Scaffold

```bash
cargo new taskboard
cd taskboard
cargo add ratatui
```

### `Cargo.toml`

```toml
[package]
name = "taskboard"
version = "0.1.0"
edition = "2024"

[dependencies]
ratatui = "0.30"
```

Ratatui 0.30 requires **Rust 1.86 or newer** — check with `rustc --version`.

Two things about that single dependency. First, Ratatui became a workspace in 0.30
(`ratatui-core`, `ratatui-widgets`, `ratatui-crossterm`, …), but as an application author you depend
on the umbrella `ratatui` crate and ignore the split entirely; it only matters if you're publishing
a widget library. Second, Ratatui re-exports its terminal backend, so crossterm is available at
`ratatui::crossterm` with no second dependency and no version-mismatch risk.

### `src/model.rs`

The domain, with no idea a terminal exists:

```rust
#[derive(Debug, Clone)]
pub struct Card {
    pub title: String,
}

impl Card {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into() }
    }
}

#[derive(Debug, Clone)]
pub struct Column {
    pub title: String,
    pub cards: Vec<Card>,
}

impl Column {
    pub fn new<T: Into<String>>(
        title: impl Into<String>,
        cards: impl IntoIterator<Item = T>,
    ) -> Self {
        Self {
            title: title.into(),
            cards: cards.into_iter().map(Card::new).collect(),
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
}

impl Default for Board {
    fn default() -> Self {
        Self {
            columns: [
                Column::new(
                    "Todo",
                    [
                        "Read the constraint docs",
                        "Sketch the board layout",
                        "Pick a colour palette",
                    ],
                ),
                Column::new("Doing", ["Split the frame into regions"]),
                Column::new("Done", ["cargo new taskboard", "cargo add ratatui"]),
            ],
        }
    }
}
```

Fixing the board at exactly three columns (`[Column; 3]` rather than `Vec<Column>`) is a deliberate
call that pays off shortly: the layout code can hand back `[Rect; 3]` and the compiler checks the
count for us.

### `src/event.rs`

Input translation, kept separate from state changes:

```rust
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
```

`event::read()` blocks, so the app burns no CPU while idle. The `KeyEventKind::Press` check matters
on Windows, where you otherwise receive both a press and a release and every keystroke fires twice.
Module 05 replaces this whole file with a non-blocking, tick-driven event loop; the `Action` enum is
the seam that lets that happen without touching anything else.

### `src/app.rs`

```rust
use std::io;

use ratatui::DefaultTerminal;

use crate::event::{self, Action};
use crate::model::Board;
use crate::ui;

#[derive(Debug, Default)]
pub struct App {
    pub board: Board,
    pub show_help: bool,
    should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.should_quit {
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
```

Draw, then wait, then apply — the entire loop. `terminal.draw` takes a closure that paints one
complete frame. Ratatui is *immediate mode*: you redescribe the whole UI every frame and it diffs
against the previous one to emit the minimum escape sequences. That's module 02's territory; for now
just note that `ui::draw` gets `&App` and can't mutate anything, which is exactly the discipline you
want in rendering code.

### `src/ui/mod.rs`

```rust
use ratatui::Frame;

use crate::app::App;

pub fn draw(_app: &App, frame: &mut Frame) {
    frame.render_widget("taskboard — press q to quit", frame.area());
}
```

### `src/main.rs`

```rust
mod app;
mod event;
mod model;
mod ui;

use std::io;

use crate::app::App;

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::new().run(terminal))
}
```

`ratatui::run` (new in 0.30) enters raw mode, switches to the alternate screen, hides the cursor,
installs a panic hook so a crash doesn't leave your terminal wrecked, runs the closure, and undoes
all of it on the way out. Older code does this by hand with `ratatui::init()` and
`ratatui::restore()`, which still works.

```bash
cargo run
```

One line of text, `q` quits. That's the skeleton — now we give it regions.

---

# Step 2 — The first split

### `src/ui/layout.rs`

```rust
use ratatui::layout::{Constraint, Layout, Rect};

/// Every region the UI needs, computed once per frame from the terminal size.
#[derive(Debug, Clone, Copy)]
pub struct AppLayout {
    pub header: Rect,
    pub body: Rect,
    pub footer: Rect,
}

impl AppLayout {
    pub fn compute(area: Rect) -> Self {
        let [header, body, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        Self { header, body, footer }
    }
}
```

### `src/ui/mod.rs`

```rust
mod layout;

use ratatui::widgets::Block;
use ratatui::Frame;

use crate::app::App;
use crate::ui::layout::AppLayout;

pub fn draw(_app: &App, frame: &mut Frame) {
    let layout = AppLayout::compute(frame.area());

    frame.render_widget(Block::bordered().title("header"), layout.header);
    frame.render_widget(Block::bordered().title("body"), layout.body);
    frame.render_widget("footer", layout.footer);
}
```

`cargo run`, then resize the window. The header and footer stay pinned; the body absorbs everything.

## What just happened

**`Rect` is the only geometry Ratatui has.**

```rust
pub struct Rect { pub x: u16, pub y: u16, pub width: u16, pub height: u16 }
```

Units are terminal cells, not pixels. Origin is top-left, `y` grows **downwards**. Everything is
`u16`, so there are no negative coordinates and no fractions — layout is integer arithmetic all the
way down. That's why terminal UIs never have sub-pixel alignment bugs and always have off-by-one
bugs. `frame.area()` hands you the whole terminal as one `Rect`.

**`Layout::vertical` means items flow top to bottom.** It's shorthand for
`Layout::default().direction(Direction::Vertical).constraints(...)`. If the naming trips you up,
read it as "the direction things stack", not "the direction the cuts run".

**`.areas()` destructures into a fixed-size array**, and the constraint count is checked against the
array size *at compile time*. Add a fourth constraint above without changing `let [header, body,
footer]` and it won't build. Pre-0.28 code used `.split()` and indexed with `[0]`, `[1]`, `[2]`,
which silently panicked at runtime whenever someone edited the constraints. This is the single
biggest ergonomics improvement in modern Ratatui and the reason `AppLayout` has named fields instead
of returning a slice.

**Constraints are the whole game.** Reference:

| Constraint | Meaning |
|---|---|
| `Length(n)` | exactly `n` cells |
| `Min(n)` | at least `n` cells, grows to fill |
| `Max(n)` | at most `n` cells |
| `Percentage(p)` | `p`% of the parent |
| `Ratio(a, b)` | `a/b` of the parent |
| `Fill(w)` | leftover space, shared with other `Fill`s in proportion to `w` |

The part that catches people out: Ratatui's layout is a **soft constraint solver** (Cassowary, via
the `kasuari` crate), not a sequential allocator. It doesn't walk your list handing out space until
it runs out. It states every constraint at once — including invisible ones like "the regions must
exactly fill the parent" and "regions must not overlap" — and finds the solution violating the fewest,
weighted by priority. Three consequences worth internalising:

- **Constraints are wishes, not guarantees.** `Length(3)` in a two-row terminal gets two rows.
  Nothing errors; your widget is simply clipped. Design for the small case.
- **They can conflict, and you get a compromise rather than an error.** Three `Percentage(50)`s in
  one layout produce something that sums to 100% and probably isn't what you meant.
- **Priority runs roughly:** fill the parent exactly → `Min`/`Max` bounds → `Length` →
  `Percentage`/`Ratio` → `Fill`. `Fill` yields to everything, which is precisely why it's the right
  choice for "whatever's left".

So `[Length(3), Fill(1), Length(1)]` reads as: three rows of header (one row of content plus two of
border), one row of footer, body takes the rest. That `Length` + `Fill` pairing is the workhorse of
almost every app shell. Mixing `Percentage` with `Length` in the same layout is where people get into
trouble — think hard about what it does at 40 columns before you reach for it.

---

# Step 3 — Header and footer

### `src/ui/header.rs`

```rust
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(app: &App, frame: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        "taskboard ".bold(),
        format!("· {} open · {} done", app.board.open(), app.board.done()).dim(),
    ]);

    let header = Paragraph::new(title).block(Block::bordered().border_type(BorderType::Rounded));

    frame.render_widget(header, area);
}
```

### `src/ui/footer.rs`

```rust
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::Frame;

pub fn draw(frame: &mut Frame, area: Rect) {
    let [left, right] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(14)]).areas(area);

    frame.render_widget(Line::from(" q quit · ? help").dim(), left);
    frame.render_widget(Line::from("resize me →").right_aligned().dim(), right);
}
```

### `src/ui/mod.rs`

```rust
mod footer;
mod header;
mod layout;

use ratatui::widgets::Block;
use ratatui::Frame;

use crate::app::App;
use crate::ui::layout::AppLayout;

pub fn draw(app: &App, frame: &mut Frame) {
    let layout = AppLayout::compute(frame.area());

    header::draw(app, frame, layout.header);
    frame.render_widget(Block::bordered().title("body"), layout.body);
    footer::draw(frame, layout.footer);
}
```

`cargo run`.

The footer is the interesting one: it takes a **one-row-tall `Rect` and splits it horizontally**.
Layout doesn't care about the scale — the same `Layout` type that carved up the whole terminal
divides a single row into a `Fill(1)` hint on the left and a fixed `Length(14)` on the right. This
composability is the point. Any `Rect` you're handed is just a new parent.

Note also that each `ui/` file takes an `area: Rect` and never asks where it came from. That's the
contract: **layout decides where, widgets decide what.** Every file in `ui/` from here on follows it.

---

# Step 4 — Nesting the board

Now `layout.rs` earns its keep. Replace it entirely.

### `src/ui/layout.rs`

```rust
use ratatui::layout::{Constraint, Layout, Rect};

/// Terminals narrower than this don't get a details pane.
const DETAILS_BREAKPOINT: u16 = 100;
/// Width of the details pane when it is shown.
const DETAILS_WIDTH: u16 = 32;

/// Every region the UI needs, computed once per frame from the terminal size.
#[derive(Debug, Clone, Copy)]
pub struct AppLayout {
    pub header: Rect,
    pub columns: [Rect; 3],
    pub details: Option<Rect>,
    pub footer: Rect,
}

impl AppLayout {
    pub fn compute(area: Rect) -> Self {
        let [header, body, footer] = Layout::vertical([
            Constraint::Length(3),
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
```

### `src/ui/board.rs`

```rust
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::model::Column;

/// One row of text plus a top and bottom border.
const CARD_HEIGHT: u16 = 3;

pub fn draw(app: &App, frame: &mut Frame, areas: [Rect; 3]) {
    for (column, area) in app.board.columns.iter().zip(areas) {
        draw_column(column, frame, area);
    }
}

fn draw_column(column: &Column, frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(Line::from(format!(" {} ({}) ", column.title, column.cards.len())).centered());

    // Work out the content area *before* the block is moved into the frame.
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let card_areas = Layout::vertical(vec![Constraint::Length(CARD_HEIGHT); column.cards.len()])
        .spacing(1)
        .flex(Flex::Start)
        .split(inner);

    for (card, &card_area) in column.cards.iter().zip(card_areas.iter()) {
        frame.render_widget(
            Paragraph::new(card.title.as_str()).block(Block::bordered()),
            card_area,
        );
    }
}
```

### `src/ui/mod.rs`

```rust
mod board;
mod footer;
mod header;
mod layout;

use ratatui::Frame;

use crate::app::App;
use crate::ui::layout::AppLayout;

pub fn draw(app: &App, frame: &mut Frame) {
    let layout = AppLayout::compute(frame.area());

    header::draw(app, frame, layout.header);
    board::draw(app, frame, layout.columns);
    footer::draw(frame, layout.footer);
}
```

`cargo run`. Drag the window wider than 100 columns — nothing appears yet, because the details
region exists but nobody draws into it. That's step 5.

## What just happened

**Layouts nest by feeding a region back in as the next parent.** `body` becomes the parent of
`board` and `details`; `board` becomes the parent of the three columns. There's no special API for
this — a `Rect` is a `Rect`.

**`[Constraint::Fill(1); 3]`** gives three equal columns. When the width doesn't divide by three the
solver hands the remainder to one column, so it ends up a cell wider. That's unavoidable with integer
cells and normal.

**`.spacing(1)`** puts a gap *between* regions. Its sibling `.margin(1)` puts a gap *around* the whole
group; `.horizontal_margin()` and `.vertical_margin()` do one axis each. They stack, and all of them
eat into the space your constraints are competing for.

**`block.inner(area)` is the most important line in the file.** A bordered block consumes one cell on
every side, so its content area is smaller than the area it was given. Lay children out against the
outer rect and they'll be drawn on top of the border. Note the ordering: `render_widget` takes the
block by value, so you must capture `inner` before handing it over. "Why is my content one cell off"
is almost always this.

**`.split()` instead of `.areas()`**, because the card count is a runtime value. `split` returns
`Rc<[Rect]>` and can't check anything at compile time — use it only when the count genuinely varies,
and `.areas()` everywhere else. (There's also `.try_areas()`, which returns a `Result` instead of
panicking on a count mismatch.)

**`.flex(Flex::Start)` packs the cards at the top.** The cards are `Length(3)` each, so in a tall
column there's slack left over, and `Flex` decides where it goes:

| Variant | Behaviour |
|---|---|
| `Flex::Start` | items packed at the start, slack at the end |
| `Flex::End` | slack at the start |
| `Flex::Center` | slack split evenly on both sides of the group |
| `Flex::SpaceBetween` | slack spread between items, none at the edges |
| `Flex::SpaceEvenly` | slack spread evenly between items *and* at the edges |
| `Flex::SpaceAround` | as `SpaceEvenly`, but gaps between items are twice the edge gaps |
| `Flex::Legacy` | pre-0.26 behaviour: all slack dumped on the last item |

Delete the `.flex(Flex::Start)` line and run it in a tall window to see why it's there — the cards
drift apart to fill the column. `Flex` only does anything when slack exists, so it's a no-op in a
layout of pure `Fill`s. It's `Length`-based layouts where it matters.

> If you're cross-referencing older tutorials: `SpaceAround` and `SpaceEvenly` swapped meanings in
> 0.30 to match CSS flexbox. What used to be `SpaceAround` is now `SpaceEvenly`.

**Responsiveness needs no special API** — `compute` just branches on `area.width`. Returning
`Option<Rect>` rather than a zero-width rect means the absence is explicit and the compiler forces
every caller to handle it. Under 100 columns the details pane doesn't exist at all, so there's no
degenerate rectangle for downstream code to defend against.

---

# Step 5 — The details pane and the help overlay

Add the centring helper to the bottom of `layout.rs`, leaving everything else in that file as it is:

### `src/ui/layout.rs` — append

```rust
use ratatui::layout::Flex;

/// Centre a rect of the given size inside `area`.
pub fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal]).flex(Flex::Center).areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}
```

(Tidy the two `use` lines into one if you like: `use ratatui::layout::{Constraint, Flex, Layout,
Rect};`.)

### `src/ui/details.rs`

```rust
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(Line::from(" Details ").centered());

    let body = Paragraph::new(vec![
        Line::from(format!("{} cards total", app.board.total())),
        Line::from(""),
        Line::from("Nothing selected.".dim()),
    ])
    .block(block);

    frame.render_widget(body, area);
}
```

### `src/ui/help.rs`

```rust
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};
use ratatui::Frame;

use super::layout::center;

const HELP_WIDTH: u16 = 48;
const HELP_HEIGHT: u16 = 8;

pub fn draw(frame: &mut Frame, area: Rect) {
    let area = center(
        area,
        Constraint::Length(HELP_WIDTH),
        Constraint::Length(HELP_HEIGHT),
    );

    let help = Paragraph::new(vec![
        Line::from(""),
        Line::from("  q / Esc    quit"),
        Line::from("  ?          toggle this help"),
        Line::from(""),
        Line::from("  Resize the terminal to watch it react.".dim()),
    ])
    .block(
        Block::bordered()
            .border_type(BorderType::Double)
            .title(Line::from(" Help ").centered()),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(help, area);
}
```

### `src/ui/mod.rs`

```rust
mod board;
mod details;
mod footer;
mod header;
mod help;
mod layout;

use ratatui::Frame;

use crate::app::App;
use crate::ui::layout::AppLayout;

pub fn draw(app: &App, frame: &mut Frame) {
    let area = frame.area();
    let layout = AppLayout::compute(area);

    header::draw(app, frame, layout.header);
    board::draw(app, frame, layout.columns);

    if let Some(details) = layout.details {
        details::draw(app, frame, details);
    }

    footer::draw(frame, layout.footer);

    if app.show_help {
        help::draw(frame, area);
    }
}
```

`cargo run`, then press `?`.

## What just happened

**Layout regions never overlap by design, so an overlay isn't a layout region.** It's a rectangle
computed independently and drawn over the top, after everything else. That's why `help::draw` gets
`frame.area()` — the whole terminal — rather than a slot from `AppLayout`.

**A single-constraint layout with `Flex::Center` centres that one region.** Apply it on each axis and
you get a centred rect of any size, which is all `center` does. Ratatui 0.30 also ships this as
`Rect::centered`, plus `centered_horizontally` and `centered_vertically`, so
`area.centered(Constraint::Length(48), Constraint::Length(8))` is equivalent. Writing it out once is
worth it because it makes the mechanism obvious, and because you'll want to modify it — clamping the
width, say.

**`frame.render_widget(Clear, area)` is not optional.** Ratatui renders into a persistent buffer, so
any cell your popup doesn't explicitly paint still holds whatever the board drew there. Without
`Clear`, column borders bleed straight through the modal. Comment that line out once to see it, then
put it back and never forget it again.

**The two-line dance in `mod.rs`** — `let area = frame.area();` before the calls — is a borrow-checker
courtesy. Writing `help::draw(frame, frame.area())` mutably borrows `frame` for the first argument
while immutably borrowing it for the second, and won't compile.

---

# Where you've landed

```
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
        ├── header.rs
        ├── board.rs
        ├── details.rs
        ├── footer.rs
        └── help.rs
```

Run it and confirm all of these:

- `?` toggles the modal cleanly over the board.
- Narrower than 100 columns, the details pane vanishes and the board reclaims the space.
- A very short window clips the cards instead of panicking.
- The footer's right-hand text stays pinned to the right edge at every width.

Structurally, the thing to notice is that `ui/layout.rs` imports nothing from `ui/` and draws
nothing, while every other `ui/` file draws and computes no geometry beyond its own interior. Module
04 tests `AppLayout::compute` directly as a pure `Rect -> Rects` function, no terminal involved.

## You should now be able to explain

- Why layout is a constraint solver rather than an allocator, and what that means when constraints
  conflict or don't fit.
- When to reach for `Length`, `Fill`, `Min` and `Max`, and why `Fill(1)` is usually the right "rest".
- What `Block::inner` is for and what breaks without it.
- Where slack goes and how `Flex` redirects it.
- Why `.areas()` beats `.split()` whenever the count is known.
- Why an overlay is not a layout region and needs `Clear`.

---

## Next: 02 — Rendering UIs

One level down. The `Buffer` behind `Frame`, what a `Cell` actually holds, the `Text`/`Line`/`Span`
hierarchy and how styles cascade through it, wrapping and alignment, and how the double-buffer diff
turns your full redraw into a handful of escape sequences. The cards stop being placeholder text and
start rendering styled, wrapped, truncated content — and `ui/theme.rs` joins the tree.
