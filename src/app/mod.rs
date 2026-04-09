use crate::config::settings::{AppConfig, Theme, ThemeColors};
use crate::models::{Group, HostsDatabase, SshHost};
use crate::tui::views::ViewMode;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Groups,
    Hosts,
    Details,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Insert,
    Search,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Alphabetical,
    Recent,
    FavoritesFirst,
    MostConnected,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    Quit,
    Resize(u16, u16),
    Key(crossterm::event::KeyEvent),
    Tick,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppCommand {
    None,
    Connect,
    AddHost,
    EditHost,
    DeleteHost,
    AddGroup,
    DeleteGroup,
    ToggleFavorite,
    CopyHostname,
    CopySshCommand,
    ImportSshConfig,
    ExportSshConfig,
    ToggleTheme,
    ShowHelp,
}

#[derive(Debug, Clone, Default)]
pub struct HostFormData {
    pub name: String,
    pub hostname: String,
    pub username: String,
    pub port: String,
    pub identity_file: String,
    pub proxy_jump: String,
    pub tags: String,
    pub notes: String,
    pub is_favorite: bool,
}

impl HostFormData {
    pub fn from_host(host: &SshHost) -> Self {
        Self {
            name: host.name.clone(),
            hostname: host.hostname.clone(),
            username: host.username.clone(),
            port: host.port.to_string(),
            identity_file: host.identity_file.clone().unwrap_or_default(),
            proxy_jump: host.proxy_jump.clone().unwrap_or_default(),
            tags: host.tags.join(","),
            notes: host.notes.clone().unwrap_or_default(),
            is_favorite: host.is_favorite,
        }
    }

    pub fn to_host(&self, id: Option<String>) -> SshHost {
        SshHost {
            id: id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: self.name.clone(),
            hostname: self.hostname.clone(),
            username: self.username.clone(),
            port: self.port.parse().unwrap_or(22),
            identity_file: if self.identity_file.is_empty() {
                None
            } else {
                Some(self.identity_file.clone())
            },
            password: None,
            tags: self
                .tags
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            notes: if self.notes.is_empty() {
                None
            } else {
                Some(self.notes.clone())
            },
            proxy_jump: if self.proxy_jump.is_empty() {
                None
            } else {
                Some(self.proxy_jump.clone())
            },
            env_vars: HashMap::new(),
            is_favorite: self.is_favorite,
            connect_count: 0,
            last_connected: None,
        }
    }
}

pub struct AppState {
    pub db: HostsDatabase,
    pub filtered_hosts: Vec<SshHost>,
    pub groups: Vec<Group>,
    pub favorite_hosts: Vec<SshHost>,
    pub recent_hosts: Vec<SshHost>,

    pub active_panel: Panel,
    pub input_mode: InputMode,
    pub sort_mode: SortMode,
    pub filter_mode: FilterMode,

    pub selected_group_index: usize,
    pub filtered_index: usize,

    pub search_query: String,
    pub command_buffer: String,

    pub view_mode: ViewMode,
    pub form_data: HostFormData,
    pub form_field_index: usize,

    pub last_command: AppCommand,
    pub message: Option<String>,

    pub should_quit: bool,

    pub config: AppConfig,
    pub theme_colors: ThemeColors,

    fuzzy_matcher: SkimMatcherV2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterMode {
    #[default]
    All,
    Favorites,
    Recent,
    Group(usize),
}

impl AppState {
    pub fn new(db: HostsDatabase) -> Self {
        let config = AppConfig::load().unwrap_or_default();
        let theme_colors = ThemeColors::from_theme(config.theme);

        let mut state = Self {
            db: db.clone(),
            filtered_hosts: Vec::new(),
            groups: db.groups.clone(),
            favorite_hosts: Vec::new(),
            recent_hosts: Vec::new(),
            active_panel: Panel::Hosts,
            input_mode: InputMode::Normal,
            sort_mode: SortMode::Alphabetical,
            filter_mode: FilterMode::All,
            selected_group_index: 0,
            filtered_index: 0,
            search_query: String::new(),
            command_buffer: String::new(),
            view_mode: ViewMode::Normal,
            form_data: HostFormData::default(),
            form_field_index: 0,
            last_command: AppCommand::None,
            message: None,
            should_quit: false,
            config,
            theme_colors,
            fuzzy_matcher: SkimMatcherV2::default(),
        };

        state.update_special_hosts(&db);
        state.apply_filter_and_sort();

        state
    }

