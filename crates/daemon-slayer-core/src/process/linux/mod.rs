use std::process::Stdio;

use tokio::process::Command;

use tracing::info;
use zbus::{dbus_proxy, zvariant::OwnedObjectPath, Connection};

#[dbus_proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Manager {
    fn list_sessions(&self) -> zbus::Result<Vec<(String, u32, String, String, OwnedObjectPath)>>;
}

pub async fn run_process_as_logged_on_users(cmd: &str) {
    let conn = Connection::system().await.unwrap();
    let proxy = ManagerProxy::new(&conn).await.unwrap();

    for (_, user_id, username, _, _) in proxy.list_sessions().await.unwrap() {
        // We assume that we're using the standard DBUS paths here under /run
        // since we don't have access to the user's DBUS_SESSION_BUS_ADDRESS variable
        // TODO: maybe we need to do something fancy here to detect if they're using nonstandard dbus config
        info!("Spawning process as user {username}");
        let mut child = Command::new("runuser")
            .args([
                "-l",
                &username,
                "-c",
                &format!(
                    "DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/{user_id}/bus {}",
                    cmd
                ),
            ])
            .stdout(Stdio::null())
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child.wait().await.unwrap();
    }
}
