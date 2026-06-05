use anyhow::Result;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

/// Real-time collaborative coding session
#[derive(Debug, Clone)]
pub struct CollaborationSession {
    pub id: String,
    pub participants: Arc<RwLock<Vec<Participant>>>,
    pub code_state: Arc<RwLock<CodeState>>,
    pub edit_history: Arc<RwLock<Vec<EditOperation>>>,
    pub cursor_positions: Arc<RwLock<HashMap<String, CursorPosition>>>,
    pub selections: Arc<RwLock<HashMap<String, Selection>>>,
    pub broadcast_tx: broadcast::Sender<CollaborationEvent>,
    pub ai_assistant: Arc<RwLock<AiAssistant>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub name: String,
    pub role: ParticipantRole,
    pub joined_at: SystemTime,
    pub last_active: SystemTime,
    pub is_active: bool,
    pub color: String, // For cursor/selection visualization
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParticipantRole {
    Owner,
    Editor,
    Viewer,
    AI,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeState {
    pub content: String,
    pub language: String,
    pub version: u64,
    pub last_modified: SystemTime,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditOperation {
    pub id: String,
    pub participant_id: String,
    pub timestamp: SystemTime,
    pub operation: OperationType,
    pub version_before: u64,
    pub version_after: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Insert {
        position: usize,
        text: String,
    },
    Delete {
        start: usize,
        end: usize,
    },
    Replace {
        start: usize,
        end: usize,
        text: String,
    },
    Format,
    Refactor {
        refactor_type: String,
        affected_range: (usize, usize),
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub start: CursorPosition,
    pub end: CursorPosition,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaborationEvent {
    ParticipantJoined(Participant),
    ParticipantLeft(String),
    CodeChanged(EditOperation),
    CursorMoved {
        participant_id: String,
        position: CursorPosition,
    },
    SelectionChanged {
        participant_id: String,
        selection: Selection,
    },
    SuggestionMade(AiSuggestion),
    CommentAdded(CodeComment),
    FileRenamed(String),
    SessionEnded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSuggestion {
    pub id: String,
    pub suggestion_type: SuggestionType,
    pub content: String,
    pub explanation: String,
    pub confidence: f32,
    pub affected_lines: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    CodeCompletion,
    BugFix,
    Optimization,
    Refactoring,
    Documentation,
    Security,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeComment {
    pub id: String,
    pub participant_id: String,
    pub line_number: usize,
    pub content: String,
    pub timestamp: SystemTime,
    pub resolved: bool,
}

#[derive(Debug, Clone)]
pub struct AiAssistant {
    pub enabled: bool,
    pub model: String,
    pub auto_suggest: bool,
    pub suggest_interval: Duration,
    pub last_suggestion: Option<SystemTime>,
}

impl CollaborationSession {
    /// Create a new collaboration session
    pub fn new(id: String, owner: Participant, initial_code: String, language: String) -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);

        Self {
            id,
            participants: Arc::new(RwLock::new(vec![owner])),
            code_state: Arc::new(RwLock::new(CodeState {
                content: initial_code,
                language,
                version: 1,
                last_modified: SystemTime::now(),
                file_path: None,
            })),
            edit_history: Arc::new(RwLock::new(Vec::new())),
            cursor_positions: Arc::new(RwLock::new(HashMap::new())),
            selections: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            ai_assistant: Arc::new(RwLock::new(AiAssistant {
                enabled: true,
                model: "codellama".to_string(),
                auto_suggest: true,
                suggest_interval: Duration::from_secs(5),
                last_suggestion: None,
            })),
        }
    }

    /// Add a participant to the session
    pub async fn add_participant(&self, participant: Participant) -> Result<()> {
        let mut participants = self.participants.write().await;

        // Check if participant already exists
        if participants.iter().any(|p| p.id == participant.id) {
            return Err(anyhow::anyhow!("Participant already in session"));
        }

        participants.push(participant.clone());

        // Broadcast join event
        let _ = self
            .broadcast_tx
            .send(CollaborationEvent::ParticipantJoined(participant));

        Ok(())
    }

    /// Remove a participant from the session
    pub async fn remove_participant(&self, participant_id: &str) -> Result<()> {
        let mut participants = self.participants.write().await;
        participants.retain(|p| p.id != participant_id);

        // Clean up cursor and selection data
        let mut cursors = self.cursor_positions.write().await;
        cursors.remove(participant_id);

        let mut selections = self.selections.write().await;
        selections.remove(participant_id);

        // Broadcast leave event
        let _ = self.broadcast_tx.send(CollaborationEvent::ParticipantLeft(
            participant_id.to_string(),
        ));

        Ok(())
    }

    /// Apply an edit operation to the code
    pub async fn apply_edit(&self, operation: EditOperation) -> Result<()> {
        let mut code_state = self.code_state.write().await;
        let mut content = code_state.content.clone();

        match &operation.operation {
            OperationType::Insert { position, text } => {
                if *position <= content.len() {
                    content.insert_str(*position, text);
                }
            }
            OperationType::Delete { start, end } => {
                if *start < content.len() && *end <= content.len() && start <= end {
                    content.replace_range(*start..*end, "");
                }
            }
            OperationType::Replace { start, end, text } => {
                if *start < content.len() && *end <= content.len() && start <= end {
                    content.replace_range(*start..*end, text);
                }
            }
            OperationType::Format => {
                // Format the code based on language
                content = self.format_code(&content, &code_state.language).await?;
            }
            OperationType::Refactor {
                refactor_type,
                affected_range,
            } => {
                // Apply refactoring
                content = self
                    .apply_refactoring(&content, refactor_type, affected_range)
                    .await?;
            }
        }

        code_state.content = content;
        code_state.version += 1;
        code_state.last_modified = SystemTime::now();

        // Add to history
        let mut history = self.edit_history.write().await;
        history.push(operation.clone());

        // Broadcast change
        let _ = self
            .broadcast_tx
            .send(CollaborationEvent::CodeChanged(operation));

        Ok(())
    }

    /// Format code based on language
    async fn format_code(&self, code: &str, language: &str) -> Result<String> {
        // Placeholder for actual formatting logic
        // In production, you'd use language-specific formatters
        match language {
            "rust" => {
                // Use rustfmt
                Ok(code.to_string())
            }
            "javascript" | "typescript" => {
                // Use prettier
                Ok(code.to_string())
            }
            "python" => {
                // Use black
                Ok(code.to_string())
            }
            _ => Ok(code.to_string()),
        }
    }

    /// Apply refactoring to code
    async fn apply_refactoring(
        &self,
        code: &str,
        refactor_type: &str,
        range: &(usize, usize),
    ) -> Result<String> {
        // Placeholder for refactoring logic
        Ok(code.to_string())
    }

    /// Update cursor position for a participant
    pub async fn update_cursor(
        &self,
        participant_id: String,
        position: CursorPosition,
    ) -> Result<()> {
        let mut cursors = self.cursor_positions.write().await;
        cursors.insert(participant_id.clone(), position.clone());

        // Broadcast cursor update
        let _ = self.broadcast_tx.send(CollaborationEvent::CursorMoved {
            participant_id,
            position,
        });

        Ok(())
    }

    /// Update selection for a participant
    pub async fn update_selection(
        &self,
        participant_id: String,
        selection: Selection,
    ) -> Result<()> {
        let mut selections = self.selections.write().await;
        selections.insert(participant_id.clone(), selection.clone());

        // Broadcast selection update
        let _ = self
            .broadcast_tx
            .send(CollaborationEvent::SelectionChanged {
                participant_id,
                selection,
            });

        Ok(())
    }

    /// Get AI suggestions for current code
    pub async fn get_ai_suggestions(
        &self,
        context: SuggestionContext,
    ) -> Result<Vec<AiSuggestion>> {
        let code_state = self.code_state.read().await;
        let ai_assistant = self.ai_assistant.read().await;

        if !ai_assistant.enabled {
            return Ok(Vec::new());
        }

        let mut suggestions = Vec::new();

        // Generate different types of suggestions based on context
        if let Some(completion) = self
            .generate_completion(&code_state.content, &context)
            .await?
        {
            suggestions.push(completion);
        }

        if let Some(bug_fix) = self
            .detect_and_suggest_fix(&code_state.content, &context)
            .await?
        {
            suggestions.push(bug_fix);
        }

        if let Some(optimization) = self
            .suggest_optimization(&code_state.content, &context)
            .await?
        {
            suggestions.push(optimization);
        }

        Ok(suggestions)
    }

    /// Generate code completion suggestion
    async fn generate_completion(
        &self,
        code: &str,
        context: &SuggestionContext,
    ) -> Result<Option<AiSuggestion>> {
        // Extract context around cursor
        let lines: Vec<&str> = code.lines().collect();

        if context.cursor_line >= lines.len() {
            return Ok(None);
        }

        let current_line = lines[context.cursor_line];

        // Simple completion logic (would use AI model in production)
        if current_line.trim().starts_with("fn ") && !current_line.contains('{') {
            return Ok(Some(AiSuggestion {
                id: uuid::Uuid::new_v4().to_string(),
                suggestion_type: SuggestionType::CodeCompletion,
                content: " {\n    // TODO: Implement function\n}".to_string(),
                explanation: "Complete function signature".to_string(),
                confidence: 0.9,
                affected_lines: vec![context.cursor_line],
            }));
        }

        Ok(None)
    }

    /// Detect bugs and suggest fixes
    async fn detect_and_suggest_fix(
        &self,
        code: &str,
        context: &SuggestionContext,
    ) -> Result<Option<AiSuggestion>> {
        // Simple bug detection (would use AI model in production)
        if code.contains(".unwrap()") {
            return Ok(Some(AiSuggestion {
                id: uuid::Uuid::new_v4().to_string(),
                suggestion_type: SuggestionType::BugFix,
                content: "Replace .unwrap() with proper error handling using ? operator".to_string(),
                explanation: "Using unwrap() can cause panics. Consider using the ? operator for better error handling.".to_string(),
                confidence: 0.8,
                affected_lines: vec![],
            }));
        }

        Ok(None)
    }

    /// Suggest code optimizations
    async fn suggest_optimization(
        &self,
        code: &str,
        context: &SuggestionContext,
    ) -> Result<Option<AiSuggestion>> {
        // Simple optimization detection (would use AI model in production)
        if code.contains("Vec::new()") && code.contains(".push(") {
            let push_count = code.matches(".push(").count();
            if push_count > 3 {
                return Ok(Some(AiSuggestion {
                    id: uuid::Uuid::new_v4().to_string(),
                    suggestion_type: SuggestionType::Optimization,
                    content: format!("Consider using Vec::with_capacity({}) for better performance", push_count),
                    explanation: "Pre-allocating vector capacity can improve performance by avoiding reallocations.".to_string(),
                    confidence: 0.7,
                    affected_lines: vec![],
                }));
            }
        }

        Ok(None)
    }

    /// Subscribe to collaboration events
    pub fn subscribe(&self) -> broadcast::Receiver<CollaborationEvent> {
        self.broadcast_tx.subscribe()
    }

    /// Get current session state
    pub async fn get_state(&self) -> SessionState {
        SessionState {
            id: self.id.clone(),
            participants: self.participants.read().await.clone(),
            code: self.code_state.read().await.clone(),
            cursor_positions: self.cursor_positions.read().await.clone(),
            selections: self.selections.read().await.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionContext {
    pub cursor_line: usize,
    pub cursor_column: usize,
    pub selected_text: Option<String>,
    pub file_type: String,
    pub trigger: SuggestionTrigger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionTrigger {
    Manual,
    Typing,
    Pause,
    Save,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub id: String,
    pub participants: Vec<Participant>,
    pub code: CodeState,
    pub cursor_positions: HashMap<String, CursorPosition>,
    pub selections: HashMap<String, Selection>,
}

/// Collaboration manager for handling multiple sessions
pub struct CollaborationManager {
    sessions: Arc<RwLock<HashMap<String, Arc<CollaborationSession>>>>,
    participant_sessions: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

impl CollaborationManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            participant_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new collaboration session
    pub async fn create_session(
        &self,
        owner: Participant,
        initial_code: String,
        language: String,
    ) -> Result<Arc<CollaborationSession>> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let session = Arc::new(CollaborationSession::new(
            session_id.clone(),
            owner.clone(),
            initial_code,
            language,
        ));

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session.clone());

        let mut participant_sessions = self.participant_sessions.write().await;
        participant_sessions
            .entry(owner.id)
            .or_insert_with(HashSet::new)
            .insert(session_id);

        Ok(session)
    }

    /// Join an existing session
    pub async fn join_session(
        &self,
        session_id: &str,
        participant: Participant,
    ) -> Result<Arc<CollaborationSession>> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?
            .clone();

        session.add_participant(participant.clone()).await?;

        let mut participant_sessions = self.participant_sessions.write().await;
        participant_sessions
            .entry(participant.id)
            .or_insert_with(HashSet::new)
            .insert(session_id.to_string());

        Ok(session)
    }

    /// Leave a session
    pub async fn leave_session(&self, session_id: &str, participant_id: &str) -> Result<()> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            session.remove_participant(participant_id).await?;
        }

        let mut participant_sessions = self.participant_sessions.write().await;
        if let Some(sessions) = participant_sessions.get_mut(participant_id) {
            sessions.remove(session_id);
        }

        Ok(())
    }

    /// Get all sessions for a participant
    pub async fn get_participant_sessions(&self, participant_id: &str) -> Vec<String> {
        let participant_sessions = self.participant_sessions.read().await;
        participant_sessions
            .get(participant_id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// End a session
    pub async fn end_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(session_id) {
            let _ = session.broadcast_tx.send(CollaborationEvent::SessionEnded);

            // Clean up participant mappings
            let mut participant_sessions = self.participant_sessions.write().await;
            for (_, sessions) in participant_sessions.iter_mut() {
                sessions.remove(session_id);
            }
        }

        Ok(())
    }
}

// Re-export uuid for convenience
pub use uuid;
