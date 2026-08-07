# Ratatui from the Ground Up — 05: Applications

> | # | Module | What you add |
> | --- | -------- | -------------- |
> | 01 | Layouts | Project structure and the region system |
> | 02 | Rendering UIs | The buffer, cells, `Text`/`Line`/`Span`, styling, the diff |
> | 03 | Widgets | Lists, gauges, tables, scrollbars, and writing your own |
> | 04 | Testing | `TestBackend`, buffer assertions, snapshot tests |
> | **05** | **Applications** | **Event architecture, errors, config, logging, shipping** |

`taskboard` renders well and is tested. It is still not an application: it blocks on a keypress, it
loses your board on exit, it hard-codes its colours, it has nowhere to log, and if it panics it
leaves your terminal in raw mode.

This module fixes all five. It's the least Ratatui-specific of the series — most of it is ordinary
Rust application plumbing — but the terminal imposes constraints that make some familiar choices
wrong, and those are what's worth your attention.

Same rules: **every code block is a complete file at a stated path**.

Five steps. Five new dependencies, added as they're needed.

---

## Step 1 — Errors that don't wreck your terminal

`io::Result` has carried us this far. It stops being enough the moment anything can fail for a reason
that isn't I/O — and a panic in raw mode is genuinely destructive.

```bash
cargo add color-eyre
```

`Cargo.toml`

```toml
[package]
name = "taskboard"
version = "0.1.0"
edition = "2024"

[dependencies]
color-eyre = "0.6"
ratatui = "0.30"

[dev-dependencies]
insta = "1"
```

### `src/terminal.rs`

```rust
use std::io;
use std::panic;

use color_eyre::Result;
use ratatui::DefaultTerminal;

/// Set up the terminal and install hooks that restore it no matter how we exit.
///
/// Returns a terminal that the caller is responsible for passing to [`restore`].
pub fn init() -> Result<DefaultTerminal> {
    install_hooks()?;
    Ok(ratatui::init())
}

/// Put the terminal back the way we found it.
pub fn restore() {
    ratatui::restore();
}

/// Make both panics and `color_eyre` reports restore the terminal before printing.
///
/// Without this, an error message is written into the alternate screen and vanishes
/// the moment the process exits, leaving the user with a wrecked shell and no clue why.
fn install_hooks() -> Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .into_hooks();

    let panic_hook = panic_hook.into_panic_hook();
    panic::set_hook(Box::new(move |info| {
        restore();
        panic_hook(info);
    }));

    let eyre_hook = eyre_hook.into_eyre_hook();
    color_eyre::eyre::set_hook(Box::new(move |error| {
        restore();
        eyre_hook(error)
    }))?;

    Ok(())
}

/// Whether stdout is a real terminal. Used to refuse to start when piped.
pub fn is_tty() -> bool {
    io::IsTerminal::is_terminal(&io::stdout())
}
```

### `src/app.rs`

```rust
use color_eyre::Result;
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

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
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
```

