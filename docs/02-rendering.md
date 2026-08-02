# Ratatui from the Ground Up — 02: Rendering UIs

> | # | Module | What you add |
> |---|--------|--------------|
> | 01 | Layouts | Project structure and the region system |
> | **02** | **Rendering UIs** | **The buffer, cells, `Text`/`Line`/`Span`, styling, the diff** |
> | 03 | Widgets | Lists, tables, gauges, stateful widgets, custom widgets |
> | 04 | Testing | `TestBackend`, buffer assertions, snapshot tests |
> | 05 | Applications | Event architecture, errors, config, logging, shipping |

Module 01 left you with regions and nothing much in them. This module fills them. Same rules as
before: **every code block is a complete file at a stated path**, and when a file changes it's shown
in full so you can replace it outright.

Where you're starting from:

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

Where you'll finish: the same tree plus `ui/theme.rs` and `ui/progress.rs`, with cards that carry
tags and notes, a header with a live completion bar drawn cell by cell, and a details pane that wraps
prose properly.

Five steps. No new dependencies.

---

# Step 1 — One place for colour

Colour scattered across six files is unmaintainable, and the first change you'll want to make is a
global one. So the palette gets a home before anything uses it.

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

/// De-emphasised text: counts, hints, placeholders.
pub const MUTED: Style = Style::new().fg(Color::DarkGray);

/// The app name and other primary emphasis.
pub const TITLE: Style = Style::new()
    .fg(Color::Cyan)
    .add_modifier(Modifier::BOLD);

/// A card's tag chip.
pub const TAG: Style = Style::new().fg(Color::Magenta);

/// The help popup's surface and frame.
pub const POPUP: Style = Style::new().bg(Color::Black);
pub const POPUP_BORDER: Style = Style::new().fg(Color::Cyan);

