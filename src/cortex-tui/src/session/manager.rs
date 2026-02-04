//! Session manager - the main interface for session operations.
//!
//! `CortexSession` wraps the storage layer and provides a convenient API
//! for managing the current session state.

use anyhow::Result;
use cortex_engine::client::{FunctionCall, Message, TokenUsage, ToolCall};

use super::storage::SessionStorage;
use super::types::{SessionMeta, SessionSummary, StoredMessage};

// ============================================================
// CORTEX SESSION
// ============================================================

/// Manages the current session state and provides operations.
///
/// This is the main interface for session operations in Cortex TUI.
/// It handles:
/// - Creating new sessions
/// - Loading existing sessions
/// - Adding messages (with auto-save)
/// - Forking sessions
/// - Session metadata management
pub struct CortexSession {
    /// Session metadata.
    pub meta: SessionMeta,
    /// Messages in memory.
    messages: Vec<StoredMessage>,
    /// Storage backend.
    storage: SessionStorage,
    /// Whether the session has unsaved changes.
    modified: bool,
}

impl CortexSession {
    /// Creates a new session.
    pub fn new(provider: &str, model: &str) -> Result<Self> {
        let storage = SessionStorage::new()?;
        let meta = SessionMeta::new(provider, model);

        // Save initial metadata
        storage.save_meta(&meta)?;

        Ok(Self {
            meta,
            messages: Vec::new(),
            storage,
            modified: false,
        })
    }

    /// Creates a session with a custom storage backend (for testing).
    pub fn with_storage(provider: &str, model: &str, storage: SessionStorage) -> Result<Self> {
        let meta = SessionMeta::new(provider, model);
        storage.save_meta(&meta)?;

        Ok(Self {
            meta,
            messages: Vec::new(),
            storage,
            modified: false,
        })
    }

    /// Loads an existing session.
    pub fn load(session_id: &str) -> Result<Self> {
        let storage = SessionStorage::new()?;
        let meta = storage.load_meta(session_id)?;
        let messages = storage.load_messages(session_id)?;

        Ok(Self {
            meta,
            messages,
            storage,
            modified: false,
        })
    }

    /// Gets the session ID.
    pub fn id(&self) -> &str {
        &self.meta.id
    }

    /// Gets the session title.
    pub fn title(&self) -> String {
        self.meta.display_title()
    }