    fn update_special_hosts(&mut self, db: &HostsDatabase) {
        self.favorite_hosts = db.hosts.iter().filter(|h| h.is_favorite).cloned().collect();

        self.recent_hosts = db
            .hosts
            .iter()
            .filter(|h| h.last_connected.is_some())
            .cloned()
            .collect::<Vec<_>>();

        self.recent_hosts
            .sort_by(|a, b| b.last_connected.cmp(&a.last_connected));
    }

    pub fn current_host(&self) -> Option<&SshHost> {
        self.filtered_hosts.get(self.filtered_index)
    }

    pub fn current_group(&self) -> Option<&Group> {
        self.groups.get(self.selected_group_index)
    }

    pub fn filter_hosts(&mut self) {
        self.apply_filter_and_sort();
    }

    fn apply_filter_and_sort(&mut self) {
        let query = self.search_query.to_lowercase();
        let matcher = &self.fuzzy_matcher;

        let base_hosts: Vec<&SshHost> = match self.filter_mode {
            FilterMode::All => self.db.hosts.iter().collect(),
            FilterMode::Favorites => self.favorite_hosts.iter().collect(),
            FilterMode::Recent => self.recent_hosts.iter().collect(),
            FilterMode::Group(idx) => {
                if let Some(group) = self.groups.get(idx) {
                    self.db
                        .hosts
                        .iter()
                        .filter(|h| group.hosts.contains(&h.id))
                        .collect()
                } else {
                    self.db.hosts.iter().collect()
                }
            }
        };

        if query.is_empty() {
            self.filtered_hosts = base_hosts.into_iter().cloned().collect();
        } else {
            let mut scored: Vec<(i64, SshHost)> = base_hosts
                .iter()
                .filter_map(|h| {
                    let name_score = matcher.fuzzy_match(&h.name, &query);
                    let host_score = matcher.fuzzy_match(&h.hostname, &query);
                    let user_score = matcher.fuzzy_match(&h.username, &query);
                    let tag_scores: i64 = h
                        .tags
                        .iter()
                        .filter_map(|t| matcher.fuzzy_match(t, &query))
                        .sum();

                    let total = name_score.unwrap_or(0) * 10
                        + host_score.unwrap_or(0) * 5
                        + user_score.unwrap_or(0) * 3
                        + tag_scores;

                    if total > 0 {
                        Some((total, (*h).clone()))
                    } else {
                        None
                    }
                })
                .collect();

            scored.sort_by(|a, b| b.0.cmp(&a.0));
            self.filtered_hosts = scored.into_iter().map(|(_, h)| h).collect();
        }

        self.sort_hosts();

        if self.filtered_index >= self.filtered_hosts.len() {
            self.filtered_index = self.filtered_hosts.len().saturating_sub(1);
        }
    }