(The `#[cfg(test)] mod tests` block from module 04 stays exactly as it was — it doesn't touch `run`.)

### `src/lib.rs`

```rust
pub mod app;
pub mod event;
pub mod model;
pub mod terminal;
pub mod ui;
```

### `src/main.rs`

```rust
use color_eyre::eyre::bail;
use color_eyre::Result;

use taskboard::app::App;
use taskboard::terminal;

fn main() -> Result<()> {
    color_eyre::install()?;

    if !terminal::is_tty() {
        bail!("taskboard needs an interactive terminal (stdout is not a tty)");
    }

    let mut term = terminal::init()?;
    let result = App::new().run(&mut term);
    terminal::restore();

    result
}
```

```bash
cargo run
```

Behaviour is unchanged. Now break it on purpose: add `panic!("boom")` inside `ui::draw`, run again,
and remove it. You get a normal backtrace on a normal shell prompt rather than a scrambled terminal.

## What just happened

**A panic in raw mode is not a normal panic.** Raw mode disables line editing and echo; the alternate
screen hides the scrollback. Panic without restoring and the message prints into a screen that's
about to be discarded, and the user is left in a shell that doesn't echo what they type. They will
have to run `reset` blind. This is the single worst failure mode a TUI has, and it's entirely
avoidable.

**`ratatui::run` from modules 01–04 already installed a panic hook.** Dropping to explicit
`init`/`restore` here isn't a fix — it's about owning the seam so a *second* hook can be layered in
for `color_eyre`, which `run` doesn't know about. If you never need custom hooks, keep using `run`.

**Two hooks, because there are two ways to fail.** `panic::set_hook` covers unwinding panics.
`color_eyre::eyre::set_hook` covers `Err` values that reach `main` and get pretty-printed. Both print
to stderr, so both must restore first. Installing one and not the other is a common half-fix.

**`main` restores explicitly *and* the hooks do.** Belt and braces: the explicit call handles the
normal path, the hooks handle the abnormal ones. `ratatui::restore` is safe to call twice.

**The tty check is a real ergonomic issue, not paranoia.** Run a TUI with stdout piped — `taskboard |
head` — and without this it writes escape sequences into the pipe and blocks forever on input. A
one-line refusal with a clear message is better than a hang.

**`color_eyre::Result<T>` is `Result<T, Report>`**, and `Report` converts from any `std::error::Error`
via `?`. That's why `run` can propagate `io::Error` from `terminal.draw` without a conversion. Use
`.wrap_err("...")` to add context as an error travels up — the reports become genuinely useful.

---

## Step 2 — A non-blocking event loop

`event::read()` blocks. That was the right call while nothing moved on its own. Now the UI needs to
update without a keypress — for a clock, an autosave indicator, an animation, or just a heartbeat.

`src/event.rs`

```rust
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use color_eyre::Result;
use ratatui::crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind};

/// How often the app wakes up with nothing to do.
const TICK_RATE: Duration = Duration::from_millis(250);

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

/// Everything the run loop can be woken by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The regular heartbeat. Advance animations, refresh clocks.
    Tick,
    /// A translated key press.
    Action(Action),
    /// The terminal changed size.
    Resize,
}

/// Reads the terminal on a background thread and posts events to a channel.
#[derive(Debug)]
pub struct EventLoop {
    receiver: Receiver<Event>,
    /// Kept alive so the channel doesn't close while the app is running.
    _sender: Sender<Event>,
}

impl EventLoop {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let producer = sender.clone();

        thread::spawn(move || {
            let mut last_tick = Instant::now();

            loop {
                let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());

                // `poll` waits at most `timeout` for input, so a tick is never late by more
                // than one poll interval.
                match event::poll(timeout) {
                    Ok(true) => match event::read() {
                        Ok(CrosstermEvent::Key(key)) if key.kind == KeyEventKind::Press => {
                            if producer.send(Event::Action(key_to_action(key))).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Resize(_, _)) => {
                            if producer.send(Event::Resize).is_err() {
                                break;
                            }
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    },
                    Ok(false) => {}
                    Err(_) => break,
                }

                if last_tick.elapsed() >= TICK_RATE {
                    last_tick = Instant::now();

                    if producer.send(Event::Tick).is_err() {
                        break;
                    }
                }
            }
        });

        Self { receiver, _sender: sender }
    }

    /// Block until the next event arrives.
    pub fn next(&self) -> Result<Event> {
        Ok(self.receiver.recv()?)
    }
}

impl Default for EventLoop {
    fn default() -> Self {
        Self::new()
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

`src/app.rs` — the run loop only

Replace `run` and add `ticks`; everything else in the file is unchanged:

```rust
    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let events = EventLoop::new();

        while !self.should_quit {
            terminal.draw(|frame| ui::draw(&mut self, frame))?;

            match events.next()? {
                Event::Action(action) => self.handle(action),
                Event::Tick => self.ticks += 1,
                Event::Resize => {}
            }

            self.frames += 1;
        }

        Ok(())
    }
```

The struct gains one field and the imports change:

```rust
use crate::event::{Action, Event, EventLoop};
```

```rust
    /// Heartbeats since start. Proves the loop runs without input.
    pub ticks: u64,