/// The accent colour belonging to the column at `index`.
pub fn column(index: usize) -> Color {
    COLUMN_ACCENTS[index]
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
mod theme;

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

### `src/ui/header.rs`

```rust
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme;

pub fn draw(app: &App, frame: &mut Frame, area: Rect) {
    let board = &app.board;
    let percent = if board.total() == 0 {
        0
    } else {
        board.done() * 100 / board.total()
    };

    let title = Line::from(vec![
        Span::styled("taskboard", theme::TITLE),
        Span::raw("   "),
        Span::styled(format!("{} open", board.open()), theme::MUTED),
        Span::styled("  ·  ", theme::MUTED),
        Span::styled(format!("{} done", board.done()), theme::MUTED),
        Span::raw("   "),
        Span::styled(format!("{percent}%"), theme::TITLE),
    ]);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::BORDER);

    frame.render_widget(Paragraph::new(title).block(block), area);
}
```

### `src/ui/footer.rs`

```rust
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::ui::theme;

pub fn draw(frame: &mut Frame, area: Rect) {
    let size = frame.area();
    let dimensions = Line::styled(format!("{}×{} ", size.width, size.height), theme::MUTED);
    let width = dimensions.width() as u16;

    let [left, right] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(width)]).areas(area);

    let keys = Line::from(vec![
        Span::styled(" q", theme::TITLE),
        Span::styled(" quit   ", theme::MUTED),
        Span::styled("?", theme::TITLE),
        Span::styled(" help", theme::MUTED),
    ]);

    frame.render_widget(keys, left);
    frame.render_widget(dimensions.right_aligned(), right);
}
```

### `src/ui/help.rs`

```rust
use ratatui::layout::{Constraint, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Clear, Padding, Paragraph};
use ratatui::Frame;

use super::layout::center;
use super::theme;

const HELP_WIDTH: u16 = 48;
const HELP_HEIGHT: u16 = 8;

pub fn draw(frame: &mut Frame, area: Rect) {
    let area = center(
        area,
        Constraint::Length(HELP_WIDTH),
        Constraint::Length(HELP_HEIGHT),
    );

    let block = Block::bordered()
        .border_type(BorderType::Double)
        .border_style(theme::POPUP_BORDER)
        .style(theme::POPUP)
        .padding(Padding::horizontal(2))
        .title(Line::styled(" Help ", theme::TITLE).centered());

    let help = Paragraph::new(vec![
        Line::raw(""),
        Line::raw("q / Esc    quit"),
        Line::raw("?          toggle this help"),
        Line::raw(""),
        Line::styled("Resize the terminal to watch it react.", theme::MUTED),
    ])
    .block(block);

    frame.render_widget(Clear, area);
    frame.render_widget(help, area);
}
```

`cargo run`. Same layout, but the chrome recedes and the numbers stand out.

## What just happened

**`Style` is four optional fields, not a colour.** Roughly: `fg`, `bg`, `add_modifier`,
`sub_modifier` (plus `underline_color`). Every field is optional, and that optionality is the entire
mechanism behind style inheritance — an unset field means "whatever the thing underneath me said".

**Styles combine by patching, not replacing.** When a `Span` inside a `Line` inside a `Paragraph`
gets rendered, Ratatui starts with the widget's style and calls `Style::patch` down the chain. Each
patch overwrites only the fields the child actually set. So a `Line` styled `MUTED` (which sets `fg`
and nothing else) sitting in a block with a black background keeps the black background. Set `fg` on
the span and the span wins for `fg` alone.

This is why the `theme` consts are deliberately narrow. `MUTED` sets a foreground and stops. If it
also set a background, it would punch a rectangle of that background into every context it's used in.

**`border_style` vs `style` — the one that will bite you.** `Block::style` applies to the block's
*whole area*, interior included, and it is applied before the block's content is drawn. So this:

```rust
Block::bordered().green()          // Stylize shorthand → goes through Block::style
```

turns your border green *and* tints everything you subsequently render inside that block, because
those child widgets mostly leave `fg` unset and inherit. That's why `board.rs` uses `border_style`
for the frame, while `help.rs` deliberately uses `style` — a popup genuinely does want its
background to cover the whole rectangle. Choose consciously; the shorthand hides which one you got.

**Named colours vs RGB.** `Color::Cyan` resolves through the user's terminal palette, so it fits
whatever theme they've chosen. `Color::Rgb(0, 255, 255)` is exactly that colour on every terminal
that supports truecolour, and can be unreadable on a light background. `Color::Indexed(n)` picks from
the 256-colour cube. For an app someone else will run, named colours are the polite default —
`LightBlue` and friends usually read better than the base variants against dark backgrounds.

**`Line::width()` is not `str::len()`.** The footer measures its own text before choosing a
constraint. Note the string contains `×` — one column wide, but two bytes in UTF-8. `len()` would
have over-reserved by one and left a gap. `Line::width()` and `Text::width()` measure *display
columns*, handling multi-byte characters and double-width CJK correctly. Any time you size a layout
from text, measure it this way.

---

# Step 2 — Cards with content

Cards are currently a bare title. Give them something to render.

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

### `src/ui/board.rs`

```rust
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Padding, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::model::{Card, Column};
use crate::ui::theme;

/// Two rows of content plus a top and bottom border.
const CARD_HEIGHT: u16 = 4;

pub fn draw(app: &App, frame: &mut Frame, areas: [Rect; 3]) {
    for ((index, column), area) in app.board.columns.iter().enumerate().zip(areas) {
        draw_column(column, theme::column(index), frame, area);
    }
}

fn draw_column(column: &Column, accent: Color, frame: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        Span::styled(
            format!(" {} ", column.title),
            Style::new().fg(accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("{} ", column.cards.len()), theme::MUTED),
    ]);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::BORDER)
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if column.cards.is_empty() {
        let empty = Paragraph::new(Line::styled("nothing here", theme::MUTED)).centered();
        frame.render_widget(empty, inner);
        return;
    }

    let card_areas = Layout::vertical(vec![Constraint::Length(CARD_HEIGHT); column.cards.len()])
        .spacing(1)
        .flex(Flex::Start)
        .split(inner);

    for (card, &card_area) in column.cards.iter().zip(card_areas.iter()) {
        draw_card(card, accent, frame, card_area);
    }
}

fn draw_card(card: &Card, accent: Color, frame: &mut Frame, area: Rect) {
    let mut lines = vec![Line::styled(card.title.as_str(), Style::new().fg(accent))];

    if let Some(tag) = &card.tag {
        lines.push(Line::styled(format!("#{tag}"), theme::TAG));
    }

    let block = Block::bordered()
        .border_style(theme::BORDER)
        .padding(Padding::horizontal(1));

    frame.render_widget(Paragraph::new(Text::from(lines)).block(block), area);
}
```

`cargo run`. Cards now have a coloured title and a tag, and narrow columns visibly cut the longer
titles off mid-word.

## What just happened

**The text hierarchy is three types deep, and each level adds exactly one thing.**

| Type | Is | Adds |
|---|---|---|
| `Span` | a run of characters with one `Style` | styling |
| `Line` | a `Vec<Span>` | horizontal alignment, one row |
| `Text` | a `Vec<Line>` | vertical extent |

A `Span` cannot contain a newline meaningfully — a newline is what makes a new `Line`. A `Line` can
mix as many styles as it has spans, which is the only way to get two colours on one row. `Text` is
what every text-accepting widget really takes; the `From` impls are why `Paragraph::new` accepts a
`&str`, a `String`, a `Span`, a `Line`, a `Vec<Line>` or a `Text` interchangeably.

**Constructors come in two flavours throughout.** `Span::raw` / `Line::raw` / `Text::raw` take
content and leave the style unset; `Span::styled` / `Line::styled` / `Text::styled` take content and
a style. Unset is not the same as default — unset inherits, as covered in step 1.

**`Stylize` is the shorthand layer over all of it.** `"hello".bold()` produces a `Span`, because
`Stylize` is implemented for `&str`, `String`, `Span`, `Line`, `Text` and most widgets. It's
excellent for one-offs and a liability in a themed app, because it scatters style decisions into
render code. This module uses explicit `theme::` consts precisely so there's one place to change
them. Use the shorthand for genuine one-offs; reach for the theme for anything that recurs.

**Lifetimes will come up.** `Line::styled(card.title.as_str(), …)` borrows from the model, which is
fine because the widget is consumed by `render_widget` in the same expression. If you want a function
that *returns* text outliving the borrow, either clone into an owned `String` or return
`Text<'static>`. `Span` holds a `Cow<'a, str>`, so both work; the `'static` version just costs an
allocation. Do not fight this by cloning the whole model — clone the string you need.

**The empty-column branch returns early.** The card loop would already do nothing with an empty
`Vec`, but rendering a placeholder needs its own path, and an early return keeps the nesting flat.

---

# Step 3 — Wrapping, truncation and the details pane

Cards truncate. Prose shouldn't. The details pane is where a card gets room to breathe.

### `src/ui/details.rs`

```rust
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Padding, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::model::Card;
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

    frame.render_widget(Paragraph::new(summary(app)), summary_area);

    match app.board.columns.iter().find_map(|column| column.cards.first()) {
        Some(card) => frame.render_widget(focus(card).wrap(Wrap { trim: true }), focus_area),
        None => frame.render_widget(
            Paragraph::new(Line::styled("Nothing to show.", theme::MUTED)),
            focus_area,
        ),
    }
}

fn summary(app: &App) -> Text<'static> {
    let mut lines: Vec<Line> = app
        .board
        .columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            Line::from(vec![
                Span::styled("■ ", Style::new().fg(theme::column(index))),
                Span::raw(column.title.clone()),
                Span::styled(format!("   {}", column.cards.len()), theme::MUTED),
            ])
        })
        .collect();

    lines.push(Line::raw(""));
    lines.push(Line::styled(
        format!("{} cards total", app.board.total()),
        theme::MUTED,
    ));

    Text::from(lines)
}

fn focus(card: &Card) -> Paragraph<'_> {
    let mut lines = vec![Line::styled(
        card.title.as_str(),
        Style::new().add_modifier(Modifier::BOLD),
    )];

    if let Some(tag) = &card.tag {
        lines.push(Line::styled(format!("#{tag}"), theme::TAG));
    }

    if !card.notes.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled(card.notes.as_str(), theme::MUTED));
    }

    Paragraph::new(Text::from(lines))
}
```

`cargo run` in a window wider than 100 columns. The notes on the first Todo card wrap across several
rows inside a 32-column pane.

## What just happened

**Without `.wrap()`, a `Paragraph` truncates.** That is the default and it is usually the right one
for a single-row label — a title that silently wraps into a second row would break a fixed-height
card layout. Truncation happens at the region's right edge, mid-word, with no ellipsis.

**`Wrap { trim }` controls leading whitespace on continuation rows.** With `trim: true` the wrapped
remainder is left-trimmed, which is what you want for prose. With `trim: false` the original leading
whitespace of each source line is preserved, which is what you want for anything indentation-bearing
— ASCII art, code, pre-formatted output. Wrapping is word-based, and a single word longer than the
region is broken mid-word rather than overflowing.

**Wrapping happens at render time, so you cannot cheaply ask how tall the result will be.**
`Text::height()` counts the `Line`s you built, not the rows they will occupy after wrapping.
`Text::width()` gives the widest unwrapped line. That asymmetry is why `CARD_HEIGHT` in `board.rs` is
a constant and the notes live here instead: a fixed-height card cannot host text of unknown height.
(Ratatui does expose `Paragraph::line_count` for exactly this, but it sits behind the
`unstable-rendered-line-info` feature flag, so it isn't used here.)

**Alignment exists at three levels and the innermost wins.** `Paragraph::centered()`,
`Line::centered()`, and the alignment carried by a `Text`. A `Line` with its own alignment overrides
the `Paragraph`'s for that row only, which is how the details block gets a centred title above
left-aligned body text.

> 0.30 renamed `Alignment` to `HorizontalAlignment`, keeping the old name as a deprecated alias.
> Both turn up in search results; the new one is correct.

**`Paragraph::scroll((y, x))` offsets the rendered text** and is how you'd page through notes longer
than the pane. It scrolls by *rendered* rows when wrapping is on, which — since you can't easily
measure those — is a good reason to reach for the `List` widget instead. That's module 03.

---

# Step 4 — Into the buffer

Everything so far went through a widget. Now go a layer down and set cells directly, to draw a
completion bar with sub-character resolution.

### `src/ui/layout.rs`

One line changes — the header grows a row. Replace the vertical split's first constraint:

```rust
use ratatui::layout::{Constraint, Flex, Layout, Rect};

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
```

### `src/ui/progress.rs`

```rust
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::Frame;

/// Left-aligned block characters from empty to full, in eighths of a cell.
const EIGHTHS: [&str; 9] = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];

/// Draw a one-row bar filling `ratio` of `area`, written cell by cell.
pub fn draw(frame: &mut Frame, area: Rect, ratio: f64, filled: Color, track: Color) {
    if area.is_empty() {
        return;
    }

    let style = Style::new().fg(filled).bg(track);
    let total = (f64::from(area.width) * 8.0 * ratio.clamp(0.0, 1.0)).round() as u16;
    let buffer = frame.buffer_mut();

    for offset in 0..area.width {
        let eighths = total.saturating_sub(offset * 8).min(8) as usize;

        if let Some(cell) = buffer.cell_mut((area.x + offset, area.y)) {
            cell.set_symbol(EIGHTHS[eighths]).set_style(style);
        }
    }
}
```

### `src/ui/header.rs`

```rust
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::ui::{progress, theme};

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

    progress::draw(
        frame,
        bar_area,
        board.ratio(),
        theme::COLUMN_ACCENTS[2],
        ratatui::style::Color::DarkGray,
    );
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
mod progress;
mod theme;

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

`cargo run`. A bar under the header title, filled to a third. Resize the window and watch the
partial block at the end of the fill change character.

## What just happened

**`Buffer` is a flat `Vec<Cell>` plus the `Rect` it covers.** That's the whole data structure. Every
widget in Ratatui, including every one you've used, ultimately writes cells into it. `Frame` is a thin
wrapper that hands you the current one via `buffer_mut()`.

**A `Cell` holds a `symbol` — a string, not a `char`.** It has to be, because one terminal cell can
contain a multi-codepoint grapheme cluster: an emoji with a skin-tone modifier, or a letter with
combining marks. Alongside the symbol: `fg`, `bg`, an underline colour, a `Modifier` bitflag set, and
a `skip` flag that tells the renderer to leave that cell alone entirely.

**Buffer coordinates are global, not relative to your `Rect`.** This is the mistake everyone makes
once. `progress::draw` writes to `area.x + offset`, not `offset`. Writing to `(0, 0)` from inside a
widget targets the top-left of the *terminal*, not the top-left of your region — and if the buffer's
own area doesn't start at the origin, `(0, 0)` may not even be in bounds.

**Prefer `cell_mut` to indexing.** Both accept anything that converts to a `Position`, so `(x, y)`
tuples work. The difference is what happens off the edge: `buffer[(x, y)]` panics, `buffer.cell_mut((x,
y))` returns `None`. In render code — which runs on every frame, at every window size, including
sizes you never tested — panicking is a poor default. (`Buffer::get` and `get_mut` are the old API
and are deprecated.)

**Cell setters chain**, returning `&mut Cell`, which is why `set_symbol(…).set_style(…)` works in one
expression.

**Why eighth-blocks.** A terminal cell is the smallest addressable unit, so a naive bar quantises to
whole cells and jitters visibly as it grows. The Unicode block elements `▏▎▍▌▋▊▉█` give eight
sub-positions per cell, so the arithmetic works in eighths and only the final partial cell is
approximate. Ratatui's built-in `Gauge` does the same thing, and you'll swap this file out for it in
module 03 — writing it by hand once makes clear what the widget is actually doing.

**When to drop to this level:** custom drawing primitives, sub-character resolution, performance-
critical paint loops. Not for anything a `Paragraph` handles. Direct buffer writes skip every
alignment, wrapping and clipping guarantee the widget layer gives you, and you become responsible for
bounds checking. Reach for it deliberately, in a file of its own, as here.

---

# Step 5 — The diff

Every frame redescribes the entire UI. That should sound expensive. It isn't, and this step shows why.

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
```

### `src/ui/footer.rs`

```rust
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme;

pub fn draw(app: &App, frame: &mut Frame, area: Rect) {
    let size = frame.area();
    let status = Line::from(vec![
        Span::styled(format!("{} frames", app.frames), theme::MUTED),
        Span::styled(format!("   {}×{} ", size.width, size.height), theme::MUTED),
    ]);
    let width = status.width() as u16;

    let [left, right] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(width)]).areas(area);

    let keys = Line::from(vec![
        Span::styled(" q", theme::TITLE),
        Span::styled(" quit   ", theme::MUTED),
        Span::styled("?", theme::TITLE),
        Span::styled(" help", theme::MUTED),
    ]);

    frame.render_widget(keys, left);
    frame.render_widget(status.right_aligned(), right);
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
mod progress;
mod theme;

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

    footer::draw(app, frame, layout.footer);

    if app.show_help {
        help::draw(frame, area);
    }
}
```

`cargo run` and press `?` a few times. The frame counter climbs one per keystroke, because
`event::read()` blocks — the app draws, then sleeps until you touch the keyboard. Module 05 adds a
tick and this number starts moving on its own.

## What just happened

**`Terminal` holds two buffers and alternates between them.** Your closure paints the current one
from scratch. Then `Terminal::flush` calls `Buffer::diff_iter` against the previous frame and emits
escape sequences only for cells that actually changed. Toggling the help popup rewrites a few hundred
cells; the other several thousand produce no output at all.

That's the bargain immediate mode makes: you get to write rendering code as a pure function of state,
with no incremental-update bookkeeping and no stale-view bugs, and the diff makes it cheap. The cost
is that your draw code runs completely every frame, so it should stay allocation-light and free of
I/O. Reading a file inside `ui::draw` is the classic way to make a TUI feel sluggish.

**The diff is cell-by-cell and style-aware.** Two cells differ if their symbol, colours, modifiers or
skip flag differ. It also understands double-width characters: a wide glyph at index `n` means index
`n+1` is skipped rather than painted.

**When the diff is wrong, `terminal.clear()` fixes it.** The previous buffer is Ratatui's model of
what's on screen. If something else writes to the terminal — a subprocess, a stray `println!`, a
library logging to stdout — that model is stale and the diff will skip cells that genuinely need
repainting. Forcing a full repaint is the escape hatch. Keeping stdout clear of anything you didn't
render is the actual fix, which is why module 05 puts logs in a file.

**Resizing is handled for you.** `Terminal::draw` calls `autoresize` first, which resizes both
buffers and forces a full redraw when the dimensions change. That's why `AppLayout::compute` reading
`frame.area()` every frame is enough to be responsive — there's no resize event to subscribe to.

**One more thing, aimed at module 04.** `Buffer` implements `PartialEq`, has a `Buffer::with_lines`
constructor that builds one from string literals, and has a `Debug` impl that prints its contents as
readable rows plus a style summary. Those three facts together are the entire basis of Ratatui's
testing story: render into a buffer, compare it to one you wrote by hand, and get a legible diff when
they disagree. That's the next-but-one module.

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
        ├── theme.rs      ← new
        ├── progress.rs   ← new
        ├── header.rs
        ├── board.rs
        ├── details.rs
        ├── footer.rs
        └── help.rs
```

Run it and confirm:

- The completion bar tracks the Done column, with a partial block at the fill edge.
- Card titles truncate in a narrow column; the details pane wraps its notes instead.
- The help popup's background covers the board completely, with no borders bleeding through.
- The frame counter increments once per keypress, not continuously.
- Every colour in the app can be changed from `ui/theme.rs` alone.

That last point is the structural test. If you find yourself editing a colour anywhere else, the
theme boundary has leaked.

## You should now be able to explain

- What a `Style` actually is, and why unset fields matter more than set ones.
- Why `Block::style` tints your content and `Block::border_style` doesn't.
- The `Span` / `Line` / `Text` split and which level owns alignment.
- Why `Line::width()` is the right way to measure text and `str::len()` is not.
- What `Wrap { trim }` decides, and why wrapped height is hard to know in advance.
- What a `Cell` holds, why buffer coordinates are global, and when to write to them directly.
- How the double-buffer diff makes a full redraw cheap, and what invalidates it.

---

## Next: 03 — Widgets

The library's own widgets, and how to write your own. `List` and `Table` with real selection state,
`Gauge` replacing the hand-rolled bar in `progress.rs`, `Tabs`, `Scrollbar`. Then the traits
underneath: `Widget` for stateless render, `StatefulWidget` for widgets that own scroll and selection
state, and why `&T` implements `Widget` for most of them. The board gains a selected card, and
`ui/board.rs` becomes a real custom widget rather than a draw function.
