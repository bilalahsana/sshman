use std::io;
use std::process::{Command, Stdio};

pub fn open_ssh_session(
    hostname: &str,
    port: u16,
    username: &str,
    identity_file: Option<&str>,
    proxy_jump: Option<&str>,
    custom_command: Option<&str>,
) -> Result<(), SshError> {
    let mut cmd = Command::new("ssh");

    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if port != 22 {
        cmd.arg("-p").arg(port.to_string());
    }

    if let Some(identity) = identity_file {
        if !identity.is_empty() {
            cmd.arg("-i").arg(identity);
        }
    }

    if let Some(proxy) = proxy_jump {
        if !proxy.is_empty() {
            cmd.arg("-J").arg(proxy);
        }
    }

    cmd.arg("-o").arg("ServerAliveInterval=60");
    cmd.arg("-o").arg("ServerAliveCountMax=3");
    cmd.arg("-o").arg("StrictHostKeyChecking=accept-new");

    if let Some(cmd_str) = custom_command {
        if !cmd_str.is_empty() {
            cmd.arg(format!("{}@{}", username, hostname));
            cmd.arg(cmd_str);
        } else {
            cmd.arg(format!("{}@{}", username, hostname));
        }
    } else {
        cmd.arg(format!("{}@{}", username, hostname));
    }

    let status = cmd
        .status()
        .map_err(|e| SshError::ConnectionFailed(e.to_string()))?;

    if status.success() {
        Ok(())
    } else {
        Err(SshError::ConnectionFailed(format!(
            "SSH exited with code: {:?}",
            status.code()
        )))
    }
}

pub fn test_connection(hostname: &str, port: u16, timeout_secs: u64) -> Result<u64, SshError> {
    let output = Command::new("ping")
        .arg("-c")
        .arg("1")
        .arg("-W")
        .arg(timeout_secs.to_string())
        .arg(hostname)
        .output()
        .map_err(|e| SshError::PingFailed(e.to_string()))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(time_idx) = stdout.find("time=") {
            let time_str = &stdout[time_idx + 5..];
            if let Some(space_idx) = time_str.find(' ') {
                let ms_str = &time_str[..space_idx];
                if let Ok(ms) = ms_str.parse::<f64>() {
                    return Ok(ms as u64);
                }
            }
        }
        Ok(0)
    } else {
        Err(SshError::HostUnreachable(hostname.to_string()))
    }
}

pub fn copy_id(hostname: &str, port: u16, username: &str) -> Result<(), SshError> {
    let mut cmd = Command::new("ssh-copy-id");

    if port != 22 {
        cmd.arg("-p").arg(port.to_string());
    }

    cmd.arg(format!("{}@{}", username, hostname));

    let status = cmd
        .status()
        .map_err(|e| SshError::KeyCopyFailed(e.to_string()))?;

    if status.success() {
        Ok(())
    } else {
        Err(SshError::KeyCopyFailed("ssh-copy-id failed".to_string()))
    }
}

#[derive(Debug)]
pub enum SshError {
    ConnectionFailed(String),
    HostUnreachable(String),
    PingFailed(String),
    KeyCopyFailed(String),
}

impl std::fmt::Display for SshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SshError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            SshError::HostUnreachable(host) => write!(f, "Host unreachable: {}", host),
            SshError::PingFailed(msg) => write!(f, "Ping failed: {}", msg),
            SshError::KeyCopyFailed(msg) => write!(f, "Key copy failed: {}", msg),
        }
    }
}

impl std::error::Error for SshError {}