```

`src/ui/footer.rs`

```rust
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme;

/// The spinner advances once per tick.
const SPINNER: [&str; 4] = ["⠋", "⠙", "⠹", "⠸"];

pub fn draw(app: &App, frame: &mut Frame, area: Rect) {
    let size = frame.area();
    let spinner = SPINNER[(app.ticks as usize) % SPINNER.len()];

    let status = Line::from(vec![
        Span::styled(spinner, theme::TITLE),
        Span::styled(format!("  {}×{} ", size.width, size.height), theme::MUTED),
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

`cargo run`. The spinner turns on its own. Press nothing.

### Step 2 - What just happened

**`event::poll(timeout)` is the primitive that makes this possible.** It returns `Ok(true)` if input
is available within the timeout, `Ok(false)` otherwise, and only then do you call `read()`, which is
guaranteed not to block. Everything else here is arithmetic to keep the tick interval honest:
`TICK_RATE.saturating_sub(last_tick.elapsed())` means input never delays a tick past its due time.

**The producer runs on its own thread and communicates by channel**, which is the pattern nearly
every non-trivial Ratatui app converges on. You could poll inline in the run loop and skip the
thread — for this app that would work — but the channel generalises: a file watcher, an HTTP client
or a database poller can all send into the same `Receiver`, and the run loop stays a single `match`.
That's the reason to build it this way now rather than later.

**Draw first, then block on the event.** If you block first, the initial frame doesn't appear until
something happens. Every immediate-mode loop has this ordering, and getting it backwards produces the
"my app starts with a blank screen" bug.

**The `_sender` field is load-bearing.** `Receiver::recv` returns `Err` when *all* senders are
dropped. The thread holds a clone; keeping the original in the struct means the channel outlives any
thread hiccup, and `recv` blocks instead of erroring. Delete the field and the app exits instantly if
the producer thread ever ends.

**`Event::Resize` is deliberately a no-op.** `Terminal::draw` calls `autoresize` internally, and
`AppLayout::compute` reads `frame.area()` every frame, so resizing already works — the variant exists
so the loop wakes immediately on resize rather than up to 250 ms later. That's the difference between
a UI that feels responsive when dragged and one that feels laggy.

**Ticks are now in the render path, which breaks a module-04 assumption.** The spinner makes output
depend on `app.ticks`, so any snapshot that renders the footer must set `ticks` explicitly in its
fixture. This is the injected-versus-ambient-state point from module 04 arriving on schedule: state
that varies must be a field you can set, never something read from the clock inside `draw`.

**Choosing a tick rate.** 250 ms is a reasonable default for a UI with a spinner. Lower it for smooth
animation, raise it — or drop ticks entirely — for a static app; each tick is a full redraw, and on
battery that matters. There's no requirement to have ticks at all.

---

## Step 3 — Configuration

`theme.rs` hard-codes the palette. Users have opinions about colours.

```bash
cargo add serde --features derive
cargo add toml
cargo add directories
```

`src/config.rs`

```rust
use std::fs;
use std::path::PathBuf;

use color_eyre::eyre::Context;
use color_eyre::Result;
use directories::ProjectDirs;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

/// User-tunable settings, loaded from `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub theme: Theme,
    /// Terminal width at which the details pane appears.
    pub details_breakpoint: u16,
    /// Width of the details pane.
    pub details_width: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Theme {
    pub todo: Color,
    pub doing: Color,
    pub done: Color,
    pub border: Color,
    pub border_focus: Color,
    pub muted: Color,
    pub title: Color,
    pub tag: Color,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            details_breakpoint: 100,
            details_width: 32,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            todo: Color::LightBlue,
            doing: Color::LightYellow,
            done: Color::LightGreen,
            border: Color::DarkGray,
            border_focus: Color::White,
            muted: Color::DarkGray,
            title: Color::Cyan,
            tag: Color::Magenta,
        }
    }
}

impl Theme {
    /// The accent colour for the column at `index`.
    pub fn column(&self, index: usize) -> Color {
        match index {
            0 => self.todo,
            1 => self.doing,
            _ => self.done,
        }
    }
}

impl Config {
    /// Load from disk, falling back to defaults when the file is absent.
    ///
    /// A malformed file is an error, not a silent fallback: quietly ignoring a typo
    /// leaves the user staring at default colours with no idea why.
    pub fn load() -> Result<Self> {
        let Some(path) = config_path() else {
            return Ok(Self::default());
        };

        if !path.exists() {
            return Ok(Self::default());
        }

        let text = fs::read_to_string(&path)
            .wrap_err_with(|| format!("reading {}", path.display()))?;

        toml::from_str(&text).wrap_err_with(|| format!("parsing {}", path.display()))
    }

    /// Write the current values out, creating the directory if needed.
    pub fn write_default() -> Result<PathBuf> {
        let path = config_path().ok_or_else(|| {
            color_eyre::eyre::eyre!("could not determine a config directory on this platform")
        })?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).wrap_err_with(|| format!("creating {}", parent.display()))?;
        }

        let text = toml::to_string_pretty(&Self::default())?;
        fs::write(&path, text).wrap_err_with(|| format!("writing {}", path.display()))?;

        Ok(path)
    }
}

