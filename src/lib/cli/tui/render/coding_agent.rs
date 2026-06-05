use super::super::state::TuiState;
use super::{centered_rect, get_spinner_char};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Tabs, Wrap},
};

/// Render the enhanced coding agent mode with all features
pub fn render_coding_agent_mode(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &TuiState,
    show_cursor: bool,
    _output_lines: &[String],
) {
    if state.coding_agent_show_help {
        render_coding_agent_help_overlay(f, area);
        return;
    }

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(4),
        ])
        .split(area);

    // === STATUS BAR WITH TABS ===
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Percentage(60)])
        .split(main_chunks[0]);

    let spinner_char = if !state.coding_agent_spinner_text.is_empty() {
        format!("{} ", get_spinner_char())
    } else {
        String::new()
    };

    let status_lines = vec![
        Line::from(vec![
            Span::styled("Model: ", Style::default().fg(Color::Gray)),
            Span::styled(&state.coding_agent_model, Style::default().fg(Color::Cyan)),
            Span::raw(" | "),
            Span::styled("Dir: ", Style::default().fg(Color::Gray)),
            Span::styled(
                &state.coding_agent_working_dir,
                Style::default().fg(Color::Blue),
            ),
        ]),
        Line::from(vec![
            Span::raw(&spinner_char),
            Span::styled(
                if !state.coding_agent_spinner_text.is_empty() {
                    &state.coding_agent_spinner_text
                } else if state.coding_agent_verify_mode {
                    "Verify Mode ON"
                } else if state.coding_agent_auto_execute {
                    "Auto-Execute ON"
                } else {
                    "Ready"
                },
                Style::default().fg(if state.coding_agent_verify_mode {
                    Color::Yellow
                } else if state.coding_agent_auto_execute {
                    Color::Green
                } else {
                    Color::White
                }),
            ),
        ]),
    ];

    let status_widget =
        Paragraph::new(status_lines).block(Block::default().borders(Borders::ALL).title("Status"));

    let tab_titles = vec!["Output", "Steps", "Context", "History"];
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title("Views"))
        .select(state.coding_agent_panel_focus)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(status_widget, status_chunks[0]);
    f.render_widget(tabs, status_chunks[1]);

    // === PROGRESS BAR (shown during multi-step execution) ===
    let (content_area, _progress_area) =
        if !state.coding_agent_execution_steps.is_empty() && state.coding_agent_panel_focus == 0 {
            let progress_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(main_chunks[1]);
            let step_progress = if !state.coding_agent_execution_steps.is_empty() {
                ((state.coding_agent_current_step as f64
                    / state.coding_agent_execution_steps.len() as f64)
                    * 100.0) as u16
            } else {
                0
            };
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title(format!(
                    "Progress: Step {}/{}",
                    state.coding_agent_current_step + 1,
                    state.coding_agent_execution_steps.len()
                )))
                .gauge_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .percent(step_progress);
            f.render_widget(gauge, progress_chunks[0]);
            (progress_chunks[1], Some(progress_chunks[0]))
        } else {
            (main_chunks[1], None)
        };

    // === MAIN CONTENT AREA ===
    let content_layout: Vec<ratatui::layout::Rect> = if state.coding_agent_panel_focus >= 2 {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(content_area)
            .to_vec()
    } else {
        vec![content_area]
    };

    match state.coding_agent_panel_focus {
        0 => render_agent_output_panel(f, content_layout[0], state),
        1 => render_agent_steps_panel(f, content_layout[0], state),
        2 => {
            render_agent_output_panel(f, content_layout[0], state);
            render_agent_context_panel(f, content_layout[1], state);
        }
        3 => {
            render_agent_output_panel(f, content_layout[0], state);
            render_agent_history_panel(f, content_layout[1], state);
        }
        _ => {}
    }

    // === INPUT AREA ===
    render_agent_input_area(f, main_chunks[2], state, show_cursor);
}

