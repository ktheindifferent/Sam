use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};
use super::get_status_color;
use super::super::state::ServiceStatus;

/// Calculate aggregate health score (0-100) based on running services.
pub fn calculate_health_score(status: &ServiceStatus) -> u16 {
    let services = [
        &status.crawler,
        &status.redis,
        &status.postgres,
        &status.http_server,
        &status.ollama,
        &status.docker,
        &status.tts,
        &status.stt,
        &status.ssh_server,
        &status.lifx,
        &status.sms,
        &status.media,
        &status.snapcast,
    ];

    let total = services.len() as f64;
    let healthy = services.iter().filter(|s| {
        let s = s.to_lowercase();
        s == "running" || s == "connected" || s == "online"
    }).count() as f64;

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
            Constraint::Length(3),    // Health score gauge
            Constraint::Percentage(45), // System metrics
            Constraint::Percentage(45), // Service overview
        ])
        .split(area);

    // Health score gauge
    let score = calculate_health_score(status);
    let gauge_color = if score >= 80 {
        Color::Green
    } else if score >= 50 {
        Color::Yellow
    } else {
        Color::Red
    };
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("System Health"))
        .gauge_style(Style::default().fg(gauge_color).add_modifier(Modifier::BOLD))
        .percent(score)
        .label(format!("{}% ({}/13 services healthy)", score, (score as f64 * 13.0 / 100.0).round() as u16));
    f.render_widget(gauge, chunks[0]);

    let system_lines = vec![
        Line::from(vec![Span::styled("System Information", Style::default().fg(Color::Cyan))]),
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
            Span::styled(status.update_count.to_string(), Style::default().fg(Color::White)),
        ]),
    ];

    let system_widget = Paragraph::new(system_lines).block(
        Block::default().borders(Borders::ALL).title("System Metrics"),
    );

    let service_lines = vec![
        Line::from(vec![Span::styled("Service Overview", Style::default().fg(Color::Cyan))]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Crawler: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.crawler, get_status_color(&status.crawler)),
        ]),
        Line::from(vec![
            Span::styled("Redis: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.redis, get_status_color(&status.redis)),
        ]),
        Line::from(vec![
            Span::styled("PostgreSQL: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.postgres, get_status_color(&status.postgres)),
        ]),
        Line::from(vec![
            Span::styled("HTTP Server: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.http_server, get_status_color(&status.http_server)),
        ]),
        Line::from(vec![
            Span::styled("Ollama AI: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.ollama, get_status_color(&status.ollama)),
        ]),
        Line::from(vec![
            Span::styled("Docker: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.docker, get_status_color(&status.docker)),
        ]),
        Line::from(vec![
            Span::styled("TTS: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.tts, get_status_color(&status.tts)),
        ]),
        Line::from(vec![
            Span::styled("STT: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.stt, get_status_color(&status.stt)),
        ]),
        Line::from(vec![
            Span::styled("SSH Server: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.ssh_server, get_status_color(&status.ssh_server)),
        ]),
        Line::from(vec![
            Span::styled("LIFX Lights: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.lifx, get_status_color(&status.lifx)),
        ]),
        Line::from(vec![
            Span::styled("Media Center: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.media, get_status_color(&status.media)),
        ]),
        Line::from(vec![
            Span::styled("Snapcast: ", Style::default().fg(Color::Yellow)),
            Span::styled(&status.snapcast, get_status_color(&status.snapcast)),
        ]),
    ];

    let service_widget = Paragraph::new(service_lines).block(
        Block::default().borders(Borders::ALL).title("Service Status"),
    );

    f.render_widget(system_widget, chunks[1]);
    f.render_widget(service_widget, chunks[2]);
}