/// `~/.config/taskboard/config.toml` on Linux, and the equivalent elsewhere.
pub fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "taskboard").map(|dirs| dirs.config_dir().join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip_through_toml() {
        let text = toml::to_string_pretty(&Config::default()).unwrap();
        let parsed: Config = toml::from_str(&text).unwrap();

        assert_eq!(parsed.details_breakpoint, 100);
        assert_eq!(parsed.theme.todo, Color::LightBlue);
    }

    #[test]
    fn partial_config_keeps_the_other_defaults() {
        let parsed: Config = toml::from_str("details_width = 40").unwrap();

        assert_eq!(parsed.details_width, 40);
        assert_eq!(parsed.details_breakpoint, 100);
        assert_eq!(parsed.theme.done, Color::LightGreen);
    }

    #[test]
    fn named_and_hex_colours_both_parse() {
        let parsed: Config = toml::from_str(
            r#"
            [theme]
            todo = "red"
            doing = "#ff8800"
            "#,
        )
        .unwrap();

        assert_eq!(parsed.theme.todo, Color::Red);
        assert_eq!(parsed.theme.doing, Color::Rgb(255, 136, 0));
    }

    #[test]
    fn a_typo_is_an_error() {
        let result: Result<Config, _> = toml::from_str("details_with = 40");

        assert!(result.is_err());
    }
}
```

`Cargo.toml`

```toml
[package]
name = "taskboard"
version = "0.1.0"
edition = "2024"

