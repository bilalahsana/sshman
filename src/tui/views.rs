use crate::app::{AppState, InputMode, Panel};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, state: &AppState) {
    match state.view_mode {
        ViewMode::AddHost | ViewMode::EditHost => {
            render_host_form(frame, state);
        }
        ViewMode::Help => {
            render_help(frame, state);
        }
        _ => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(20),
                    Constraint::Percentage(50),
                    Constraint::Percentage(30),
                ])
                .split(frame.area());

            render_groups_panel(frame, state, chunks[0]);
            render_hosts_panel(frame, state, chunks[1]);
            render_details_panel(frame, state, chunks[2]);

            render_status_bar(frame, state);
        }
    }
}

fn render_host_form(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let title = match state.view_mode {
        ViewMode::AddHost => "Add New Host",
        ViewMode::EditHost => "Edit Host",
        _ => "Form",
    };

    let header = Paragraph::new(title)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Host Configuration"),
        )
        .alignment(Alignment::Center);

    frame.render_widget(header, chunks[0]);

    render_form_fields(frame, state, chunks[1]);

    let help_text = if state.input_mode == InputMode::Insert {
        "Tab: next field | ESC: cancel | Enter: save"
    } else {
        "Use keys to navigate"
    };

    let footer = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(footer, chunks[2]);
}

fn render_form_fields(frame: &mut Frame, state: &AppState, area: Rect) {
    let form = &state.form_data;
    let fields = [
        ("Name", &form.name),
        ("Hostname", &form.hostname),
        ("Username", &form.username),
        ("Port", &form.port),
        ("Identity File", &form.identity_file),
        ("Proxy Jump", &form.proxy_jump),
        ("Tags (comma separated)", &form.tags),
        ("Notes", &form.notes),
    ];

    let items: Vec<ListItem> = fields
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            let is_selected = i == state.form_field_index;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let value_display = if value.is_empty() {
                format!("[{}]", label)
            } else {
                value.to_string()
            };
            let line = format!("{}: {}", label, value_display);
            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Fields"))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(list, area);
}

fn render_help(frame: &mut Frame, _state: &AppState) {
    let help_text = r#"
╔════════════════════════════════════════════════════════════════╗
║                     SSHMAN KEYBINDINGS                        ║
╠════════════════════════════════════════════════════════════════╣
║  Navigation                                                   ║
║    j/k       - move down/up                                   ║
║    h/l       - switch panels left/right                       ║
║    gg        - jump to top                                     ║
║    G         - jump to bottom                                  ║
║                                                              ║
║  Actions                                                     ║
║    Enter     - connect to SSH                                 ║
║    a         - add new host                                   ║
║    e         - edit selected host                             ║
║    d         - delete selected host                           ║
║    f         - toggle favorite                                ║
║    p         - ping test connection                           ║
║    r         - ssh-copy-id to host                            ║
║    y         - copy hostname to clipboard                     ║
║    c         - copy SSH command to clipboard                  ║
║    t         - cycle theme                                    ║
║                                                              ║
║  Search & Commands                                           ║
║    /         - search hosts                                   ║
║    :i        - import ~/.ssh/config                          ║
║    :e        - export to ~/.ssh/config                       ║
║    :csv      - export to CSV                                  ║
║    s         - cycle sort mode                                ║
║    ?         - show this help                                 ║
║                                                              ║
║  General                                                     ║
║    q         - quit                                          ║
║    ESC       - cancel / go back                              ║
╚════════════════════════════════════════════════════════════════╝

                    Press any key to exit
"#;

    let paragraph = Paragraph::new(help_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, frame.area());
}

fn render_groups_panel(frame: &mut Frame, state: &AppState, area: Rect) {
    let mut all_items: Vec<ListItem> = vec![
        ListItem::new("★ All Hosts").style(Style::default().fg(Color::White)),
        ListItem::new("♥ Favorites").style(Style::default().fg(Color::Magenta)),
        ListItem::new("⏰ Recent").style(Style::default().fg(Color::Yellow)),
    ];

    if !state.groups.is_empty() {
        all_items.push(ListItem::new("--- Groups ---").style(Style::default().fg(Color::DarkGray)));

        let group_items: Vec<ListItem> = state
            .groups
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let style =
                    if i + 3 == state.selected_group_index && state.active_panel == Panel::Groups {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                ListItem::new(g.name.clone()).style(style)
            })
            .collect();
        all_items.extend(group_items);
    }

    let filter_label = match state.filter_mode {
        crate::app::FilterMode::All => "All Hosts",
        crate::app::FilterMode::Favorites => "Favorites",
        crate::app::FilterMode::Recent => "Recent",
        crate::app::FilterMode::Group(_) => "Group",
    };

    let list = List::new(all_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Filter: {}", filter_label))
                .border_style(if state.active_panel == Panel::Groups {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                }),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(list, area);
}

