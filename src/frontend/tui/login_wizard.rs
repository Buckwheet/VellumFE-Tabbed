//! Login wizard — TUI flow for adding a new Direct (eAccess) session.
//!
//! Flow:
//!   Step 1: Credentials (account + password)
//!   Step 2: Game selection (fetched from eAccess)
//!   Step 3: Character selection (fetched from eAccess)
//!   → Connects and saves to sessions.toml

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Widget},
};

/// Known GemStone IV game codes.
pub const GAMES: &[(&str, &str)] = &[
    ("GS3", "GemStone IV (Prime)"),
    ("GSX", "GemStone IV (Platinum)"),
    ("GSF", "GemStone IV (Shattered)"),
];

#[derive(Debug, Clone, PartialEq)]
pub enum WizardStep {
    Credentials,
    GameSelect,
    CharSelect,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CredField {
    Account,
    Password,
}

/// Result produced when the wizard completes or is cancelled.
#[derive(Debug, Clone)]
pub enum WizardResult {
    Connect {
        account: String,
        password: String,
        game_code: String,
        character: String,
    },
    Cancel,
}

pub struct LoginWizard {
    pub step: WizardStep,
    // Step 1
    pub account: String,
    pub password: String,
    pub cred_field: CredField,
    // Step 2
    pub game_selected: usize,
    // Step 3
    pub characters: Vec<String>,
    pub char_selected: usize,
    // Error message to display
    pub error: Option<String>,
    /// Produced when wizard finishes
    pub result: Option<WizardResult>,
}

impl LoginWizard {
    pub fn new() -> Self {
        Self {
            step: WizardStep::Credentials,
            account: String::new(),
            password: String::new(),
            cred_field: CredField::Account,
            game_selected: 0,
            characters: Vec::new(),
            char_selected: 0,
            error: None,
            result: None,
        }
    }