    /// Sets the session title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.meta.title = Some(title.into());
        self.modified = true;
    }

    /// Gets all messages.
    pub fn messages(&self) -> &[StoredMessage] {
        &self.messages
    }

    /// Gets the number of messages.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Checks if there are any messages.
    pub fn has_messages(&self) -> bool {
        !self.messages.is_empty()
    }

    /// Gets the last message.
    pub fn last_message(&self) -> Option<&StoredMessage> {
        self.messages.last()
    }

    /// Gets a message by ID.
    pub fn get_message(&self, id: &str) -> Option<&StoredMessage> {
        self.messages.iter().find(|m| m.id == id)
    }

    // ========================================================================
    // MESSAGE OPERATIONS
    // ========================================================================

    /// Adds a user message.
    pub fn add_user_message(&mut self, content: &str) -> &StoredMessage {
        let message = StoredMessage::user(content);
        self.add_message_internal(message)
    }

    /// Adds an assistant message.
    pub fn add_assistant_message(&mut self, content: &str, tokens: TokenUsage) -> &StoredMessage {
        let message = StoredMessage::assistant(content)
            .with_tokens(tokens.input_tokens, tokens.output_tokens);

        // Update token counts in metadata
        self.meta
            .add_tokens(tokens.input_tokens, tokens.output_tokens);

        self.add_message_internal(message)
    }

    /// Adds an assistant message with reasoning.
    pub fn add_assistant_message_with_reasoning(
        &mut self,
        content: &str,
        reasoning: &str,
        tokens: TokenUsage,
    ) -> &StoredMessage {
        let message = StoredMessage::assistant(content)
            .with_tokens(tokens.input_tokens, tokens.output_tokens)
            .with_reasoning(reasoning);

        self.meta
            .add_tokens(tokens.input_tokens, tokens.output_tokens);
        self.add_message_internal(message)
    }

    /// Adds a system message.
    pub fn add_system_message(&mut self, content: &str) -> &StoredMessage {
        let message = StoredMessage::system(content);
        self.add_message_internal(message)
    }

    /// Adds a tool result message for agentic continuation.
    pub fn add_tool_result(
        &mut self,
        tool_call_id: &str,
        output: &str,
        _success: bool,
    ) -> &StoredMessage {
        let message = StoredMessage::tool_result(tool_call_id, output);
        self.add_message_internal(message)
    }

    /// Adds a pre-built message directly (for messages with tool calls).
    pub fn add_message_raw(&mut self, message: StoredMessage) -> &StoredMessage {
        self.add_message_internal(message)
    }

    /// Updates token counts in metadata.
    ///
    /// Saves metadata to disk. If save fails, the in-memory state is still
    /// updated but marked as modified for later retry.
    pub fn add_tokens(&mut self, input: i64, output: i64) {
        self.meta.add_tokens(input, output);
        if let Err(e) = self.storage.save_meta(&self.meta) {
            tracing::error!(
                session_id = %self.meta.id,
                error = %e,
                "Failed to save metadata after token update"
            );
            self.modified = true;
        }
    }

    /// Adds a pre-built Message (cortex_core::widgets::Message) to the session.
    pub fn add_message(&mut self, message: cortex_core::widgets::Message) {
        let stored = match message.role {
            cortex_core::widgets::MessageRole::User => StoredMessage::user(&message.content),
            cortex_core::widgets::MessageRole::Assistant => {
                StoredMessage::assistant(&message.content)
            }
            cortex_core::widgets::MessageRole::System => StoredMessage::system(&message.content),
            cortex_core::widgets::MessageRole::Tool => StoredMessage::system(&message.content), // Treat tool as system
        };
        self.add_message_internal(stored);
    }

    /// Removes and returns the last exchange (user + assistant messages).
    /// Returns None if there are fewer than 2 messages.
    ///
    /// Only updates in-memory state after successful storage operations.
    /// If storage fails, the messages are restored to maintain consistency.
    pub fn pop_last_exchange(&mut self) -> Option<Vec<cortex_core::widgets::Message>> {
        if self.messages.len() < 2 {
            return None;
        }

        // Pop messages from memory temporarily
        let last = self.messages.pop();
        let prev = self.messages.pop();

        // Build result from popped messages
        let mut result = Vec::new();
        let mut popped_messages = Vec::new();

        if let Some(ref msg) = prev {
            let role = match msg.role.as_str() {
                "assistant" => cortex_core::widgets::MessageRole::Assistant,
                "user" => cortex_core::widgets::MessageRole::User,
                _ => cortex_core::widgets::MessageRole::System,
            };
            result.push(cortex_core::widgets::Message {
                role,
                content: msg.content.clone(),
                timestamp: None,
                is_streaming: false,
                tool_name: None,
            });
            popped_messages.push(msg.clone());
        }

        if let Some(ref msg) = last {
            let role = match msg.role.as_str() {
                "assistant" => cortex_core::widgets::MessageRole::Assistant,
                "user" => cortex_core::widgets::MessageRole::User,
                _ => cortex_core::widgets::MessageRole::System,
            };
            result.push(cortex_core::widgets::Message {
                role,
                content: msg.content.clone(),
                timestamp: None,
                is_streaming: false,
                tool_name: None,
            });
            popped_messages.push(msg.clone());
        }

        // Try to save updated state to storage
        let rewrite_result = self.storage.rewrite_messages(&self.meta.id, &self.messages);

        match rewrite_result {
            Ok(()) => {
                // Storage succeeded, update metadata
                self.meta.message_count = self.messages.len() as u32;

                if let Err(e) = self.storage.save_meta(&self.meta) {
                    tracing::error!(
                        session_id = %self.meta.id,
                        error = %e,
                        "Failed to save metadata after undo - history is updated but metadata may be stale"
                    );
                    self.modified = true;
                }

                Some(result)
            }
            Err(e) => {
                // Storage failed - restore messages to maintain consistency
                tracing::error!(
                    session_id = %self.meta.id,
                    error = %e,
                    "Failed to rewrite messages after undo - restoring original state"
                );

                // Restore in reverse order (prev was popped second, so push it first)
                if let Some(msg) = prev {
                    self.messages.push(msg);
                }
                if let Some(msg) = last {
                    self.messages.push(msg);
                }

                // Return None to indicate the operation failed
                None
            }
        }
    }

    /// Internal method to add a message and persist it.
    ///
    /// Only updates in-memory state after successful storage operations.
    /// This ensures consistency between disk and memory state.
    fn add_message_internal(&mut self, message: StoredMessage) -> &StoredMessage {
        // Try to append to storage first - only update memory state on success
        match self.storage.append_message(&self.meta.id, &message) {
            Ok(()) => {
                // Storage succeeded, now update metadata
                self.meta.increment_messages();

                // Try to save metadata - if this fails, we still keep the message
                // since it was already persisted to history
                if let Err(e) = self.storage.save_meta(&self.meta) {
                    tracing::error!(
                        session_id = %self.meta.id,
                        error = %e,
                        "Failed to save metadata after message append - message is saved but metadata may be stale"
                    );
                }

                // Add to in-memory list only after storage success
                self.messages.push(message);
            }
            Err(e) => {
                // Storage failed - log error but still add to memory for this session
                // This allows the conversation to continue even if persistence fails
                tracing::error!(
                    session_id = %self.meta.id,
                    error = %e,
                    "Failed to save message to storage - message exists only in memory"
                );

                // Still add to memory so the conversation can continue
                self.messages.push(message);
                self.modified = true; // Mark as modified since we have unsaved changes
            }
        }

        self.messages.last().expect("message was just added")
    }

    /// Converts messages to API format for completion requests.
    /// Filters out messages with empty content (except assistant with tool_calls).
    /// Empty messages are filtered to avoid sending invalid requests to providers.
    pub fn messages_for_api(&self) -> Vec<Message> {
        self.messages
            .iter()
            .filter_map(|m| {
                // Filter out messages with empty content
                // Exception: assistant messages with tool_calls are kept even if content is empty
                let has_tool_calls = !m.tool_calls.is_empty();
                let has_content = !m.content.is_empty();

                // Skip messages with no content AND no tool_calls
                // (User/system/tool messages with empty content are filtered out)
                if !has_content && !has_tool_calls && m.role != "tool" {
                    // Tool results should always be included even if empty
                    // (they indicate the tool ran)
                    if m.role == "user" || m.role == "system" {
                        return None;
                    }
                    // Assistant without content and without tool_calls -> skip
                    if m.role == "assistant" {
                        return None;
                    }
                }

                let msg = match m.role.as_str() {
                    "user" => Message::user(&m.content),
                    "assistant" => {
                        // Build tool_calls first
                        let tool_calls: Option<Vec<ToolCall>> = if m.tool_calls.is_empty() {
                            None
                        } else {
                            Some(
                                m.tool_calls
                                    .iter()
                                    .map(|tc| ToolCall {
                                        id: tc.id.clone(),
                                        call_type: "function".to_string(),
                                        function: FunctionCall {
                                            name: tc.name.clone(),
                                            arguments: tc.input.to_string(),
                                        },
                                    })
                                    .collect(),
                            )
                        };

                        // Create message directly with proper structure
                        // This ensures tool_calls are attached even when content is empty
                        // Providers (OpenRouter/Anthropic/Bedrock) will handle empty content
                        // correctly when tool_calls are present
                        Message {
                            role: cortex_engine::client::MessageRole::Assistant,
                            content: cortex_engine::client::MessageContent::Text(m.content.clone()),
                            tool_call_id: None,
                            tool_calls,
                        }
                    }
                    "system" => Message::system(&m.content),
                    "tool" => {
                        // Tool result message
                        if let Some(ref tool_call_id) = m.tool_call_id {
                            Message::tool_result(tool_call_id, &m.content)
                        } else {
                            // Fallback: treat as user message if no tool_call_id
                            Message::user(&m.content)
                        }
                    }
                    _ => Message::user(&m.content),
                };
                Some(msg)
            })
            .collect()
    }

    // ========================================================================
    // PERSISTENCE
    // ========================================================================

    /// Saves the session (metadata and any pending changes).
    pub fn save(&mut self) -> Result<()> {
        self.storage.save_meta(&self.meta)?;
        self.modified = false;
        Ok(())
    }

    /// Reloads the session from disk.
    pub fn reload(&mut self) -> Result<()> {
        self.meta = self.storage.load_meta(&self.meta.id)?;
        self.messages = self.storage.load_messages(&self.meta.id)?;
        self.modified = false;
        Ok(())
    }

    /// Checks if there are unsaved changes.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    // ========================================================================
    // FORK OPERATIONS
    // ========================================================================

    /// Forks this session, creating a new session with the same messages.
    pub fn fork(&self, up_to_message_id: Option<&str>) -> Result<CortexSession> {
        let mut new_meta = SessionMeta::new(&self.meta.provider, &self.meta.model);
        new_meta.forked_from = Some(self.meta.id.clone());
        new_meta.title = Some(format!("Fork of {}", self.meta.display_title()));

        self.storage
            .fork_session(&self.meta.id, &new_meta, up_to_message_id)?;

        let messages = self.storage.load_messages(&new_meta.id)?;
        new_meta.message_count = messages.len() as u32;

        Ok(CortexSession {
            meta: new_meta,
            messages,
            storage: SessionStorage::new()?,
            modified: false,
        })
    }

    // ========================================================================
    // TITLE GENERATION
    // ========================================================================

    /// Auto-generates a title from the first user message.
    pub fn auto_title(&mut self) {
        if self.meta.title.is_some() {
            return;
        }

        let first_user_message = self.messages.iter().find(|m| m.is_user());
        if let Some(msg) = first_user_message {
            // Take first 50 characters, cut at word boundary
            let content = &msg.content;
            let title = if content.len() <= 50 {
                content.clone()
            } else {
                let truncated = &content[..50];
                if let Some(last_space) = truncated.rfind(' ') {
                    format!("{}...", &truncated[..last_space])
                } else {
                    format!("{}...", truncated)
                }
            };
            self.meta.title = Some(title);
            self.modified = true;
        }
    }

    // ========================================================================
    // STATIC METHODS
    // ========================================================================

    /// Lists all sessions.
    pub fn list_all() -> Result<Vec<SessionSummary>> {
        let storage = SessionStorage::new()?;
        storage.list_sessions()
    }

    /// Lists recent sessions.
    pub fn list_recent(limit: usize) -> Result<Vec<SessionSummary>> {
        let storage = SessionStorage::new()?;
        storage.list_recent_sessions(limit)
    }

    /// Deletes a session.
    pub fn delete(session_id: &str) -> Result<()> {
        let storage = SessionStorage::new()?;
        storage.delete_session(session_id)
    }

    /// Archives a session.
    pub fn archive(session_id: &str) -> Result<()> {
        let storage = SessionStorage::new()?;
        storage.archive_session(session_id)
    }

    /// Checks if a session exists.
    pub fn exists(session_id: &str) -> Result<bool> {
        let storage = SessionStorage::new()?;
        Ok(storage.exists(session_id))
    }

    /// Forks an existing session by ID (static method).
    /// Creates a new session with all messages from the source.
    pub fn fork_from_id(source_session_id: &str) -> Result<CortexSession> {
        let source = Self::load(source_session_id)?;
        source.fork(None)
    }

    /// Loads messages from storage (for resume picker).
    pub fn load_messages(&self) -> Result<Vec<StoredMessage>> {
        self.storage.load_messages(&self.meta.id)
    }
}

