use std::fs::{self, File};
use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::LazyLock;

use async_trait::async_trait;
#[cfg(feature = "socket-activation")]
use daemon_slayer_core::socket_activation;
use daemon_slayer_core::Label;
use launchd::sockets::SocketFamily;
use launchd::{Launchd, SocketOptions, Sockets};
use regex::Regex;
use tokio::process::Command;

use crate::config::{Builder, Config, Level};
use crate::{Manager, State, Status};

macro_rules! regex {
    ($name:ident, $re:literal $(,)?) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($re).unwrap());
    };
}

static NOT_FOUND: &str = "could not find service";

regex!(STATE_RE, r"state = (\w+)");
regex!(PID_RE, r"pid = (\w+)");
regex!(EXIT_CODE_RE, r"last exit code = (\w+)");

#[derive(Clone, Debug)]
pub struct LaunchdServiceManager {
    config: Builder,
}

impl LaunchdServiceManager {
    pub(crate) fn from_builder(builder: Builder) -> io::Result<Self> {
        Ok(Self { config: builder })
    }

    async fn run_launchctl(&self, arguments: Vec<&str>) -> io::Result<String> {
        self.run_cmd("launchctl", arguments).await
    }

    async fn run_cmd(&self, command: &str, arguments: Vec<&str>) -> io::Result<String> {
        let output = Command::new(command)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args(&arguments)
            .output()
            .await
            .map_err(|e| {
                io::Error::new(
                    e.kind(),
                    format!("Error running launchd command \"{command} {arguments:?}\": {e:?}"),
                )
            })?;

        if !output.status.success() {
            let output = self.decode_output(output.stderr, command, &arguments)?;
            let output_lower = output.to_ascii_lowercase();
            if output_lower.contains(NOT_FOUND) {
                return Ok(output_lower);
            }
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Error running launchd command \"{command} {arguments:?}\": {output}"),
            ));
        }

        let out = self
            .decode_output(output.stdout, command, &arguments)?
            .to_ascii_lowercase();
        Ok(out)
    }

    fn decode_output(
        &self,
        out_bytes: Vec<u8>,
        command: &str,
        arguments: &Vec<&str>,
    ) -> io::Result<String> {
        Ok(String::from_utf8(out_bytes)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Error decoding output from launchd command \"{command} {arguments:?}\": \
                         {e:?}"
                    ),
                )
            })?
            .trim()
            .to_owned())
    }

    async fn domain_target(&self) -> io::Result<String> {
        match self.config.service_level {
            Level::System => Ok("system".to_string()),
            Level::User => {
                let id = self.run_cmd("id", vec!["-u"]).await?;
                Ok(format!("gui/{id}"))
            }
        }
    }

    async fn service_target(&self) -> io::Result<String> {
        let domain_target = self.domain_target().await?;
        Ok(format!("{domain_target}/{}", self.name()))
    }

    fn user_agent_dir(&self) -> io::Result<PathBuf> {
        let user_dirs = directories::UserDirs::new().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "User directories could not be found",
            )
        })?;
        let home_dir = user_dirs.home_dir();
        Ok(home_dir.join("Library").join("LaunchAgents"))
    }

    fn get_plist_path(&self) -> io::Result<PathBuf> {
        let path = match self.config.service_level {
            Level::System => PathBuf::from("/Library/LaunchDaemons"),
            Level::User => self.user_agent_dir()?,
        };
        if !path.exists() {
            std::fs::create_dir_all(&path).map_err(|e| {
                io::Error::new(
                    e.kind(),
                    format!("Error creating plist path {path:#?}: {e:?}"),
                )
            })?;
        }
        Ok(path.join(format!("{}.plist", self.name())))
    }

    fn get_match_or_default<'a>(&self, re: &Regex, output: &'a str) -> Option<&'a str> {
        let captures = re.captures(output);
        let capture = captures?.get(1)?;
        let str_cap: &'a str = capture.as_str();
        Some(str_cap)
    }

    async fn update_autostart(&mut self) -> io::Result<()> {
        let was_started = self.status().await?.state == State::Started;
        if was_started {
            self.stop().await?;
        }
        let plist_path = self.get_plist_path()?;
        let mut config =
            Launchd::from_file(&plist_path).map_err(|e| from_launchd_error(plist_path, e))?;

        config = config.with_run_at_load(self.config.autostart);
        let path = self.get_plist_path()?;
        self.launchctl_bootout().await?;
        let created_file = std::fs::File::create(&path).map_err(|e| {
            io::Error::new(e.kind(), format!("Error creating plist path {path:#?}"))
        })?;
        config
            .to_writer_xml(created_file)
            .map_err(|e| from_launchd_error(&path, e))?;
        self.launchctl_bootstrap().await?;

        if was_started {
            self.start().await?;
        } else {
            self.stop().await?;
        }

        Ok(())
    }

    fn find_pid(&self, output: &str) -> Option<u32> {
        self.get_match_or_default(&PID_RE, output)
            .map(|pid| pid.parse::<u32>().unwrap_or(0))
    }

    async fn launchctl_bootout(&self) -> io::Result<()> {
        let path = self.get_plist_path()?;
        self.run_launchctl(vec![
            "bootout",
            &self.service_target().await?,
            &path.to_string_lossy(),
        ])
        .await?;
        Ok(())
    }

    async fn launchctl_bootstrap(&self) -> io::Result<()> {
        let path = self.get_plist_path()?;
        self.run_launchctl(vec![
            "bootstrap",
            &self.domain_target().await?,
            &path.to_string_lossy(),
        ])
        .await?;
        Ok(())
    }

    async fn launchctl_print(&self) -> io::Result<String> {
        self.run_launchctl(vec!["print", &self.service_target().await?])
            .await
    }

    async fn launchctl_kickstart(&self) -> io::Result<()> {
        self.run_launchctl(vec!["kickstart", &self.service_target().await?])
            .await?;
        Ok(())
    }

    async fn launchctl_restart(&self) -> io::Result<()> {
        self.run_launchctl(vec!["kickstart", "-k", &self.service_target().await?])
            .await?;
        Ok(())
    }

    async fn launchctl_stop(&self) -> io::Result<()> {
        self.run_launchctl(vec!["stop", &self.name()]).await?;
        Ok(())
    }
}

