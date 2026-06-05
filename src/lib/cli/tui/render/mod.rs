pub mod coding_agent;
pub mod command;
pub mod database;
pub mod files;
pub mod help;
pub mod logs;
pub mod nav_bar;
pub mod palette;
pub mod services;
pub mod status_bar;
pub mod system_info;
pub mod toasts;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Span,
};

/// Get spinner character for animation
pub fn get_spinner_char() -> char {
    let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let index = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        / 100) as usize
        % spinner_chars.len();
    spinner_chars[index]
}

/// Create centered rect for modals/overlays
pub fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Get color style based on status string
pub fn get_status_color(status: &str) -> Style {
    match status {
        "running" | "connected" | "online" => Style::default().fg(Color::Green),
        "stopped" | "disconnected" | "offline" => Style::default().fg(Color::Red),
        "not installed" | "disabled" => Style::default().fg(Color::DarkGray),
        "error" => Style::default().fg(Color::Magenta),
        _ => Style::default().fg(Color::Gray),
    }
}
