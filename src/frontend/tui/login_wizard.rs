//! Warlock-style multi-profile character picker.
//!
//! Two modes:
//!   List — shows saved profiles, Enter to connect, N/E/D to manage
//!   Edit — form to add or edit a profile

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Widget},
};

pub const GAMES: &[(&str, &str)] = &[
    ("GS3", "GemStone IV (Prime)"),
    ("GSX", "GemStone IV (Platinum)"),
    ("GSF", "GemStone IV (Shattered)"),
    ("DR", "DragonRealms"),
];

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub account: String,
    pub character: String,
    pub game_code: String,
    pub use_lich: bool,
    pub lich_host: Option<String>,
    pub lich_port: Option<u16>,
}

impl Profile {
    pub fn lich_host(&self) -> &str {
        self.lich_host.as_deref().unwrap_or("127.0.0.1")
    }
    pub fn lich_port(&self) -> u16 {
        self.lich_port.unwrap_or(8000)
    }
}

#[derive(Debug, Clone)]
pub enum PickerResult {
    Connect(Profile),
    Quit,
}

#[derive(Debug, Clone, PartialEq)]
enum Mode {
    List,
    Edit,
}

#[derive(Debug, Clone, PartialEq)]
enum EditField {
    Name,
    Account,
    Password,
    Game,
    Character,
    UseLich,
    LichHost,
    LichPort,
}

const EDIT_FIELDS_NO_LICH: &[EditField] = &[
    EditField::Name,
    EditField::Account,
    EditField::Password,
    EditField::Game,
    EditField::Character,
    EditField::UseLich,
];

const EDIT_FIELDS_LICH: &[EditField] = &[
    EditField::Name,
    EditField::Account,
    EditField::Password,
    EditField::Game,
    EditField::Character,
    EditField::UseLich,
    EditField::LichHost,
    EditField::LichPort,
];

pub struct ProfilePicker {
    pub result: Option<PickerResult>,
    profiles: Vec<Profile>,
    selected: usize,
    mode: Mode,
    edit_idx: Option<usize>, // None = new profile
    // edit form state
    field_idx: usize,
    f_name: String,
    f_account: String,
    f_password: String,
    f_game_idx: usize,
    f_character: String,
    f_use_lich: bool,
    f_lich_host: String,
    f_lich_port: String,
    pub characters: Vec<String>,
    pub needs_fetch: bool,
    char_list_open: bool,
    char_list_idx: usize,
    error: Option<String>,
}

impl ProfilePicker {
    pub fn new(profiles: Vec<Profile>) -> Self {
        Self {
            result: None,
            profiles,
            selected: 0,
            mode: Mode::List,
            edit_idx: None,
            field_idx: 0,
            f_name: String::new(),
            f_account: String::new(),
            f_password: String::new(),
            f_game_idx: 0,
            f_character: String::new(),
            f_use_lich: false,
            f_lich_host: "127.0.0.1".to_string(),
            f_lich_port: "8000".to_string(),
            characters: Vec::new(),
            needs_fetch: false,
            char_list_open: false,
            char_list_idx: 0,
            error: None,
        }
    }

    fn fields(&self) -> &[EditField] {
        if self.f_use_lich {
            EDIT_FIELDS_LICH
        } else {
            EDIT_FIELDS_NO_LICH
        }
    }

    fn current_field(&self) -> &EditField {
        let fields = self.fields();
        &fields[self.field_idx.min(fields.len() - 1)]
    }

    pub fn is_list_mode(&self) -> bool {
        self.mode == Mode::List
    }

    pub fn type_char(&mut self, c: char) {
        if self.mode == Mode::List {
            return;
        }
        match self.current_field() {
            EditField::Name => self.f_name.push(c),
            EditField::Account => self.f_account.push(c),
            EditField::Password => self.f_password.push(c),
            EditField::Character => self.f_character.push(c),
            EditField::LichHost => self.f_lich_host.push(c),
            EditField::LichPort => {
                if c.is_ascii_digit() {
                    self.f_lich_port.push(c);
                }
            }
            EditField::Game | EditField::UseLich => {}
        }
    }

    pub fn backspace(&mut self) {
        if self.mode == Mode::List {
            return;
        }
        match self.current_field() {
            EditField::Name => {
                self.f_name.pop();
            }
            EditField::Account => {
                self.f_account.pop();
            }
            EditField::Password => {
                self.f_password.pop();
            }
            EditField::Character => {
                self.f_character.pop();
            }
            EditField::LichHost => {
                self.f_lich_host.pop();
            }
            EditField::LichPort => {
                self.f_lich_port.pop();
            }
            EditField::Game | EditField::UseLich => {}
        }
    }

