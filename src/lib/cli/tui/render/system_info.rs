use super::super::state::{ServiceStatus, SERVICE_CATALOG};
use super::get_status_color;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};

/// Calculate aggregate health score (0-100) based on running services.
pub fn calculate_health_score(status: &ServiceStatus) -> u16 {
    let total = SERVICE_CATALOG.len() as f64;
    let healthy = status.healthy_service_count() as f64;

    ((healthy / total) * 100.0).round() as u16
}

/// Render the system information mode
pub fn render_system_info_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    status: &ServiceStatus,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Health score gauge
            Constraint::Percentage(45), // System metrics
            Constraint::Percentage(45), // Service overview
        ])
        .split(area);

    // Health score gauge
    let score = calculate_health_score(status);
    let healthy_services = status.healthy_service_count();
    let total_services = SERVICE_CATALOG.len();
    let gauge_color = if score >= 80 {
        Color::Green
    } else if score >= 50 {
        Color::Yellow
    } else {
        Color::Red
    };
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("System Health"),
        )
        .gauge_style(
            Style::default()
                .fg(gauge_color)
                .add_modifier(Modifier::BOLD),
        )
        .percent(score)
        .label(format!(
            "{}% ({}/{} services healthy)",
            score, healthy_services, total_services
        ));
    f.render_widget(gauge, chunks[0]);

    let system_lines = vec![
        Line::from(vec![Span::styled(
            "System Information",
            Style::default().fg(Color::Cyan),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("CPU Usage: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.cpu_usage, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Memory Usage: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.memory_usage, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Disk Usage: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.disk_usage, Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Update Count: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                status.update_count.to_string(),
                Style::default().fg(Color::White),
            ),
        ]),
    ];

    let system_widget = Paragraph::new(system_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("System Metrics"),
    );

    let mut service_lines = vec![
        Line::from(vec![Span::styled(
            "Service Overview",
            Style::default().fg(Color::Cyan),
        )]),
        Line::from(""),
    ];
    service_lines.extend(status.service_rows().into_iter().map(|(entry, status)| {
        Line::from(vec![
            Span::styled(
                format!("{}: ", entry.label),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(status.to_string(), get_status_color(status)),
        ])
    }));

    let service_widget = Paragraph::new(service_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Service Status"),
    );

    f.render_widget(system_widget, chunks[1]);
    f.render_widget(service_widget, chunks[2]);
}