#[async_trait]
impl Manager for LaunchdServiceManager {
    async fn on_config_changed(&mut self) -> io::Result<()> {
        let snapshot = self.config.user_config.snapshot();
        self.config.user_config.reload();
        let current = self.config.user_config.load();
        if current.environment_variables != snapshot.environment_variables {
            self.reload_config().await?;
        }
        Ok(())
    }

    async fn reload_config(&mut self) -> io::Result<()> {
        let current_state = self.status().await?.state;
        self.config.user_config.reload();
        self.stop().await?;
        self.launchctl_bootout().await?;
        self.install().await?;
        if current_state == State::Started {
            self.start().await?;
        }
        Ok(())
    }

    async fn install(&self) -> io::Result<()> {
        let vars = self.config.environment_variables().into_iter().collect();
        let file = Launchd::new(self.name(), self.config.program.full_name())
            .map_err(|e| from_launchd_error(self.config.program.full_name(), e))?
            .with_program_arguments(
                self.config
                    .full_arguments_iter()
                    .map(|a| a.to_owned())
                    .collect(),
            )
            .with_run_at_load(self.config.autostart)
            .with_environment_variables(vars);

        #[cfg(feature = "socket-activation")]
        let file = file.with_socket(Sockets::Dictionary(
            self.config
                .activation_socket_config
                .iter()
                .map(|c| {
                    let mut options = SocketOptions::new();
                    let socket_type = c.socket_type();
                    if socket_type == socket_activation::SocketType::Ipc {
                        options = options
                            .with_family(SocketFamily::Unix)
                            .with_path_name(c.addr())
                            .unwrap();
                    } else {
                        let addr: SocketAddr = c.addr().parse().unwrap();
                        options = options
                            .with_node_name(addr.ip().to_string())
                            .with_service_name(addr.port().to_string());
                    }
                    if socket_type == socket_activation::SocketType::Udp {
                        options = options.with_type(launchd::sockets::SocketType::Dgram);
                    }
                    (c.name().to_owned(), options)
                })
                .collect(),
        ));

        let path = self.get_plist_path()?;
        let created_file = File::create(&path).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("Error creating plist file {path:#?}: {e:?}"),
            )
        })?;
        file.to_writer_xml(created_file)
            .map_err(|e| from_launchd_error(&path, e))?;

        self.launchctl_bootstrap().await?;
        Ok(())
    }

    async fn uninstall(&self) -> io::Result<()> {
        let path = self.get_plist_path()?;
        self.launchctl_bootout().await?;
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                io::Error::new(e.kind(), format!("Error removing plist file {path:?}"))
            })?;
        }

        Ok(())
    }

    async fn start(&self) -> io::Result<()> {
        if self.config.has_sockets() {
            self.launchctl_bootstrap().await?;
        } else {
            self.launchctl_kickstart().await?;
        }

        Ok(())
    }

    async fn stop(&self) -> io::Result<()> {
        self.launchctl_stop().await?;
        if self.config.has_sockets() {
            self.launchctl_bootout().await?;
        }

        Ok(())
    }

    async fn enable_autostart(&mut self) -> io::Result<()> {
        self.config.autostart = true;
        self.update_autostart().await?;
        Ok(())
    }

    async fn disable_autostart(&mut self) -> io::Result<()> {
        self.config.autostart = false;
        self.update_autostart().await?;
        Ok(())
    }

    async fn restart(&self) -> io::Result<()> {
        match self.status().await?.state {
            State::Started => {
                self.launchctl_restart().await?;
            }
            State::Listening => {}
            _ => {
                self.start().await?;
            }
        }

        Ok(())
    }

    async fn status(&self) -> io::Result<Status> {
        let plist_path = self.get_plist_path()?;
        if !plist_path.exists() {
            return Ok(Status {
                state: State::NotInstalled,
                autostart: None,
                pid: None,
                id: None,
                last_exit_code: None,
            });
        }
        let plist =
            Launchd::from_file(&plist_path).map_err(|e| from_launchd_error(plist_path, e))?;
        let output = self.launchctl_print().await?;
        let found = !output.contains(NOT_FOUND);
        if !found && !self.config.has_sockets() {
            return Ok(Status {
                state: State::NotInstalled,
                autostart: None,
                pid: None,
                id: None,
                last_exit_code: None,
            });
        }
        let state = match self.get_match_or_default(&STATE_RE, &output) {
            Some("running") => State::Started,
            _ => {
                if self.config.has_sockets() && found {
                    State::Listening
                } else {
                    State::Stopped
                }
            }
        };

        let pid = self.find_pid(&output);

        // We get the autostart status from the plist file instead of the print command because
        // the format changed sometime between Mac OS 11 and 12 so it seems that it's not very
        // stable. Unfortunately this means we can't detect if the version of the plist file
        // that's actually loaded has autostart or not.
        let autostart = plist.run_at_load;

        let last_exit_code = self
            .get_match_or_default(&EXIT_CODE_RE, &output)
            .map(|code| code.parse::<i32>().unwrap_or(0));

        Ok(Status {
            state,
            pid,
            id: None,
            autostart,
            last_exit_code,
        })
    }

    async fn pid(&self) -> io::Result<Option<u32>> {
        let output = self.launchctl_print().await?;
        Ok(self.find_pid(&output))
    }

    fn display_name(&self) -> &str {
        self.config.display_name()
    }

    fn name(&self) -> String {
        self.config.label.qualified_name()
    }

    fn label(&self) -> &Label {
        &self.config.label
    }

    fn config(&self) -> Config {
        self.config.clone().into()
    }

    fn arguments(&self) -> &Vec<String> {
        &self.config.arguments
    }

    fn description(&self) -> &str {
        &self.config.description
    }

    async fn status_command(&self) -> io::Result<crate::Command> {
        let target = self.service_target().await?;
        Ok(crate::Command {
            program: "launchctl".to_owned(),
            args: vec!["print".to_owned(), target],
        })
    }
}

fn from_launchd_error(path: impl AsRef<Path>, err: launchd::Error) -> io::Error {
    let path = path.as_ref();
    match err {
        launchd::Error::PathConversion => io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Error parsing path {path:#?}"),
        ),
        launchd::Error::Read(e) => match e.as_io() {
            Some(io_err) => io::Error::new(
                io_err.kind(),
                format!("Error reading path {path:#?}: {io_err:?}: {e:?}"),
            ),
            None => io::Error::new(
                io::ErrorKind::Other,
                format!("Error reading path {path:#?}: {e:?}"),
            ),
        },
        launchd::Error::Write(e) => match e.as_io() {
            Some(io_err) => io::Error::new(
                io_err.kind(),
                format!("Error writing path {path:#?}: {io_err:?}: {e:?}"),
            ),
            None => io::Error::new(
                io::ErrorKind::Other,
                format!("Error writing path {path:#?}: {e:?}"),
            ),
        },
        _ => io::Error::new(
            io::ErrorKind::Other,
            format!("Unknown plist error: {path:#?}"),
        ),
    }
}