// ============================================================
// TOKEN USAGE HELPERS
// ============================================================

impl CortexSession {
    /// Gets total input tokens used.
    pub fn total_input_tokens(&self) -> i64 {
        self.meta.total_input_tokens
    }

    /// Gets total output tokens used.
    pub fn total_output_tokens(&self) -> i64 {
        self.meta.total_output_tokens
    }

    /// Gets total tokens used.
    pub fn total_tokens(&self) -> i64 {
        self.meta.total_tokens()
    }

    /// Formats token usage for display.
    pub fn format_tokens(&self) -> String {
        let total = self.total_tokens();
        if total < 1000 {
            format!("{}", total)
        } else if total < 1_000_000 {
            format!("{:.1}K", total as f64 / 1000.0)
        } else {
            format!("{:.1}M", total as f64 / 1_000_000.0)
        }
    }
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_session() -> (CortexSession, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = SessionStorage::with_dir(temp_dir.path().to_path_buf());
        let session = CortexSession::with_storage("cortex", "test-model", storage).unwrap();
        (session, temp_dir)
    }

    #[test]
    fn test_new_session() {
        let (session, _temp) = create_test_session();
        assert!(!session.id().is_empty());
        assert_eq!(session.meta.provider, "cortex");
        assert_eq!(session.message_count(), 0);
    }

