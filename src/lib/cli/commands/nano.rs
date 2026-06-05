use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Cursor position in the editor
#[derive(Debug, Clone, Copy)]
struct CursorPosition {
    row: usize,
    col: usize,
}

impl Default for CursorPosition {
    fn default() -> Self {
        Self { row: 0, col: 0 }
    }
}

/// Editor state and data
#[derive(Debug)]
struct EditorState {
    /// Lines of text in the file
    lines: Vec<String>,
    /// Current cursor position
    cursor: CursorPosition,
    /// Scroll offset for vertical scrolling
    scroll_offset: usize,
    /// Horizontal scroll offset
    horizontal_offset: usize,
    /// File path (None for new file)
    file_path: Option<PathBuf>,
    /// Whether the file has been modified
    modified: bool,
    /// Editor viewport size
    viewport_height: usize,
    viewport_width: usize,
    /// Status message
    status_message: String,
    /// Whether to show help
    show_help: bool,
    /// Whether we're in exit confirmation mode
    exit_confirm: bool,
    /// Whether to always show key help at bottom
    show_key_help: bool,
}

impl EditorState {
    fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: CursorPosition::default(),
            scroll_offset: 0,
            horizontal_offset: 0,
            file_path: None,
            modified: false,
            viewport_height: 20,
            viewport_width: 80,
            status_message: "New file".to_string(),
            show_help: false,
            exit_confirm: false,
            show_key_help: true,
        }
    }

    fn from_file(path: &Path) -> Result<Self, std::io::Error> {
        let content = fs::read_to_string(path)?;
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };

        Ok(Self {
            lines,
            cursor: CursorPosition::default(),
            scroll_offset: 0,
            horizontal_offset: 0,
            file_path: Some(path.to_path_buf()),
            modified: false,
            viewport_height: 20,
            viewport_width: 80,
            status_message: format!("Opened: {}", path.display()),
            show_help: false,
            exit_confirm: false,
            show_key_help: true,
        })
    }

    fn save(&mut self) -> Result<(), std::io::Error> {
        let path = match &self.file_path {
            Some(p) => p.clone(),
            None => {
                // For new files without a name, save as "untitled.txt"
                let default_path = std::path::PathBuf::from("untitled.txt");
                self.file_path = Some(default_path.clone());
                self.status_message = "Saved as untitled.txt (no filename specified)".to_string();
                default_path
            }
        };

        let content = self.lines.join("\n");
        fs::write(&path, content)?;
        self.modified = false;
        if self.status_message.contains("no filename specified") {
            // Keep the existing message for new files
        } else {
            self.status_message = format!("Saved: {}", path.display());
        }
        Ok(())
    }

    fn save_as(&mut self, path: PathBuf) -> Result<(), std::io::Error> {
        self.file_path = Some(path.clone());
        self.save()
    }

    /// Get the current line content
    fn current_line(&self) -> &str {
        &self.lines[self.cursor.row]
    }

    /// Get mutable reference to current line
    fn current_line_mut(&mut self) -> &mut String {
        &mut self.lines[self.cursor.row]
    }

    /// Insert character at cursor position
    fn insert_char(&mut self, ch: char) {
        let col = self.cursor.col;
        let line = self.current_line_mut();
        line.insert(col, ch);
        self.cursor.col += 1;
        self.modified = true;
    }

    /// Delete character at cursor position
    fn delete_char(&mut self) {
        if self.cursor.col > 0 {
            let col = self.cursor.col;
            let line = self.current_line_mut();
            line.remove(col - 1);
            self.cursor.col -= 1;
            self.modified = true;
        } else if self.cursor.row > 0 {
            // Join with previous line
            let current_line = self.lines.remove(self.cursor.row);
            self.cursor.row -= 1;
            self.cursor.col = self.lines[self.cursor.row].len();
            self.lines[self.cursor.row].push_str(&current_line);
            self.modified = true;
        }
    }

    /// Insert new line at cursor position
    fn insert_newline(&mut self) {
        let col = self.cursor.col;
        let row = self.cursor.row;
        let line = self.lines[row].clone();
        let (left, right) = line.split_at(col);
        self.lines[row] = left.to_string();
        self.lines.insert(row + 1, right.to_string());
        self.cursor.row += 1;
        self.cursor.col = 0;
        self.modified = true;
    }

    /// Move cursor up
    fn move_up(&mut self) {
        if self.cursor.row > 0 {
            self.cursor.row -= 1;
            let line_len = self.lines[self.cursor.row].len();
            if self.cursor.col > line_len {
                self.cursor.col = line_len;
            }
        }
    }

    /// Move cursor down
    fn move_down(&mut self) {
        if self.cursor.row < self.lines.len() - 1 {
            self.cursor.row += 1;
            let line_len = self.lines[self.cursor.row].len();
            if self.cursor.col > line_len {
                self.cursor.col = line_len;
            }
        }
    }

    /// Move cursor left
    fn move_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        } else if self.cursor.row > 0 {
            self.cursor.row -= 1;
            self.cursor.col = self.lines[self.cursor.row].len();
        }
    }

    /// Move cursor right
    fn move_right(&mut self) {
        let line_len = self.current_line().len();
        if self.cursor.col < line_len {
            self.cursor.col += 1;
        } else if self.cursor.row < self.lines.len() - 1 {
            self.cursor.row += 1;
            self.cursor.col = 0;
        }
    }

    /// Move to beginning of line
    fn move_home(&mut self) {
        self.cursor.col = 0;
    }

    /// Move to end of line
    fn move_end(&mut self) {
        self.cursor.col = self.current_line().len();
    }

    /// Update scroll offset to keep cursor visible
    fn update_scroll(&mut self) {
        // Vertical scrolling
        if self.cursor.row < self.scroll_offset {
            self.scroll_offset = self.cursor.row;
        } else if self.cursor.row >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = self.cursor.row - self.viewport_height + 1;
        }

        // Horizontal scrolling
        if self.cursor.col < self.horizontal_offset {
            self.horizontal_offset = self.cursor.col;
        } else if self.cursor.col >= self.horizontal_offset + self.viewport_width {
            self.horizontal_offset = self.cursor.col - self.viewport_width + 1;
        }
    }
}

