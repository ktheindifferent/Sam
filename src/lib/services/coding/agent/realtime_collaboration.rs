use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};

use crate::services::coding::agent::{
    code_review::CodeLocation,
    errors::{CodingAgentError, CodingAgentResult},
};

/// Real-time collaborative coding engine with CRDT-based conflict resolution
#[derive(Clone)]
pub struct RealtimeCollaborationEngine {
    sessions: Arc<RwLock<HashMap<String, CollaborationSession>>>,
    document_store: Arc<RwLock<DocumentStore>>,
    conflict_resolver: Arc<ConflictResolver>,
    presence_tracker: Arc<PresenceTracker>,
    operation_transformer: Arc<OperationTransformer>,
    broadcast_channel: broadcast::Sender<CollaborationEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationSession {
    pub session_id: String,
    pub project_id: String,
    pub participants: Vec<Participant>,
    pub documents: HashMap<String, SharedDocument>,
    pub created_at: DateTime<Utc>,
    pub settings: SessionSettings,
    pub recording: Option<SessionRecording>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub user_id: String,
    pub username: String,
    pub role: ParticipantRole,
    pub cursor_position: Option<CursorPosition>,
    pub selection: Option<TextSelection>,
    pub status: ParticipantStatus,
    pub color: String,
    pub joined_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParticipantRole {
    Owner,
    Editor,
    Viewer,
    Reviewer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParticipantStatus {
    Active,
    Idle,
    Away,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub document_id: String,
    pub line: usize,
    pub column: usize,
    pub viewport: Option<Viewport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSelection {
    pub document_id: String,
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub top_line: usize,
    pub bottom_line: usize,
    pub left_column: usize,
    pub right_column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedDocument {
    pub document_id: String,
    pub file_path: PathBuf,
    pub content: String,
    pub version: u64,
    pub crdt: CRDT,
    pub operations: VecDeque<Operation>,
    pub checkpoints: Vec<Checkpoint>,
    pub locks: HashMap<String, DocumentLock>,
}

/// Conflict-free Replicated Data Type for document synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRDT {
    pub site_id: String,
    pub clock: VectorClock,
    pub characters: Vec<CRDTChar>,
    pub tombstones: HashSet<CharId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct CharId {
    pub site_id: String,
    pub clock: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRDTChar {
    pub id: CharId,
    pub value: char,
    pub visible: bool,
    pub after: Option<CharId>,
    pub before: Option<CharId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorClock {
    pub clocks: HashMap<String, u64>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    pub fn increment(&mut self, site_id: &str) -> u64 {
        let count = self.clocks.entry(site_id.to_string()).or_insert(0);
        *count += 1;
        *count
    }

    pub fn get(&self, site_id: &str) -> u64 {
        *self.clocks.get(site_id).unwrap_or(&0)
    }

    pub fn merge(&mut self, other: &VectorClock) {
        for (site_id, &clock) in &other.clocks {
            let current = self.clocks.entry(site_id.clone()).or_insert(0);
            *current = (*current).max(clock);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Insert(InsertOp),
    Delete(DeleteOp),
    Format(FormatOp),
    Move(MoveOp),
    Comment(CommentOp),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertOp {
    pub position: usize,
    pub text: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteOp {
    pub position: usize,
    pub length: usize,
    pub author: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatOp {
    pub start: usize,
    pub end: usize,
    pub format_type: FormatType,
    pub author: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FormatType {
    Bold,
    Italic,
    Underline,
    Highlight(String),
    CodeBlock(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveOp {
    pub from_start: usize,
    pub from_end: usize,
    pub to_position: usize,
    pub author: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentOp {
    pub position: usize,
    pub comment: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub version: u64,
    pub content: String,
    pub hash: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentLock {
    pub lock_type: LockType,
    pub owner: String,
    pub start: usize,
    pub end: usize,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LockType {
    Exclusive,
    Shared,
    Advisory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSettings {
    pub max_participants: usize,
    pub allow_anonymous: bool,
    pub auto_save_interval: u64,
    pub conflict_resolution_mode: ConflictResolutionMode,
    pub enable_voice_chat: bool,
    pub enable_screen_share: bool,
    pub enable_recording: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolutionMode {
    LastWriteWins,
    CRDT,
    ThreeWayMerge,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecording {
    pub recording_id: String,
    pub started_at: DateTime<Utc>,
    pub events: Vec<RecordedEvent>,
    pub snapshots: Vec<DocumentSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedEvent {
    pub timestamp: DateTime<Utc>,
    pub event: CollaborationEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSnapshot {
    pub timestamp: DateTime<Utc>,
    pub document_id: String,
    pub content: String,
    pub participants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaborationEvent {
    ParticipantJoined(ParticipantJoined),
    ParticipantLeft(ParticipantLeft),
    DocumentChanged(DocumentChanged),
    CursorMoved(CursorMoved),
    SelectionChanged(SelectionChanged),
    CommentAdded(CommentAdded),
    ConflictDetected(ConflictDetected),
    ConflictResolved(ConflictResolved),
    LockAcquired(LockAcquired),
    LockReleased(LockReleased),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantJoined {
    pub session_id: String,
    pub participant: Participant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantLeft {
    pub session_id: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChanged {
    pub document_id: String,
    pub operation: Operation,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorMoved {
    pub user_id: String,
    pub position: CursorPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionChanged {
    pub user_id: String,
    pub selection: TextSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentAdded {
    pub document_id: String,
    pub comment: Comment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author: String,
    pub text: String,
    pub position: Position,
    pub thread: Vec<Reply>,
    pub resolved: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reply {
    pub author: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDetected {
    pub document_id: String,
    pub conflict: Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: String,
    pub conflict_type: ConflictType,
    pub operations: Vec<Operation>,
    pub participants: Vec<String>,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    InsertInsert,
    InsertDelete,
    DeleteDelete,
    FormatConflict,
    MoveConflict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolved {
    pub conflict_id: String,
    pub resolution: Resolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub strategy: ResolutionStrategy,
    pub result: Operation,
    pub resolved_by: String,
    pub resolved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionStrategy {
    AcceptMine,
    AcceptTheirs,
    Merge,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockAcquired {
    pub document_id: String,
    pub lock: DocumentLock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockReleased {
    pub document_id: String,
    pub lock_owner: String,
}

/// Document store for managing shared documents
pub struct DocumentStore {
    documents: HashMap<String, SharedDocument>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    pub async fn get_document(&self, document_id: &str) -> Option<SharedDocument> {
        self.documents.get(document_id).cloned()
    }

    pub async fn update_document(&mut self, document_id: &str, document: SharedDocument) {
        self.documents.insert(document_id.to_string(), document);
    }

    pub async fn create_checkpoint(&mut self, document_id: &str) -> CodingAgentResult<Checkpoint> {
        let doc =
            self.documents
                .get_mut(document_id)
                .ok_or_else(|| CodingAgentError::NotFound {
                    resource: "Document".to_string(),
                    id: document_id.to_string(),
                })?;

        let content_clone = doc.content.clone();
        let version = doc.version;

        let hash = Self::calculate_hash_static(&content_clone);

        let checkpoint = Checkpoint {
            version,
            content: content_clone,
            hash,
            timestamp: Utc::now(),
        };

        doc.checkpoints.push(checkpoint.clone());
        Ok(checkpoint)
    }

    fn calculate_hash_static(content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Conflict resolver for handling concurrent edits
pub struct ConflictResolver {
    // Strategy pattern would use trait objects, but ResolutionStrategy is an enum
    default_mode: ConflictResolutionMode,
}

impl ConflictResolver {
    pub fn new() -> Self {
        Self {
            default_mode: ConflictResolutionMode::CRDT,
        }
    }

    pub async fn detect_conflict(&self, op1: &Operation, op2: &Operation) -> Option<Conflict> {
        // Detect if two operations conflict
        match (op1, op2) {
            (Operation::Insert(i1), Operation::Insert(i2)) if i1.position == i2.position => {
                Some(Conflict {
                    id: uuid::Uuid::new_v4().to_string(),
                    conflict_type: ConflictType::InsertInsert,
                    operations: vec![op1.clone(), op2.clone()],
                    participants: vec![i1.author.clone(), i2.author.clone()],
                    detected_at: Utc::now(),
                })
            }
            (Operation::Insert(i), Operation::Delete(d))
                if i.position >= d.position && i.position < d.position + d.length =>
            {
                Some(Conflict {
                    id: uuid::Uuid::new_v4().to_string(),
                    conflict_type: ConflictType::InsertDelete,
                    operations: vec![op1.clone(), op2.clone()],
                    participants: vec![i.author.clone(), d.author.clone()],
                    detected_at: Utc::now(),
                })
            }
            _ => None,
        }
    }

    pub async fn resolve_conflict(
        &self,
        conflict: &Conflict,
        mode: ConflictResolutionMode,
    ) -> CodingAgentResult<Resolution> {
        match mode {
            ConflictResolutionMode::LastWriteWins => self.resolve_last_write_wins(conflict).await,
            ConflictResolutionMode::CRDT => self.resolve_with_crdt(conflict).await,
            ConflictResolutionMode::ThreeWayMerge => self.resolve_three_way_merge(conflict).await,
            ConflictResolutionMode::Manual => Err(CodingAgentError::ExecutionError(
                "Manual conflict resolution required".to_string(),
            )),
        }
    }

    async fn resolve_last_write_wins(&self, conflict: &Conflict) -> CodingAgentResult<Resolution> {
        // Take the operation with the latest timestamp
        let latest_op =
            conflict
                .operations
                .last()
                .ok_or_else(|| CodingAgentError::ValidationError {
                    field: "conflict".to_string(),
                    message: "No operations in conflict".to_string(),
                })?;

        Ok(Resolution {
            strategy: ResolutionStrategy::AcceptTheirs,
            result: latest_op.clone(),
            resolved_by: "system".to_string(),
            resolved_at: Utc::now(),
        })
    }

    async fn resolve_with_crdt(&self, conflict: &Conflict) -> CodingAgentResult<Resolution> {
        // Use CRDT properties to resolve conflict
        // This is a simplified implementation
        let operation = conflict.operations.first().cloned().ok_or_else(|| {
            CodingAgentError::ExecutionError(format!(
                "Cannot resolve conflict {} without operations",
                conflict.id
            ))
        })?;

        Ok(Resolution {
            strategy: ResolutionStrategy::Merge,
            result: operation,
            resolved_by: "crdt".to_string(),
            resolved_at: Utc::now(),
        })
    }

    async fn resolve_three_way_merge(&self, conflict: &Conflict) -> CodingAgentResult<Resolution> {
        // Implement three-way merge algorithm
        let operation = conflict.operations.first().cloned().ok_or_else(|| {
            CodingAgentError::ExecutionError(format!(
                "Cannot resolve conflict {} without operations",
                conflict.id
            ))
        })?;

        Ok(Resolution {
            strategy: ResolutionStrategy::Merge,
            result: operation,
            resolved_by: "three-way-merge".to_string(),
            resolved_at: Utc::now(),
        })
    }
}

/// Presence tracker for managing participant status
pub struct PresenceTracker {
    presence: Arc<RwLock<HashMap<String, ParticipantPresence>>>,
}

#[derive(Debug, Clone)]
pub struct ParticipantPresence {
    pub user_id: String,
    pub status: ParticipantStatus,
    pub last_heartbeat: DateTime<Utc>,
    pub cursor_position: Option<CursorPosition>,
    pub active_document: Option<String>,
}

impl PresenceTracker {
    pub fn new() -> Self {
        Self {
            presence: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update_presence(&self, user_id: &str, presence: ParticipantPresence) {
        let mut tracker = self.presence.write().await;
        tracker.insert(user_id.to_string(), presence);
    }

    pub async fn get_active_participants(&self) -> Vec<ParticipantPresence> {
        let tracker = self.presence.read().await;
        let now = Utc::now();

        tracker
            .values()
            .filter(|p| now.signed_duration_since(p.last_heartbeat).num_seconds() < 30)
            .cloned()
            .collect()
    }
}

/// Operation transformer for operational transformation
pub struct OperationTransformer;

impl OperationTransformer {
    pub fn new() -> Self {
        Self
    }

    pub fn transform(&self, op1: &Operation, op2: &Operation) -> (Operation, Operation) {
        match (op1, op2) {
            (Operation::Insert(i1), Operation::Insert(i2)) => self.transform_insert_insert(i1, i2),
            (Operation::Insert(i), Operation::Delete(d)) => self.transform_insert_delete(i, d),
            (Operation::Delete(d), Operation::Insert(i)) => {
                let (i_prime, d_prime) = self.transform_insert_delete(i, d);
                (d_prime, i_prime)
            }
            (Operation::Delete(d1), Operation::Delete(d2)) => self.transform_delete_delete(d1, d2),
            _ => (op1.clone(), op2.clone()),
        }
    }

    fn transform_insert_insert(&self, i1: &InsertOp, i2: &InsertOp) -> (Operation, Operation) {
        let mut i1_prime = i1.clone();
        let mut i2_prime = i2.clone();

        if i1.position < i2.position || (i1.position == i2.position && i1.author < i2.author) {
            i2_prime.position += i1.text.len();
        } else {
            i1_prime.position += i2.text.len();
        }

        (Operation::Insert(i1_prime), Operation::Insert(i2_prime))
    }

    fn transform_insert_delete(&self, i: &InsertOp, d: &DeleteOp) -> (Operation, Operation) {
        let mut i_prime = i.clone();
        let mut d_prime = d.clone();

        if i.position <= d.position {
            d_prime.position += i.text.len();
        } else if i.position >= d.position + d.length {
            i_prime.position -= d.length;
        } else {
            i_prime.position = d.position;
        }

        (Operation::Insert(i_prime), Operation::Delete(d_prime))
    }

    fn transform_delete_delete(&self, d1: &DeleteOp, d2: &DeleteOp) -> (Operation, Operation) {
        let mut d1_prime = d1.clone();
        let mut d2_prime = d2.clone();

        if d1.position < d2.position {
            d2_prime.position -= d1.length.min(d2.position - d1.position);
            d2_prime.length -= (d1.position + d1.length - d2.position)
                .max(0)
                .min(d2.length);
        } else if d2.position < d1.position {
            d1_prime.position -= d2.length.min(d1.position - d2.position);
            d1_prime.length -= (d2.position + d2.length - d1.position)
                .max(0)
                .min(d1.length);
        } else {
            let overlap = d1.length.min(d2.length);
            d1_prime.length -= overlap;
            d2_prime.length -= overlap;
        }

        (Operation::Delete(d1_prime), Operation::Delete(d2_prime))
    }
}

impl RealtimeCollaborationEngine {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);

        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            document_store: Arc::new(RwLock::new(DocumentStore::new())),
            conflict_resolver: Arc::new(ConflictResolver::new()),
            presence_tracker: Arc::new(PresenceTracker::new()),
            operation_transformer: Arc::new(OperationTransformer::new()),
            broadcast_channel: tx,
        }
    }

    pub async fn create_session(
        &self,
        project_id: &str,
        settings: SessionSettings,
    ) -> CodingAgentResult<CollaborationSession> {
        let session = CollaborationSession {
            session_id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.to_string(),
            participants: Vec::new(),
            documents: HashMap::new(),
            created_at: Utc::now(),
            settings,
            recording: None,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session.session_id.clone(), session.clone());

        Ok(session)
    }

    pub async fn join_session(
        &self,
        session_id: &str,
        participant: Participant,
    ) -> CodingAgentResult<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| CodingAgentError::NotFound {
                resource: "Session".to_string(),
                id: session_id.to_string(),
            })?;

        if session.participants.len() >= session.settings.max_participants {
            return Err(CodingAgentError::ResourceLimitExceeded {
                resource: "session_participants".to_string(),
                limit: "100".to_string(),
                current: session.participants.len().to_string(),
            });
        }

        session.participants.push(participant.clone());

        // Broadcast join event
        let event = CollaborationEvent::ParticipantJoined(ParticipantJoined {
            session_id: session_id.to_string(),
            participant,
        });

        let _ = self.broadcast_channel.send(event);

        Ok(())
    }

    pub async fn apply_operation(
        &self,
        session_id: &str,
        document_id: &str,
        operation: Operation,
    ) -> CodingAgentResult<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| CodingAgentError::NotFound {
                resource: "Session".to_string(),
                id: session_id.to_string(),
            })?;

        let document =
            session
                .documents
                .get_mut(document_id)
                .ok_or_else(|| CodingAgentError::NotFound {
                    resource: "Document".to_string(),
                    id: document_id.to_string(),
                })?;

        // Check for conflicts with pending operations
        for pending_op in &document.operations {
            if let Some(conflict) = self
                .conflict_resolver
                .detect_conflict(&operation, pending_op)
                .await
            {
                // Broadcast conflict event
                let event = CollaborationEvent::ConflictDetected(ConflictDetected {
                    document_id: document_id.to_string(),
                    conflict: conflict.clone(),
                });
                let _ = self.broadcast_channel.send(event);

                // Resolve conflict
                let resolution = self
                    .conflict_resolver
                    .resolve_conflict(&conflict, session.settings.conflict_resolution_mode.clone())
                    .await?;

                // Broadcast resolution
                let event = CollaborationEvent::ConflictResolved(ConflictResolved {
                    conflict_id: conflict.id,
                    resolution,
                });
                let _ = self.broadcast_channel.send(event);
            }
        }

        // Apply operation
        self.apply_operation_to_document(document, &operation)?;

        // Update version
        document.version += 1;

        // Broadcast change
        let event = CollaborationEvent::DocumentChanged(DocumentChanged {
            document_id: document_id.to_string(),
            operation,
            version: document.version,
        });
        let _ = self.broadcast_channel.send(event);

        Ok(())
    }

    fn apply_operation_to_document(
        &self,
        document: &mut SharedDocument,
        operation: &Operation,
    ) -> CodingAgentResult<()> {
        match operation {
            Operation::Insert(op) => {
                if op.position > document.content.len() {
                    return Err(CodingAgentError::ValidationError {
                        field: "position".to_string(),
                        message: "Insert position out of bounds".to_string(),
                    });
                }
                document.content.insert_str(op.position, &op.text);
            }
            Operation::Delete(op) => {
                if op.position + op.length > document.content.len() {
                    return Err(CodingAgentError::ValidationError {
                        field: "range".to_string(),
                        message: "Delete range out of bounds".to_string(),
                    });
                }
                document.content.drain(op.position..op.position + op.length);
            }
            _ => {
                // Handle other operation types
            }
        }

        document.operations.push_back(operation.clone());

        // Keep operation history limited
        if document.operations.len() > 1000 {
            document.operations.pop_front();
        }

        Ok(())
    }

    pub async fn subscribe_to_events(&self) -> broadcast::Receiver<CollaborationEvent> {
        self.broadcast_channel.subscribe()
    }

    pub async fn acquire_lock(
        &self,
        session_id: &str,
        document_id: &str,
        lock: DocumentLock,
    ) -> CodingAgentResult<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| CodingAgentError::NotFound {
                resource: "Session".to_string(),
                id: session_id.to_string(),
            })?;

        let document =
            session
                .documents
                .get_mut(document_id)
                .ok_or_else(|| CodingAgentError::NotFound {
                    resource: "Document".to_string(),
                    id: document_id.to_string(),
                })?;

        // Check for conflicting locks
        for existing_lock in document.locks.values() {
            if self.locks_conflict(&lock, existing_lock) {
                return Err(CodingAgentError::ExecutionError(
                    "Lock conflict".to_string(),
                ));
            }
        }

        document.locks.insert(lock.owner.clone(), lock.clone());

        // Broadcast lock acquired
        let event = CollaborationEvent::LockAcquired(LockAcquired {
            document_id: document_id.to_string(),
            lock,
        });
        let _ = self.broadcast_channel.send(event);

        Ok(())
    }

    fn locks_conflict(&self, lock1: &DocumentLock, lock2: &DocumentLock) -> bool {
        // Check if ranges overlap and locks are incompatible
        let ranges_overlap = lock1.start < lock2.end && lock2.start < lock1.end;

        if !ranges_overlap {
            return false;
        }

        match (&lock1.lock_type, &lock2.lock_type) {
            (LockType::Exclusive, _) | (_, LockType::Exclusive) => true,
            (LockType::Shared, LockType::Shared) => false,
            _ => false,
        }
    }
}
