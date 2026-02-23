//! Per-session state for a single GemStone IV connection.
//!
//! Each Session owns its own connection, parser, config, and UI state.
//! The SessionManager holds a Vec<Session> and routes input/output to the active one.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc;

/// Unique identifier for a session.
pub type SessionId = usize;

/// How this session connects to the game.
#[derive(Debug, Clone)]
pub enum ConnectionMode {
    /// Connect via Lich proxy (host:port)
    LichProxy { host: String, port: u16 },
    /// Connect directly via eAccess SGE
    Direct {
        account: String,
        password: String,
        character: String,
        game_code: String,
    },
}

/// Current connection state of a session.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    /// Not yet connected
    Disconnected,
    /// Attempting to connect
    Connecting,
    /// Connected and receiving data
    Connected,
    /// Connection lost, will retry
    Reconnecting,
    /// Fatal error
    Error(String),
}

/// All state owned by a single game session.
pub struct Session {
    /// Unique ID for this session
    pub id: SessionId,
    /// Display name shown in the tab bar (e.g. character name)
    pub label: String,
    /// How to connect
    pub mode: ConnectionMode,
    /// Current connection status
    pub status: SessionStatus,
    /// Atomic unread counter — incremented by network task when session is not active.
    /// Shared with the spawned network task.
    pub unread: Arc<AtomicUsize>,
    /// Number of unread messages received while this session was not active
    pub unread_count: usize,
    /// Shared atomic: stores the currently active session ID.
    /// Network task checks: if active_session_id.load() != self.id → increment unread.
    pub active_session_id: Arc<AtomicUsize>,
    /// Channel to send commands to the game server
    pub command_tx: Option<mpsc::UnboundedSender<String>>,
    /// Whether sound alerts are enabled for this session
    pub sound_enabled: bool,
    /// Whether TTS is enabled for this session
    pub tts_enabled: bool,
}

impl Session {
    pub fn new(id: SessionId, label: String, mode: ConnectionMode) -> Self {
        Self {
            id,
            label,
            mode,
            status: SessionStatus::Disconnected,
            unread: Arc::new(AtomicUsize::new(0)),
            unread_count: 0,
            active_session_id: Arc::new(AtomicUsize::new(usize::MAX)),
            command_tx: None,
            sound_enabled: true,
            tts_enabled: true,
        }
    }

    /// Send a command to the game server.
    pub fn send_command(&self, cmd: String) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(cmd);
        }
    }

    /// Mark all messages as read (called when session becomes active).
    pub fn clear_unread(&mut self) {
        self.unread_count = 0;
        self.unread.store(0, Ordering::Relaxed);
    }

    /// Increment unread counter (called when message arrives on inactive session).
    pub fn increment_unread(&mut self) {
        self.unread_count = self.unread_count.saturating_add(1);
    }

    /// Sync atomic unread counter into unread_count (call from main loop).
    pub fn sync_unread(&mut self) {
        self.unread_count = self.unread.load(Ordering::Relaxed);
    }

    pub fn is_connected(&self) -> bool {
        self.status == SessionStatus::Connected
    }
}