/// Render the editor interface
fn render_editor(f: &mut Frame, state: &EditorState) {
    let size = f.area();

    // Main layout - always reserve space for key help at bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),                                      // Main editor area
            Constraint::Length(2),                                   // Status line
            Constraint::Length(3),                                   // Key help at bottom
            Constraint::Length(if state.show_help { 6 } else { 0 }), // Extended help area
        ])
        .split(size);

    // Render main editor area
    render_main_editor(f, chunks[0], state);

    // Render status line
    render_status_line(f, chunks[1], state);

    // Always render key help at bottom
    render_key_help(f, chunks[2], state);

    // Render extended help if requested
    if state.show_help {
        render_help(f, chunks[3]);
    }
}

/// Render the main editor text area
fn render_main_editor(f: &mut Frame, area: Rect, state: &EditorState) {
    let mut text_lines = Vec::new();

    // Calculate visible lines
    let start_line = state.scroll_offset;
    let end_line = (start_line + area.height as usize).min(state.lines.len());

    for (line_idx, line) in state.lines[start_line..end_line].iter().enumerate() {
        let actual_line_idx = start_line + line_idx;

        // Apply horizontal scrolling
        let visible_text = if state.horizontal_offset < line.len() {
            &line[state.horizontal_offset..]
        } else {
            ""
        };

        // Truncate to viewport width
        let truncated_text = if visible_text.len() > area.width as usize {
            &visible_text[..area.width as usize]
        } else {
            visible_text
        };

        // Highlight current line
        let style = if actual_line_idx == state.cursor.row {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };

        text_lines.push(Line::from(Span::styled(truncated_text, style)));
    }

    let title = match &state.file_path {
        Some(path) => format!(
            "nano - {} {}",
            path.display(),
            if state.modified { "*" } else { "" }
        ),
        None => format!("nano - New File {}", if state.modified { "*" } else { "" }),
    };

    let paragraph = Paragraph::new(text_lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// Render the status line
fn render_status_line(f: &mut Frame, area: Rect, state: &EditorState) {
    let status_text = if state.exit_confirm {
        format!(
            "Save changes before closing? (Y)es/(N)o/(C)ancel: {}",
            state.status_message
        )
    } else {
        format!(
            "Line {}/{} Col {} | {}",
            state.cursor.row + 1,
            state.lines.len(),
            state.cursor.col + 1,
            state.status_message
        )
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().bg(Color::Blue).fg(Color::White))
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(status, area);
}

/// Render key help at bottom
fn render_key_help(f: &mut Frame, area: Rect, state: &EditorState) {
    let help_text = if state.exit_confirm {
        "Y: Save and exit   N: Exit without saving   C: Cancel and continue editing"
    } else {
        "^X Exit   ^O Save   ^G Help   ^K Cut Line   ^U Uncut   ^W Search   ^V Page Down   ^Y Page Up"
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("Key Help"));

    f.render_widget(help, area);
}

/// Render help information
fn render_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from("nano Help - Key Bindings:"),
        Line::from(""),
        Line::from("Ctrl+X: Exit editor    Ctrl+O: Save file     Ctrl+G: Toggle help"),
        Line::from("Ctrl+K: Cut line       Ctrl+U: Uncut line    Ctrl+W: Search"),
        Line::from("Arrow keys: Navigate   Home/End: Line start/end"),
        Line::from("Enter: New line        Backspace: Delete     Delete: Delete forward"),
    ];

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: false });

    f.render_widget(help, area);
}

