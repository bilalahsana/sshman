use crate::models::{HostsDatabase, SshHost};
use crate::storage;
use anyhow::Result;

pub fn create_host(
    name: String,
    hostname: String,
    username: String,
    port: Option<u16>,
    identity_file: Option<String>,
    tags: Option<Vec<String>>,
    notes: Option<String>,
    proxy_jump: Option<String>,
) -> Result<SshHost> {
    let mut host = SshHost::new(name, hostname, username);

    if let Some(p) = port {
        host.port = p;
    }
    host.identity_file = identity_file;
    host.tags = tags.unwrap_or_default();
    host.notes = notes;
    host.proxy_jump = proxy_jump;

    Ok(host)
}

pub fn update_connection_stats(host_id: &str, db: &mut HostsDatabase) -> Result<()> {
    if let Some(host) = db.hosts.iter_mut().find(|h| h.id == host_id) {
        host.connect_count += 1;
        host.last_connected = Some(chrono::Utc::now().timestamp());
        storage::save_hosts(db)?;
    }
    Ok(())
}
