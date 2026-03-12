use ratatui::{
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Render the help screen
pub fn render_help_mode(f: &mut ratatui::Frame, area: ratatui::layout::Rect, _scroll: u16) {
    let help_text = vec![
        Line::from(vec![Span::styled("S.A.M. TUI Help", ratatui::style::Style::default().fg(ratatui::style::Color::Cyan))]),
        Line::from(""),
        Line::from(vec![Span::styled("Navigation:", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))]),
        Line::from("  F1 - Command Mode (default)"),
        Line::from("  F2 - Services Management"),
        Line::from("  F3 - System Logs (with scrolling & filtering)"),
        Line::from("  F4 - System Information"),
        Line::from("  F5 - Database Management"),
        Line::from("  F6 - File Browser"),
        Line::from("  F7 - This Help Screen"),
        Line::from("  F8 - AI Coding Agent (Interactive)"),
        Line::from(""),
        Line::from(vec![Span::styled("Commands:", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))]),
        Line::from("  Ctrl+C - Exit application"),
        Line::from("  Page Up/Down - Scroll output"),
        Line::from("  Up/Down - Navigate lists"),
        Line::from("  Enter - Execute command/action"),
        Line::from("  Tab - Auto-complete (in command mode)"),
        Line::from(""),
        Line::from(vec![Span::styled("Service Management:", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))]),
        Line::from("  Space - Start/Stop service"),
        Line::from("  R - Restart service"),
        Line::from("  L - View service logs"),
        Line::from(""),
        Line::from(vec![Span::styled("Log Viewer (F3):", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))]),
        Line::from("  Up/Down - Scroll logs"),
        Line::from("  Page Up/Down - Fast scroll"),
        Line::from("  / - Enter filter mode"),
        Line::from("  c - Clear filter"),
        Line::from("  ESC - Exit filter mode"),
        Line::from(""),
        Line::from(vec![Span::styled("Tips:", ratatui::style::Style::default().fg(ratatui::style::Color::Green))]),
        Line::from("  - Use arrow keys to navigate"),
        Line::from("  - Status colors: Green=running, Red=stopped, Gray=unknown"),
        Line::from("  - System metrics update every 2 seconds"),
    ];

    let help_widget = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help & Documentation"))
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(help_widget, area);
}