    pub fn type_char(&mut self, c: char) {
        match self.step {
            WizardStep::Credentials => match self.cred_field {
                CredField::Account => self.account.push(c),
                CredField::Password => self.password.push(c),
            },
            _ => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.step {
            WizardStep::Credentials => match self.cred_field {
                CredField::Account => {
                    self.account.pop();
                }
                CredField::Password => {
                    self.password.pop();
                }
            },
            _ => {}
        }
    }

    pub fn move_up(&mut self) {
        match self.step {
            WizardStep::Credentials => {
                self.cred_field = CredField::Account;
            }
            WizardStep::GameSelect => {
                if self.game_selected > 0 {
                    self.game_selected -= 1;
                }
            }
            WizardStep::CharSelect => {
                if self.char_selected > 0 {
                    self.char_selected -= 1;
                }
            }
        }
    }

    pub fn move_down(&mut self) {
        match self.step {
            WizardStep::Credentials => {
                self.cred_field = CredField::Password;
            }
            WizardStep::GameSelect => {
                if self.game_selected + 1 < GAMES.len() {
                    self.game_selected += 1;
                }
            }
            WizardStep::CharSelect => {
                if self.char_selected + 1 < self.characters.len() {
                    self.char_selected += 1;
                }
            }
        }
    }

    pub fn tab(&mut self) {
        if self.step == WizardStep::Credentials {
            self.cred_field = match self.cred_field {
                CredField::Account => CredField::Password,
                CredField::Password => CredField::Account,
            };
        }
    }

    /// Confirm current step. Returns true if an async operation is needed (fetch characters).
    /// The caller must call `set_characters` after fetching, then call `confirm` again.
    pub fn confirm(&mut self) -> bool {
        self.error = None;
        match self.step {
            WizardStep::Credentials => {
                if self.account.trim().is_empty() {
                    self.error = Some("Account name is required".to_string());
                    return false;
                }
                if self.password.is_empty() {
                    self.error = Some("Password is required".to_string());
                    return false;
                }
                // Advance to game select
                self.step = WizardStep::GameSelect;
                false
            }
            WizardStep::GameSelect => {
                // Advance to char select — caller must fetch characters
                self.step = WizardStep::CharSelect;
                true // signal: fetch needed
            }
            WizardStep::CharSelect => {
                if self.characters.is_empty() {
                    self.error = Some("No characters available".to_string());
                    return false;
                }
                let character = self.characters[self.char_selected].clone();
                let (game_code, _) = GAMES[self.game_selected];
                self.result = Some(WizardResult::Connect {
                    account: self.account.clone(),
                    password: self.password.clone(),
                    game_code: game_code.to_string(),
                    character,
                });
                false
            }
        }
    }

    pub fn back(&mut self) {
        self.error = None;
        match self.step {
            WizardStep::Credentials => {
                self.result = Some(WizardResult::Cancel);
            }
            WizardStep::GameSelect => {
                self.step = WizardStep::Credentials;
            }
            WizardStep::CharSelect => {
                self.step = WizardStep::GameSelect;
                self.characters.clear();
            }
        }
    }

    pub fn set_characters(&mut self, chars: Vec<String>) {
        self.characters = chars;
        self.char_selected = 0;
    }

    pub fn set_error(&mut self, msg: String) {
        self.error = Some(msg);
        // Go back to credentials on auth error
        self.step = WizardStep::Credentials;
    }

    pub fn selected_game_code(&self) -> &str {
        GAMES
            .get(self.game_selected)
            .map(|(c, _)| *c)
            .unwrap_or("GS3")
    }
}

// ── Rendering ───────────────────────────────────────────────────────────────

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

pub fn render_wizard(wizard: &LoginWizard, area: Rect, buf: &mut Buffer) {
    let popup = centered_rect(56, 16, area);
    Clear.render(popup, buf);

    let title = match wizard.step {
        WizardStep::Credentials => " VellumFE — Connect to GemStone IV ",
        WizardStep::GameSelect => " VellumFE — Connect to GemStone IV ",
        WizardStep::CharSelect => " VellumFE — Connect to GemStone IV ",
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    block.render(popup, buf);

    match wizard.step {
        WizardStep::Credentials => render_credentials(wizard, inner, buf),
        WizardStep::GameSelect => render_list(
            &GAMES.iter().map(|(_, name)| *name).collect::<Vec<_>>(),
            wizard.game_selected,
            inner,
            buf,
        ),
        WizardStep::CharSelect => render_list(
            &wizard
                .characters
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
            wizard.char_selected,
            inner,
            buf,
        ),
    }

    // Error line
    if let Some(err) = &wizard.error {
        let ey = popup.bottom().saturating_sub(2);
        let msg = format!(" ⚠ {}", err);
        buf.set_string(popup.x + 1, ey, &msg, Style::default().fg(Color::Red));
    }

    // Help line
    let hy = popup.bottom().saturating_sub(1);
    let help = match wizard.step {
        WizardStep::Credentials => " Tab=Switch field  Enter=Next  Esc=Cancel",
        WizardStep::GameSelect => " ↑↓=Select  Enter=Next  Esc=Back",
        WizardStep::CharSelect => " ↑↓=Select  Enter=Connect  Esc=Back",
    };
    buf.set_string(popup.x + 1, hy, help, Style::default().fg(Color::DarkGray));
}

fn render_credentials(wizard: &LoginWizard, area: Rect, buf: &mut Buffer) {
    let mut y = area.y + 1;

    let fields: &[(&str, &str, bool)] = &[
        (
            "Account ",
            &wizard.account,
            wizard.cred_field == CredField::Account,
        ),
        (
            "Password",
            &wizard.password,
            wizard.cred_field == CredField::Password,
        ),
    ];

    for (label, value, focused) in fields {
        if y >= area.bottom() {
            break;
        }
        let style = if *focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let display = if *label == "Password" {
            "•".repeat(value.len())
        } else {
            value.to_string()
        };
        let cursor = if *focused { "█" } else { "" };
        let line = format!("  {}: [{}{}]", label, display, cursor);
        buf.set_string(area.x, y, &line, style);
        y += 2;
    }

    // Add key hint footer
    let hint_text = " [Tab] Next field  [Enter] Continue  [Esc] Cancel ";
    let hint_style = Style::default().fg(Color::DarkGray);
    buf.set_string(
        area.x,
        area.bottom().saturating_sub(1),
        hint_text,
        hint_style,
    );
}

fn render_list(items: &[&str], selected: usize, area: Rect, buf: &mut Buffer) {
    let mut y = area.y + 1;
    for (i, item) in items.iter().enumerate() {
        if y >= area.bottom() {
            break;
        }
        let is_sel = i == selected;
        let style = if is_sel {
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let prefix = if is_sel { "▶ " } else { "  " };
        buf.set_string(area.x, y, &format!("{}{}", prefix, item), style);
        y += 1;
    }
}
