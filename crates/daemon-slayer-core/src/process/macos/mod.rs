use regex::Regex;
use std::{io, process::Stdio};
use tokio::{io::AsyncWriteExt, process::Command};
use tracing::info;

// from https://scriptingosx.com/2020/02/getting-the-current-user-in-macos-update/

pub async fn run_process_as_current_user(cmd: &str, _visible: bool) -> io::Result<String> {
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
    args.extend(shlex::split(cmd).unwrap().into_iter());

    let output = Command::new("sudo")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(args)
        .output()?;

    String::from_utf8(output.stdout)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}