[dependencies]
color-eyre = "0.6"
directories = "5"
ratatui = { version = "0.30", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"

[dev-dependencies]
insta = "1"
```

`src/ui/theme.rs`

```rust
use ratatui::style::{Color, Modifier, Style};

use crate::config::Theme;

/// Styles derived from the user's configured colours, computed once at startup.
#[derive(Debug, Clone)]
pub struct Styles {
    pub columns: [Color; 3],
    pub border: Style,
    pub border_focus: Style,
    pub muted: Style,
    pub title: Style,
    pub tag: Style,
    pub selected: Style,
    pub selected_blur: Style,
    pub popup: Style,
    pub popup_border: Style,
}

impl Styles {
    pub fn from_config(theme: &Theme) -> Self {
        Self {
            columns: [theme.todo, theme.doing, theme.done],
            border: Style::new().fg(theme.border),
            border_focus: Style::new().fg(theme.border_focus),
            muted: Style::new().fg(theme.muted),
            title: Style::new().fg(theme.title).add_modifier(Modifier::BOLD),
            tag: Style::new().fg(theme.tag),
            selected: Style::new()
                .bg(theme.border)
                .add_modifier(Modifier::BOLD),
            selected_blur: Style::new().add_modifier(Modifier::DIM),
            popup: Style::new().bg(Color::Black),
            popup_border: Style::new().fg(theme.title),
        }
    }

    /// The accent colour belonging to the column at `index`.
    pub fn column(&self, index: usize) -> Color {
        self.columns[index]
    }
}

impl Default for Styles {
    fn default() -> Self {
        Self::from_config(&Theme::default())
    }
}
```

`App` gains a `pub styles: Styles` and a `pub config: Config`, built in `App::with_config(config)`;
every `theme::MUTED` in `ui/` becomes `app.styles.muted`, and `theme::column(i)` becomes
`app.styles.column(i)`. `AppLayout::compute` takes the two breakpoint values as arguments instead of
reading consts.

Then create a starter file and edit it:

```bash
cargo run -- --write-config
```

### Step 3 - What just happened

**`#[serde(default)]` on the struct is what makes a partial config work.** Every absent field falls
back to `Default`, so a user's file can contain one line. Without it, a config missing any field
fails to parse — which is the behaviour that makes people hate config files.

**`deny_unknown_fields` is the other half, and it's the one people skip.** Combined with `default`, a
typo like `details_with` would otherwise be silently ignored and fall back — leaving the user certain
they configured something and equally certain the app is broken. Erroring on unknown keys is kinder
than accepting them. `a_typo_is_an_error` pins that behaviour.

**A malformed config is a hard error here, deliberately.** The alternative — log a warning and use
defaults — hides the problem inside a UI that has nowhere to show warnings. Fail at startup, before
the alternate screen, where the message is visible.

**Ratatui's `serde` feature is what lets `Color` appear in the struct at all.** It deserialises named
variants (`"red"`, `"light-blue"`), hex strings (`"#ff8800"` → `Color::Rgb`), and bare indices
(`"12"` → `Color::Indexed`). That's a lot of expressiveness for one feature flag, and it's why the
config type can use `Color` directly rather than parsing strings by hand.

**Styles are resolved once at startup, not per frame.** `Styles::from_config` runs during
construction; `draw` only reads. Building `Style` values inside the render pass would be wasted work
on every frame — small, but it's the kind of thing that accumulates in a loop that runs four times a
second.

**`directories` handles the platform differences** — `~/.config/taskboard` on Linux,
`~/Library/Application Support` on macOS, `%APPDATA%` on Windows. Hard-coding `~/.config` is a bug on
two of the three.

---

## Step 4 — Logging

`println!` is unavailable: stdout is the UI. Anything you print lands in the middle of your board and
desynchronises Ratatui's diff.

```bash
cargo add tracing
cargo add tracing-subscriber --features env-filter
cargo add tracing-appender
```

`src/logging.rs`

```rust
use std::fs;
use std::path::PathBuf;

use color_eyre::eyre::Context;
use color_eyre::Result;
use directories::ProjectDirs;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

/// Start writing logs to a file.
///
/// Returns a guard that must be held for the lifetime of the program; dropping it
/// flushes the buffered writer. Bind it to a named variable, never to `_`.
pub fn init() -> Result<WorkerGuard> {
    let path = log_path().ok_or_else(|| {
        color_eyre::eyre::eyre!("could not determine a data directory on this platform")
    })?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).wrap_err_with(|| format!("creating {}", parent.display()))?;
    }

    let file = fs::File::create(&path).wrap_err_with(|| format!("creating {}", path.display()))?;
    let (writer, guard) = tracing_appender::non_blocking(file);

    tracing_subscriber::fmt()
        .with_writer(writer)
        // Never colour a log file, and never write to stdout.
        .with_ansi(false)
        .with_target(false)
        .with_env_filter(
            EnvFilter::try_from_env("TASKBOARD_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!(path = %path.display(), "logging started");

    Ok(guard)
}

pub fn log_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "taskboard").map(|dirs| dirs.data_local_dir().join("taskboard.log"))
}
```

`src/app.rs` — instrumenting the interesting transitions

Add `use tracing::{debug, info};` and log where a bug would want a trail:

```rust
    fn move_selected(&mut self, delta: isize) {
        let Some(index) = self.lists[self.focus].selected() else {
            debug!("move requested with no selection");
            return;
        };

        let count = self.board.columns.len() as isize;
        let target = (self.focus as isize + delta).rem_euclid(count) as usize;

        if let Some(landed) = self.board.move_card(self.focus, index, target) {
            info!(from = self.focus, to = target, index, landed, "moved card");
            self.lists[target].select(Some(landed));
            self.clamp_selection(self.focus);
        }
    }
```

`src/main.rs`

```rust
use color_eyre::eyre::bail;
use color_eyre::Result;

use taskboard::app::App;
use taskboard::config::Config;
use taskboard::{logging, terminal};

fn main() -> Result<()> {
    color_eyre::install()?;

    // Held until the end of main so buffered log lines are flushed on exit.
    let _guard = logging::init()?;

    let config = Config::load()?;

    if !terminal::is_tty() {
        bail!("taskboard needs an interactive terminal (stdout is not a tty)");
    }

    let mut term = terminal::init()?;
    let result = App::with_config(config).run(&mut term);
    terminal::restore();

    tracing::info!("shutting down");

    result
}
```

Run it, move some cards, quit, then:

```bash
tail -f ~/.local/share/taskboard/taskboard.log
TASKBOARD_LOG=debug cargo run
```

### Step 4 - What just happened

**Logging to a file is not a preference, it's a requirement.** Ratatui's diff assumes it is the only
thing writing to the terminal. A stray `println!` shifts the cursor, and every subsequent frame
paints in the wrong place until something forces a full redraw. This is the cause of "my TUI is
corrupted after an error" — a library logging to stdout, usually.

**`tracing_appender::non_blocking` moves file writes to a background thread**, so a slow disk can't
stall the render loop. The `WorkerGuard` is the catch: dropping it flushes and shuts down the worker,
so it must live as long as the program. `let _guard = ...` keeps it; `let _ = ...` drops it
immediately and you silently lose most of your log. This is the most common `tracing-appender`
mistake by a distance.

**`with_ansi(false)` because a log file is not a terminal.** The default writes colour escapes, which
turn `grep` output into noise.

**`EnvFilter` gives you a runtime verbosity dial** with no rebuild: `TASKBOARD_LOG=debug`, or
`TASKBOARD_LOG=taskboard::app=trace,info` to raise one module. Ship at `info` and let users turn it
up when they file a bug.

**Structured fields beat interpolation.** `info!(from = self.focus, to = target, "moved card")`
records named values, so the output is greppable and machine-parseable, and the message stays
constant across events. `info!("moved card from {} to {}", ...)` throws that away.

**Log state transitions, not frames.** There's no logging in `ui/`, and there shouldn't be — the
render pass runs four times a second and would drown everything useful. Log the things that change
state and the things that fail.

---

## Step 5 — Shipping

Argument parsing, persistence, and the build.

```bash
cargo add clap --features derive
cargo add serde_json
```

`src/storage.rs`

```rust
use std::fs;
use std::path::PathBuf;

use color_eyre::eyre::Context;
use color_eyre::Result;
use directories::ProjectDirs;

use crate::model::Board;

/// Read the saved board, or a fresh default one if nothing is saved yet.
pub fn load(path: Option<&PathBuf>) -> Result<Board> {
    let Some(path) = resolve(path) else {
        return Ok(Board::default());
    };

    if !path.exists() {
        return Ok(Board::default());
    }

    let text =
        fs::read_to_string(&path).wrap_err_with(|| format!("reading {}", path.display()))?;

    serde_json::from_str(&text).wrap_err_with(|| format!("parsing {}", path.display()))
}

/// Write the board out, atomically enough that a crash mid-write can't truncate it.
pub fn save(board: &Board, path: Option<&PathBuf>) -> Result<()> {
    let Some(path) = resolve(path) else {
        return Ok(());
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).wrap_err_with(|| format!("creating {}", parent.display()))?;
    }

    let text = serde_json::to_string_pretty(board)?;
    let temp = path.with_extension("json.tmp");

    fs::write(&temp, text).wrap_err_with(|| format!("writing {}", temp.display()))?;
    fs::rename(&temp, &path).wrap_err_with(|| format!("replacing {}", path.display()))?;

    Ok(())
}

fn resolve(override_path: Option<&PathBuf>) -> Option<PathBuf> {
    match override_path {
        Some(path) => Some(path.clone()),
        None => ProjectDirs::from("", "", "taskboard")
            .map(|dirs| dirs.data_local_dir().join("board.json")),
    }
}
```

`src/model.rs` — derives only

Add the serde derives to the three types; the rest of the file is unchanged:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub title: String,
    pub tag: Option<String>,
    #[serde(default)]
    pub notes: String,
}
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub title: String,
    pub cards: Vec<Card>,
}
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub columns: [Column; 3],
}
```

`src/cli.rs`

```rust
use std::path::PathBuf;

