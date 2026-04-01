use ratatui::{
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use super::get_status_color;
use super::super::state::ServiceStatus;

/// Render the command mode interface
pub fn render_command_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    status: &ServiceStatus,
    input: &str,
    show_cursor: bool,
    output_lines: &[String],
    scroll_offset: u16,
    output_height: &mut usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);

    *output_height = chunks[1].height.max(1) as usize;

    let cursor_char = if show_cursor { "_" } else { " " };
    let input_display = format!("{input}{cursor_char}");

    let status_lines = vec![
        Line::from(vec![Span::styled(
            "Services: ",
            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
        )]),
        Line::from(vec![
            Span::styled("Crawler: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
            Span::styled(&status.crawler, get_status_color(&status.crawler)),
            Span::raw("  "),
            Span::styled("Redis: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
            Span::styled(&status.redis, get_status_color(&status.redis)),
            Span::raw("  "),
            Span::styled("PostgreSQL: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
            Span::styled(&status.postgres, get_status_color(&status.postgres)),
            Span::raw("  "),
            Span::styled("Ollama: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
            Span::styled(&status.ollama, get_status_color(&status.ollama)),
        ]),
        Line::from(vec![
            Span::styled("Docker: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
            Span::styled(&status.docker, get_status_color(&status.docker)),
            Span::raw("  "),
            Span::styled("TTS: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
            Span::styled(&status.tts, get_status_color(&status.tts)),
            Span::raw("  "),
            Span::styled("STT: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
            Span::styled(&status.stt, get_status_color(&status.stt)),
            Span::raw("  "),
            Span::styled("SSH: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
            Span::styled(&status.ssh_server, get_status_color(&status.ssh_server)),
            Span::raw("  "),
            Span::styled("LIFX: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
            Span::styled(&status.lifx, get_status_color(&status.lifx)),
            Span::raw("  "),
            Span::styled("Media: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
            Span::styled(&status.media, get_status_color(&status.media)),
        ]),
        Line::from(vec![
            Span::styled("System: ", ratatui::style::Style::default().fg(ratatui::style::Color::Cyan)),
            Span::styled("CPU: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
            Span::styled(&status.cpu_usage, ratatui::style::Style::default().fg(ratatui::style::Color::White)),
            Span::raw("  "),
            Span::styled("Memory: ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
            Span::styled(&status.memory_usage, ratatui::style::Style::default().fg(ratatui::style::Color::White)),
        ]),
    ];

    let status_widget = Paragraph::new(status_lines).block(
        Block::default().borders(Borders::ALL).title("System Status"),
    );

    let output: Vec<Line> = output_lines
        .iter()
        .map(|l| Line::from(Span::raw(l)))
        .collect();

    let output_widget = Paragraph::new(output)
        .block(Block::default().borders(Borders::ALL).title("Command Output"))
        .scroll((scroll_offset, 0))
        .wrap(ratatui::widgets::Wrap { trim: false });

    let input_widget = Paragraph::new(input_display).block(
        Block::default().borders(Borders::ALL).title("Command Input"),
    );

    f.render_widget(status_widget, chunks[0]);
    f.render_widget(output_widget, chunks[1]);
    f.render_widget(input_widget, chunks[2]);
}