    pub fn move_up(&mut self) {
        match self.mode {
            Mode::List => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            Mode::Edit => {
                if self.char_list_open {
                    if self.char_list_idx > 0 {
                        self.char_list_idx -= 1;
                    }
                    return;
                }
                if self.field_idx > 0 {
                    self.field_idx -= 1;
                }
            }
        }
    }

    pub fn move_down(&mut self) {
        match self.mode {
            Mode::List => {
                if self.selected + 1 < self.profiles.len() {
                    self.selected += 1;
                }
            }
            Mode::Edit => {
                if self.char_list_open {
                    if self.char_list_idx + 1 < self.characters.len() {
                        self.char_list_idx += 1;
                    }
                    return;
                }
                let max = self.fields().len() - 1;
                if self.field_idx < max {
                    self.field_idx += 1;
                }
            }
        }
    }

    pub fn cycle_game(&mut self, forward: bool) {
        if self.mode != Mode::Edit {
            return;
        }
        match self.current_field() {
            EditField::Game => {
                if forward {
                    self.f_game_idx = (self.f_game_idx + 1) % GAMES.len();
                } else if self.f_game_idx > 0 {
                    self.f_game_idx -= 1;
                } else {
                    self.f_game_idx = GAMES.len() - 1;
                }
            }
            EditField::UseLich => {
                self.f_use_lich = !self.f_use_lich;
            }
            _ => {}
        }
    }

    pub fn tab(&mut self) {
        if self.mode == Mode::Edit {
            let max = self.fields().len() - 1;
            if self.field_idx < max {
                self.field_idx += 1;
            }
        }
    }

    /// Returns true if a character fetch should be triggered.
    pub fn confirm(&mut self) -> bool {
        self.needs_fetch = false;
        match self.mode {
            Mode::List => {
                if self.profiles.is_empty() {
                    return false;
                }
                let p = self.profiles[self.selected].clone();
                self.result = Some(PickerResult::Connect(p));
                false
            }
            Mode::Edit => {
                // If char list is open, confirm the selection
                if self.char_list_open {
                    if let Some(name) = self.characters.get(self.char_list_idx) {
                        self.f_character = name.clone();
                    }
                    self.char_list_open = false;
                    return false;
                }
                // On Character field: open list if available, else trigger fetch
                if *self.current_field() == EditField::Character {
                    if !self.characters.is_empty() {
                        // pre-select current character in list
                        self.char_list_idx = self
                            .characters
                            .iter()
                            .position(|c| c == &self.f_character)
                            .unwrap_or(0);
                        self.char_list_open = true;
                        return false;
                    }
                    if !self.f_account.is_empty() && !self.f_password.is_empty() {
                        self.needs_fetch = true;
                        return true;
                    }
                }
                // On last field, save profile
                let last = self.fields().len() - 1;
                if self.field_idx == last {
                    self.save_edit();
                    return false;
                }
                self.field_idx += 1;
                false
            }
        }
    }

    fn save_edit(&mut self) {
        if self.f_name.is_empty() || self.f_account.is_empty() || self.f_character.is_empty() {
            self.error = Some("Name, account, and character are required.".to_string());
            return;
        }
        let profile = Profile {
            name: self.f_name.clone(),
            account: self.f_account.clone(),
            character: self.f_character.clone(),
            game_code: GAMES[self.f_game_idx].0.to_string(),
            use_lich: self.f_use_lich,
            lich_host: if self.f_use_lich && !self.f_lich_host.is_empty() {
                Some(self.f_lich_host.clone())
            } else {
                None
            },
            lich_port: if self.f_use_lich {
                self.f_lich_port.parse().ok()
            } else {
                None
            },
        };
        if let Some(idx) = self.edit_idx {
            self.profiles[idx] = profile;
        } else {
            self.profiles.push(profile);
            self.selected = self.profiles.len() - 1;
        }
        self.mode = Mode::List;
        self.error = None;
    }

    pub fn back(&mut self) {
        if self.char_list_open {
            self.char_list_open = false;
            return;
        }
        match self.mode {
            Mode::Edit => {
                self.mode = Mode::List;
                self.error = None;
            }
            Mode::List => {
                self.result = Some(PickerResult::Quit);
            }
        }
    }

    pub fn new_profile(&mut self) {
        self.edit_idx = None;
        self.field_idx = 0;
        self.f_name.clear();
        self.f_account.clear();
        self.f_password.clear();
        self.f_game_idx = 0;
        self.f_character.clear();
        self.f_use_lich = false;
        self.f_lich_host = "127.0.0.1".to_string();
        self.f_lich_port = "8000".to_string();
        self.characters.clear();
        self.char_list_open = false;
        self.char_list_idx = 0;
        self.error = None;
        self.mode = Mode::Edit;
    }

