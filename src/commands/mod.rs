use crate::app::{
    AppCommand, AppEvent, AppState, FilterMode, HostFormData, InputMode, Panel, SortMode,
};
use crate::ssh;
use crate::storage;
use crate::storage::ssh_config as ssh_config_import;
use crate::tui::views::ViewMode;
use crate::utils;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_key_event(event: KeyEvent, state: &mut AppState) -> AppEvent {
    match state.input_mode {
        InputMode::Normal => handle_normal_mode(event, state),
        InputMode::Search => handle_search_mode(event, state),
        InputMode::Command => handle_command_mode(event, state),
        InputMode::Insert => handle_insert_mode(event, state),
    }
}

fn handle_normal_mode(event: KeyEvent, state: &mut AppState) -> AppEvent {
    match event.code {
        KeyCode::Char('q') if event.modifiers.is_empty() => {
            state.should_quit = true;
            AppEvent::Quit
        }
        KeyCode::Char('j') if event.modifiers.is_empty() => {
            if state.active_panel == Panel::Groups {
                let max_index = 2 + state.groups.len();
                if state.selected_group_index < max_index {
                    state.selected_group_index += 1;
                    state.update_filter_from_group();
                }
            } else {
                state.move_selection_down();
            }
            AppEvent::None
        }
        KeyCode::Char('k') if event.modifiers.is_empty() => {
            if state.active_panel == Panel::Groups {
                if state.selected_group_index > 0 {
                    state.selected_group_index -= 1;
                    state.update_filter_from_group();
                }
            } else {
                state.move_selection_up();
            }
            AppEvent::None
        }
        KeyCode::Char('G') if event.modifiers.is_empty() => {
            state.move_to_bottom();
            AppEvent::None
        }
        KeyCode::Char('g') if event.modifiers.is_empty() => {
            state.move_to_top();
            AppEvent::None
        }
        KeyCode::Char('/') => {
            state.input_mode = InputMode::Search;
            state.search_query.clear();
            AppEvent::None
        }
        KeyCode::Char(':') => {
            state.input_mode = InputMode::Command;
            state.command_buffer.clear();
            AppEvent::None
        }
        KeyCode::Char('h') if event.modifiers.is_empty() => {
            state.active_panel = match state.active_panel {
                Panel::Hosts => Panel::Groups,
                Panel::Details => Panel::Hosts,
                Panel::Groups => Panel::Groups,
            };
            AppEvent::None
        }
        KeyCode::Char('l') if event.modifiers.is_empty() => {
            state.active_panel = match state.active_panel {
                Panel::Groups => Panel::Hosts,
                Panel::Hosts => Panel::Details,
                Panel::Details => Panel::Details,
            };
            AppEvent::None
        }
        KeyCode::Char('s') => {
            let new_mode = match state.sort_mode {
                SortMode::Alphabetical => SortMode::FavoritesFirst,
                SortMode::FavoritesFirst => SortMode::Recent,
                SortMode::Recent => SortMode::MostConnected,
                SortMode::MostConnected => SortMode::Alphabetical,
            };
            state.set_sort_mode(new_mode);
            AppEvent::None
        }
        KeyCode::Char('a') => {
            state.start_add_host();
            AppEvent::None
        }
        KeyCode::Char('e') => {
            if state.current_host().is_some() {
                state.start_edit_host();
            }
            AppEvent::None
        }
        KeyCode::Char('d') => {
            if state.delete_current_host() {
                // Message already set
            }
            AppEvent::None
        }
        KeyCode::Char('f') => {
            let host_id = state.current_host().map(|h| h.id.clone());
            if let Some(id) = host_id {
                if let Some(h) = state.db.hosts.iter_mut().find(|h| h.id == id) {
                    h.is_favorite = !h.is_favorite;
                    state.filter_hosts();
                    let _ = storage::save_hosts(&state.db);
                }
            }
            AppEvent::None
        }
        KeyCode::Char('y') => {
            if let Some(host) = state.current_host() {
                if let Err(e) = utils::copy_to_clipboard(&host.hostname) {
                    state.message = Some(format!("Failed to copy: {}", e));
                } else {
                    state.message = Some("Hostname copied to clipboard".to_string());
                }
            }
            AppEvent::None
        }
        KeyCode::Char('c') => {
            if let Some(host) = state.current_host() {
                let cmd = host.ssh_command();
                if let Err(e) = utils::copy_to_clipboard(&cmd) {
                    state.message = Some(format!("Failed to copy: {}", e));
                } else {
                    state.message = Some("SSH command copied to clipboard".to_string());
                }
            }
            AppEvent::None
        }
        KeyCode::Enter => {
            let host = state.current_host().cloned();
            if let Some(h) = host {
                state.last_command = AppCommand::Connect;
                state.message = Some(format!("Connecting to {}...", h.name));

                let result = ssh::open_ssh_session(
                    &h.hostname,
                    h.port,
                    &h.username,
                    h.identity_file.as_deref(),
                    h.proxy_jump.as_deref(),
                    None,
                );

                match result {
                    Ok(_) => {
                        if let Some(host_id) = state
                            .db
                            .hosts
                            .iter()
                            .find(|hh| hh.id == h.id)
                            .map(|hh| hh.id.clone())
                        {
                            if let Some(db_host) =
                                state.db.hosts.iter_mut().find(|hh| hh.id == host_id)
                            {
                                db_host.connect_count += 1;
                                db_host.last_connected = Some(chrono::Utc::now().timestamp());
                                let _ = storage::save_hosts(&state.db);
                            }
                        }
                        state.message = Some("Connection closed".to_string());
                    }
                    Err(e) => {
                        state.message = Some(format!("Connection failed: {}", e));
                    }
                }
                AppEvent::None
            } else {
                AppEvent::None
            }
        }
        KeyCode::Char('p') => {
            let host = state.current_host().cloned();
            if let Some(h) = host {
                state.message = Some(format!("Testing connection to {}...", h.name));
                match ssh::test_connection(&h.hostname, h.port, 5) {
                    Ok(latency) => {
                        state.message = Some(format!("{} is reachable ({}ms)", h.name, latency));
                    }
                    Err(e) => {
                        state.message = Some(format!("{} is unreachable: {}", h.name, e));
                    }
                }
            }
            AppEvent::None
        }
        KeyCode::Char('r') => {
            let host = state.current_host().cloned();
            if let Some(h) = host {
                if let Err(e) = ssh::copy_id(&h.hostname, h.port, &h.username) {
                    state.message = Some(format!("Key copy failed: {}", e));
                } else {
                    state.message = Some("SSH key copied successfully".to_string());
                }
            }
            AppEvent::None
        }
        KeyCode::Char('t') => {
            state.cycle_theme();
            state.message = Some(format!("Theme: {}", state.current_theme_name()));
            AppEvent::None
        }
        KeyCode::Char('?') => {
            state.view_mode = ViewMode::Help;
            AppEvent::None
        }
        KeyCode::Esc => {
            state.view_mode = ViewMode::Normal;
            AppEvent::None
        }
        KeyCode::Up => {
            state.move_selection_up();
            AppEvent::None
        }
        KeyCode::Down => {
            state.move_selection_down();
            AppEvent::None
        }
        _ => AppEvent::None,
    }
}

