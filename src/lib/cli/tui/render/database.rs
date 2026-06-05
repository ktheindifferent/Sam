use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

struct TableSnapshot {
    name: &'static str,
    rows: usize,
    preview: Vec<String>,
    error: Option<String>,
}

/// Render the database management mode
pub fn render_database_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    tables: &[String],
    selected: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(20)])
        .split(area);

    let snapshots = load_table_snapshots();
    let table_names = if tables.is_empty() {
        snapshots
            .iter()
            .map(|snapshot| snapshot.name.to_string())
            .collect::<Vec<_>>()
    } else {
        tables.to_vec()
    };
    let selected = selected.min(table_names.len().saturating_sub(1));

    let items = table_names
        .iter()
        .enumerate()
        .map(|(index, table)| {
            let count = snapshots
                .iter()
                .find(|snapshot| snapshot.name == table.as_str())
                .map(|snapshot| snapshot.rows)
                .unwrap_or_default();
            let style = if index == selected {
                Style::default().bg(Color::Yellow).fg(Color::Black)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(table.clone(), style),
                Span::styled(
                    format!(" ({count})"),
                    Style::default().fg(Color::DarkGray).patch(style),
                ),
            ]))
        })
        .collect::<Vec<_>>();

    let mut state = ListState::default();
    state.select(Some(selected));

    let table_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Config Tables"),
        )
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black));
    f.render_stateful_widget(table_list, chunks[0], &mut state);

    let selected_name = table_names
        .get(selected)
        .map(String::as_str)
        .unwrap_or("settings");
    let selected_snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.name == selected_name);

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Table: ", Style::default().fg(Color::Yellow)),
            Span::styled(selected_name, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Rows: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                selected_snapshot
                    .map(|snapshot| snapshot.rows.to_string())
                    .unwrap_or_else(|| "0".to_string()),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
    ];

    if let Some(snapshot) = selected_snapshot {
        if let Some(error) = &snapshot.error {
            lines.push(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red)),
                Span::styled(error, Style::default().fg(Color::White)),
            ]));
        } else if snapshot.preview.is_empty() {
            lines.push(Line::from("No records returned."));
        } else {
            lines.push(Line::from(vec![Span::styled(
                "Preview",
                Style::default().fg(Color::Cyan),
            )]));
            lines.push(Line::from(""));
            for record in &snapshot.preview {
                lines.push(Line::from(record.clone()));
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from("Up/Down: select table | R: refresh"));

    let details = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Records"))
        .wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(details, chunks[1]);
}

fn load_table_snapshots() -> Vec<TableSnapshot> {
    vec![load_settings_snapshot(), load_services_snapshot()]
}

fn load_settings_snapshot() -> TableSnapshot {
    match crate::memory::config::Setting::select(None, None, None, None) {
        Ok(settings) => TableSnapshot {
            name: "settings",
            rows: settings.len(),
            preview: settings
                .iter()
                .take(20)
                .map(|setting| format!("{} = {:?}", setting.key, setting.values))
                .collect(),
            error: None,
        },
        Err(e) => TableSnapshot {
            name: "settings",
            rows: 0,
            preview: Vec::new(),
            error: Some(e.to_string()),
        },
    }
}

fn load_services_snapshot() -> TableSnapshot {
    match crate::memory::config::Service::select(None, None, None, None) {
        Ok(services) => TableSnapshot {
            name: "services",
            rows: services.len(),
            preview: services
                .iter()
                .take(20)
                .map(|service| {
                    format!(
                        "{} -> {}",
                        if service.identifier.is_empty() {
                            service.oid.as_str()
                        } else {
                            service.identifier.as_str()
                        },
                        service.endpoint
                    )
                })
                .collect(),
            error: None,
        },
        Err(e) => TableSnapshot {
            name: "services",
            rows: 0,
            preview: Vec::new(),
            error: Some(e.to_string()),
        },
    }
}