use clap::Parser;

/// A terminal kanban board.
#[derive(Debug, Parser)]
#[command(name = "taskboard", version, about)]
pub struct Cli {
    /// Board file to open, instead of the default location.
    #[arg(value_name = "FILE")]
    pub board: Option<PathBuf>,

    /// Write a default config file and exit.
    #[arg(long)]
    pub write_config: bool,

    /// Print the paths taskboard reads and writes, then exit.
    #[arg(long)]
    pub paths: bool,
}
```

`src/main.rs`

```rust
use clap::Parser;
use color_eyre::eyre::bail;
use color_eyre::Result;

use taskboard::app::App;
use taskboard::cli::Cli;
use taskboard::config::{self, Config};
use taskboard::{logging, storage, terminal};

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    if cli.write_config {
        let path = Config::write_default()?;
        println!("wrote {}", path.display());
        return Ok(());
    }

    if cli.paths {
        print_path("config", config::config_path());
        print_path("log", logging::log_path());
        print_path("board", storage::default_board_path());
        return Ok(());
    }

    // Held until the end of main so buffered log lines are flushed on exit.
    let _guard = logging::init()?;

    let config = Config::load()?;
    let board = storage::load(cli.board.as_ref())?;

    if !terminal::is_tty() {
        bail!("taskboard needs an interactive terminal (stdout is not a tty)");
    }

    let mut term = terminal::init()?;
    let app = App::with_config(config).with_board(board);
    let result = app.run(&mut term);
    terminal::restore();

    match result {
        Ok(app) => {
            storage::save(app.board(), cli.board.as_ref())?;
            tracing::info!("saved and exited cleanly");
            Ok(())
        }
        Err(error) => {
            tracing::error!(%error, "exiting with an error");
            Err(error)
        }
    }
}

