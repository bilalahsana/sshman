use crate::models::{HostsDatabase, SshHost};
use std::fs;
use std::io::{self, BufRead};

pub fn import_ssh_config(path: Option<&str>) -> anyhow::Result<HostsDatabase> {
    let config_path = path
        .map(|p| p.to_string())
        .unwrap_or_else(|| shellexpand::tilde("~/.ssh/config").to_string());

    let file = fs::File::open(&config_path)?;
    let reader = io::BufReader::new(file);

    let mut db = HostsDatabase::new();
    let mut hosts: Vec<SshHost> = Vec::new();
    let mut current_name = String::new();
    let mut current_hostname = String::new();
    let mut current_username = String::new();
    let mut current_port = 22u16;
    let mut current_identity: Option<String> = None;
    let mut current_proxy: Option<String> = None;
    let mut in_host_block = false;

    for line in reader.lines() {
        let line = line?.trim().to_string();

        if line.to_lowercase().starts_with("host ") {
            if in_host_block {
                let name = current_name.clone();
                let hostname = if current_hostname.is_empty() {
                    name.clone()
                } else {
                    current_hostname.clone()
                };
                if !name.is_empty() && !hostname.is_empty() {
                    let mut host = SshHost::new(name, hostname, current_username.clone());
                    host.port = current_port;
                    host.identity_file = current_identity.clone();
                    host.proxy_jump = current_proxy.clone();
                    hosts.push(host);
                }

                current_name.clear();
                current_hostname.clear();
                current_username.clear();
                current_port = 22;
                current_identity = None;
                current_proxy = None;
            }

            current_name = line[5..].trim().to_string();
            in_host_block = true;
        } else if in_host_block {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() == 2 {
                let key = parts[0].to_lowercase();
                let value = parts[1].trim().to_string();

                match key.as_str() {
                    "hostname" => current_hostname = value,
                    "user" => current_username = value,
                    "port" => current_port = value.parse().unwrap_or(22),
                    "identityfile" => current_identity = Some(value),
                    "proxyjump" | "j" => current_proxy = Some(value),
                    _ => {}
                }
            }
        }
    }

    if in_host_block {
        let name = current_name.clone();
        let hostname = if current_hostname.is_empty() {
            name.clone()
        } else {
            current_hostname.clone()
        };
        if !name.is_empty() && !hostname.is_empty() {
            let mut host = SshHost::new(name, hostname, current_username.clone());
            host.port = current_port;
            host.identity_file = current_identity;
            host.proxy_jump = current_proxy;
            hosts.push(host);
        }
    }

    db.hosts = hosts;
    Ok(db)
}

pub fn export_ssh_config(db: &HostsDatabase, path: Option<&str>) -> anyhow::Result<()> {
    let config_path = path
        .map(|p| p.to_string())
        .unwrap_or_else(|| shellexpand::tilde("~/.ssh/config").to_string());

    let mut content = String::new();

    content.push_str("# SSHMan exported config\n");
    content.push_str("# Do not edit manually unless you know what you're doing\n\n");

    for host in &db.hosts {
        content.push_str(&format!("Host {}\n", host.name));

        if host.hostname != host.name {
            content.push_str(&format!("    HostName {}\n", host.hostname));
        }

        if !host.username.is_empty() {
            content.push_str(&format!("    User {}\n", host.username));
        }

        if host.port != 22 {
            content.push_str(&format!("    Port {}\n", host.port));
        }

        if let Some(ref identity) = host.identity_file {
            if !identity.is_empty() {
                content.push_str(&format!("    IdentityFile {}\n", identity));
            }
        }

        if let Some(ref proxy) = host.proxy_jump {
            if !proxy.is_empty() {
                content.push_str(&format!("    ProxyJump {}\n", proxy));
            }
        }

        for (key, value) in &host.env_vars {
            content.push_str(&format!("    SetEnv {}={}\n", key, value));
        }

        content.push_str("\n");
    }

    fs::write(&config_path, content)?;

    Ok(())
}

pub fn export_to_csv(db: &HostsDatabase, path: Option<&str>) -> anyhow::Result<()> {
    let csv_path = path
        .map(|p| p.to_string())
        .unwrap_or_else(|| "sshman_hosts.csv".to_string());

    let mut content = String::from("name,hostname,username,port,identity_file,proxy_jump,tags,notes,is_favorite,connect_count\n");

    for host in &db.hosts {
        content.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            escape_csv(&host.name),
            escape_csv(&host.hostname),
            escape_csv(&host.username),
            host.port,
            escape_csv(host.identity_file.as_deref().unwrap_or("")),
            escape_csv(host.proxy_jump.as_deref().unwrap_or("")),
            escape_csv(&host.tags.join(",")),
            escape_csv(host.notes.as_deref().unwrap_or("")),
            host.is_favorite,
            host.connect_count
        ));
    }

    fs::write(&csv_path, content)?;

    Ok(())
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
