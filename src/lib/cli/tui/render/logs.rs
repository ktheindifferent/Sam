use super::super::state::TuiState;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget, TuiWidgetState};

/// Render the logs mode with scrolling and filtering support
pub fn render_logs_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &TuiState,
    show_cursor: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);

    let cursor_char = if show_cursor && state.log_input_mode {
        "_"
    } else {
        " "
    };
    let filter_display = format!("{}{}", state.log_filter_text, cursor_char);
    let filter_title = if state.log_input_mode {
        "Filter (Type to filter, ESC to exit, Enter to apply)"
    } else {
        "Filter (Press / to edit, Use Up/Down to scroll)"
    };

    let filter_widget = Paragraph::new(filter_display)
        .block(Block::default().borders(Borders::ALL).title(filter_title));

    let effective_widget_state = if !state.log_filter_text.is_empty() {
        let filter_lower = state.log_filter_text.to_lowercase();

        if filter_lower == "error" || filter_lower == "err" {
            TuiWidgetState::new().set_default_display_level(log::LevelFilter::Error)
        } else if filter_lower == "warn" || filter_lower == "warning" {
            TuiWidgetState::new().set_default_display_level(log::LevelFilter::Warn)
        } else if filter_lower == "info" {
            TuiWidgetState::new().set_default_display_level(log::LevelFilter::Info)
        } else if filter_lower == "debug" {
            TuiWidgetState::new().set_default_display_level(log::LevelFilter::Debug)
        } else if filter_lower == "trace" {
            TuiWidgetState::new().set_default_display_level(log::LevelFilter::Trace)
        } else {
            TuiWidgetState::new()
                .set_default_display_level(log::LevelFilter::Off)
                .set_level_for_target(&state.log_filter_text, log::LevelFilter::Trace)
        }
    } else {
        TuiWidgetState::new().set_default_display_level(log::LevelFilter::Debug)
    };

    let log_title = if !state.log_filter_text.is_empty() {
        format!(
            "System Logs - Filter: '{}' | Scroll: {}",
            state.log_filter_text, state.log_scroll_offset
        )
    } else {
        format!("System Logs - Scroll: {}", state.log_scroll_offset)
    };

    let tui_logger_widget = TuiLoggerWidget::default()
        .block(Block::default().borders(Borders::ALL).title(log_title))
        .output_separator('|')
        .output_level(Some(TuiLoggerLevelOutput::Long))
        .output_target(true)
        .output_timestamp(Some("%H:%M:%S".to_string()))
        .style_error(ratatui::style::Style::default().fg(ratatui::style::Color::Red))
        .style_warn(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))
        .style_info(ratatui::style::Style::default().fg(ratatui::style::Color::Blue))
        .style_debug(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan))
        .style_trace(ratatui::style::Style::default().fg(ratatui::style::Color::Gray))
        .state(&effective_widget_state);

    let help_text = if state.log_input_mode {
        "ESC: Exit filter | Enter: Apply filter | Backspace: Delete | Type: error/warn/info/debug/trace or target name"
    } else {
        "↑/↓: Scroll | PageUp/Down: Page | /: Filter | c: Clear | +/-: Log Level | Space: Toggle | ←/→: Display Level"
    };

    let help_widget =
        Paragraph::new(help_text).block(Block::default().borders(Borders::ALL).title("Controls"));

    f.render_widget(filter_widget, chunks[0]);
    f.render_widget(tui_logger_widget, chunks[1]);
    f.render_widget(help_widget, chunks[2]);
}
