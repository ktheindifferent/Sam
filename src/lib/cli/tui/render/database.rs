use ratatui::widgets::{Block, Borders, Paragraph};

/// Render the database management mode
pub fn render_database_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    _tables: &[String],
    _selected: usize,
) {
    let placeholder = Paragraph::new("Database management mode\n\nFeatures coming soon:\n- Table browser\n- Query executor\n- Schema viewer")
        .block(Block::default().borders(Borders::ALL).title("Database Management"));
    f.render_widget(placeholder, area);
}
