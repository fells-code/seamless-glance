use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{stdout, Result};

pub fn suspend_tui() -> Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, Show)?;
    Ok(())
}

pub fn resume_tui() -> Result<()> {
    execute!(stdout(), EnterAlternateScreen, Hide)?;
    enable_raw_mode()?;
    Ok(())
}
