mod app;
mod event;
mod model;
mod ui;

use std::io;

use crate::app::App;

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::new().run(terminal))
}