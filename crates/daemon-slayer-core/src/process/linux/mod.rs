use std::{env, io, process::Stdio};

use tokio::process::Command;

use tracing::{info, warn};
use zbus::{dbus_proxy, zvariant::OwnedObjectPath, Connection};

#[dbus_proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Manager {
    #[allow(clippy::type_complexity)]
    fn list_sessions(&self) -> zbus::Result<Vec<(String, u32, String, String, OwnedObjectPath)>>;
}

pub async fn run_process_as_current_user(cmd: &str, _visible: bool) -> io::Result<String> {
    let conn = Connection::system()
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))?;
    let proxy = ManagerProxy::new(&conn)
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))?;
    let sessions = proxy
        .list_sessions()
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    if sessions.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Unable to locate user session",
        ));
    }

    if sessions.len() > 1 {
        warn!("More than one logon session found. Running process using the first user found");
    }
    let (_, user_id, username, _, _) = &sessions[0];
    info!("Spawning process as user {username}");
    let display = env::var("DISPLAY").unwrap_or_else(|_| ":0".to_owned());

    // We assume that we're using the standard DBUS paths here under /run
    // since we don't have access to the user's DBUS_SESSION_BUS_ADDRESS variable
    // TODO: maybe we need to do something fancy here to detect if they're using nonstandard dbus config
    let output = Command::new("runuser")
        .args([
            "-l",
            username,
            "-c",
            &format!("DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/{user_id}/bus DISPLAY={display} {cmd}",),
        ])
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await?;

    String::from_utf8(output.stdout)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}
