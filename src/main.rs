mod api;
mod app;
mod config;
mod error;
mod models;
mod ui;

use std::io;
use std::sync::Arc;
use std::sync::mpsc;

use anyhow::{Context, Result};
use crossterm::ExecutableCommand;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::api::OpenRouterClient;
use crate::app::App;
use crate::config::load_config;
use crate::error::AppError;

fn main() -> Result<()> {
    let config = load_config()?;
    let api_key = config.api_key.ok_or(AppError::MissingApiKey)?;
    let client = Arc::new(OpenRouterClient::new(api_key)?);

    let (event_tx, event_rx) = mpsc::channel();
    let mut app = App::new(client, event_tx, event_rx);

    let mut terminal = setup_terminal()?;
    let result = app.run(&mut terminal);
    restore_terminal(&mut terminal)?;

    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode().context("enable raw mode")?;
    let mut stdout = io::stdout();
    stdout
        .execute(EnterAlternateScreen)
        .context("enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).context("create terminal")?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode().context("disable raw mode")?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor().context("show cursor")?;
    Ok(())
}
