//! Session picker screen — shown on first run or when no sessions are configured.
//!
//! Layout:
//!   ┌─ Sessions ──────────────────────────────────────────┐
//!   │  [Buckwheet]  Lich  localhost:8000                  │
//!   │  [Altchar]    Lich  localhost:8001                  │
//!   │  ──────────────────────────────────────────────     │
//!   │  [+ Add Session]                                    │
//!   │  [Connect]  [Remove]                                │
//!   └─────────────────────────────────────────────────────┘
//!
//! Add Session form (inline):
//!   Mode: [Lich Proxy ▼]
//!   Label:  [____________]
//!   Host:   [localhost___]
//!   Port:   [8000________]
//!   [Save]  [Cancel]

use crate::sessions_config::{SessionEntry, SessionModeConfig, SessionsConfig};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

/// Which element has keyboard focus in the picker.
#[derive(Debug, Clone, PartialEq)]
pub enum PickerFocus {
    /// Navigating the session list (index into sessions + "Add" row)
    List,
    /// Filling in the add-session form
    Form,
}

/// Which field in the add-session form is focused.
#[derive(Debug, Clone, PartialEq)]
pub enum FormField {
    Label,
    Host,
    Port,
}

/// State for the add-session form.
#[derive(Debug, Default, Clone)]
pub struct AddForm {
    pub label: String,
    pub host: String,
    pub port: String,
    pub focused_field: Option<FormField>,
    /// true = Lich, false = Direct (Direct is Phase 4)
    pub lich_mode: bool,
}

impl AddForm {
    pub fn new() -> Self {
        Self {
            label: String::new(),
            host: "localhost".to_string(),
            port: "8000".to_string(),
            focused_field: Some(FormField::Label),
            lich_mode: true,
        }
    }

    pub fn focused_field(&self) -> &Option<FormField> {
        &self.focused_field
    }

    pub fn type_char(&mut self, c: char) {
        match &self.focused_field {
            Some(FormField::Label) => self.label.push(c),
            Some(FormField::Host) => self.host.push(c),
            Some(FormField::Port) => {
                if c.is_ascii_digit() {
                    self.port.push(c);
                }
            }
            None => {}
        }
    }

    pub fn backspace(&mut self) {
        match &self.focused_field {
            Some(FormField::Label) => {
                self.label.pop();
            }
            Some(FormField::Host) => {
                self.host.pop();
            }
            Some(FormField::Port) => {
                self.port.pop();
            }
            None => {}
        }
    }

    pub fn next_field(&mut self) {
        self.focused_field = match &self.focused_field {
            Some(FormField::Label) => Some(FormField::Host),
            Some(FormField::Host) => Some(FormField::Port),
            Some(FormField::Port) | None => Some(FormField::Label),
        };
    }

    pub fn prev_field(&mut self) {
        self.focused_field = match &self.focused_field {
            Some(FormField::Label) => Some(FormField::Port),
            Some(FormField::Host) => Some(FormField::Label),
            Some(FormField::Port) | None => Some(FormField::Host),
        };
    }

    /// Validate and build a SessionEntry. Returns None if invalid.
    pub fn to_entry(&self) -> Option<SessionEntry> {
        let label = self.label.trim().to_string();
        if label.is_empty() {
            return None;
        }
        let port: u16 = self.port.trim().parse().ok()?;
        Some(SessionEntry {
            label,
            mode: SessionModeConfig::Lich,
            host: Some(self.host.trim().to_string()),
            port: Some(port),
            account: None,
            character: None,
            game_code: None,
            auto_connect: false,
        })
    }
}

/// The session picker screen state.
pub struct SessionPicker {
    /// Loaded session list (mirrors SessionsConfig)
    pub sessions: Vec<SessionEntry>,
    /// Currently highlighted row (0..sessions.len() = session, sessions.len() = "Add" row)
    pub selected: usize,
    pub focus: PickerFocus,
    pub form: Option<AddForm>,
    /// Action requested by the picker (consumed by runtime)
    pub action: Option<PickerAction>,
}

