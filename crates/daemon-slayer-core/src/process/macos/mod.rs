use std::process::Stdio;
use std::{env, io};

use regex::Regex;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::info;

use crate::Label;
use crate::process::get_admin_var;

// from https://scriptingosx.com/2020/02/getting-the-current-user-in-macos-update/

pub async fn run_process_as_current_user(
    label: &Label,
    cmd: &str,
    _visible: bool,
) -> io::Result<String> {
    let is_admin = matches!(
        env::var(get_admin_var(label))
            .map(|v| v.to_lowercase())
            .as_deref(),
        Ok("1" | "true")
    );

    let cmd_args = shlex::split(cmd).unwrap();

    if !is_admin {
        let output = Command::new(&cmd_args[0])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args(&cmd_args[1..])
            .output()
            .await?;

        return String::from_utf8(output.stdout)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()));
    }

    let mut user_info = Command::new("scutil")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    user_info
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"show State:/Users/ConsoleUser")
        .await
        .unwrap();
    let user_info = user_info.wait_with_output().await.unwrap();
    let user_info = String::from_utf8(user_info.stdout).unwrap();

    let cap = Regex::new(r"(?m)UID\s+:\s+(.*)$")
        .unwrap()
        .captures(&user_info)
        .unwrap();

    let uid = cap.get(1).unwrap().as_str().to_string();
    info!("Spawning process as user {uid}");
    let mut args = vec!["launchctl".to_owned(), "asuser".to_owned(), uid];
    args.extend(cmd_args.into_iter());

    let output = Command::new("sudo")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(args)
        .output()
        .await?;

    String::from_utf8(output.stdout)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}