    fn sort_hosts(&mut self) {
        match self.sort_mode {
            SortMode::Alphabetical => {
                self.filtered_hosts
                    .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
            SortMode::Recent => {
                self.filtered_hosts
                    .sort_by(|a, b| b.last_connected.cmp(&a.last_connected));
            }
            SortMode::FavoritesFirst => {
                self.filtered_hosts
                    .sort_by(|a, b| match (a.is_favorite, b.is_favorite) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    });
            }
            SortMode::MostConnected => {
                self.filtered_hosts
                    .sort_by(|a, b| b.connect_count.cmp(&a.connect_count));
            }
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.filtered_index > 0 {
            self.filtered_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.filtered_index < self.filtered_hosts.len().saturating_sub(1) {
            self.filtered_index += 1;
        }
    }

    pub fn move_to_top(&mut self) {
        self.filtered_index = 0;
    }

    pub fn move_to_bottom(&mut self) {
        self.filtered_index = self.filtered_hosts.len().saturating_sub(1);
    }

    pub fn start_add_host(&mut self) {
        self.form_data = HostFormData::default();
        self.form_field_index = 0;
        self.view_mode = ViewMode::AddHost;
        self.input_mode = InputMode::Insert;
    }

    pub fn start_edit_host(&mut self) {
        if let Some(host) = self.current_host() {
            self.form_data = HostFormData::from_host(host);
            self.form_field_index = 0;
            self.view_mode = ViewMode::EditHost;
            self.input_mode = InputMode::Insert;
        }
    }

    pub fn save_host(&mut self) -> bool {
        if self.form_data.name.is_empty()
            || self.form_data.hostname.is_empty()
            || self.form_data.username.is_empty()
        {
            self.message = Some("Name, hostname and username are required".to_string());
            return false;
        }

        match self.view_mode {
            ViewMode::AddHost => {
                let host = self.form_data.to_host(None);
                self.db.hosts.push(host);
                self.message = Some("Host added".to_string());
            }
            ViewMode::EditHost => {
                let host_id = self.current_host().map(|h| h.id.clone());
                if let Some(id) = host_id {
                    if let Some(existing) = self.db.hosts.iter_mut().find(|h| h.id == id) {
                        let new_host = self.form_data.to_host(Some(id));
                        *existing = new_host;
                        self.message = Some("Host updated".to_string());
                    }
                }
            }
            _ => return false,
        }

        self.filter_hosts();
        self.view_mode = ViewMode::Normal;
        self.input_mode = InputMode::Normal;

        if let Err(e) = crate::storage::save_hosts(&self.db) {
            self.message = Some(format!("Error saving: {}", e));
            return false;
        }

        true
    }

    pub fn delete_current_host(&mut self) -> bool {
        let host_id = self.current_host().map(|h| h.id.clone());
        if let Some(id) = host_id {
            let len_before = self.db.hosts.len();
            self.db.hosts.retain(|h| h.id != id);
            if self.db.hosts.len() < len_before {
                let db_copy = self.db.clone();
                self.update_special_hosts(&db_copy);
                self.filter_hosts();
                self.message = Some("Host deleted".to_string());
                if let Err(e) = crate::storage::save_hosts(&self.db) {
                    self.message = Some(format!("Error saving: {}", e));
                    return false;
                }
                return true;
            }
        }
        false
    }

    pub fn add_group(&mut self, name: String) {
        let group = Group::new(name);
        self.db.groups.push(group.clone());
        self.groups.push(group);
        let _ = crate::storage::save_hosts(&self.db);
    }

    pub fn delete_group(&mut self, index: usize) -> bool {
        if index < self.groups.len() {
            let group = self.groups[index].clone();
            self.db.groups.retain(|g| g.id != group.id);
            self.groups.retain(|g| g.id != group.id);
            if self.selected_group_index >= self.groups.len() + 3 {
                self.selected_group_index = (self.groups.len() + 2).saturating_sub(1);
            }
            let _ = crate::storage::save_hosts(&self.db);
            return true;
        }
        false
    }

    pub fn set_filter_mode(&mut self, mode: FilterMode) {
        self.filter_mode = mode;
        self.filter_hosts();
    }

    pub fn set_sort_mode(&mut self, mode: SortMode) {
        self.sort_mode = mode;
        self.filter_hosts();
    }

    pub fn update_filter_from_group(&mut self) {
        let mode = if self.selected_group_index < 3 {
            match self.selected_group_index {
                0 => FilterMode::All,
                1 => FilterMode::Favorites,
                2 => FilterMode::Recent,
                _ => FilterMode::All,
            }
        } else {
            let group_idx = self.selected_group_index - 3;
            if group_idx < self.groups.len() {
                FilterMode::Group(group_idx)
            } else {
                FilterMode::All
            }
        };
        self.filter_mode = mode;
        self.filter_hosts();
    }

    pub fn cycle_theme(&mut self) {
        let themes = Theme::all();
        let current_idx = themes
            .iter()
            .position(|t| *t == self.config.theme)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % themes.len();
        self.config.theme = themes[next_idx];
        self.theme_colors = ThemeColors::from_theme(self.config.theme);
        let _ = self.config.save();
    }

    pub fn current_theme_name(&self) -> &'static str {
        self.config.theme.name()
    }
}