fn render_hosts_panel(frame: &mut Frame, state: &AppState, area: Rect) {
    let items: Vec<ListItem> = state
        .filtered_hosts
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let icon = if h.is_favorite { "★" } else { "○" };
            let style = if i == state.filtered_index && state.active_panel == Panel::Hosts {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let line = format!("{} {}@{}:{}", icon, h.username, h.hostname, h.port);
            ListItem::new(line).style(style)
        })
        .collect();

    let title = if state.search_query.is_empty() {
        format!("Hosts ({})", state.filtered_hosts.len())
    } else {
        format!(
            "Search: {} ({})",
            state.search_query,
            state.filtered_hosts.len()
        )
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(if state.active_panel == Panel::Hosts {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                }),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(list, area);
}

fn render_details_panel(frame: &mut Frame, state: &AppState, area: Rect) {
    let content = if let Some(host) = state.current_host() {
        let last_conn = host
            .last_connected
            .map(|t| {
                chrono::DateTime::from_timestamp(t, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            })
            .unwrap_or_else(|| "never".to_string());

        let details = format!(
            "{}\n\nHost: {}:{}\nUser: {}@{}\nIdentity: {}\nProxy: {}\nTags: {}\n\nNotes:\n{}\n\nConnections: {}\nLast: {}",
            host.name,
            host.hostname,
            host.port,
            host.username,
            host.hostname,
            host.identity_file.as_deref().unwrap_or("none"),
            host.proxy_jump.as_deref().unwrap_or("none"),
            host.tags.join(", "),
            host.notes.as_deref().unwrap_or("none"),
            host.connect_count,
            last_conn
        );
        Paragraph::new(details)
    } else {
        Paragraph::new("No host selected").style(Style::default().fg(Color::DarkGray))
    };

    let widget = content.block(
        Block::default()
            .borders(Borders::ALL)
            .title("Details")
            .border_style(Style::default().fg(Color::White)),
    );

    frame.render_widget(widget, area);
}

fn render_status_bar(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    let left = match state.input_mode {
        InputMode::Normal => format!(
            "[{}] {} hosts",
            state.active_panel.name(),
            state.filtered_hosts.len()
        ),
        InputMode::Search => format!("Search: {}", state.search_query),
        InputMode::Command => format!(":{}", state.command_buffer),
        InputMode::Insert => "Edit mode".to_string(),
    };

    let right = if let Some(ref msg) = state.message {
        msg.clone()
    } else {
        match state.input_mode {
            InputMode::Normal => {
                format!("q:quit /:search s:sort t:theme a:add e:edit d:delete p:ping Enter:connect [{}]", state.current_theme_name())
            }
            InputMode::Search => "ESC:cancel Enter:search".to_string(),
            InputMode::Command => "ESC:cancel Enter:execute".to_string(),
            InputMode::Insert => "Tab:next field ESC:cancel Enter:save".to_string(),
        }
    };

    let left_len = left.len();
    let right_len = right.len();
    let padding = (area.width as usize).saturating_sub(left_len + right_len);

    let status = Paragraph::new(Line::from(vec![
        Span::raw(&left),
        Span::raw(" ".repeat(padding)),
        Span::raw(&right),
    ]))
    .block(Block::default().borders(Borders::NONE));

    let status_area = Rect::new(
        area.x,
        area.height.saturating_sub(1),
        area.width,
        area.height,
    );
    frame.render_widget(status, status_area);
}

impl Panel {
    fn name(&self) -> &'static str {
        match self {
            Panel::Groups => "Groups",
            Panel::Hosts => "Hosts",
            Panel::Details => "Details",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Normal,
    AddHost,
    EditHost,
    Search,
    Help,
    CommandPalette,
}
