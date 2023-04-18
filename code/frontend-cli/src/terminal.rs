use std::{io, io::Stdout};

use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tracing::info;
use tui::backend::CrosstermBackend;

use crate::CANCEL_TOKEN;

pub type Terminal = tui::Terminal<CrosstermBackend<Stdout>>;

fn setup_blocking() -> anyhow::Result<Terminal> {
    // setup terminal
    info!("Setting up terminal");
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    execute!(
        terminal.backend_mut(),
        cursor::Show,
        cursor::SetCursorStyle::BlinkingBar
    )?;

    Ok(terminal)
}

fn stop_blocking(mut terminal: Terminal) -> anyhow::Result<()> {
    execute!(
        terminal.backend_mut(),
        cursor::SetCursorStyle::SteadyBlock,
        crossterm::cursor::Hide
    )?;

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;

    terminal.show_cursor()?;

    // restore terminal
    disable_raw_mode()?;

    info!("Exiting");

    CANCEL_TOKEN.cancel();

    Ok(())
}

pub async fn setup() -> anyhow::Result<Terminal> {
    tokio::task::spawn_blocking(setup_blocking).await?
}

pub async fn stop(terminal: Terminal) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(move || stop_blocking(terminal)).await?
}