/// Handle editor key events
async fn handle_key_event(
    key_code: KeyCode,
    modifiers: KeyModifiers,
    state: &mut EditorState,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Handle exit confirmation mode
    if state.exit_confirm {
        match key_code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Save and exit
                if let Err(e) = state.save() {
                    state.status_message = format!("Error saving: {}", e);
                    state.exit_confirm = false;
                    return Ok(false);
                }
                return Ok(true);
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                // Exit without saving
                return Ok(true);
            }
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Esc => {
                // Cancel exit
                state.exit_confirm = false;
                state.status_message = "Cancelled".to_string();
                return Ok(false);
            }
            _ => {
                // Ignore other keys in exit confirmation mode
                return Ok(false);
            }
        }
    }

    match (key_code, modifiers) {
        // Exit editor
        (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
            if state.modified {
                state.exit_confirm = true;
                state.status_message = "File has been modified".to_string();
                return Ok(false);
            }
            return Ok(true);
        }

        // Save file
        (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
            if let Err(e) = state.save() {
                state.status_message = format!("Error saving: {}", e);
            }
        }

        // Toggle help
        (KeyCode::Char('g'), KeyModifiers::CONTROL) => {
            state.show_help = !state.show_help;
        }

        // Navigation
        (KeyCode::Up, _) => state.move_up(),
        (KeyCode::Down, _) => state.move_down(),
        (KeyCode::Left, _) => state.move_left(),
        (KeyCode::Right, _) => state.move_right(),
        (KeyCode::Home, _) => state.move_home(),
        (KeyCode::End, _) => state.move_end(),

        // Text editing
        (KeyCode::Enter, _) => state.insert_newline(),
        (KeyCode::Backspace, _) => state.delete_char(),
        (KeyCode::Char(ch), _) => state.insert_char(ch),

        // Page navigation
        (KeyCode::PageUp, _) => {
            for _ in 0..state.viewport_height {
                state.move_up();
            }
        }
        (KeyCode::PageDown, _) => {
            for _ in 0..state.viewport_height {
                state.move_down();
            }
        }

        _ => {} // Ignore other keys
    }

    Ok(false)
}

/// Main nano editor function
pub async fn run_nano_editor(file_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize editor state
    let mut state = match file_path {
        Some(path) => {
            let path_buf = PathBuf::from(path);
            if path_buf.exists() {
                EditorState::from_file(&path_buf).unwrap_or_else(|_| {
                    let mut state = EditorState::new();
                    state.file_path = Some(path_buf);
                    state.status_message = "New file".to_string();
                    state
                })
            } else {
                let mut state = EditorState::new();
                state.file_path = Some(path_buf);
                state.status_message = "New file".to_string();
                state.exit_confirm = false;
                state.show_key_help = true;
                state
            }
        }
        None => EditorState::new(),
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Ensure terminal is restored on panic
    std::panic::set_hook(Box::new(|_| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }));

    let result = run_editor_loop(&mut terminal, &mut state).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

/// Main editor event loop
async fn run_editor_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut EditorState,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Update viewport size
        let size = terminal.size()?;
        // Account for status line (2), key help (3), and extended help if shown
        let reserved_height = if state.show_help { 11 } else { 5 };
        state.viewport_height = size.height.saturating_sub(reserved_height) as usize;
        state.viewport_width = size.width.saturating_sub(2) as usize;

        // Update scroll to keep cursor visible
        state.update_scroll();

        // Render the interface
        terminal.draw(|f| render_editor(f, state))?;

        // Handle events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                let should_exit =
                    handle_key_event(key_event.code, key_event.modifiers, state).await?;

                if should_exit {
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Handle nano command from CLI
pub async fn handle_nano(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    if parts.len() > 2 {
        let mut lines = output_lines.lock().await;
        lines.push("Usage: nano [filename]".to_string());
        return;
    }

    let file_path = if parts.len() == 2 {
        Some(parts[1])
    } else {
        None
    };

    {
        let mut lines = output_lines.lock().await;
        lines.push("Starting nano editor...".to_string());
        lines.push("__TUI_RESTART_NEEDED__".to_string()); // Signal TUI restart
    }

    // Run the editor
    if let Err(e) = run_nano_editor(file_path).await {
        let mut lines = output_lines.lock().await;
        lines.push(format!("nano error: {}", e));
    }
}
