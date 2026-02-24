//! Session switching key commands.
//!
//! Key handler returns these strings; runtime intercepts and acts on them.

pub fn is_session_cmd(cmd: &str) -> bool {
    cmd.starts_with("//session:")
}

pub enum SessionCmd {
    SwitchToIndex(usize),
    Next,
    Prev,
    New,
    Close,
    ToggleCompact,
    Broadcast,
    ToggleSound,
    ToggleTts,
}

impl SessionCmd {
    pub fn parse(cmd: &str) -> Option<Self> {
        let payload = cmd.strip_prefix("//session:")?;
        match payload {
            "next" => Some(Self::Next),
            "prev" => Some(Self::Prev),
            "new" => Some(Self::New),
            "close" => Some(Self::Close),
            "compact" => Some(Self::ToggleCompact),
            "broadcast" => Some(Self::Broadcast),
            "sound" => Some(Self::ToggleSound),
            "tts" => Some(Self::ToggleTts),
            s if s.starts_with("switch:") => {
                let idx: usize = s.strip_prefix("switch:")?.parse().ok()?;
                Some(Self::SwitchToIndex(idx))
            }
            _ => None,
        }
    }

    pub fn switch(index: usize) -> String {
        format!("//session:switch:{}", index)
    }
    pub fn next() -> &'static str {
        "//session:next"
    }
    pub fn prev() -> &'static str {
        "//session:prev"
    }
    pub fn new_session() -> &'static str {
        "//session:new"
    }
    pub fn close() -> &'static str {
        "//session:close"
    }
    pub fn compact() -> &'static str {
        "//session:compact"
    }
    pub fn sound() -> &'static str {
        "//session:sound"
    }
    pub fn tts() -> &'static str {
        "//session:tts"
    }
}
