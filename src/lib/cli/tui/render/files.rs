use ratatui::widgets::{Block, Borders, Paragraph};

/// Render the file browser mode
pub fn render_files_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    _path: &std::path::Path,
) {
    let placeholder = Paragraph::new("File browser mode\n\nFeatures coming soon:\n- Directory navigation\n- File operations\n- Quick file viewer")
        .block(Block::default().borders(Borders::ALL).title("File Browser"));
    f.render_widget(placeholder, area);
}
