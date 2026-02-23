//! Session switching key commands.

/// Session command strings for runtime communication.
pub struct SessionCmd;

impl SessionCmd {
    pub fn switch(index: usize) -> String {
        format!("//session:switch:{}", index)
    }
    pub fn next() -> &'static str { "//session:next" }
    pub fn prev() -> &'static str { "//session:prev" }
}