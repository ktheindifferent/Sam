use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
};
use super::super::state::{Notification, NotificationLevel};

/// Render notification toasts at top-right
pub fn render_toasts(
    f: &mut ratatui::Frame,
    area: Rect,
    notifications: &[Notification],
) {
    // Only show non-expired notifications, max 3
    let active: Vec<&Notification> = notifications
        .iter()
        .filter(|n| n.created_at.elapsed() < n.duration)
        .rev()
        .take(3)
        .collect();

    if active.is_empty() {
        return;
    }

    // Position at top-right
    let toast_width = 40u16.min(area.width.saturating_sub(2));
    let toast_x = area.x + area.width.saturating_sub(toast_width + 1);

    for (i, notification) in active.iter().enumerate() {
        let toast_y = area.y + 1 + (i as u16 * 3);
        if toast_y + 3 > area.y + area.height {
            break;
        }

        let toast_area = Rect::new(toast_x, toast_y, toast_width, 3);

        let color = match notification.level {
            NotificationLevel::Info => Color::Blue,
            NotificationLevel::Success => Color::Green,
            NotificationLevel::Warning => Color::Yellow,
            NotificationLevel::Error => Color::Red,
        };

        let toast = Paragraph::new(Span::styled(&notification.message, Style::default().fg(Color::White)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(color))
                    .title(match notification.level {
                        NotificationLevel::Info => "Info",
                        NotificationLevel::Success => "Success",
                        NotificationLevel::Warning => "Warning",
                        NotificationLevel::Error => "Error",
                    }),
            );

        f.render_widget(ratatui::widgets::Clear, toast_area);
        f.render_widget(toast, toast_area);
    }
}