    #[test]
    fn test_add_messages() {
        let (mut session, _temp) = create_test_session();

        session.add_user_message("Hello!");
        assert_eq!(session.message_count(), 1);

        let tokens = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
        };
        session.add_assistant_message("Hi there!", tokens);
        assert_eq!(session.message_count(), 2);
        assert_eq!(session.total_tokens(), 150);
    }

    #[test]
    fn test_messages_for_api() {
        let (mut session, _temp) = create_test_session();

        session.add_user_message("Hello!");
        session.add_assistant_message("Hi!", TokenUsage::default());

        let api_messages = session.messages_for_api();
        assert_eq!(api_messages.len(), 2);
    }

    #[test]
    fn test_auto_title() {
        let (mut session, _temp) = create_test_session();

        session.add_user_message("What is the weather like today in Paris?");
        session.auto_title();

        assert!(session.meta.title.is_some());
        assert!(session.meta.title.as_ref().unwrap().contains("weather"));
    }

    #[test]
    fn test_fork_session() {
        let (mut session, _temp) = create_test_session();

        session.add_user_message("Hello!");
        session.add_assistant_message("Hi!", TokenUsage::default());

        let forked = session.fork(None).unwrap();
        assert_ne!(forked.id(), session.id());
        assert_eq!(forked.message_count(), 2);
        assert!(forked.meta.forked_from.is_some());
    }

    #[test]
    fn test_format_tokens() {
        let (mut session, _temp) = create_test_session();

        session.meta.total_input_tokens = 500;
        session.meta.total_output_tokens = 200;
        assert_eq!(session.format_tokens(), "700");

        session.meta.total_input_tokens = 5000;
        session.meta.total_output_tokens = 2000;
        assert_eq!(session.format_tokens(), "7.0K");

        session.meta.total_input_tokens = 500000;
        session.meta.total_output_tokens = 600000;
        assert_eq!(session.format_tokens(), "1.1M");
    }
}
