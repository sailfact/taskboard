use std::io;
use std::panic;

use color_eyre::Result;
use color_eyre::eyre::Ok;
use ratatui::DefaultTerminal;

pub fn init() -> Result<DefaultTerminal> {
    install_hooks()?;
    Ok(ratatui::init())
}

pub fn restore() {
    ratatui::restore();
}

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
pub fn is_tty() -> bool {
    io::IsTerminal::is_terminal(&io::stdout())
}