    pub fn edit_selected(&mut self) {
        if self.profiles.is_empty() {
            return;
        }
        let p = self.profiles[self.selected].clone();
        self.edit_idx = Some(self.selected);
        self.field_idx = 0;
        self.f_name = p.name;
        self.f_account = p.account;
        self.f_password = String::new(); // don't pre-fill password
        self.f_game_idx = GAMES
            .iter()
            .position(|(code, _)| *code == p.game_code)
            .unwrap_or(0);
        self.f_character = p.character;
        self.f_use_lich = p.use_lich;
        self.f_lich_host = p.lich_host.unwrap_or_else(|| "127.0.0.1".to_string());
        self.f_lich_port = p.lich_port.unwrap_or(8000).to_string();
        self.characters.clear();
        self.char_list_open = false;
        self.char_list_idx = 0;
        self.error = None;
        self.mode = Mode::Edit;
    }

    pub fn delete_selected(&mut self) {
        if self.profiles.is_empty() {
            return;
        }
        self.profiles.remove(self.selected);
        if self.selected > 0 && self.selected >= self.profiles.len() {
            self.selected -= 1;
        }
    }

    pub fn set_characters(&mut self, chars: Vec<String>) {
        self.characters = chars;
        self.char_list_open = false;
        self.char_list_idx = 0;
        if self.characters.len() == 1 {
            self.f_character = self.characters[0].clone();
        } else if self.f_character.is_empty() {
            if let Some(first) = self.characters.first() {
                self.f_character = first.clone();
            }
        }
        self.needs_fetch = false;
    }

    pub fn set_error(&mut self, msg: String) {
        self.error = Some(msg);
        self.needs_fetch = false;
    }

    pub fn needs_fetch(&self) -> bool {
        self.needs_fetch
    }

    pub fn get_fetch_params(&self) -> Option<(String, String, String)> {
        if self.f_account.is_empty() || self.f_password.is_empty() {
            return None;
        }
        Some((
            self.f_account.clone(),
            self.f_password.clone(),
            GAMES[self.f_game_idx].0.to_string(),
        ))
    }

    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }
}

// ─── Rendering ───────────────────────────────────────────────────────────────

pub fn render_picker(picker: &ProfilePicker, area: Rect, buf: &mut Buffer) {
    let width: u16 = 64.min(area.width.saturating_sub(4));
    let height: u16 = if picker.mode == Mode::Edit { 18 } else { 14 };
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect {
        x,
        y,
        width,
        height,
    };

    // Clear background
    Clear.render(popup, buf);

    let block = Block::default()
        .title(" VellumFE — Character Select ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    block.render(popup, buf);

    let inner = Rect {
        x: popup.x + 2,
        y: popup.y + 1,
        width: popup.width.saturating_sub(4),
        height: popup.height.saturating_sub(2),
    };

    match picker.mode {
        Mode::List => render_list(picker, inner, buf),
        Mode::Edit => render_edit(picker, inner, buf),
    }
}

fn render_list(picker: &ProfilePicker, area: Rect, buf: &mut Buffer) {
    let header = Line::from(vec![Span::styled(
        "Saved Characters",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]);
    buf.set_line(area.x, area.y, &header, area.width);

    let divider = "─".repeat(area.width as usize);
    buf.set_string(
        area.x,
        area.y + 1,
        &divider,
        Style::default().fg(Color::DarkGray),
    );

    if picker.profiles.is_empty() {
        let msg = Line::from(Span::styled(
            "No profiles saved. Press N to add one.",
            Style::default().fg(Color::DarkGray),
        ));
        buf.set_line(area.x, area.y + 3, &msg, area.width);
    } else {
        for (i, p) in picker.profiles.iter().enumerate() {
            let row_y = area.y + 2 + i as u16;
            if row_y >= area.y + area.height.saturating_sub(3) {
                break;
            }
            let selected = i == picker.selected;
            let arrow = if selected { "▶ " } else { "  " };
            let conn_type = if p.use_lich {
                format!("Lich :{}", p.lich_port())
            } else {
                "Direct".to_string()
            };
            let label = format!(
                "{}{:<24} {:<14} {:<5} {}",
                arrow, p.name, p.account, p.game_code, conn_type
            );
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            buf.set_string(area.x, row_y, &label, style);
        }
    }

    // Footer
    let footer_y = area.y + area.height.saturating_sub(2);
    let footer = Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(Color::Green)),
        Span::raw(" Connect  "),
        Span::styled("[N]", Style::default().fg(Color::Yellow)),
        Span::raw(" New  "),
        Span::styled("[E]", Style::default().fg(Color::Yellow)),
        Span::raw(" Edit  "),
        Span::styled("[D]", Style::default().fg(Color::Red)),
        Span::raw(" Delete  "),
        Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
        Span::raw(" Quit"),
    ]);
    buf.set_line(area.x, footer_y, &footer, area.width);
}

