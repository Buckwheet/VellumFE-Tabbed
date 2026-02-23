//! Tab bar widget showing all active sessions.
//!
//! Renders a horizontal bar at the top of the screen:
//! [● Buckwheet] [Altchar 3] [+ New]
//!
//! ● = connected indicator
//! 3 = unread message count

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

/// Data for a single tab entry.
pub struct TabEntry<'a> {
    pub label: &'a str,
    pub is_active: bool,
    pub is_connected: bool,
    pub unread: usize,
}

/// The tab bar widget.
pub struct TabBar<'a> {
    pub tabs: Vec<TabEntry<'a>>,
    pub compact: bool,
}

impl<'a> TabBar<'a> {
    pub fn new(tabs: Vec<TabEntry<'a>>) -> Self {
        Self { tabs, compact: false }
    }

    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }
}

impl<'a> Widget for TabBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 { return; }

        let mut x = area.x;
        let y = area.y;

        for tab in &self.tabs {
            let status = if tab.is_connected { "●" } else { "○" };
            let status_color = if tab.is_connected { Color::Green } else { Color::DarkGray };

            let label = if self.compact {
                // Compact: just first char of label + unread dot
                let abbrev: String = tab.label.chars().take(2).collect();
                if tab.unread > 0 {
                    format!(" {}{} ● ", status, abbrev)
                } else {
                    format!(" {}{} ", status, abbrev)
                }
            } else {
                // Full: status + label + unread count
                if tab.unread > 0 {
                    format!(" {} {} {} ", status, tab.label, tab.unread)
                } else {
                    format!(" {} {} ", status, tab.label)
                }
            };

            let (bg, fg) = if tab.is_active {
                (Color::Blue, Color::White)
            } else {
                (Color::DarkGray, Color::Gray)
            };

            let style = Style::default().bg(bg).fg(fg);
            let active_mod = if tab.is_active {
                style.add_modifier(Modifier::BOLD)
            } else {
                style
            };

            // Render each character of the tab label
            for (i, ch) in label.chars().enumerate() {
                let cx = x + i as u16;
                if cx >= area.x + area.width { break; }
                // Color the status dot differently
                let char_style = if ch == '●' || ch == '○' {
                    active_mod.fg(status_color)
                } else {
                    active_mod
                };
                buf.get_mut(cx, y)
                    .set_char(ch)
                    .set_style(char_style);
            }

            x += label.len() as u16;
            if x >= area.x + area.width { break; }

            // Separator between tabs
            if x < area.x + area.width {
                buf.get_mut(x, y)
                    .set_char('│')
                    .set_style(Style::default().fg(Color::DarkGray));
                x += 1;
            }
        }

        // Fill remaining space
        while x < area.x + area.width {
            buf.get_mut(x, y)
                .set_char(' ')
                .set_style(Style::default().bg(Color::Reset));
            x += 1;
        }
    }
}