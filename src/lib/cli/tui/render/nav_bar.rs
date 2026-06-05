use super::super::state::TuiMode;
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Render the navigation bar at the top of the TUI
pub fn render_nav_bar(f: &mut ratatui::Frame, area: ratatui::layout::Rect, current_mode: &TuiMode) {
    let nav_items = vec![
        ("F1", "Command", *current_mode == TuiMode::Command),
        ("F2", "Services", *current_mode == TuiMode::Services),
        ("F3", "Logs", *current_mode == TuiMode::Logs),
        ("F4", "System", *current_mode == TuiMode::SystemInfo),
        ("F5", "Database", *current_mode == TuiMode::Database),
        ("F6", "Files", *current_mode == TuiMode::Files),
        ("F7", "Help", *current_mode == TuiMode::Help),
        ("F8", "AI Code", *current_mode == TuiMode::CodingAgent),
    ];

    let nav_line = Line::from(
        nav_items
            .iter()
            .flat_map(|(key, name, active)| {
                let style = if *active {
                    Style::default()
                        .fg(ratatui::style::Color::Black)
                        .bg(ratatui::style::Color::Yellow)
                } else {
                    Style::default().fg(ratatui::style::Color::Yellow)
                };
                vec![
                    Span::styled(*key, style),
                    Span::styled(format!(" {} ", name), style),
                    Span::raw(" "),
                ]
            })
            .collect::<Vec<_>>(),
    );

    let nav_widget =
        Paragraph::new(nav_line).block(Block::default().borders(Borders::ALL).title("Navigation"));
    f.render_widget(nav_widget, area);
}
