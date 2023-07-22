use crate::{
    config::{Builder, Config, Level},
    Info, Manager, State,
};
use daemon_slayer_core::{async_trait, Label};
use launchd::Launchd;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;

macro_rules! regex {
    ($name: ident, $re:literal $(,)?) => {
        static $name: Lazy<Regex> = Lazy::new(|| Regex::new($re).unwrap());
    };
}

regex!(STATE_RE, r"state = (\w+)");
regex!(PID_RE, r"pid = (\w+)");
regex!(EXIT_CODE_RE, r"last exit code = (\w+)");

#[derive(Clone, Debug)]
pub struct LaunchdServiceManager {
    config: Builder,
}

impl LaunchdServiceManager {
    pub(crate) fn from_builder(builder: Builder) -> Result<Self, io::Error> {
        Ok(Self { config: builder })
    }

    async fn run_launchctl(&self, arguments: Vec<&str>) -> Result<String, io::Error> {
        self.run_cmd("launchctl", arguments).await
    }

    async fn run_cmd(&self, command: &str, arguments: Vec<&str>) -> Result<String, io::Error> {
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
                    format!("Error running launchd command \"{command} {arguments:#?}\": {e:?}"),
                )
            })?;

        let out_bytes = if output.status.success() {
            output.stdout
        } else {
            output.stderr
        };
        let out = String::from_utf8(out_bytes)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Error decoding output from launchd command \"{command} {arguments:#?}\": {e:?}"),
                )
            })?
            .trim()
            .to_ascii_lowercase();
        Ok(out)
    }

    async fn service_target(&self) -> Result<String, io::Error> {
        match self.config.service_level {
            Level::System => Ok(format!("system/{}", self.name())),
            Level::User => {
                let id = self.run_cmd("id", vec!["-u"]).await?;
                Ok(format!("gui/{id}/{}", self.name()))
            }
        }
    }

    fn user_agent_dir(&self) -> Result<PathBuf, io::Error> {
        let user_dirs = directories::UserDirs::new().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "User directories could not be found",
            )
        })?;
        let home_dir = user_dirs.home_dir();
        Ok(home_dir.join("Library").join("LaunchAgents"))
    }

    fn get_plist_path(&self) -> Result<PathBuf, io::Error> {
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

    async fn update_autostart(&mut self) -> Result<(), io::Error> {
        let was_started = self.info().await?.state == State::Started;
        if was_started {
            self.stop().await?;
        }
        let plist_path = self.get_plist_path()?;
        let mut config =
            Launchd::from_file(&plist_path).map_err(|e| from_launchd_error(plist_path, e))?;

        config = config.with_run_at_load(self.config.autostart);
        let path = self.get_plist_path()?;
        self.run_launchctl(vec!["unload", &path.to_string_lossy()])
            .await?;
        let created_file = std::fs::File::create(&path).map_err(|e| {
            io::Error::new(e.kind(), format!("Error creating plist path {path:#?}"))
        })?;
        config
            .to_writer_xml(created_file)
            .map_err(|e| from_launchd_error(&path, e))?;
        self.run_launchctl(vec!["load", &path.to_string_lossy()])
            .await?;

        if was_started {
            self.start().await?;
        } else {
            self.stop().await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Manager for LaunchdServiceManager {
    async fn on_config_changed(&mut self) -> Result<(), io::Error> {
        let snapshot = self.config.user_config.snapshot();
        self.config.user_config.reload();
        let current = self.config.user_config.load();
        if current.environment_variables != snapshot.environment_variables {
            self.reload_config().await?;
        }
        Ok(())
    }

    async fn reload_config(&mut self) -> Result<(), io::Error> {
        let current_state = self.info().await?.state;
        self.config.user_config.reload();
        self.stop().await?;
        let path = self.get_plist_path()?;
        self.run_launchctl(vec!["unload", &path.to_string_lossy()])
            .await?;
        self.install().await?;
        if current_state == State::Started {
            self.start().await?;
        }
        Ok(())
    }

    async fn install(&self) -> Result<(), io::Error> {
        let mut file = Launchd::new(self.name(), self.config.program.full_name())
            .map_err(|e| from_launchd_error(self.config.program.full_name(), e))?
            .with_program_arguments(
                self.config
                    .full_arguments_iter()
                    .map(|a| a.to_owned())
                    .collect(),
            )
            .with_run_at_load(self.config.autostart);

        let vars: Vec<(String, String)> = self.config.environment_variables();
        for (key, value) in vars {
            file = file.with_environment_variable(key, value);
        }

        let path = self.get_plist_path()?;
        let created_file = File::create(&path).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("Error creating plist file {path:#?}: {e:?}"),
            )
        })?;
        file.to_writer_xml(created_file)
            .map_err(|e| from_launchd_error(&path, e))?;

        self.run_launchctl(vec!["load", &path.to_string_lossy()])
            .await?;

        Ok(())
    }

    async fn uninstall(&self) -> Result<(), io::Error> {
        let path = self.get_plist_path()?;
        self.run_launchctl(vec!["unload", &path.to_string_lossy()])
            .await?;
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                io::Error::new(e.kind(), format!("Error removing plist file {path:?}"))
            })?;
        }

        Ok(())
    }

    async fn start(&self) -> Result<(), io::Error> {
        self.run_launchctl(vec!["start", &self.name()]).await?;
        Ok(())
    }

    async fn stop(&self) -> Result<(), io::Error> {
        self.run_launchctl(vec!["stop", &self.name()]).await?;
        Ok(())
    }

    async fn enable_autostart(&mut self) -> Result<(), io::Error> {
        self.config.autostart = true;
        self.update_autostart().await?;
        Ok(())
    }

    async fn disable_autostart(&mut self) -> Result<(), io::Error> {
        self.config.autostart = false;
        self.update_autostart().await?;
        Ok(())
    }

    async fn restart(&self) -> Result<(), io::Error> {
        self.run_launchctl(vec!["kickstart", "-k", &self.service_target().await?])
            .await?;
        Ok(())
    }

    async fn info(&self) -> Result<Info, io::Error> {
        let plist_path = self.get_plist_path()?;
        if !plist_path.exists() {
            return Ok(Info {
                label: self.config.label.clone(),
                state: State::NotInstalled,
                autostart: None,
                pid: None,
                id: None,
                last_exit_code: None,
            });
        }
        let plist =
            Launchd::from_file(&plist_path).map_err(|e| from_launchd_error(plist_path, e))?;
        let output = self
            .run_launchctl(vec!["print", &self.service_target().await?])
            .await?;
        if output.contains("could not find service") {
            return Ok(Info {
                label: self.config.label.clone(),
                state: State::NotInstalled,
                autostart: None,
                pid: None,
                id: None,
                last_exit_code: None,
            });
        }
        let state = match self.get_match_or_default(&STATE_RE, &output) {
            Some("running") => State::Started,
            _ => State::Stopped,
        };

        let pid = self
            .get_match_or_default(&PID_RE, &output)
            .map(|pid| pid.parse::<u32>().unwrap_or(0));

        // We get the autostart status from the plist file instead of the print command because
        // the format changed sometime between Mac OS 11 and 12 so it seems that it's not very stable.
        // Unfortunately this means we can't detect if the version of the plist file that's actually loaded
        // has autostart or not.
        let autostart = plist.run_at_load;

        let last_exit_code = self
            .get_match_or_default(&EXIT_CODE_RE, &output)
            .map(|code| code.parse::<i32>().unwrap_or(0));

        Ok(Info {
            label: self.config.label.clone(),
            state,
            pid,
            id: None,
            autostart,
            last_exit_code,
        })
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