fn handle_search_mode(event: KeyEvent, state: &mut AppState) -> AppEvent {
    match event.code {
        KeyCode::Esc => {
            state.input_mode = InputMode::Normal;
            state.search_query.clear();
            state.filter_hosts();
            AppEvent::None
        }
        KeyCode::Enter => {
            state.input_mode = InputMode::Normal;
            state.filter_hosts();
            AppEvent::None
        }
        KeyCode::Char(c) => {
            state.search_query.push(c);
            state.filter_hosts();
            AppEvent::None
        }
        KeyCode::Backspace => {
            state.search_query.pop();
            state.filter_hosts();
            AppEvent::None
        }
        _ => AppEvent::None,
    }
}

fn handle_command_mode(event: KeyEvent, state: &mut AppState) -> AppEvent {
    match event.code {
        KeyCode::Esc => {
            state.input_mode = InputMode::Normal;
            state.command_buffer.clear();
            AppEvent::None
        }
        KeyCode::Enter => {
            state.input_mode = InputMode::Normal;
            let cmd = state.command_buffer.clone();
            state.command_buffer.clear();
            execute_command(&cmd, state)
        }
        KeyCode::Char(c) => {
            state.command_buffer.push(c);
            AppEvent::None
        }
        KeyCode::Backspace => {
            state.command_buffer.pop();
            AppEvent::None
        }
        _ => AppEvent::None,
    }
}

