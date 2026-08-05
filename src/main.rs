use std::io;

use taskboard::app::App;

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::new().run(terminal))
}
