use crate::config::{ensure_dirs, hosts_file};
use crate::models::{Group, HostsDatabase, SshHost};
use anyhow::Result;

pub mod ssh_config;

pub fn load_hosts() -> Result<HostsDatabase> {
    ensure_dirs()?;
    let path = hosts_file().unwrap();
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        let db: HostsDatabase = serde_json::from_str(&content)?;
        Ok(db)
    } else {
        Ok(HostsDatabase::new())
    }
}

pub fn save_hosts(db: &HostsDatabase) -> Result<()> {
    ensure_dirs()?;
    let path = hosts_file().unwrap();
    let content = serde_json::to_string_pretty(db)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn add_host(host: SshHost, db: &mut HostsDatabase) {
    db.hosts.push(host);
}

pub fn update_host(host: SshHost, db: &mut HostsDatabase) -> Option<SshHost> {
    if let Some(existing) = db.hosts.iter_mut().find(|h| h.id == host.id) {
        *existing = host.clone();
        Some(host)
    } else {
        None
    }
}

pub fn delete_host(id: &str, db: &mut HostsDatabase) -> bool {
    let len_before = db.hosts.len();
    db.hosts.retain(|h| h.id != id);
    db.hosts.len() < len_before
}

pub fn add_group(group: Group, db: &mut HostsDatabase) {
    db.groups.push(group);
}

pub fn delete_group(id: &str, db: &mut HostsDatabase) -> bool {
    let len_before = db.groups.len();
    db.groups.retain(|g| g.id != id);
    db.groups.len() < len_before
}
