use super::super::state::ServiceStatus;
use super::get_status_color;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

/// Render the services management mode
pub fn render_services_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    status: &ServiceStatus,
    selected: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let services = status.service_rows();

    let service_items: Vec<ListItem> = services
        .iter()
        .enumerate()
        .map(|(i, (entry, status_val))| {
            let style = if i == selected {
                ratatui::style::Style::default()
                    .bg(ratatui::style::Color::Yellow)
                    .fg(ratatui::style::Color::Black)
            } else {
                ratatui::style::Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{}: ", entry.label), style),
                Span::styled(*status_val, get_status_color(status_val).patch(style)),
            ]))
        })
        .collect();

    let service_list = List::new(service_items)
        .block(Block::default().borders(Borders::ALL).title("Services"))
        .highlight_style(
            ratatui::style::Style::default()
                .bg(ratatui::style::Color::Yellow)
                .fg(ratatui::style::Color::Black),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    let (selected_name, selected_status) = if selected < services.len() {
        let (entry, status) = services[selected];
        (entry.label, status)
    } else {
        ("Unknown", "Unknown")
    };

    let details_lines = vec![
        Line::from(vec![
            Span::styled(
                "Service: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(
                selected_name,
                ratatui::style::Style::default().fg(ratatui::style::Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Status: ",
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
            ),
            Span::styled(selected_status, get_status_color(selected_status)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions:",
            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
        )]),
        Line::from("  [Space] Start/Stop Service"),
        Line::from("  [S/T] Start or Stop Service"),
        Line::from("  [R] Restart Service"),
        Line::from("  [L] View Filtered Logs"),
        Line::from("  [Enter] Service Details"),
    ];

    let details_widget = Paragraph::new(details_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Service Details"),
    );

    f.render_stateful_widget(service_list, chunks[0], &mut list_state);
    f.render_widget(details_widget, chunks[1]);
}
