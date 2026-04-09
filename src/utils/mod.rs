use std::process::Command;

pub fn open_ssh_connection(
    hostname: &str,
    port: u16,
    username: &str,
    identity_file: Option<&str>,
    proxy_jump: Option<&str>,
) -> anyhow::Result<()> {
    let mut cmd = Command::new("ssh");

    if port != 22 {
        cmd.arg("-p").arg(port.to_string());
    }

    if let Some(identity) = identity_file {
        cmd.arg("-i").arg(identity);
    }

    if let Some(proxy) = proxy_jump {
        cmd.arg("-J").arg(proxy);
    }

    cmd.arg(format!("{}@{}", username, hostname));

    cmd.status()?;

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    use std::io::Write;
    let mut child = Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .stdin(std::process::Stdio::piped())
        .spawn()?;

    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(text.as_bytes())?;
    }

    child.wait()?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    use std::io::Write;
    let mut child = Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()?;

    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(text.as_bytes())?;
    }

    child.wait()?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    use std::process::Command;
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "echo", text, "|", "clip"]);
    cmd.status()?;
    Ok(())
}
