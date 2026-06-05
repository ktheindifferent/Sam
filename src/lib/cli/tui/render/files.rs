use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::time::SystemTime;

#[derive(Debug)]
struct FileEntry {
    name: String,
    path: std::path::PathBuf,
    kind: &'static str,
    size: u64,
    modified: Option<SystemTime>,
}

/// Render the file browser mode
pub fn render_files_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    path: &std::path::Path,
    selected: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Path: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                path.display().to_string(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from("Enter: open directory | Backspace: parent | R: refresh"),
    ])
    .block(Block::default().borders(Borders::ALL).title("File Browser"));
    f.render_widget(header, chunks[0]);

    match read_entries(path) {
        Ok(entries) => {
            render_entry_list(f, chunks[1], &entries, selected);
            render_selection_details(f, chunks[2], &entries, selected);
        }
        Err(error) => {
            let widget = Paragraph::new(format!("Unable to read directory:\n{error}")).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Directory Listing"),
            );
            f.render_widget(widget, chunks[1]);
            render_error_footer(f, chunks[2]);
        }
    }
}

fn render_entry_list(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    entries: &[FileEntry],
    selected: usize,
) {
    if entries.is_empty() {
        let empty = Paragraph::new("Directory is empty.").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Directory Listing"),
        );
        f.render_widget(empty, area);
        return;
    }

    let selected = selected.min(entries.len().saturating_sub(1));
    let items = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let selected_style = if index == selected {
                Style::default().bg(Color::Yellow).fg(Color::Black)
            } else {
                Style::default()
            };
            let kind_style = match entry.kind {
                "dir" => Style::default().fg(Color::Cyan).patch(selected_style),
                "file" => Style::default().fg(Color::White).patch(selected_style),
                _ => Style::default().fg(Color::DarkGray).patch(selected_style),
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("[{}] ", entry.kind), kind_style),
                Span::styled(entry.name.clone(), selected_style),
                Span::styled(
                    format!("  {}", display_size(entry)),
                    Style::default().fg(Color::DarkGray).patch(selected_style),
                ),
            ]))
        })
        .collect::<Vec<_>>();

    let mut state = ListState::default();
    state.select(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Directory Listing"),
        )
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black));
    f.render_stateful_widget(list, area, &mut state);
}

fn render_selection_details(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    entries: &[FileEntry],
    selected: usize,
) {
    let selected = selected.min(entries.len().saturating_sub(1));
    let lines = if let Some(entry) = entries.get(selected) {
        vec![
            Line::from(vec![
                Span::styled("Selected: ", Style::default().fg(Color::Yellow)),
                Span::styled(&entry.name, Style::default().fg(Color::White)),
                Span::styled("  Type: ", Style::default().fg(Color::Yellow)),
                Span::styled(entry.kind, Style::default().fg(Color::White)),
                Span::styled("  Size: ", Style::default().fg(Color::Yellow)),
                Span::styled(display_size(entry), Style::default().fg(Color::White)),
                Span::styled("  Modified: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format_modified(entry.modified),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("Path: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    entry.path.display().to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
        ]
    } else {
        vec![
            Line::from("Directory is empty."),
            Line::from("Enter: open directory | Backspace: parent | R: refresh"),
        ]
    };

    let footer = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Selection Details"),
    );
    f.render_widget(footer, area);
}

fn render_error_footer(f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    let footer = Paragraph::new("Backspace: parent | R: retry | F1-F8: switch modes")
        .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(footer, area);
}

fn read_entries(path: &std::path::Path) -> std::io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let kind = if metadata.is_dir() {
            "dir"
        } else if metadata.is_file() {
            "file"
        } else {
            "other"
        };
        entries.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry.path(),
            kind,
            size: metadata.len(),
            modified: metadata.modified().ok(),
        });
    }

    entries.sort_by(|a, b| {
        let a_dir = a.kind == "dir";
        let b_dir = b.kind == "dir";
        b_dir
            .cmp(&a_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

fn display_size(entry: &FileEntry) -> String {
    if entry.kind == "dir" {
        "-".to_string()
    } else {
        format_size(entry.size)
    }
}

fn format_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = size as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", size, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn format_modified(modified: Option<SystemTime>) -> String {
    let Some(modified) = modified else {
        return "unknown".to_string();
    };

    match modified.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => {
            let timestamp = duration.as_secs();
            if let Some(datetime) = chrono::DateTime::from_timestamp(timestamp as i64, 0) {
                datetime
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
            } else {
                "unknown".to_string()
            }
        }
        Err(_) => "unknown".to_string(),
    }
}
