//! Manages all active sessions and tracks which one is currently focused.

use crate::session::{ConnectionMode, Session, SessionId};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct SessionManager {
    sessions: Vec<Session>,
    active_id: Option<SessionId>,
    next_id: SessionId,
    /// Shared atomic storing the currently active session ID.
    /// Cloned into each Session so network tasks can check without locking.
    pub active_session_id: Arc<AtomicUsize>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            active_id: None,
            next_id: 0,
            active_session_id: Arc::new(AtomicUsize::new(usize::MAX)),
        }
    }

    /// Add a new session and return its ID.
    pub fn add(&mut self, label: String, mode: ConnectionMode) -> SessionId {
        let id = self.next_id;
        self.next_id += 1;
        let mut session = Session::new(id, label, mode);
        // Share the manager-level active_session_id so network tasks can check it
        session.active_session_id = self.active_session_id.clone();
        self.sessions.push(session);
        if self.active_id.is_none() {
            self.active_id = Some(id);
            self.active_session_id.store(id, Ordering::Relaxed);
        }
        id
    }

    /// Remove a session by ID.
    pub fn remove(&mut self, id: SessionId) {
        self.sessions.retain(|s| s.id != id);
        if self.active_id == Some(id) {
            self.active_id = self.sessions.first().map(|s| s.id);
        }
    }

    /// Get the currently active session.
    pub fn active(&self) -> Option<&Session> {
        self.active_id.and_then(|id| self.get(id))
    }

    /// Get the currently active session mutably.
    pub fn active_mut(&mut self) -> Option<&mut Session> {
        let id = self.active_id?;
        self.get_mut(id)
    }

    /// Switch focus to a session by ID.
    pub fn set_active(&mut self, id: SessionId) {
        if self.sessions.iter().any(|s| s.id == id) {
            self.active_id = Some(id);
            self.active_session_id.store(id, Ordering::Relaxed);
            if let Some(s) = self.get_mut(id) {
                s.clear_unread();
            }
        }
    }

    /// Switch to session by 1-based index (for Ctrl+1..9).
    pub fn set_active_by_index(&mut self, index: usize) {
        if let Some(id) = self.sessions.get(index.saturating_sub(1)).map(|s| s.id) {
            self.set_active(id);
        }
    }

    /// Switch to next session.
    pub fn next(&mut self) {
        let len = self.sessions.len();
        if len < 2 {
            return;
        }
        if let Some(pos) = self.active_pos() {
            let next_id = self.sessions[(pos + 1) % len].id;
            self.set_active(next_id);
        }
    }

    /// Switch to previous session.
    pub fn prev(&mut self) {
        let len = self.sessions.len();
        if len < 2 {
            return;
        }
        if let Some(pos) = self.active_pos() {
            let prev_id = self.sessions[(pos + len - 1) % len].id;
            self.set_active(prev_id);
        }
    }

    pub fn get(&self, id: SessionId) -> Option<&Session> {
        self.sessions.iter().find(|s| s.id == id)
    }

    pub fn get_mut(&mut self, id: SessionId) -> Option<&mut Session> {
        self.sessions.iter_mut().find(|s| s.id == id)
    }

    pub fn all(&self) -> &[Session] {
        &self.sessions
    }

    pub fn all_mut(&mut self) -> &mut [Session] {
        &mut self.sessions
    }

    /// Sync atomic unread counters into unread_count for all sessions.
    /// Call this from the main loop each tick for inactive sessions.
    pub fn sync_unread_all(&mut self) {
        let active_id = self.active_id;
        for s in &mut self.sessions {
            if Some(s.id) != active_id {
                s.sync_unread();
            }
        }
    }

    /// Send a command to all connected sessions.
    pub fn broadcast(&self, cmd: &str) {
        for session in &self.sessions {
            session.send_command(cmd.to_string());
        }
    }

    /// Send a command to all connected sessions except the active one.
    pub fn broadcast_others(&self, cmd: &str) {
        let active_id = self.active_id;
        for session in &self.sessions {
            if Some(session.id) != active_id {
                session.send_command(cmd.to_string());
            }
        }
    }

    pub fn count(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    fn active_pos(&self) -> Option<usize> {
        let id = self.active_id?;
        self.sessions.iter().position(|s| s.id == id)
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
