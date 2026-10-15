use crossterm::{
    cursor::{Hide, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{stdout, Result};

pub fn suspend_tui() -> Result<()> {
    disable_raw_mode()?;
    // Released while a command runs so the shell does not receive mouse
    // escape sequences.
    execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen, Show)?;
    Ok(())
}

pub fn resume_tui() -> Result<()> {
    execute!(stdout(), EnterAlternateScreen, Hide, EnableMouseCapture)?;
    enable_raw_mode()?;
    Ok(())
}
