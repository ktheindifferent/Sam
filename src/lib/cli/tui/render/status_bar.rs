use super::super::state::{ServiceStatus, SERVICE_CATALOG};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Sparkline},
};

/// Render the status bar with sparkline CPU/Memory charts
pub fn render_status_bar(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    status: &ServiceStatus,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // CPU sparkline
            Constraint::Percentage(40), // Memory sparkline
            Constraint::Percentage(20), // Status text
        ])
        .split(area);

    // CPU sparkline
    let cpu_data = status.cpu_history.as_u64_vec();
    let cpu_sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("CPU {}", &status.cpu_usage)),
        )
        .data(&cpu_data)
        .max(100)
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(cpu_sparkline, chunks[0]);

    // Memory sparkline
    let mem_data = status.memory_history.as_u64_vec();
    let mem_sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Mem {}", &status.memory_usage)),
        )
        .data(&mem_data)
        .max(100)
        .style(Style::default().fg(Color::Green));
    f.render_widget(mem_sparkline, chunks[1]);

    // Status text
    let running_count = status.healthy_service_count();
    let total_services = SERVICE_CATALOG.len();
    let status_text = Line::from(vec![
        Span::styled(
            format!("{}/{}", running_count, total_services),
            Style::default().fg(Color::Green),
        ),
        Span::raw(" svcs "),
        Span::styled(
            format!("#{}", status.update_count),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let status_widget = ratatui::widgets::Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(status_widget, chunks[2]);
}