/// Style a single line with diff-aware coloring.
fn style_output_line<'a>(l: &'a str) -> Line<'a> {
    // Diff coloring
    if l.starts_with('+') && !l.starts_with("+++") {
        return Line::from(Span::styled(l, Style::default().fg(Color::Green)));
    }
    if l.starts_with('-') && !l.starts_with("---") {
        return Line::from(Span::styled(l, Style::default().fg(Color::Red)));
    }
    if l.starts_with("@@") {
        return Line::from(Span::styled(l, Style::default().fg(Color::Cyan)));
    }
    if l.starts_with("---") || l.starts_with("+++") {
        return Line::from(Span::styled(
            l,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if l.starts_with("diff ") {
        return Line::from(Span::styled(
            l,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
    }
    // Status icons
    if l.starts_with("✅") {
        Line::from(Span::styled(l, Style::default().fg(Color::Green)))
    } else if l.starts_with("❌") || l.contains("Error:") {
        Line::from(Span::styled(l, Style::default().fg(Color::Red)))
    } else if l.starts_with("⏳") || l.starts_with("🤖") {
        Line::from(Span::styled(l, Style::default().fg(Color::Yellow)))
    } else if l.starts_with("💬") {
        Line::from(Span::styled(l, Style::default().fg(Color::Blue)))
    } else if l.starts_with("📋") {
        Line::from(Span::styled(l, Style::default().fg(Color::Magenta)))
    } else if l.starts_with("   ") {
        Line::from(Span::styled(l, Style::default().fg(Color::Gray)))
    } else {
        Line::from(Span::raw(l))
    }
}

fn render_agent_output_panel(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &TuiState,
) {
    let content_lines: Vec<Line> = if !state.coding_agent_execution_log.is_empty() {
        state
            .coding_agent_execution_log
            .iter()
            .map(|l| style_output_line(l))
            .collect()
    } else if !state.coding_agent_response.is_empty() {
        state
            .coding_agent_response
            .lines()
            .map(|l| style_output_line(l))
            .collect()
    } else {
        vec![
            Line::from(Span::styled(
                "🤖 AI Coding Assistant",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Commands:",
                Style::default().fg(Color::Yellow),
            )),
            Line::from("  • Type a task and press Enter to execute"),
            Line::from("  • Use 'verify:' prefix for step-by-step verification"),
            Line::from("  • Press Tab to switch between panels"),
            Line::from("  • Press F1 for help (or type 'help')"),
            Line::from(""),
            Line::from(Span::styled(
                "Features:",
                Style::default().fg(Color::Yellow),
            )),
            Line::from("  • Multi-step task execution"),
            Line::from("  • Command history (Up/Down arrows)"),
            Line::from("  • Context-aware suggestions"),
            Line::from("  • Real-time execution monitoring"),
        ]
    };

    // Calculate page info for pagination indicator
    let visible_height = area.height.saturating_sub(2) as usize; // minus borders
    let total_lines = content_lines.len();
    let current_page = if visible_height > 0 {
        (state.coding_agent_scroll_offset as usize / visible_height) + 1
    } else {
        1
    };
    let total_pages = if visible_height > 0 && total_lines > 0 {
        ((total_lines - 1) / visible_height) + 1
    } else {
        1
    };

    let title = if total_pages > 1 {
        format!("Output [{}/{}]", current_page, total_pages)
    } else {
        "Output".to_string()
    };

    let output = Paragraph::new(content_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(if state.coding_agent_panel_focus == 0 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        )
        .scroll((state.coding_agent_scroll_offset, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(output, area);
}

fn render_agent_steps_panel(f: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &TuiState) {
    if state.coding_agent_execution_steps.is_empty() {
        let placeholder = Paragraph::new(
            "No execution steps yet.\n\nSteps will appear here when you execute a task.",
        )
        .style(Style::default().fg(Color::Gray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Execution Steps"),
        );
        f.render_widget(placeholder, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let items: Vec<ListItem> = state
        .coding_agent_execution_steps
        .iter()
        .enumerate()
        .map(|(i, step)| {
            let style = if i < state.coding_agent_current_step {
                Style::default().fg(Color::Green)
            } else if i == state.coding_agent_current_step {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let prefix = if i < state.coding_agent_current_step {
                "✅ "
            } else if i == state.coding_agent_current_step {
                "⏳ "
            } else {
                "⏸  "
            };

            ListItem::new(format!("{}{}", prefix, step)).style(style)
        })
        .collect();

    let steps_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Steps"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    let progress = if !state.coding_agent_execution_steps.is_empty() {
        (state.coding_agent_current_step as f64 / state.coding_agent_execution_steps.len() as f64
            * 100.0) as u16
    } else {
        0
    };

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .gauge_style(Style::default().fg(Color::Green))
        .percent(progress)
        .label(format!("{}%", progress));

    f.render_widget(steps_list, chunks[0]);
    f.render_widget(gauge, chunks[1]);
}

fn render_agent_context_panel(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &TuiState,
) {
    let items: Vec<ListItem> = if state.coding_agent_context.is_empty() {
        vec![ListItem::new("No context yet").style(Style::default().fg(Color::Gray))]
    } else {
        state
            .coding_agent_context
            .iter()
            .map(|ctx| ListItem::new(ctx.as_str()))
            .collect()
    };

    let context_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Context")
            .border_style(if state.coding_agent_panel_focus == 2 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            }),
    );

    f.render_widget(context_list, area);
}

fn render_agent_history_panel(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &TuiState,
) {
    let items: Vec<ListItem> = if state.coding_agent_history.is_empty() {
        vec![ListItem::new("No history yet").style(Style::default().fg(Color::Gray))]
    } else {
        state
            .coding_agent_history
            .iter()
            .enumerate()
            .map(|(i, cmd)| {
                let style = if i == state.coding_agent_history_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(format!("{}: {}", i + 1, cmd)).style(style)
            })
            .collect()
    };

    let history_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("History")
            .border_style(if state.coding_agent_panel_focus == 3 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            }),
    );

    f.render_widget(history_list, area);
}

fn render_agent_input_area(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &TuiState,
    show_cursor: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(1)])
        .split(area);

    let cursor_char = if show_cursor && state.coding_agent_input_mode {
        "_"
    } else {
        " "
    };
    let input_display = format!("{}{}", state.coding_agent_input, cursor_char);

    let input_title = if state.coding_agent_executor.is_some() {
        "Task (Enter to queue | Ctrl+C to cancel | Tab to switch panels)"
    } else if state.coding_agent_verify_mode {
        "Task [VERIFY MODE] (Enter to execute with verification)"
    } else if state.coding_agent_auto_execute {
        "Task [AUTO MODE] (Enter to execute immediately)"
    } else {
        "Task (Enter to execute | F1 for help | Tab to switch panels)"
    };

    let input = Paragraph::new(input_display).style(Style::default()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(input_title)
            .border_style(if state.coding_agent_input_mode {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            }),
    );

    let shortcuts = vec![
        ("F1", "Help"),
        ("Tab", "Switch Panel"),
        ("Ctrl+V", "Verify Mode"),
        ("Ctrl+A", "Auto Mode"),
        ("↑↓", "History"),
        ("PgUp/Dn", "Scroll"),
        ("Ctrl+C", "Cancel"),
        ("ESC", "Exit Input"),
    ];

    let shortcut_text = shortcuts
        .iter()
        .map(|(key, desc)| format!("[{}] {}", key, desc))
        .collect::<Vec<_>>()
        .join("  ");

    let shortcuts_widget = Paragraph::new(shortcut_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(input, chunks[0]);
    f.render_widget(shortcuts_widget, chunks[1]);
}

fn render_coding_agent_help_overlay(f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    f.render_widget(Clear, area);

    let help_area = centered_rect(80, 80, area);

    let help_text = vec![
        Line::from(Span::styled(
            "🤖 AI Coding Agent - Help",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "COMMANDS:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  verify:<task>    - Execute with step-by-step verification"),
        Line::from("  auto:<task>      - Execute immediately without confirmation"),
        Line::from("  help             - Show this help screen"),
        Line::from("  clear            - Clear the output"),
        Line::from("  reset            - Reset the agent state"),
        Line::from(""),
        Line::from(Span::styled(
            "KEYBOARD SHORTCUTS:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  F1               - Toggle this help screen"),
        Line::from("  Tab              - Switch between panels"),
        Line::from("  Ctrl+V           - Toggle verification mode"),
        Line::from("  Ctrl+A           - Toggle auto-execute mode"),
        Line::from("  Up/Down          - Navigate command history"),
        Line::from("  Page Up/Down     - Scroll output"),
        Line::from("  Ctrl+C           - Cancel current execution"),
        Line::from("  ESC              - Exit input mode / Close help"),
        Line::from("  Enter            - Execute command"),
        Line::from(""),
        Line::from(Span::styled(
            "PANELS:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Output           - Main execution output"),
        Line::from("  Steps            - Step-by-step execution progress"),
        Line::from("  Context          - Conversation context and state"),
        Line::from("  History          - Command history"),
        Line::from(""),
        Line::from(Span::styled(
            "Press ESC to close this help",
            Style::default().fg(Color::Gray),
        )),
    ];

    let help_widget = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Left);

    f.render_widget(help_widget, help_area);
}
