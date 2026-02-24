//! Tab bar widget showing all active sessions.
//!
//! Renders a horizontal bar at the top of the screen:
//! [● Buckwheet] [… Altchar 3] [! Mule]
//!
//! ● = connected, ○ = disconnected, … = connecting, ↻ = reconnecting, ! = error
//! 3 = unread message count

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

/// Data for a single tab entry.
pub struct TabEntry<'a> {
    pub label: &'a str,
    pub is_active: bool,
    /// Status symbol: "●" "○" "…" "↻" "!"
    pub status: &'a str,
    pub unread: usize,
    pub sound_enabled: bool,
    pub tts_enabled: bool,
}

/// The tab bar widget.
pub struct TabBar<'a> {
    pub tabs: Vec<TabEntry<'a>>,
    pub compact: bool,
}

impl<'a> TabBar<'a> {
    pub fn new(tabs: Vec<TabEntry<'a>>) -> Self {
        Self {
            tabs,
            compact: false,
        }
    }

    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }
}

fn status_color(sym: &str) -> Color {
    match sym {
        "●" => Color::Green,
        "…" => Color::Yellow,
        "↻" => Color::Cyan,
        "!" => Color::Red,
        _ => Color::DarkGray,
    }
}

impl<'a> Widget for TabBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }

        let mut x = area.x;
        let y = area.y;

        for tab in &self.tabs {
            let sym = tab.status;
            let sym_color = status_color(sym);

            let label = if self.compact {
                let abbrev: String = tab.label.chars().take(2).collect();
                let mute = if !tab.sound_enabled { "🔇" } else { "" };
                let tts = if !tab.tts_enabled { "🔕" } else { "" };
                if tab.unread > 0 {
                    format!(" {}{}{}{} ● ", sym, abbrev, mute, tts)
                } else {
                    format!(" {}{}{}{} ", sym, abbrev, mute, tts)
                }
            } else {
                let mute = if !tab.sound_enabled { " 🔇" } else { "" };
                let tts = if !tab.tts_enabled { " 🔕" } else { "" };
                if tab.unread > 0 {
                    format!(" {} {}{}{} {} ", sym, tab.label, mute, tts, tab.unread)
                } else {
                    format!(" {} {}{}{} ", sym, tab.label, mute, tts)
                }
            };

            let (bg, fg) = if tab.is_active {
                (Color::Blue, Color::White)
            } else {
                (Color::DarkGray, Color::Gray)
            };

            let base_style = Style::default().bg(bg).fg(fg);
            let active_style = if tab.is_active {
                base_style.add_modifier(Modifier::BOLD)
            } else {
                base_style
            };

            for (i, ch) in label.chars().enumerate() {
                let cx = x + i as u16;
                if cx >= area.x + area.width {
                    break;
                }
                // Color the status symbol distinctly
                let char_style = if label.chars().nth(i).map_or(false, |_| {
                    // first non-space char is the status symbol
                    i == 1
                }) {
                    active_style.fg(sym_color)
                } else {
                    active_style
                };
                buf.cell_mut((cx, y))
                    .unwrap()
                    .set_char(ch)
                    .set_style(char_style);
            }

            x += label.chars().count() as u16;
            if x >= area.x + area.width {
                break;
            }

            if x < area.x + area.width {
                buf.cell_mut((x, y))
                    .unwrap()
                    .set_char('│')
                    .set_style(Style::default().fg(Color::DarkGray));
                x += 1;
            }
        }

        // Fill remaining space
        while x < area.x + area.width {
            buf.cell_mut((x, y))
                .unwrap()
                .set_char(' ')
                .set_style(Style::default().bg(Color::Reset));
            x += 1;
        }
    }
}
