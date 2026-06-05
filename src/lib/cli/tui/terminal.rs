use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag to track terminal state
pub static TERMINAL_NEEDS_RESTORE: AtomicBool = AtomicBool::new(false);

/// Restore terminal state (Unix)
#[cfg(unix)]
pub fn restore_terminal_state() {
    if TERMINAL_NEEDS_RESTORE.load(Ordering::SeqCst) {
        let _ = execute!(
            io::stdout(),
            crossterm::cursor::Show,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
        let _ = io::stdout().flush();
        let _ = io::stderr().flush();
        TERMINAL_NEEDS_RESTORE.store(false, Ordering::SeqCst);
    }
}

/// Restore terminal state (non-Unix)
#[cfg(not(unix))]
pub fn restore_terminal_state() {
    if TERMINAL_NEEDS_RESTORE.load(Ordering::SeqCst) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
        let _ = io::stdout().flush();
        TERMINAL_NEEDS_RESTORE.store(false, Ordering::SeqCst);
    }
}

/// Signal handlers for Unix systems
#[cfg(unix)]
pub extern "C" fn handle_suspend(_sig: i32) {
    restore_terminal_state();
}

#[cfg(unix)]
pub extern "C" fn handle_continue(_sig: i32) {
    let _ = enable_raw_mode();
    let _ = execute!(io::stdout(), EnterAlternateScreen, crossterm::cursor::Hide);
    TERMINAL_NEEDS_RESTORE.store(true, Ordering::SeqCst);
}

#[cfg(unix)]
pub extern "C" fn handle_resize(_sig: i32) {
    // Terminal size changed - force a redraw
    // The main loop will handle the actual resize
}

/// Detect if terminal state has been corrupted
pub fn check_terminal_corruption() -> bool {
    crossterm::terminal::size().is_err()
}

/// Force terminal refresh/restore
pub fn force_terminal_refresh(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if check_terminal_corruption() {
        log::warn!("Terminal corruption detected, attempting refresh");
        restore_terminal_state();
        std::thread::sleep(std::time::Duration::from_millis(50));

        let _ = enable_raw_mode();
        let _ = execute!(io::stdout(), EnterAlternateScreen, crossterm::cursor::Hide);
        TERMINAL_NEEDS_RESTORE.store(true, Ordering::SeqCst);
        terminal.clear()?;
    }
    Ok(())
}

/// RAII guard that restores terminal state when dropped
pub struct TerminalRestoreGuard;

impl Drop for TerminalRestoreGuard {
    fn drop(&mut self) {
        restore_terminal_state();
    }
}

/// Take over the terminal for an interactive SSH session
/// Returns true if TUI needs to be restarted
#[cfg(unix)]
pub fn tui_takeover_ssh_session(ssh_command: &str) -> bool {
    restore_terminal_state();

    println!("\r[Starting SSH session: {}]", ssh_command);
    println!("[The TUI will return when SSH session ends]");
    io::stdout().flush().unwrap();

    let exit_status = std::process::Command::new("sh")
        .arg("-c")
        .arg(ssh_command)
        .status();

    match exit_status {
        Ok(status) if status.success() => {
            println!("\r[SSH session completed successfully]");
        }
        Ok(status) => {
            println!("\r[SSH session ended with exit code: {:?}]", status.code());
        }
        Err(e) => {
            println!("\r[SSH session failed: {}]", e);
        }
    }

    println!("[Press Enter to return to TUI...]");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);

    true
}