fn handle_insert_mode(event: KeyEvent, state: &mut AppState) -> AppEvent {
    match event.code {
        KeyCode::Esc => {
            state.input_mode = InputMode::Normal;
            state.view_mode = ViewMode::Normal;
            state.form_data = HostFormData::default();
            AppEvent::None
        }
        KeyCode::Tab => {
            state.form_field_index = (state.form_field_index + 1) % 8;
            AppEvent::None
        }
        KeyCode::Enter => {
            if state.form_field_index == 7 {
                state.save_host();
            } else {
                state.form_field_index += 1;
            }
            AppEvent::None
        }
        KeyCode::Up => {
            state.form_field_index = if state.form_field_index == 0 {
                7
            } else {
                state.form_field_index - 1
            };
            AppEvent::None
        }
        KeyCode::Down => {
            state.form_field_index = (state.form_field_index + 1) % 8;
            AppEvent::None
        }
        KeyCode::Char(c) => {
            let field = match state.form_field_index {
                0 => &mut state.form_data.name,
                1 => &mut state.form_data.hostname,
                2 => &mut state.form_data.username,
                3 => &mut state.form_data.port,
                4 => &mut state.form_data.identity_file,
                5 => &mut state.form_data.proxy_jump,
                6 => &mut state.form_data.tags,
                7 => &mut state.form_data.notes,
                _ => return AppEvent::None,
            };
            field.push(c);
            AppEvent::None
        }
        KeyCode::Backspace => {
            let field = match state.form_field_index {
                0 => &mut state.form_data.name,
                1 => &mut state.form_data.hostname,
                2 => &mut state.form_data.username,
                3 => &mut state.form_data.port,
                4 => &mut state.form_data.identity_file,
                5 => &mut state.form_data.proxy_jump,
                6 => &mut state.form_data.tags,
                7 => &mut state.form_data.notes,
                _ => return AppEvent::None,
            };
            field.pop();
            AppEvent::None
        }
        _ => AppEvent::None,
    }
}

fn execute_command(cmd: &str, state: &mut AppState) -> AppEvent {
    let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
    match parts.first().map(|s| *s) {
        Some("q") | Some("quit") => {
            state.should_quit = true;
            AppEvent::Quit
        }
        Some("add") => {
            state.start_add_host();
            AppEvent::None
        }
        Some("help") | Some("?") => {
            state.view_mode = ViewMode::Help;
            AppEvent::None
        }
        Some("import") | Some("i") => {
            match ssh_config_import::import_ssh_config(None) {
                Ok(imported) => {
                    let count = imported.hosts.len();
                    state.db.hosts.extend(imported.hosts);
                    state.db.groups.extend(imported.groups);
                    if let Ok(_) = storage::save_hosts(&state.db) {
                        state.filter_hosts();
                        state.message = Some(format!("Imported {} hosts from SSH config", count));
                    }
                }
                Err(e) => {
                    state.message = Some(format!("Import failed: {}", e));
                }
            }
            AppEvent::None
        }
        Some("export") | Some("e") => {
            match ssh_config_import::export_ssh_config(&state.db, None) {
                Ok(_) => {
                    state.message = Some(format!(
                        "Exported {} hosts to SSH config",
                        state.db.hosts.len()
                    ));
                }
                Err(e) => {
                    state.message = Some(format!("Export failed: {}", e));
                }
            }
            AppEvent::None
        }
        Some("csv") => {
            match ssh_config_import::export_to_csv(&state.db, None) {
                Ok(_) => {
                    state.message = Some(format!(
                        "Exported {} hosts to sshman_hosts.csv",
                        state.db.hosts.len()
                    ));
                }
                Err(e) => {
                    state.message = Some(format!("Export failed: {}", e));
                }
            }
            AppEvent::None
        }
        _ => {
            state.message = Some(format!("Unknown command: {}", cmd));
            AppEvent::None
        }
    }
}