/// Actions the picker can request from the runtime.
#[derive(Debug, Clone)]
pub enum PickerAction {
    /// Connect to the session at this index in the list
    Connect(usize),
    /// Remove the session at this index
    Remove(usize),
    /// Save a new Lich session entry
    AddSession(SessionEntry),
    /// Open the Direct login wizard
    OpenWizard,
    /// User pressed Escape with no sessions — quit
    Quit,
}

impl SessionPicker {
    pub fn new(config: &SessionsConfig) -> Self {
        Self {
            sessions: config.sessions.clone(),
            selected: 0,
            focus: PickerFocus::List,
            form: None,
            action: None,
        }
    }

    /// Returns true if the picker should remain visible (no sessions connected yet).
    pub fn is_active(&self) -> bool {
        self.action.is_none()
    }

    // ── Navigation ──────────────────────────────────────────────────────────

    pub fn move_up(&mut self) {
        if self.focus == PickerFocus::List {
            let rows = self.sessions.len() + 1; // +1 for "Add" row
            if rows > 0 {
                self.selected = (self.selected + rows - 1) % rows;
            }
        } else if let Some(form) = &mut self.form {
            form.prev_field();
        }
    }

    pub fn move_down(&mut self) {
        if self.focus == PickerFocus::List {
            let rows = self.sessions.len() + 1;
            if rows > 0 {
                self.selected = (self.selected + 1) % rows;
            }
        } else if let Some(form) = &mut self.form {
            form.next_field();
        }
    }

    pub fn confirm(&mut self) {
        if self.focus == PickerFocus::Form {
            // Save form
            if let Some(form) = &self.form {
                if form.lich_mode {
                    if let Some(entry) = form.to_entry() {
                        self.action = Some(PickerAction::AddSession(entry));
                    }
                } else {
                    // Direct mode — open the full wizard
                    self.action = Some(PickerAction::OpenWizard);
                }
            }
            return;
        }

        let add_row = self.sessions.len();
        if self.selected == add_row {
            // Open add form
            self.focus = PickerFocus::Form;
            self.form = Some(AddForm::new());
        } else {
            // Connect to selected session
            self.action = Some(PickerAction::Connect(self.selected));
        }
    }

    pub fn remove_selected(&mut self) {
        if self.focus == PickerFocus::List && self.selected < self.sessions.len() {
            self.action = Some(PickerAction::Remove(self.selected));
        }
    }

    pub fn cancel_form(&mut self) {
        if self.focus == PickerFocus::Form {
            self.focus = PickerFocus::List;
            self.form = None;
        } else if self.sessions.is_empty() {
            self.action = Some(PickerAction::Quit);
        }
    }

    pub fn type_char(&mut self, c: char) {
        if let Some(form) = &mut self.form {
            form.type_char(c);
        }
    }

    pub fn backspace(&mut self) {
        if let Some(form) = &mut self.form {
            form.backspace();
        }
    }

    pub fn tab_field(&mut self) {
        if let Some(form) = &mut self.form {
            form.next_field();
        }
    }

    /// Toggle Lich/Direct mode in the add form (F2)
    pub fn toggle_mode(&mut self) {
        if let Some(form) = &mut self.form {
            form.lich_mode = !form.lich_mode;
        }
    }
}

// ── Rendering ───────────────────────────────────────────────────────────────

const PICKER_WIDTH: u16 = 60;
const PICKER_HEIGHT: u16 = 20;

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

