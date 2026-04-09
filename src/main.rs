use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tracing_subscriber::EnvFilter;

mod app;
mod commands;
mod config;
mod models;
mod services;
mod ssh;
mod storage;
mod tui;
mod utils;

use app::AppState;
use storage::load_hosts;

#[derive(Parser)]
#[command(name = "sshman")]
#[command(about = "Blazing-fast terminal SSH Session Manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    List,
    Add {
        name: String,
        hostname: String,
        username: String,
        #[arg(short, long)]
        port: Option<u16>,
        #[arg(short, long)]
        identity: Option<String>,
    },
    Edit {
        id: String,
    },
    Delete {
        id: String,
    },
    Connect {
        name: Option<String>,
        #[arg(short, long)]
        id: Option<String>,
    },
    Ping {
        name: Option<String>,
        #[arg(short, long)]
        id: Option<String>,
    },
    ImportSsh {
        #[arg(short, long)]
        path: Option<String>,
    },
    ExportSsh {
        #[arg(short, long)]
        path: Option<String>,
    },
    ExportCsv {
        #[arg(short, long)]
        path: Option<String>,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        match cmd {
            Commands::List => {
                let db = load_hosts()?;
                for host in &db.hosts {
                    println!(
                        "{} {}@{}:{}",
                        host.name, host.username, host.hostname, host.port
                    );
                }
            }
            Commands::Add {
                name,
                hostname,
                username,
                port,
                identity,
            } => {
                let mut db = load_hosts()?;
                let mut host = models::SshHost::new(name, hostname, username);
                host.port = port.unwrap_or(22);
                host.identity_file = identity;
                db.hosts.push(host);
                storage::save_hosts(&db)?;
                println!("Host added successfully");
            }
            Commands::Delete { id } => {
                let mut db = load_hosts()?;
                let len_before = db.hosts.len();
                db.hosts.retain(|h| h.id != id);
                if db.hosts.len() < len_before {
                    storage::save_hosts(&db)?;
                    println!("Host deleted");
                } else {
                    println!("Host not found");
                }
            }
            Commands::Edit { .. } => {
                println!("Edit not implemented in CLI yet");
            }
            Commands::Connect { name, id } => {
                let db = load_hosts()?;
                let host = find_host(&db, name.as_deref(), id.as_deref());
                if let Some(h) = host {
                    println!("Connecting to {}...", h.name);
                    match ssh::open_ssh_session(
                        &h.hostname,
                        h.port,
                        &h.username,
                        h.identity_file.as_deref(),
                        h.proxy_jump.as_deref(),
                        None,
                    ) {
                        Ok(_) => println!("Connection closed"),
                        Err(e) => eprintln!("Connection failed: {}", e),
                    }
                } else {
                    println!("Host not found");
                }
            }
            Commands::Ping { name, id } => {
                let db = load_hosts()?;
                let host = find_host(&db, name.as_deref(), id.as_deref());
                if let Some(h) = host {
                    match ssh::test_connection(&h.hostname, h.port, 5) {
                        Ok(latency) => println!("{} is reachable ({}ms)", h.name, latency),
                        Err(e) => println!("{} is unreachable: {}", h.name, e),
                    }
                } else {
                    println!("Host not found");
                }
            }
            Commands::ImportSsh { path } => {
                match storage::ssh_config::import_ssh_config(path.as_deref()) {
                    Ok(imported) => {
                        let mut db = load_hosts()?;
                        let count = imported.hosts.len();
                        db.hosts.extend(imported.hosts);
                        db.groups.extend(imported.groups);
                        storage::save_hosts(&db)?;
                        println!("Imported {} hosts from SSH config", count);
                    }
                    Err(e) => eprintln!("Import failed: {}", e),
                }
            }
            Commands::ExportSsh { path } => {
                let db = load_hosts()?;
                match storage::ssh_config::export_ssh_config(&db, path.as_deref()) {
                    Ok(_) => println!("Exported {} hosts to SSH config", db.hosts.len()),
                    Err(e) => eprintln!("Export failed: {}", e),
                }
            }
            Commands::ExportCsv { path } => {
                let db = load_hosts()?;
                match storage::ssh_config::export_to_csv(&db, path.as_deref()) {
                    Ok(_) => println!("Exported {} hosts to CSV", db.hosts.len()),
                    Err(e) => eprintln!("Export failed: {}", e),
                }
            }
        }
        return Ok(());
    }

    run_tui()
}

fn find_host<'a>(
    db: &'a models::HostsDatabase,
    name: Option<&str>,
    id: Option<&str>,
) -> Option<&'a models::SshHost> {
    if let Some(id) = id {
        db.hosts.iter().find(|h| h.id == id)
    } else if let Some(name) = name {
        db.hosts.iter().find(|h| h.name == name)
    } else {
        None
    }
}

fn run_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let db = load_hosts().unwrap_or_default();
    let mut state = AppState::new(db);

    loop {
        terminal.draw(|f| tui::views::render(f, &state))?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    commands::handle_key_event(key, &mut state);
                }
            }
        }

        if state.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
