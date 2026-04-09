use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SshHost {
    pub id: String,
    pub name: String,
    pub hostname: String,
    pub username: String,
    pub port: u16,
    pub identity_file: Option<String>,
    pub password: Option<String>,
    pub tags: Vec<String>,
    pub notes: Option<String>,
    pub proxy_jump: Option<String>,
    pub env_vars: HashMap<String, String>,
    pub is_favorite: bool,
    pub connect_count: u32,
    pub last_connected: Option<i64>,
}

impl SshHost {
    pub fn new(name: String, hostname: String, username: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            hostname,
            username,
            port: 22,
            identity_file: None,
            password: None,
            tags: Vec::new(),
            notes: None,
            proxy_jump: None,
            env_vars: HashMap::new(),
            is_favorite: false,
            connect_count: 0,
            last_connected: None,
        }
    }

    pub fn ssh_command(&self) -> String {
        let mut cmd = format!("ssh {}@{}", self.username, self.hostname);
        if self.port != 22 {
            cmd.push_str(&format!(" -p {}", self.port));
        }
        if let Some(ref identity) = self.identity_file {
            cmd.push_str(&format!(" -i {}", identity));
        }
        if let Some(ref proxy) = self.proxy_jump {
            cmd.push_str(&format!(" -J {}", proxy));
        }
        cmd
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub hosts: Vec<String>,
    pub is_expanded: bool,
}

impl Group {
    pub fn new(name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            hosts: Vec::new(),
            is_expanded: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HostsDatabase {
    pub hosts: Vec<SshHost>,
    pub groups: Vec<Group>,
}

impl HostsDatabase {
    pub fn new() -> Self {
        Self::default()
    }
}