/// Render the session picker as a centered overlay.
pub fn render_picker(picker: &SessionPicker, area: Rect, buf: &mut Buffer) {
    let popup = centered_rect(PICKER_WIDTH, PICKER_HEIGHT, area);

    // Clear background
    Clear.render(popup, buf);

    let block = Block::default()
        .title(" Sessions ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    block.render(popup, buf);

    if picker.focus == PickerFocus::Form {
        if let Some(form) = &picker.form {
            render_form(form, inner, buf);
            return;
        }
    }

    render_list(picker, inner, buf);
}

fn render_list(picker: &SessionPicker, area: Rect, buf: &mut Buffer) {
    let mut y = area.y;

    // Header
    let header = Line::from(vec![
        Span::styled("  Label", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("                Mode    Address"),
    ]);
    if y < area.bottom() {
        buf.set_line(area.x, y, &header, area.width);
        y += 1;
    }

    // Separator
    if y < area.bottom() {
        let sep = "─".repeat(area.width as usize);
        buf.set_string(area.x, y, &sep, Style::default().fg(Color::DarkGray));
        y += 1;
    }

    // Session rows
    for (i, session) in picker.sessions.iter().enumerate() {
        if y >= area.bottom() {
            break;
        }
        let is_selected = picker.selected == i;
        let style = if is_selected {
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let mode_str = match session.mode {
            SessionModeConfig::Lich => "Lich",
            SessionModeConfig::Direct => "Direct",
        };
        let addr = match (&session.host, session.port) {
            (Some(h), Some(p)) => format!("{}:{}", h, p),
            _ => "—".to_string(),
        };
        let prefix = if is_selected { "▶ " } else { "  " };
        let row = format!("{}{:<20} {:<8} {}", prefix, &session.label, mode_str, addr);
        buf.set_string(area.x, y, &row, style);
        y += 1;
    }

    // Separator before Add row
    if y < area.bottom() {
        let sep = "─".repeat(area.width as usize);
        buf.set_string(area.x, y, &sep, Style::default().fg(Color::DarkGray));
        y += 1;
    }

    // "Add Session" row
    if y < area.bottom() {
        let add_row = picker.sessions.len();
        let is_selected = picker.selected == add_row;
        let style = if is_selected {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let prefix = if is_selected { "▶ " } else { "  " };
        buf.set_string(area.x, y, &format!("{}[+ Add Session]", prefix), style);
        y += 1;
    }

    // Help line at bottom
    let help_y = area.bottom().saturating_sub(1);
    if help_y > y {
        let help = " Enter=Connect  Del=Remove  Esc=Quit";
        buf.set_string(area.x, help_y, help, Style::default().fg(Color::DarkGray));
    }
}

fn render_form(form: &AddForm, area: Rect, buf: &mut Buffer) {
    let mut y = area.y;

    let mode_label = if form.lich_mode {
        "[Lich Proxy]  Direct     "
    } else {
        " Lich Proxy  [Direct]    "
    };
    let title = Line::from(vec![
        Span::styled(" Mode: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(mode_label, Style::default().fg(Color::Cyan)),
        Span::styled(" F2=toggle", Style::default().fg(Color::DarkGray)),
    ]);
    if y < area.bottom() {
        buf.set_line(area.x, y, &title, area.width);
        y += 2;
    }

    if form.lich_mode {
        let fields: &[(&str, &str, bool)] = &[
            (
                "Label",
                &form.label,
                form.focused_field == Some(FormField::Label),
            ),
            (
                "Host ",
                &form.host,
                form.focused_field == Some(FormField::Host),
            ),
            (
                "Port ",
                &form.port,
                form.focused_field == Some(FormField::Port),
            ),
        ];
        for (label, value, focused) in fields {
            if y >= area.bottom() {
                break;
            }
            let field_style = if *focused {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let cursor = if *focused { "█" } else { "" };
            buf.set_string(
                area.x,
                y,
                &format!("  {}: [{}{}]", label, value, cursor),
                field_style,
            );
            y += 1;
        }
        y += 1;
        if y < area.bottom() {
            buf.set_string(
                area.x,
                y,
                "  Tab=Next field  Enter=Save  Esc=Cancel",
                Style::default().fg(Color::DarkGray),
            );
        }
    } else {
        if y < area.bottom() {
            buf.set_string(
                area.x,
                y,
                "  Press Enter to open the login wizard.",
                Style::default().fg(Color::Gray),
            );
            y += 1;
        }
        if y < area.bottom() {
            buf.set_string(
                area.x,
                y,
                "  (account → game → character)",
                Style::default().fg(Color::DarkGray),
            );
            y += 2;
        }
        if y < area.bottom() {
            buf.set_string(
                area.x,
                y,
                "  Enter=Open wizard  Esc=Cancel",
                Style::default().fg(Color::DarkGray),
            );
        }
    }
}