fn print_path(label: &str, path: Option<std::path::PathBuf>) {
    match path {
        Some(path) => println!("{label:>7}: {}", path.display()),
        None => println!("{label:>7}: <unavailable>"),
    }
}
```

`App::run` now returns `Result<Self>` rather than `Result<()>`, so `main` can save the board it hands
back, and `storage` gains a `pub fn default_board_path()` wrapping `resolve(None)`.

`Cargo.toml`

```toml
[package]
name = "taskboard"
version = "0.1.0"
edition = "2024"
description = "A terminal kanban board"
license = "MIT"
readme = "README.md"
repository = "https://github.com/you/taskboard"
keywords = ["tui", "kanban", "ratatui", "terminal"]
categories = ["command-line-utilities"]

[dependencies]
clap = { version = "4", features = ["derive"] }
color-eyre = "0.6"
directories = "5"
ratatui = { version = "0.30", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
tracing = "0.1"
tracing-appender = "0.2"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
insta = "1"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

```bash
cargo build --release
./target/release/taskboard --help
./target/release/taskboard --paths
```

### Step 5 - What just happened

**Save on exit, and give `run` an owner to hand the board back to.** Returning `Result<Self>` from
`run` keeps persistence out of the run loop entirely — `main` owns the lifecycle, `App` owns the
state. The alternative, saving inside `handle`, means a disk write on every keystroke and an error
path with nowhere to report to.

**The temp-file-then-rename dance in `save` is not ceremony.** `fs::write` truncates before writing,
so a crash or a full disk mid-write leaves a zero-length board file and the user's data is gone.
`rename` within the same directory is atomic on every platform that matters: either the old file or
the new one exists, never a half-written one. This is the standard pattern for any file a user would
be upset to lose.

**Three subcommand-free flags, and two of them exit before the TUI starts.** `--paths` in particular
pays for itself the first time someone asks where the config lives. Anything that prints to stdout
must happen before `terminal::init`.

**`serde(default)` on `Card::notes`** means a board file written by an older version, before notes
existed, still loads. Schema evolution costs one attribute if you plan for it and a migration if you
don't.

**The release profile matters more than usual for a TUI**, because startup latency is felt directly:
you type the command and wait for the screen. `lto = true` and `codegen-units = 1` trade build time
for a meaningfully faster, smaller binary; `strip = true` drops debug symbols. Expect a `--release`
build to take several times longer than a debug one.

**What's deliberately absent:** async. `taskboard` does no network I/O and no long-running work, so
threads and a channel are the whole story. Reach for `tokio` when you have genuinely concurrent I/O —
and know that it changes the shape of everything, because the event loop becomes a `select!` over
streams. Adding it before you need it buys a large dependency tree and a harder debugging story for
no benefit.

---

## Where you've landed

```bash
taskboard/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── cli.rs        ← new
│   ├── config.rs     ← new
│   ├── logging.rs    ← new
│   ├── storage.rs    ← new
│   ├── terminal.rs   ← new
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
    ├── render.rs
    ├── exact.rs
    ├── snapshots.rs
    └── snapshots/
```

The shape worth noticing: `src/` splits cleanly into **the domain** (`model`), **the state machine**
(`app`, `event`), **the presentation** (`ui/`), and **the plumbing** (`cli`, `config`, `logging`,
`storage`, `terminal`). Only `ui/` knows Ratatui exists. Only `app` knows both the model and the UI.
Nothing in `model` knows about any of it.

Run the checklist:

- `cargo test` passes. If the footer snapshots fail, it's the spinner — pin `ticks` in the fixture.
- `cargo clippy -- -D warnings` is clean.
- `taskboard --paths` prints three plausible paths for your platform.
- Editing the config changes the colours; introducing a typo produces a readable error before the UI
  appears.
- Moving a card, quitting, and restarting shows the card where you left it.
- `panic!` inserted anywhere still leaves you at a working shell prompt.
- `taskboard | head` refuses cleanly instead of hanging.

### You should now be able to explain

- Why a TUI needs two hooks, and what a panic in raw mode does to a terminal.
- How `event::poll` turns a blocking read into a tick loop, and why draw comes before block.
- Why the event producer gets a thread and a channel even when polling inline would work.
- Why `serde(default)` and `deny_unknown_fields` belong together.
- Why logging must go to a file, and what the `WorkerGuard` is for.
- Why saving a file means writing a temp file and renaming it.

---

## Where to go next

You've built the whole stack: regions, cells, widgets, tests, and the plumbing around them. The
things most worth adding to `taskboard` from here, roughly in order of how much they'll teach you:

**Mouse support.** `crossterm::event::EnableMouseCapture`, then `Event::Mouse` in the event loop and
`Rect::contains` to hit-test against your `AppLayout`. It's the payoff for having layout as a
separate, inspectable value — you already know where everything is.

**Text input.** A card editor means a cursor, and `Frame::set_cursor_position` is how you show one.
Handling insertion, deletion and unicode correctly is genuinely fiddly; `tui-textarea` exists and is
good.

**Undo.** Keep a `Vec<Board>` of snapshots, or record `Action`s and implement inverses. The second is
harder and much more instructive about what your state machine really is.

**A widget worth publishing.** If you build something reusable, `ratatui-core` is the crate to depend
on rather than `ratatui` — that's what the 0.30 workspace split was for, and it keeps your users from
pulling in backends they don't need.

The community's best reference is the Ratatui repository's own `examples/` directory — `cargo run
--example <name>` in a checkout — and the widget showcase on ratatui.rs. Both are maintained against
the current release, which is more than can be said for most tutorials, this one included the moment
0.31 lands.
