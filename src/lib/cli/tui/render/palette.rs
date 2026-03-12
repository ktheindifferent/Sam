use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};
use super::{centered_rect, super::state::CommandPalette};

/// Render the command palette overlay
pub fn render_command_palette(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    palette: &CommandPalette,
) {
    if !palette.visible {
        return;
    }

    let popup = centered_rect(60, 50, area);
    f.render_widget(Clear, popup);

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Min(3),
        ])
        .split(popup);

    // Search input
    let search = Paragraph::new(format!("{}|", palette.query))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Command Palette (Ctrl+P)")
                .border_style(Style::default().fg(Color::Yellow)),
        );
    f.render_widget(search, chunks[0]);

    // Filtered actions
    let query_lower = palette.query.to_lowercase();
    let filtered: Vec<&super::super::state::PaletteAction> = palette.actions
        .iter()
        .filter(|a| {
            query_lower.is_empty()
                || a.label.to_lowercase().contains(&query_lower)
                || a.description.to_lowercase().contains(&query_lower)
        })
        .collect();

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, action)| {
            let style = if i == palette.selected {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(&action.label, style.add_modifier(Modifier::BOLD)),
                Span::styled(format!("  {}", action.description), style.fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Actions"));
    f.render_widget(list, chunks[1]);
}
