use color_eyre::Result;
use color_eyre::eyre::bail;

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