fn render_edit(picker: &ProfilePicker, area: Rect, buf: &mut Buffer) {
    let title = if picker.edit_idx.is_some() {
        "Edit Profile"
    } else {
        "New Profile"
    };
    let header = Line::from(Span::styled(
        title,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ));
    buf.set_line(area.x, area.y, &header, area.width);

    let fields = picker.fields();
    let active_field = &fields[picker.field_idx.min(fields.len() - 1)];

    let rows: &[(&str, String)] = &[
        ("Profile Name", picker.f_name.clone()),
        ("Account", picker.f_account.clone()),
        ("Password", "●".repeat(picker.f_password.len())),
        (
            "Game",
            format!(
                "< {} >  {}",
                GAMES[picker.f_game_idx].0, GAMES[picker.f_game_idx].1
            ),
        ),
        (
            "Character",
            if picker.characters.is_empty() {
                if picker.f_character.is_empty() {
                    "[Enter to fetch]".to_string()
                } else {
                    picker.f_character.clone()
                }
            } else if picker.f_character.is_empty() {
                "[Enter to pick]".to_string()
            } else {
                format!("{}  [Enter to change]", picker.f_character)
            },
        ),
        (
            "Use Lich",
            if picker.f_use_lich { "Yes" } else { "No" }.to_string(),
        ),
    ];

    let lich_rows: &[(&str, String)] = &[
        ("Lich Host", picker.f_lich_host.clone()),
        ("Lich Port", picker.f_lich_port.clone()),
    ];

    let all_rows: Vec<(&str, String, &EditField)> = rows
        .iter()
        .zip(EDIT_FIELDS_NO_LICH.iter())
        .map(|((label, val), field)| (*label, val.clone(), field))
        .chain(if picker.f_use_lich {
            lich_rows
                .iter()
                .zip([EditField::LichHost, EditField::LichPort].iter())
                .map(|((label, val), field)| (*label, val.clone(), field))
                .collect::<Vec<_>>()
        } else {
            vec![]
        })
        .collect();

    for (i, (label, value, field)) in all_rows.iter().enumerate() {
        let row_y = area.y + 2 + i as u16;
        if row_y >= area.y + area.height.saturating_sub(3) {
            break;
        }
        let is_active = std::ptr::eq(*field as *const EditField, active_field as *const EditField)
            || *field == active_field;
        let label_style = if is_active {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let value_style = if is_active {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        let prefix = if is_active { "▶ " } else { "  " };
        buf.set_string(area.x, row_y, prefix, label_style);
        buf.set_string(area.x + 2, row_y, &format!("{:<14}", label), label_style);
        buf.set_string(area.x + 16, row_y, value, value_style);
    }

    // Inline character dropdown
    if picker.char_list_open && !picker.characters.is_empty() {
        // find the row_y of the Character field
        let char_field_row = all_rows
            .iter()
            .position(|(_, _, f)| **f == EditField::Character)
            .unwrap_or(4) as u16;
        let drop_y = area.y + 2 + char_field_row + 1;
        let drop_x = area.x + 16;
        let drop_w = area.width.saturating_sub(16);
        for (i, name) in picker.characters.iter().enumerate() {
            let y = drop_y + i as u16;
            if y >= area.y + area.height.saturating_sub(3) {
                break;
            }
            let selected = i == picker.char_list_idx;
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            };
            let label = format!(
                "{}{:<width$}",
                if selected { "▶ " } else { "  " },
                name,
                width = drop_w.saturating_sub(2) as usize
            );
            buf.set_string(drop_x, y, &label, style);
        }
    }

    // Error or hint
    let hint_y = area.y + area.height.saturating_sub(2);
    if let Some(ref err) = picker.error {
        let err_line = Line::from(Span::styled(err.as_str(), Style::default().fg(Color::Red)));
        buf.set_line(area.x, hint_y, &err_line, area.width);
    } else {
        let hint = Line::from(vec![
            Span::styled("[Tab/Enter]", Style::default().fg(Color::Green)),
            Span::raw(" Next  "),
            Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
            Span::raw(" Back"),
        ]);
        buf.set_line(area.x, hint_y, &hint, area.width);
    }
}
